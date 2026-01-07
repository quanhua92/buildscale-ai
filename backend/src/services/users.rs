use crate::{Config, DbConn};
use crate::{
    error::{Error, Result},
    models::{
        users::{LoginUser, LoginResult, NewUser, NewUserSession, RefreshTokenResult, RegisterUser, User},
        requests::{UserWorkspaceRegistrationRequest, UserWorkspaceResult, CreateWorkspaceRequest}
    },
    queries::{users, sessions},
    services::{jwt, workspaces},
    validation::{validate_email, validate_password, validate_full_name, validate_session_token, validate_required_string},
};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::{Duration, Utc};
use sqlx::Acquire;
use subtle::ConstantTimeEq;
use uuid::Uuid;

/// Registers a new user with comprehensive validation and password hashing
pub async fn register_user(conn: &mut DbConn, register_user: RegisterUser) -> Result<User> {
    // Validate email format
    validate_email(&register_user.email)?;

    // Use constant-time comparison for password confirmation to prevent timing attacks
    if register_user.password.len() != register_user.confirm_password.len() {
        return Err(Error::Validation("Passwords do not match".to_string()));
    }

    let password_bytes = register_user.password.as_bytes();
    let confirm_bytes = register_user.confirm_password.as_bytes();

    // subtle's ct_eq returns Choice(1) if equal, Choice(0) if not equal
    if password_bytes.ct_eq(confirm_bytes).unwrap_u8() == 0 {
        return Err(Error::Validation("Passwords do not match".to_string()));
    }

    // Validate password strength
    validate_password(&register_user.password)?;

    // Validate full name format
    validate_full_name(&register_user.full_name)?;

    // Hash the password using Argon2
    let password_hash = generate_password_hash(&register_user.password)?;

    // Create NewUser struct with sanitized email
    let new_user = NewUser {
        email: validate_required_string(&register_user.email, "Email")?.to_lowercase(),
        password_hash: Some(password_hash),
        full_name: register_user.full_name.map(|name| validate_required_string(&name, "Full name")).transpose()?,
    };

    // Insert user into database
    let user = users::create_user(conn, new_user).await?;

    Ok(user)
}

/// Registers a new user and creates their first workspace in one transaction
///
/// This operation ensures atomicity - both user creation and workspace creation
/// succeed together, or both fail together. No orphaned users or incomplete
/// workspaces are left behind.
///
/// Comprehensive validation is performed before starting the transaction
/// to fail fast on invalid input.
pub async fn register_user_with_workspace(conn: &mut DbConn, request: UserWorkspaceRegistrationRequest) -> Result<UserWorkspaceResult> {
    // Validate all input first before starting transaction
    validate_email(&request.email)?;

    // Use constant-time comparison for password confirmation to prevent timing attacks
    if request.password.len() != request.confirm_password.len() {
        return Err(Error::Validation("Passwords do not match".to_string()));
    }

    let password_bytes = request.password.as_bytes();
    let confirm_bytes = request.confirm_password.as_bytes();

    // subtle's ct_eq returns Choice(1) if equal, Choice(0) if not equal
    if password_bytes.ct_eq(confirm_bytes).unwrap_u8() == 0 {
        return Err(Error::Validation("Passwords do not match".to_string()));
    }

    validate_password(&request.password)?;
    validate_full_name(&request.full_name)?;

    // Import workspace validation function
    use crate::validation::validate_workspace_name;
    validate_workspace_name(&request.workspace_name)?;

    // Start a transaction for atomic user + workspace creation
    let mut tx = conn.begin().await.map_err(|e| {
        Error::Internal(format!("Failed to begin transaction: {}", e))
    })?;

    // Register the user within the transaction
    let user = register_user(&mut tx, RegisterUser {
        email: request.email,
        password: request.password,
        confirm_password: request.confirm_password,
        full_name: request.full_name,
    }).await?;

    // Create the user's first workspace with default roles and owner as admin
    let workspace_request = CreateWorkspaceRequest {
        name: request.workspace_name,
        owner_id: user.id,
    };
    let workspace_result = workspaces::create_workspace(&mut tx, workspace_request).await?;

    // Commit the transaction - both user and workspace are now persisted atomically
    tx.commit().await.map_err(|e| {
        Error::Internal(format!("Failed to commit transaction: {}", e))
    })?;

    Ok(UserWorkspaceResult {
        user,
        workspace: workspace_result,
    })
}

