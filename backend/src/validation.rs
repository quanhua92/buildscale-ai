//! Input validation utilities for the backend service layer.
//!
//! This module provides comprehensive validation functions for user input,
//! ensuring data integrity and security across all service operations.

use crate::error::{Error, Result};

/// Validates email format using comprehensive checks
///
/// # Arguments
/// * `email` - The email address to validate
///
/// # Returns
/// * `Ok(())` if the email is valid
/// * `Err(Error)` with descriptive message if invalid
///
/// # Examples
/// ```
/// use backend::validation::validate_email;
/// use backend::error::Error;
///
/// validate_email("user@example.com").unwrap(); // Valid
/// assert!(validate_email("invalid-email").is_err()); // Returns Error
/// ```
pub fn validate_email(email: &str) -> Result<()> {
    let email = email.trim();

    // Basic format validation
    if email.is_empty() {
        return Err(Error::Validation("Email cannot be empty".to_string()));
    }

    // Length validation
    if email.len() > 254 {
        return Err(Error::Validation("Email address is too long (max 254 characters)".to_string()));
    }

    // Check for basic structure
    if !email.contains('@') || email.starts_with('@') || email.ends_with('@') {
        return Err(Error::Validation("Invalid email format: must contain @ symbol not at start or end".to_string()));
    }

    // Split into local and domain parts
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return Err(Error::Validation("Invalid email format: must contain exactly one @ symbol".to_string()));
    }

    let (local_part, domain) = (parts[0], parts[1]);

    // Validate local part
    if local_part.is_empty() {
        return Err(Error::Validation("Invalid email format: local part cannot be empty".to_string()));
    }

    if local_part.len() > 64 {
        return Err(Error::Validation("Invalid email format: local part is too long (max 64 characters)".to_string()));
    }

    // Validate domain part
    if domain.is_empty() {
        return Err(Error::Validation("Invalid email format: domain part cannot be empty".to_string()));
    }

    if domain.len() > 253 {
        return Err(Error::Validation("Invalid email format: domain is too long (max 253 characters)".to_string()));
    }

    // Check domain has at least one dot
    if !domain.contains('.') {
        return Err(Error::Validation("Invalid email format: domain must contain at least one dot".to_string()));
    }

    // Check for consecutive dots
    if email.contains("..") {
        return Err(Error::Validation("Invalid email format: cannot contain consecutive dots".to_string()));
    }

    // Check for invalid characters including spaces
    let invalid_chars = ['<', '>', '(', ')', '[', ']', '\\', ',', ';', ':', '"', ' '];
    for char in invalid_chars.iter() {
        if email.contains(*char) {
            return Err(Error::Validation(format!("Invalid email format: cannot contain '{}'", char)));
        }
    }

    Ok(())
}

/// Validates password strength and format
///
/// # Arguments
/// * `password` - The password to validate
///
/// # Returns
/// * `Ok(())` if the password meets requirements
/// * `Err(Error)` with descriptive message if invalid
pub fn validate_password(password: &str) -> Result<()> {
    // Length validation
    if password.len() < 8 {
        return Err(Error::Validation("Password must be at least 8 characters long".to_string()));
    }

    if password.len() > 128 {
        return Err(Error::Validation("Password is too long (max 128 characters)".to_string()));
    }

    // Check for common weak patterns
    if password.to_lowercase() == "password"
        || password.to_lowercase() == "12345678"
        || password.to_lowercase() == "qwerty123"
        || password.to_lowercase() == "admin123" {
        return Err(Error::Validation("Password is too common and weak".to_string()));
    }

    // Check for whitespace
    if password.contains(' ') {
        return Err(Error::Validation("Password cannot contain spaces".to_string()));
    }

    Ok(())
}

