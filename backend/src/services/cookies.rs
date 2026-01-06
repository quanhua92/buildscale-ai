use crate::error::{Error, Result};
use crate::services::jwt;
use uuid::Uuid;

/// Cookie names for token storage
pub const ACCESS_TOKEN_COOKIE: &str = "access_token";
pub const REFRESH_TOKEN_COOKIE: &str = "refresh_token";

/// Cookie security configuration
///
/// Controls how cookies are created and secured for browser clients
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CookieConfig {
    /// Name of the access token cookie (default: "access_token")
    pub access_token_name: String,
    /// Name of the refresh token cookie (default: "refresh_token")
    pub refresh_token_name: String,
    /// HttpOnly flag prevents JavaScript access (XSS protection)
    pub http_only: bool,
    /// Secure flag ensures HTTPS-only transmission (should be true in production)
    pub secure: bool,
    /// SameSite attribute for CSRF protection
    pub same_site: SameSite,
    /// Path attribute to limit cookie scope
    pub path: String,
    /// Optional domain attribute (e.g., ".example.com" for subdomain sharing)
    pub domain: Option<String>,
}

/// SameSite cookie attribute for CSRF protection
#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SameSite {
    /// Strict mode - cookie not sent with cross-site requests
    Strict,
    /// Lax mode - cookie sent with top-level navigations
    Lax,
    /// None mode - cookie sent with all requests (requires Secure)
    None,
}

impl Default for CookieConfig {
    fn default() -> Self {
        Self {
            access_token_name: "access_token".to_string(),
            refresh_token_name: "refresh_token".to_string(),
            http_only: true,
            secure: false, // Set to true in production
            same_site: SameSite::Lax,  // Allows top-level navigations from emails, Slack, OAuth, etc.
            path: "/".to_string(),
            domain: None,
        }
    }
}

/// Extracts JWT from either Authorization header or cookie
///
/// Priority order:
/// 1. Authorization header (for API/mobile clients)
/// 2. Cookie (fallback for browser clients)
///
/// # Arguments
/// * `auth_header` - Optional Authorization header value (e.g., "Bearer <token>")
/// * `cookie_value` - Optional cookie value (for access_token)
///
/// # Returns
/// The extracted JWT token string
///
/// # Example
/// ```rust,no_run
/// use backend::services::cookies::extract_jwt_token;
///
/// // Try header first, fallback to cookie
/// let token = extract_jwt_token(
///     Some("Bearer eyJhbGc..."),
///     Some("cookie_token_value")
/// ).unwrap();
/// ```
pub fn extract_jwt_token(
    auth_header: Option<&str>,
    cookie_value: Option<&str>,
) -> Result<String> {
    // Priority 1: Authorization header
    if let Some(header) = auth_header {
        if header.starts_with("Bearer ") {
            let token = header[7..].to_string();
            if !token.is_empty() {
                return Ok(token);
            }
        }
    }

    // Priority 2: Cookie
    if let Some(cookie) = cookie_value {
        if !cookie.is_empty() {
            return Ok(cookie.to_string());
        }
    }

    Err(Error::Authentication(
        "No valid token found in Authorization header or cookie".to_string()
    ))
}

/// Extracts refresh token from cookie
///
/// Refresh tokens are typically stored in cookies for browser clients.
///
/// # Arguments
/// * `cookie_value` - Optional cookie value (for refresh_token)
///
/// # Returns
/// The extracted refresh token string
///
/// # Example
/// ```rust,no_run
/// use backend::services::cookies::extract_refresh_token;
///
/// let token = extract_refresh_token(
///     Some("a1b2c3d4...")
/// ).unwrap();
/// ```
pub fn extract_refresh_token(cookie_value: Option<&str>) -> Result<String> {
    if let Some(cookie) = cookie_value {
        if !cookie.is_empty() {
            return Ok(cookie.to_string());
        }
    }

    Err(Error::Authentication(
        "No valid refresh token found in cookie".to_string()
    ))
}