/// Generates a password hash using Argon2
pub fn generate_password_hash(password: &str) -> Result<String> {
    // Hash the password using Argon2
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| Error::Validation(format!("Failed to hash password: {}", e)))?
        .to_string();

    Ok(password_hash)
}

/// Verifies a password against a password hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| Error::Validation(format!("Invalid password hash: {}", e)))?;

    let argon2 = Argon2::default();

    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(Error::Validation(format!(
            "Password verification failed: {}",
            e
        ))),
    }
}

/// Logs in a user with comprehensive email and password validation
pub async fn login_user(conn: &mut DbConn, login_user: LoginUser) -> Result<LoginResult> {
    // Validate email format
    validate_email(&login_user.email)?;

    // Validate password is not empty (password strength check not needed for login)
    if login_user.password.is_empty() {
        return Err(Error::Validation("Password cannot be empty".to_string()));
    }

    // Find user by email (case-insensitive, sanitized)
    let sanitized_email = validate_required_string(&login_user.email, "Email")?.to_lowercase();
    let user = users::get_user_by_email(conn, &sanitized_email).await?
        .ok_or_else(|| Error::Authentication("Invalid email or password".to_string()))?;

    // Verify password
    let is_valid = match &user.password_hash {
        Some(hash) => verify_password(&login_user.password, hash)?,
        None => false, // OAuth-only users cannot login with password
    };
    if !is_valid {
        return Err(Error::Authentication("Invalid email or password".to_string()));
    }

    // Load config
    let config = Config::load()?;

    // Generate JWT access token (short-lived, 15 minutes by default)
    let access_token = jwt::generate_jwt(
        user.id,
        &config.jwt.secret,
        config.jwt.access_token_expiration_minutes,
    )?;
    let access_token_expires_at = Utc::now() + Duration::minutes(config.jwt.access_token_expiration_minutes);

    // Generate refresh token (session token, long-lived, 30 days by default)
    let refresh_token = generate_session_token()?;
    let refresh_token_expires_at = Utc::now() + Duration::hours(config.sessions.expiration_hours);

    // Create session (for refresh token)
    let new_session = NewUserSession {
        user_id: user.id,
        token: refresh_token.clone(),
        expires_at: refresh_token_expires_at,
    };

    let session = sessions::create_session(conn, new_session).await?;

    Ok(LoginResult {
        user,
        access_token,
        refresh_token: session.token,
        access_token_expires_at,
        refresh_token_expires_at: session.expires_at,
    })
}

/// Validates a session token and returns the associated user
pub async fn validate_session(conn: &mut DbConn, session_token: &str) -> Result<User> {
    // Verify HMAC signature first (fast fail before DB lookup)
    let config = Config::load()?;
    crate::services::refresh_tokens::verify_refresh_token(session_token, &config)?;

    // Validate session token format
    validate_session_token(session_token)?;

    // Get valid session by token
    let sanitized_token = validate_required_string(session_token, "Session token")?;
    let session = sessions::get_valid_session_by_token(conn, &sanitized_token).await?
        .ok_or_else(|| Error::InvalidToken("Invalid or expired session token".to_string()))?;

    // Get user by session user_id
    let user = users::get_user_by_id(conn, session.user_id).await?
        .ok_or_else(|| Error::InvalidToken("User not found".to_string()))?;

    Ok(user)
}

/// Logs out a user by invalidating their session token
pub async fn logout_user(conn: &mut DbConn, session_token: &str) -> Result<()> {
    // Validate session token format
    validate_session_token(session_token)?;

    // Delete session by token
    let sanitized_token = validate_required_string(session_token, "Session token")?;
    let rows_affected = sessions::delete_session_by_token(conn, &sanitized_token).await?;

    if rows_affected == 0 {
        return Err(Error::InvalidToken("Invalid session token".to_string()));
    }

    Ok(())
}

