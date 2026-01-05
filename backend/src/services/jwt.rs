use crate::error::{Error, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject - user_id as string
    pub sub: String,
    /// Expiration time as Unix timestamp
    pub exp: i64,
    /// Issued at time as Unix timestamp
    pub iat: i64,
}

/// Generates a JWT access token for a user
///
/// # Arguments
/// * `user_id` - The user's UUID
/// * `secret` - The JWT secret key for signing
/// * `expiration_minutes` - Token expiration time in minutes (from config)
///
/// # Returns
/// A JWT token string
///
/// # Example
/// ```rust,no_run
/// use backend::services::jwt::generate_jwt;
/// use uuid::Uuid;
///
/// let user_id = Uuid::now_v7();
/// let token = generate_jwt(user_id, "my-secret", 15)?;
/// # Ok::<(), backend::error::Error>(())
/// ```
pub fn generate_jwt(user_id: Uuid, secret: &str, expiration_minutes: i64) -> Result<String> {
    let now = Utc::now();
    let expiration = now + Duration::minutes(expiration_minutes);

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration.timestamp(),
        iat: now.timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .map_err(|e| Error::Internal(format!("Failed to generate JWT: {}", e)))
}

/// Verifies a JWT token and returns the claims if valid
///
/// # Arguments
/// * `token` - The JWT token string
/// * `secret` - The JWT secret key for verification
///
/// # Returns
/// The decoded claims if the token is valid
///
/// # Errors
/// Returns an error if the token is invalid, expired, or has a bad signature
///
/// # Example
/// ```rust,no_run
/// use backend::services::jwt::{generate_jwt, verify_jwt};
/// use uuid::Uuid;
///
/// let user_id = Uuid::now_v7();
/// let token = generate_jwt(user_id, "my-secret", 15)?;
/// let claims = verify_jwt(&token, "my-secret")?;
/// # Ok::<(), backend::error::Error>(())
/// ```
pub fn verify_jwt(token: &str, secret: &str) -> Result<Claims> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )
    .map_err(|e| {
        // Check error kind to provide better error messages
        let error_msg = e.to_string().to_lowercase();
        if error_msg.contains("expired") {
            Error::Authentication("Token has expired".to_string())
        } else if error_msg.contains("signature") {
            Error::Authentication("Invalid token signature".to_string())
        } else {
            Error::Authentication(format!("Invalid token: {}", e))
        }
    })?;

    Ok(token_data.claims)
}

/// Extracts user_id from a valid JWT token
///
/// # Arguments
/// * `token` - The JWT token string
/// * `secret` - The JWT secret key for verification
///
/// # Returns
/// The user's UUID
///
/// # Example
/// ```rust,no_run
/// use backend::services::jwt::{generate_jwt, get_user_id_from_token};
/// use uuid::Uuid;
///
/// let user_id = Uuid::now_v7();
/// let token = generate_jwt(user_id, "my-secret", 15)?;
/// let extracted_id = get_user_id_from_token(&token, "my-secret")?;
/// # Ok::<(), backend::error::Error>(())
/// ```
pub fn get_user_id_from_token(token: &str, secret: &str) -> Result<Uuid> {
    let claims = verify_jwt(token, secret)?;
    Uuid::parse_str(&claims.sub)
        .map_err(|_| Error::Internal("Invalid user_id in token".to_string()))
}

/// Validates JWT from Authorization header and returns user_id
/// Format: "Authorization: Bearer <token>"
///
/// # Arguments
/// * `auth_header` - The Authorization header value (optional)
/// * `secret` - The JWT secret key for verification
///
/// # Returns
/// The user's UUID if the token is valid
///
/// # Example
/// ```rust,no_run
/// use backend::services::jwt::{generate_jwt, authenticate_jwt_token};
/// use uuid::Uuid;
///
/// let user_id = Uuid::now_v7();
/// let token = generate_jwt(user_id, "my-secret", 15)?;
/// let auth_header = format!("Bearer {}", token);
/// let extracted_id = authenticate_jwt_token(Some(&auth_header), "my-secret")?;
/// # Ok::<(), backend::error::Error>(())
/// ```
pub fn authenticate_jwt_token(auth_header: Option<&str>, secret: &str) -> Result<Uuid> {
    let token = extract_token_from_header(auth_header)?;
    get_user_id_from_token(&token, secret)
}

/// Extracts the Bearer token from the Authorization header
///
/// # Arguments
/// * `auth_header` - The Authorization header value (optional)
///
/// # Returns
/// The extracted token string
fn extract_token_from_header(auth_header: Option<&str>) -> Result<String> {
    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = header[7..].to_string();
            if token.is_empty() {
                return Err(Error::Authentication("Empty token".to_string()));
            }
            Ok(token)
        }
        Some(_) => Err(Error::Authentication(
            "Invalid Authorization header format. Expected: 'Bearer <token>'".to_string()
        )),
        None => Err(Error::Authentication("Missing Authorization header".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_jwt() {
        let user_id = Uuid::now_v7();
        let secret = "test-secret-key-for-testing";
        let expiration_minutes = 15;
        let token = generate_jwt(user_id, secret, expiration_minutes).unwrap();
        assert!(!token.is_empty());
        assert!(token.contains('.'));
    }

    #[test]
    fn test_verify_jwt_valid() {
        let user_id = Uuid::now_v7();
        let secret = "test-secret-key-for-testing";
        let expiration_minutes = 15;
        let token = generate_jwt(user_id, secret, expiration_minutes).unwrap();
        let claims = verify_jwt(&token, secret).unwrap();
        assert_eq!(claims.sub, user_id.to_string());
    }

    #[test]
    fn test_verify_jwt_invalid_signature() {
        let user_id = Uuid::now_v7();
        let secret = "test-secret-key-for-testing";
        let expiration_minutes = 15;
        let token = generate_jwt(user_id, secret, expiration_minutes).unwrap();
        let result = verify_jwt(&token, "wrong-secret");
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_jwt_invalid_format() {
        let token = "invalid.token.here";
        let secret = "test-secret-key-for-testing";
        let result = verify_jwt(token, secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_user_id_from_token() {
        let user_id = Uuid::now_v7();
        let secret = "test-secret-key-for-testing";
        let expiration_minutes = 15;
        let token = generate_jwt(user_id, secret, expiration_minutes).unwrap();
        let extracted_id = get_user_id_from_token(&token, secret).unwrap();
        assert_eq!(extracted_id, user_id);
    }

    #[test]
    fn test_extract_token_from_header_valid() {
        let token = "my-jwt-token";
        let header = format!("Bearer {}", token);
        let extracted = extract_token_from_header(Some(&header)).unwrap();
        assert_eq!(extracted, token);
    }

    #[test]
    fn test_extract_token_from_header_missing() {
        let result = extract_token_from_header(None);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_token_from_header_invalid_format() {
        let result = extract_token_from_header(Some("InvalidFormat"));
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_token_from_header_empty() {
        let result = extract_token_from_header(Some("Bearer "));
        assert!(result.is_err());
    }
}
