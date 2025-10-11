use crate::DbConn;
use crate::{
    error::{Error, Result},
    models::users::{NewUser, RegisterUser, User},
    queries::users,
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
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(register_user.password.as_bytes(), &salt)
        .map_err(|e| Error::Validation(format!("Failed to hash password: {}", e)))?
        .to_string();

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
