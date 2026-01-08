//! JWT authentication middleware with user caching
//!
//! This module provides middleware for validating JWT tokens and caching
//! authenticated user data to reduce database queries.

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    config::Config,
    error::{Error, Result},
    models::users::User,
    queries,
    services::jwt::authenticate_jwt_token_from_anywhere,
    state::AppState,
};

use secrecy::ExposeSecret;

/// Authenticated user extracted from JWT token
///
/// This struct is added to request extensions by the JWT middleware
/// after successful validation and caching.
#[derive(Debug, Clone, Serialize)]
pub struct AuthenticatedUser {
    /// User's unique identifier
    pub id: Uuid,
    /// User's email address
    pub email: String,
    /// User's full name (optional)
    pub full_name: Option<String>,
}

impl From<User> for AuthenticatedUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            full_name: user.full_name,
        }
    }
}

/// JWT authentication middleware with user caching
///
/// This middleware validates JWT tokens from Authorization headers or cookies,
/// caches user details to reduce database queries, and adds the authenticated
/// user to request extensions for handler access.
///
/// # Token Sources
/// - **Authorization header** (API/Mobile clients): `Bearer <token>`
/// - **Cookie** (Browser clients): `access_token=<token>`
///
/// # Behavior
/// 1. Extracts JWT token from header or cookie (header takes priority)
/// 2. Validates JWT signature and expiration
/// 3. Checks cache for user details (cache key: `user:{user_id}`)
/// 4. On cache miss: queries database and caches user with configured TTL
/// 5. Adds `AuthenticatedUser` to request extensions
/// 6. Returns 401 if token is invalid, expired, or missing
///
/// # Usage
/// Apply this middleware to protected routes using `route_layer()`:
///
/// ```ignore
/// Router::new()
///     .route("/protected", get(protected_handler))
///     .route_layer(middleware::from_fn_with_state(
///         state.clone(),
///         jwt_auth_middleware,
///     ))
/// ```
pub async fn jwt_auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response> {
    // 1. Validate JWT and get user_id from Authorization header OR Cookie
    let config = Config::load()?;
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());

    // Extract cookie value from Cookie header
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let access_token = cookie_header.and_then(|h| extract_cookie_value(h, "access_token"));

    let user_id = authenticate_jwt_token_from_anywhere(
        auth_header,
        access_token.as_deref(),
        &config.jwt.secret.expose_secret(),
    )?;

    // 2. Check cache for user details
    let cache_key = format!("user:{}", user_id);
    if let Some(cached_user) = state.user_cache.get(&cache_key).await? {
        // Cache hit - add to extensions and continue
        request.extensions_mut().insert(cached_user);
        return Ok(next.run(request).await);
    }

    // 3. Cache miss - query database for user info
    let mut conn = state.pool.acquire().await?;
    let user = queries::users::get_user_by_id(&mut conn, user_id)
        .await?
        .ok_or_else(|| Error::Authentication("User not found".to_string()))?;

    // 4. Cache user details with configurable TTL from config (cache the User object)
    let authenticated_user: AuthenticatedUser = user.clone().into();
    state
        .user_cache
        .set_ex(
            &cache_key,
            user.clone(),
            config.cache.user_cache_ttl_seconds,
        )
        .await?;

    // 5. Add to extensions and continue
    request.extensions_mut().insert(authenticated_user);
    Ok(next.run(request).await)
}

/// Extract specific cookie value from Cookie header
///
/// # Arguments
/// * `cookie_str` - Cookie header value
/// * `cookie_name` - Name of the cookie to extract
///
/// # Returns
/// * `Some(token)` - Cookie value if found
/// * `None` - Cookie not found
fn extract_cookie_value(cookie_str: &str, cookie_name: &str) -> Option<String> {
    cookie_str
        .split(';')
        .map(|s| s.trim())
        .find(|cookie| cookie.starts_with(&format!("{}=", cookie_name)))
        .and_then(|cookie| cookie.split('=').nth(1).map(|s| s.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_cookie_value() {
        let cookie_str = "access_token=abc123; refresh_token=def456";
        assert_eq!(
            extract_cookie_value(cookie_str, "access_token"),
            Some("abc123".to_string())
        );
        assert_eq!(
            extract_cookie_value(cookie_str, "refresh_token"),
            Some("def456".to_string())
        );
        assert_eq!(extract_cookie_value(cookie_str, "nonexistent"), None);
    }

    #[test]
    fn test_extract_cookie_value_with_spaces() {
        let cookie_str = "access_token=token123; other=value";
        assert_eq!(
            extract_cookie_value(cookie_str, "access_token"),
            Some("token123".to_string())
        );
    }

    #[test]
    fn test_extract_cookie_value_empty() {
        let cookie_str = "access_token=; other=value";
        // Empty cookie value returns empty string (not None)
        assert_eq!(
            extract_cookie_value(cookie_str, "access_token"),
            Some("".to_string())
        );
    }
}