/// Validates workspace name format and constraints
///
/// # Arguments
/// * `name` - The workspace name to validate
///
/// # Returns
/// * `Ok(())` if the name is valid
/// * `Err(Error)` with descriptive message if invalid
pub fn validate_workspace_name(name: &str) -> Result<()> {
    let name = name.trim();

    if name.is_empty() {
        return Err(Error::Validation("Workspace name cannot be empty".to_string()));
    }

    if name.len() > 100 {
        return Err(Error::Validation("Workspace name must be less than 100 characters".to_string()));
    }

    // Check for valid characters (letters, numbers, spaces, hyphens, underscores)
    if !name.chars().all(|c| c.is_alphanumeric() || c.is_whitespace() || c == '-' || c == '_') {
        return Err(Error::Validation("Workspace name can only contain letters, numbers, spaces, hyphens, and underscores".to_string()));
    }

    // Check for control characters
    if name.chars().any(|c| c.is_control()) {
        return Err(Error::Validation("Workspace name cannot contain control characters".to_string()));
    }

    Ok(())
}

/// Validates full name format
///
/// # Arguments
/// * `full_name` - The full name to validate (optional)
///
/// # Returns
/// * `Ok(())` if the name is valid or empty
/// * `Err(Error)` with descriptive message if invalid
pub fn validate_full_name(full_name: &Option<String>) -> Result<()> {
    if let Some(name) = full_name {
        let name = name.trim();

        if !name.is_empty() {
            if name.len() > 100 {
                return Err(Error::Validation("Full name must be less than 100 characters".to_string()));
            }

            // Check for valid characters (letters, spaces, hyphens, apostrophes, periods)
            if !name.chars().all(|c| c.is_alphabetic() || c.is_whitespace() || c == '-' || c == '\'' || c == '.') {
                return Err(Error::Validation("Full name can only contain letters, spaces, hyphens, apostrophes, and periods".to_string()));
            }

            // Check for control characters
            if name.chars().any(|c| c.is_control()) {
                return Err(Error::Validation("Full name cannot contain control characters".to_string()));
            }
        }
    }

    Ok(())
}

/// Validates session token format (UUID v7)
///
/// # Arguments
/// * `token` - The session token to validate
///
/// # Returns
/// * `Ok(())` if the token format is valid
/// * `Err(Error)` with descriptive message if invalid
pub fn validate_session_token(token: &str) -> Result<()> {
    let token = token.trim();

    if token.is_empty() {
        return Err(Error::Validation("Session token cannot be empty".to_string()));
    }

    // New format: hex:hex (approximately 129 chars: 64 + 1 + 64)
    if token.contains(':') {
        let parts: Vec<&str> = token.split(':').collect();
        if parts.len() != 2 {
            return Err(Error::Validation("Invalid token format".to_string()));
        }

        // Check hex encoding (64 chars for 32 bytes + 64 chars for 32 bytes signature)
        if parts[0].len() != 64 || parts[1].len() != 64 {
            return Err(Error::Validation("Invalid token length".to_string()));
        }

        // Verify both parts are valid hex
        if !parts[0].chars().all(|c| c.is_ascii_hexdigit()) ||
           !parts[1].chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(Error::Validation("Token must be hex-encoded".to_string()));
        }

        Ok(())
    } else {
        // Legacy UUID format no longer supported
        Err(Error::Validation("Invalid token format".to_string()))
    }
}

/// Validates UUID format
///
/// # Arguments
/// * `uuid_str` - The UUID string to validate
///
/// # Returns
/// * `Ok(uuid::Uuid)` if valid
/// * `Err(Error)` with descriptive message if invalid
pub fn validate_uuid(uuid_str: &str) -> Result<uuid::Uuid> {
    let uuid_str = uuid_str.trim();

    if uuid_str.is_empty() {
        return Err(Error::Validation("UUID cannot be empty".to_string()));
    }

    uuid::Uuid::parse_str(uuid_str)
        .map_err(|_| Error::Validation("Invalid UUID format".to_string()))
}

/// Sanitizes string input by trimming whitespace
///
/// # Arguments
/// * `input` - The input string to sanitize
///
/// # Returns
/// * Sanitized string with trimmed whitespace
/// * Empty string if input was None or only whitespace
pub fn sanitize_string(input: &str) -> String {
    input.trim().to_string()
}

