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
use secrecy::ExposeSecret;
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
        config.jwt.secret.expose_secret(),
        config.jwt.access_token_expiration_minutes,
    )?;
    let access_token_expires_at = Utc::now() + Duration::minutes(config.jwt.access_token_expiration_minutes);

    // Generate refresh token (session token, long-lived, 30 days by default)
    let refresh_token = generate_session_token()?;
    let refresh_token_expires_at = Utc::now() + Duration::hours(config.sessions.expiration_hours);

    // Hash the token for database storage
    let token_hash = sessions::hash_session_token(&refresh_token);

    // Create session (for refresh token)
    let new_session = NewUserSession {
        user_id: user.id,
        token_hash,
        expires_at: refresh_token_expires_at,
    };

    let session = sessions::create_session(conn, new_session).await?;

    Ok(LoginResult {
        user,
        access_token,
        refresh_token,  // Return unhashed token to client
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

    // Hash the token for database lookup
    let sanitized_token = validate_required_string(session_token, "Session token")?;
    let token_hash = sessions::hash_session_token(&sanitized_token);

    // Get valid session by token hash
    let session = sessions::get_valid_session_by_token_hash(conn, &token_hash).await?
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

    // Delete session by token hash
    let sanitized_token = validate_required_string(session_token, "Session token")?;
    let token_hash = sessions::hash_session_token(&sanitized_token);
    let rows_affected = sessions::delete_session_by_token_hash(conn, &token_hash).await?;

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

    // Get current session by hashing token
    let sanitized_token = validate_required_string(session_token, "Session token")?;
    let token_hash = sessions::hash_session_token(&sanitized_token);
    let session = sessions::get_session_by_token_hash(conn, &token_hash).await?
        .ok_or_else(|| Error::InvalidToken("Invalid session token".to_string()))?;

    // Check if session is expired
    if session.expires_at < Utc::now() {
        return Err(Error::SessionExpired("Session has expired".to_string()));
    }

    // Calculate new expiration time
    let new_expires_at = Utc::now() + Duration::hours(hours_to_extend);

    // Update session
    let _updated_session = sessions::refresh_session(conn, session.id, new_expires_at).await?;

    Ok(session_token.to_string())  // Return original token
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
/// A RefreshTokenResult containing the new access token, rotated refresh token, and expiration time
///
/// # Token Rotation
/// Each refresh request generates a NEW refresh token and invalidates the old one.
/// This is an OAuth 2.0 security best practice that prevents token theft replay attacks.
///
/// # Errors
/// Returns an error if the refresh token is invalid or expired
pub async fn refresh_access_token(
    conn: &mut DbConn,
    refresh_token: &str,
) -> Result<RefreshTokenResult> {
    let old_token_hash = sessions::hash_session_token(refresh_token);

    // STEP 1: Check if token was revoked (THEFT DETECTION with grace period)
    if let Some(revoked) = sessions::get_revoked_token(conn, &old_token_hash).await? {
        // Calculate how long ago the token was revoked
        let time_since_revocation = Utc::now() - revoked.revoked_at;

        // Grace period: 5 minutes
        // - Allows legitimate double-clicks/retries without error
        // - Still detects theft after grace period expires
        if time_since_revocation.num_minutes() >= 5 {
            // TOKEN THEFT DETECTED: Old token used after grace period!
            // Revoke ALL sessions for this user immediately
            tracing::warn!(
                user_id = %revoked.user_id,
                security_event = "token_theft_detected",
                "Potential security breach: Old refresh token used after rotation. Revoking all sessions."
            );

            let _ = sessions::delete_sessions_by_user(conn, revoked.user_id).await?;

            // Clean up revoked token records
            let _ = sessions::delete_revoked_tokens_by_user(conn, revoked.user_id).await?;

            return Err(Error::TokenTheftDetected(
                "Potential security breach detected. Your refresh token was used after rotation. All sessions have been revoked for your protection. Please login again and consider changing your password.".to_string()
            ));
        }

        // Within grace period: This is likely a legitimate double-click or retry
        // Generate new access token without rotating refresh token (transparent to client)
        tracing::info!(
            user_id = %revoked.user_id,
            security_event = "token_reused_within_grace_period",
            "Refresh token reused within grace period ({} minutes old), returning access token",
            time_since_revocation.num_minutes()
        );

        // Load config and generate new access token
        let config = Config::load()?;
        let access_token = jwt::generate_jwt(
            revoked.user_id,
            config.jwt.secret.expose_secret(),
            config.jwt.access_token_expiration_minutes,
        )?;

        let expires_at = Utc::now() + Duration::minutes(config.jwt.access_token_expiration_minutes);

        // Return success with access token only (no refresh token rotation)
        return Ok(RefreshTokenResult {
            access_token,
            refresh_token: None,  // None - client should keep using their current token
            expires_at,
        });
    }

    // STEP 2: Validate the refresh token (session) exists and is not expired
    let session = sessions::get_valid_session_by_token_hash(conn, &old_token_hash)
        .await?
        .ok_or_else(|| Error::InvalidToken("Invalid or expired refresh token".to_string()))?;

    // Load config
    let config = Config::load()?;

    // STEP 3: Generate NEW refresh token (rotation)
    let new_refresh_token = generate_session_token()?;
    let new_token_hash = sessions::hash_session_token(&new_refresh_token);

    // STEP 4: Start transaction for atomic token rotation
    // CRITICAL: Recording revoked token + updating session must be atomic
    let mut tx = conn.begin().await.map_err(|e| {
        Error::Internal(format!("Failed to begin transaction: {}", e))
    })?;

    // STEP 5: Record old token as revoked BEFORE updating session
    // CRITICAL: If two requests race, the second one will detect theft
    let _ = sessions::create_revoked_token(&mut tx, session.user_id, &old_token_hash).await?;

    // STEP 6: Update session with new token hash (invalidates old token)
    let _updated_session = sessions::update_session_token_hash(
        &mut tx,
        session.id,
        &new_token_hash,
    ).await?;

    // STEP 7: Commit transaction (atomic operation complete)
    tx.commit().await.map_err(|e| {
        Error::Internal(format!("Failed to commit transaction: {}", e))
    })?;

    // Log successful token rotation (for audit trail)
    tracing::info!(
        user_id = %session.user_id,
        security_event = "token_rotated",
        "Refresh token rotated successfully"
    );

    // STEP 8: Generate new access token (JWT)
    let access_token = jwt::generate_jwt(
        session.user_id,
        config.jwt.secret.expose_secret(),
        config.jwt.access_token_expiration_minutes,
    )?;

    let expires_at = Utc::now() + Duration::minutes(config.jwt.access_token_expiration_minutes);

    Ok(RefreshTokenResult {
        access_token,
        refresh_token: Some(new_refresh_token),  // Some during normal rotation
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

    // Hash the token for database lookup
    let token_hash = sessions::hash_session_token(session_token.trim());

    // Use existing query function
    sessions::get_session_by_token_hash(conn, &token_hash).await
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