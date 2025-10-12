use crate::DbConn;
use crate::{
    error::{Error, Result},
    models::{
        users::{NewUser, RegisterUser, User},
        requests::{UserWorkspaceRegistrationRequest, UserWorkspaceResult, CreateWorkspaceRequest}
    },
    queries::users,
    services::workspaces,
};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

/// Registers a new user with password validation and hashing
pub async fn register_user(conn: &mut DbConn, register_user: RegisterUser) -> Result<User> {
    // Validate that password and confirm_password match
    if register_user.password != register_user.confirm_password {
        return Err(Error::Validation("Passwords do not match".to_string()));
    }

    // Validate password length (minimum 8 characters)
    if register_user.password.len() < 8 {
        return Err(Error::Validation(
            "Password must be at least 8 characters long".to_string(),
        ));
    }

    // Hash the password using Argon2
    let password_hash = generate_password_hash(&register_user.password)?;

    // Create NewUser struct
    let new_user = NewUser {
        email: register_user.email,
        password_hash,
        full_name: register_user.full_name,
    };

    // Insert user into database
    let user = users::create_user(conn, new_user).await?;

    Ok(user)
}

/// Registers a new user and creates their first workspace in one transaction
pub async fn register_user_with_workspace(conn: &mut DbConn, request: UserWorkspaceRegistrationRequest) -> Result<UserWorkspaceResult> {
    // Validate that password and confirm_password match
    if request.password != request.confirm_password {
        return Err(Error::Validation("Passwords do not match".to_string()));
    }

    // Validate password length (minimum 8 characters)
    if request.password.len() < 8 {
        return Err(Error::Validation(
            "Password must be at least 8 characters long".to_string(),
        ));
    }

    // Validate workspace name is not empty
    if request.workspace_name.trim().is_empty() {
        return Err(Error::Validation("Workspace name cannot be empty".to_string()));
    }

    // Validate workspace name length (maximum 100 characters)
    if request.workspace_name.len() > 100 {
        return Err(Error::Validation(
            "Workspace name must be less than 100 characters".to_string(),
        ));
    }

    // Register the user
    let user = register_user(conn, RegisterUser {
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
    let workspace_result = workspaces::create_workspace(conn, workspace_request).await?;

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