/// Validates JWT from header OR cookie and returns user_id
///
/// This is a convenience wrapper that extracts the token using `extract_jwt_token()`
/// and then validates it using JWT verification.
///
/// # Arguments
/// * `auth_header` - Optional Authorization header value
/// * `cookie_value` - Optional cookie value (for access_token)
/// * `secret` - JWT secret for verification
///
/// # Returns
/// The user's UUID if token is valid
///
/// # Example
/// ```rust,no_run
/// use backend::services::cookies::authenticate_jwt_token_multi_source;
///
/// let user_id = authenticate_jwt_token_multi_source(
///     Some("Bearer eyJhbGc..."),
///     Some("cookie_token"),
///     "jwt_secret"
/// ).unwrap();
/// ```
pub fn authenticate_jwt_token_multi_source(
    auth_header: Option<&str>,
    cookie_value: Option<&str>,
    secret: &str,
) -> Result<Uuid> {
    let token = extract_jwt_token(auth_header, cookie_value)?;
    jwt::get_user_id_from_token(&token, secret)
}

/// Builds a Set-Cookie header value for access token
///
/// # Arguments
/// * `token` - The JWT access token
/// * `config` - Cookie configuration
///
/// # Returns
/// A Set-Cookie header value string
///
/// # Example
/// ```rust,no_run
/// use backend::services::cookies::{build_access_token_cookie, CookieConfig, SameSite};
///
/// let config = CookieConfig {
///     access_token_name: "access_token".to_string(),
///     refresh_token_name: "refresh_token".to_string(),
///     http_only: true,
///     secure: true,
///     same_site: SameSite::Lax,  // Default: allows links from emails, Slack, OAuth
///     path: "/".to_string(),
///     domain: None,
/// };
///
/// let cookie = build_access_token_cookie("my_token", &config);
/// // Returns: "access_token=my_token; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=900"
/// ```
pub fn build_access_token_cookie(token: &str, config: &CookieConfig) -> String {
    let same_site_str = match config.same_site {
        SameSite::Strict => "Strict",
        SameSite::Lax => "Lax",
        SameSite::None => "None",
    };

    format!(
        "{}={}; HttpOnly{}; SameSite={}; Path={}; Max-Age={}",
        config.access_token_name,
        token,
        if config.secure { "; Secure" } else { "" },
        same_site_str,
        config.path,
        900 // 15 minutes (access token expiration)
    )
}

/// Builds a Set-Cookie header value for refresh token
///
/// # Arguments
/// * `token` - The refresh token (HMAC-signed)
/// * `config` - Cookie configuration
///
/// # Returns
/// A Set-Cookie header value string
///
/// # Example
/// ```rust,no_run
/// use backend::services::cookies::{build_refresh_token_cookie, CookieConfig, SameSite};
///
/// let config = CookieConfig {
///     access_token_name: "access_token".to_string(),
///     refresh_token_name: "refresh_token".to_string(),
///     http_only: true,
///     secure: true,
///     same_site: SameSite::Lax,  // Default: allows links from emails, Slack, OAuth
///     path: "/".to_string(),
///     domain: None,
/// };
///
/// let cookie = build_refresh_token_cookie("my_refresh_token", &config);
/// // Returns: "refresh_token=my_refresh_token; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=2592000"
/// ```
pub fn build_refresh_token_cookie(token: &str, config: &CookieConfig) -> String {
    let same_site_str = match config.same_site {
        SameSite::Strict => "Strict",
        SameSite::Lax => "Lax",
        SameSite::None => "None",
    };

    format!(
        "{}={}; HttpOnly{}; SameSite={}; Path={}; Max-Age={}",
        config.refresh_token_name,
        token,
        if config.secure { "; Secure" } else { "" },
        same_site_str,
        config.path,
        2592000 // 30 days (refresh token expiration)
    )
}

