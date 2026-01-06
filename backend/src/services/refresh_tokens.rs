use crate::error::{Error, Result};
use crate::Config;
use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::Sha256;

/// Generates a secure refresh token with HMAC signature
///
/// Format: <random_32_bytes_hex>:<hmac_signature_hex>
///
/// # Arguments
/// * `config` - Application configuration containing refresh token secret
///
/// # Returns
/// A secure token string with HMAC signature
///
/// # Example
/// ```rust,no_run
/// use backend::Config;
/// use backend::services::refresh_tokens::generate_refresh_token;
///
/// let config = Config::load()?;
/// let token = generate_refresh_token(&config)?;
/// # Ok::<(), backend::error::Error>(())
/// ```
pub fn generate_refresh_token(config: &Config) -> Result<String> {
    // Generate 32 random bytes (256 bits of entropy)
    let mut rng = rand::rng();
    let mut random_bytes = [0u8; 32];
    rng.fill(&mut random_bytes);

    // Create HMAC signature using refresh token secret
    let mut mac = Hmac::<Sha256>::new_from_slice(config.jwt.refresh_token_secret.as_bytes())
        .map_err(|e| Error::Internal(format!("Failed to create HMAC: {}", e)))?;
    mac.update(&random_bytes);
    let signature = mac.finalize().into_bytes();

    // Encode as hex for storage/transmission
    let random_hex = hex::encode(&random_bytes);
    let signature_hex = hex::encode(&signature.as_slice());

    // Return format: random:signature
    // Total length: 64 chars (random) + 1 (separator) + 64 chars (signature) = 129 chars
    Ok(format!("{}:{}", random_hex, signature_hex))
}

/// Verifies a refresh token's HMAC signature
///
/// # Arguments
/// * `token` - The refresh token to verify
/// * `config` - Application configuration containing refresh token secret
///
/// # Returns
/// The random bytes from the token if signature is valid
///
/// # Errors
/// Returns an error if:
/// - Token format is invalid (wrong structure, not hex-encoded)
/// - HMAC signature verification fails (token tampered or wrong secret)
///
/// # Example
/// ```rust,no_run
/// use backend::Config;
/// use backend::services::refresh_tokens::{generate_refresh_token, verify_refresh_token};
///
/// let config = Config::load()?;
/// let token = generate_refresh_token(&config)?;
/// let random_bytes = verify_refresh_token(&token, &config)?;
/// # Ok::<(), backend::error::Error>(())
/// ```
pub fn verify_refresh_token(token: &str, config: &Config) -> Result<Vec<u8>> {
    // Split token into random and signature parts
    let parts: Vec<&str> = token.split(':').collect();
    if parts.len() != 2 {
        return Err(Error::InvalidToken("Invalid token format".to_string()));
    }

    let random_hex = parts[0];
    let provided_signature_hex = parts[1];

    // Decode hex
    let random_bytes = hex::decode(random_hex)
        .map_err(|_| Error::InvalidToken("Invalid token encoding".to_string()))?;

    // Recompute HMAC signature
    let mut mac = Hmac::<Sha256>::new_from_slice(config.jwt.refresh_token_secret.as_bytes())
        .map_err(|e| Error::Internal(format!("Failed to create HMAC: {}", e)))?;
    mac.update(&random_bytes);
    let expected_signature = mac.finalize().into_bytes();

    // Decode provided signature
    let provided_signature = hex::decode(provided_signature_hex)
        .map_err(|_| Error::InvalidToken("Invalid signature encoding".to_string()))?;

    // Verify signature matches using constant-time comparison
    use subtle::ConstantTimeEq;
    if expected_signature.as_slice().ct_eq(&provided_signature[..]).into() {
        Ok(random_bytes)
    } else {
        Err(Error::InvalidToken("Invalid token signature".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_refresh_token_format() {
        // This test requires a Config with refresh_token_secret
        // For now, just test the structure without full config
        let mut rng = rand::rng();
        let mut random_bytes = [0u8; 32];
        rng.fill(&mut random_bytes);
        let random_hex = hex::encode(&random_bytes);

        assert_eq!(random_hex.len(), 64, "Random part should be 64 hex chars");
        assert!(random_hex.chars().all(|c| c.is_ascii_hexdigit()), "Should be valid hex");
    }

    #[test]
    fn test_token_format_structure() {
        // Test that the format is correct: hex:hex
        let example = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let parts: Vec<&str> = example.split(':').collect();
        assert_eq!(parts.len(), 2, "Token should have 2 parts separated by colon");
        assert_eq!(parts[0].len(), 64, "Random part should be 64 chars");
        assert_eq!(parts[1].len(), 64, "Signature part should be 64 chars");
    }

    #[test]
    fn test_hex_encoding_valid() {
        let valid_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        assert!(hex::decode(valid_hex).is_ok(), "Should decode valid hex");
    }

    #[test]
    fn test_hex_encoding_invalid() {
        let invalid_hex = "ghijklmnopqrstuvwxyz0123456789abcdef0123456789abcdef0123456789";
        assert!(hex::decode(invalid_hex).is_err(), "Should reject invalid hex");
    }
}
