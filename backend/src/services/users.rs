use crate::DbConn;
use crate::{
    error::{Error, Result},
    models::{
        users::{LoginUser, LoginResult, NewUser, NewUserSession, RegisterUser, User},
        requests::{UserWorkspaceRegistrationRequest, UserWorkspaceResult, CreateWorkspaceRequest}
    },
    queries::{users, sessions},
    services::workspaces,
    validation::{validate_email, validate_password, validate_full_name, validate_session_token, validate_required_string},
};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::{Duration, Utc};
use sqlx::Acquire;
use uuid::Uuid;

/// Registers a new user with comprehensive validation and password hashing
pub async fn register_user(conn: &mut DbConn, register_user: RegisterUser) -> Result<User> {
    // Validate email format
    validate_email(&register_user.email)?;

    // Validate that password and confirm_password match
    if register_user.password != register_user.confirm_password {
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
        password_hash,
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

    if request.password != request.confirm_password {
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
    let is_valid = verify_password(&login_user.password, &user.password_hash)?;
    if !is_valid {
        return Err(Error::Authentication("Invalid email or password".to_string()));
    }

    // Generate secure session token
    let session_token = generate_session_token()?;

    // Set session expiration (7 days from now)
    let expires_at = Utc::now() + Duration::hours(168);

    // Create session
    let new_session = NewUserSession {
        user_id: user.id,
        token: session_token.clone(),
        expires_at,
    };

    let session = sessions::create_session(conn, new_session).await?;

    Ok(LoginResult {
        user,
        session_token: session.token,
        expires_at: session.expires_at,
    })
}

/// Validates a session token and returns the associated user
pub async fn validate_session(conn: &mut DbConn, session_token: &str) -> Result<User> {
    // Validate session token format
    validate_session_token(session_token)?;

    // Get valid session by token
    let sanitized_token = validate_required_string(session_token, "Session token")?;
    let session = sessions::get_valid_session_by_token(conn, &sanitized_token).await?
        .ok_or_else(|| Error::InvalidToken("Invalid or expired session token".to_string()))?;

    // Get user by session user_id
    let user = users::get_user_by_id(conn, session.user_id).await?;

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

    if hours_to_extend > 168 { // Max 7 days
        return Err(Error::Validation("Cannot extend session by more than 168 hours (7 days)".to_string()));
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

/// Generates a secure session token using UUID v7
pub fn generate_session_token() -> Result<String> {
    let token = Uuid::now_v7().to_string();
    Ok(token)
}

/// Gets a user by their ID, returns None if not found
pub async fn get_user_by_id(conn: &mut DbConn, user_id: Uuid) -> Result<Option<User>> {
    // Use existing query function
    match users::get_user_by_id(conn, user_id).await {
        Ok(user) => Ok(Some(user)),
        Err(crate::error::Error::Sqlx(sqlx::Error::RowNotFound)) => Ok(None),
        Err(e) => Err(e),
    }
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