/// Builds a Set-Cookie header value to clear a token
///
/// Used during logout to invalidate cookies by setting Max-Age=0
///
/// # Arguments
/// * `token_name` - The name of the cookie to clear (e.g., "access_token")
///
/// # Returns
/// A Set-Cookie header value string that clears the cookie
///
/// # Example
/// ```rust,no_run
/// use backend::services::cookies::build_clear_token_cookie;
///
/// let cookie = build_clear_token_cookie("access_token");
/// // Returns: "access_token=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0"
/// ```
pub fn build_clear_token_cookie(token_name: &str) -> String {
    format!(
        "{}=; HttpOnly; SameSite=Strict; Path={}; Max-Age=0",
        token_name,
        "/" // Always use root path for clearing
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_jwt_token_from_header() {
        let header = "Bearer my-token";
        let token = extract_jwt_token(Some(header), None).unwrap();
        assert_eq!(token, "my-token");
    }

    #[test]
    fn test_extract_jwt_token_from_cookie() {
        let cookie = "my-token";
        let token = extract_jwt_token(None, Some(cookie)).unwrap();
        assert_eq!(token, "my-token");
    }

    #[test]
    fn test_extract_jwt_token_priority_header_over_cookie() {
        let header = "Bearer header-token";
        let cookie = "cookie-token";
        let token = extract_jwt_token(Some(header), Some(cookie)).unwrap();
        assert_eq!(token, "header-token");
    }

    #[test]
    fn test_extract_jwt_token_no_token() {
        let result = extract_jwt_token(None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_jwt_token_empty_header() {
        let result = extract_jwt_token(Some("Bearer "), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_jwt_token_invalid_header_format() {
        let result = extract_jwt_token(Some("InvalidFormat"), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_refresh_token_from_cookie() {
        let cookie = "my-refresh-token";
        let token = extract_refresh_token(Some(cookie)).unwrap();
        assert_eq!(token, "my-refresh-token");
    }

    #[test]
    fn test_extract_refresh_token_missing() {
        let result = extract_refresh_token(None);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_refresh_token_empty() {
        let result = extract_refresh_token(Some(""));
        assert!(result.is_err());
    }

    #[test]
    fn test_build_access_token_cookie() {
        let config = CookieConfig {
            access_token_name: "access_token".to_string(),
            refresh_token_name: "refresh_token".to_string(),
            http_only: true,
            secure: false,
            same_site: SameSite::Strict,
            path: "/".to_string(),
            domain: None,
        };

        let cookie = build_access_token_cookie("my-token", &config);
        assert!(cookie.contains("access_token=my-token"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(cookie.contains("Path=/"));
        assert!(cookie.contains("Max-Age=900"));
        assert!(!cookie.contains("Secure"));
    }

    #[test]
    fn test_build_access_token_cookie_with_secure() {
        let config = CookieConfig {
            access_token_name: "access_token".to_string(),
            refresh_token_name: "refresh_token".to_string(),
            http_only: true,
            secure: true,
            same_site: SameSite::Strict,
            path: "/".to_string(),
            domain: None,
        };

        let cookie = build_access_token_cookie("my-token", &config);
        assert!(cookie.contains("Secure"));
    }

    #[test]
    fn test_build_refresh_token_cookie() {
        let config = CookieConfig::default();

        let cookie = build_refresh_token_cookie("my-refresh-token", &config);
        assert!(cookie.contains("refresh_token=my-refresh-token"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Path=/"));
        assert!(cookie.contains("Max-Age=2592000"));
    }

    #[test]
    fn test_build_clear_token_cookie() {
        let cookie = build_clear_token_cookie("access_token");
        assert!(cookie.contains("access_token="));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(cookie.contains("Path=/"));
        assert!(cookie.contains("Max-Age=0"));
    }

    #[test]
    fn test_cookie_config_default() {
        let config = CookieConfig::default();
        assert_eq!(config.access_token_name, "access_token");
        assert_eq!(config.refresh_token_name, "refresh_token");
        assert!(config.http_only);
        assert!(!config.secure);
        assert!(matches!(config.same_site, SameSite::Lax));
        assert_eq!(config.path, "/");
        assert!(config.domain.is_none());
    }
}