/// Refreshes a session by extending its expiration time
pub async fn refresh_session(conn: &mut DbConn, session_token: &str, hours_to_extend: i64) -> Result<String> {
    // Validate session token format
    validate_session_token(session_token)?;

    // Validate hours to extend
    if hours_to_extend <= 0 {
        return Err(Error::Validation("Hours to extend must be positive".to_string()));
    }

    // Load config to get max extension time
    let config = Config::load()?;
    if hours_to_extend > config.sessions.expiration_hours {
        return Err(Error::Validation(format!(
            "Cannot extend session by more than {} hours",
            config.sessions.expiration_hours
        )));
    }

    // Get current session
    let sanitized_token = validate_required_string(session_token, "Session token")?;
    let session = sessions::get_session_by_token(conn, &sanitized_token).await?
        .ok_or_else(|| Error::InvalidToken("Invalid session token".to_string()))?;

    // Check if session is expired
    if session.expires_at < Utc::now() {
        return Err(Error::SessionExpired("Session has expired".to_string()));
    }

    // Calculate new expiration time
    let new_expires_at = Utc::now() + Duration::hours(hours_to_extend);

    // Update session
    let updated_session = sessions::refresh_session(conn, session.id, new_expires_at).await?;

    Ok(updated_session.token)
}

/// Refreshes the access token using a valid refresh token (session)
///
/// This function validates the refresh token (session) and generates a new access token (JWT)
/// without requiring the user to log in again with their credentials.
///
/// # Arguments
/// * `conn` - Database connection
/// * `refresh_token` - The refresh token (session token)
///
/// # Returns
/// A RefreshTokenResult containing the new access token and expiration time
///
/// # Errors
/// Returns an error if the refresh token is invalid or expired
pub async fn refresh_access_token(
    conn: &mut DbConn,
    refresh_token: &str,
) -> Result<RefreshTokenResult> {
    // Validate the refresh token (session) exists and is not expired
    let session = sessions::get_valid_session_by_token(conn, refresh_token)
        .await?
        .ok_or_else(|| Error::InvalidToken("Invalid or expired refresh token".to_string()))?;

    // Load config to get JWT secret and expiration
    let config = Config::load()?;

    // Generate new access token (JWT)
    let access_token = jwt::generate_jwt(
        session.user_id,
        &config.jwt.secret,
        config.jwt.access_token_expiration_minutes,
    )?;

    let expires_at = Utc::now() + Duration::minutes(config.jwt.access_token_expiration_minutes);

    Ok(RefreshTokenResult {
        access_token,
        expires_at,
    })
}

/// Generates a secure refresh token using HMAC-signed random bytes
pub fn generate_session_token() -> Result<String> {
    let config = Config::load()?;
    crate::services::refresh_tokens::generate_refresh_token(&config)
}

/// Updates a user's password with validation
pub async fn update_password(conn: &mut DbConn, user_id: Uuid, new_password: &str) -> Result<()> {
    // Validate password length (minimum 8 characters)
    if new_password.len() < 8 {
        return Err(Error::Validation("Password must be at least 8 characters long".to_string()));
    }

    // Hash the password using existing utility
    let password_hash = generate_password_hash(new_password)?;

    // Update password using existing query function
    users::update_user_password(conn, user_id, &password_hash).await
}

/// Gets session information without user validation
pub async fn get_session_info(conn: &mut DbConn, session_token: &str) -> Result<Option<crate::models::users::UserSession>> {
    // Validate token format
    if session_token.trim().is_empty() {
        return Err(Error::Validation("Session token cannot be empty".to_string()));
    }

    // Use existing query function
    sessions::get_session_by_token(conn, session_token.trim()).await
        .map_err(|e| e.into())
}

/// Checks if an email is available for registration
pub async fn is_email_available(conn: &mut DbConn, email: &str) -> Result<bool> {
    // Validate email format using comprehensive validation
    validate_email(email)?;

    // Check if user exists using existing query function (case-insensitive)
    let sanitized_email = validate_required_string(email, "Email")?.to_lowercase();
    let existing_user = users::get_user_by_email(conn, &sanitized_email).await?;
    Ok(existing_user.is_none())
}

/// Gets all active sessions for a user
pub async fn get_user_active_sessions(conn: &mut DbConn, user_id: Uuid) -> Result<Vec<crate::models::users::UserSession>> {
    crate::services::sessions::get_user_active_sessions(conn, user_id).await
        .map_err(|e| e.into())
}

/// Revokes all sessions for a user
pub async fn revoke_all_user_sessions(conn: &mut DbConn, user_id: Uuid) -> Result<u64> {
    crate::services::sessions::revoke_all_user_sessions(conn, user_id).await
        .map_err(|e| e.into())
}