/// Validates that a string is not empty after sanitization
///
/// # Arguments
/// * `input` - The input string to validate
/// * `field_name` - Name of the field for error messages
///
/// # Returns
/// * `Ok(String)` with sanitized string
/// * `Err(Error)` if empty after sanitization
pub fn validate_required_string(input: &str, field_name: &str) -> Result<String> {
    let sanitized = sanitize_string(input);

    if sanitized.is_empty() {
        return Err(Error::Validation(format!("{} cannot be empty", field_name)));
    }

    Ok(sanitized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_email_valid() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("test.email+tag@domain.co.uk").is_ok());
        assert!(validate_email("user_name@sub.domain.com").is_ok());
    }

    #[test]
    fn test_validate_email_invalid() {
        assert!(validate_email("").is_err());
        assert!(validate_email("invalid-email").is_err());
        assert!(validate_email("@domain.com").is_err());
        assert!(validate_email("user@").is_err());
        assert!(validate_email("user@@domain.com").is_err());
        assert!(validate_email("user@domain").is_err());
        assert!(validate_email("user name@domain.com").is_err());
        assert!(validate_email("user@domain..com").is_err());
    }

    #[test]
    fn test_validate_password_valid() {
        assert!(validate_password("validpassword123").is_ok());
        assert!(validate_password("MySecureP@ssw0rd!").is_ok());
        assert!(validate_password("eightchar").is_ok());
    }

    #[test]
    fn test_validate_password_invalid() {
        assert!(validate_password("").is_err());
        assert!(validate_password("short").is_err());
        assert!(validate_password("password").is_err());
        assert!(validate_password("12345678").is_err());
        assert!(validate_password("space in password").is_err());
        assert!(validate_password("a".repeat(130).as_str()).is_err());
    }

    #[test]
    fn test_validate_workspace_name_valid() {
        assert!(validate_workspace_name("My Workspace").is_ok());
        assert!(validate_workspace_name("Team-Project_2024").is_ok());
        assert!(validate_workspace_name("Development").is_ok());
    }

    #[test]
    fn test_validate_workspace_name_invalid() {
        assert!(validate_workspace_name("").is_err());
        assert!(validate_workspace_name("   ").is_err());
        assert!(validate_workspace_name("Workspace@Home").is_err());
        assert!(validate_workspace_name("Invalid!Name").is_err());
        assert!(validate_workspace_name("a".repeat(101).as_str()).is_err());
    }

    #[test]
    fn test_validate_full_name_valid() {
        assert!(validate_full_name(&Some("John Doe".to_string())).is_ok());
        assert!(validate_full_name(&Some("Mary-Jane O'Connor".to_string())).is_ok());
        assert!(validate_full_name(&Some("Dr. Jane Smith Jr.".to_string())).is_ok());
        assert!(validate_full_name(&None).is_ok());
        assert!(validate_full_name(&Some("".to_string())).is_ok());
        assert!(validate_full_name(&Some("   ".to_string())).is_ok());
    }

    #[test]
    fn test_validate_full_name_invalid() {
        assert!(validate_full_name(&Some("John123".to_string())).is_err());
        assert!(validate_full_name(&Some("John@Doe".to_string())).is_err());
        assert!(validate_full_name(&Some("a".repeat(101))).is_err()); // Too long
    }

    #[test]
    fn test_sanitize_string() {
        assert_eq!(sanitize_string("  hello world  "), "hello world");
        assert_eq!(sanitize_string("\ttest\n"), "test");
        assert_eq!(sanitize_string(""), "");
        assert_eq!(sanitize_string("   "), "");
    }

    #[test]
    fn test_validate_required_string() {
        assert!(validate_required_string("hello", "field").is_ok());
        assert!(validate_required_string("  hello  ", "field").is_ok());
        assert!(validate_required_string("", "field").is_err());
        assert!(validate_required_string("   ", "field").is_err());
    }
}