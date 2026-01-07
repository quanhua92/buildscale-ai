use axum::{
    extract::State,
    http::{
        header::{AUTHORIZATION, COOKIE, SET_COOKIE},
        HeaderMap, HeaderValue,
    },
    response::{IntoResponse, Json, Response},
};
use crate::{
    error::Result,
    models::users::{LoginUser, RegisterUser},
    services::{
        cookies::{build_access_token_cookie, build_refresh_token_cookie, CookieConfig},
        users,
    },
    state::AppState,
};

/// Custom response type for login that sets multiple Set-Cookie headers
pub struct LoginResponse {
    json_body: serde_json::Value,
    access_cookie: String,
    refresh_cookie: String,
}

impl IntoResponse for LoginResponse {
    fn into_response(self) -> Response {
        // Convert JSON to response
        let json_response = Json(self.json_body.clone()).into_response();

        // Get the body from the JSON response
        let (mut parts, body) = json_response.into_parts();

        // Set both Set-Cookie headers using append to allow multiple values
        if let Ok(access_cookie) = HeaderValue::from_str(&self.access_cookie) {
            parts.headers.append(SET_COOKIE, access_cookie);
        }
        if let Ok(refresh_cookie) = HeaderValue::from_str(&self.refresh_cookie) {
            parts.headers.append(SET_COOKIE, refresh_cookie);
        }

        // Rebuild response with combined headers
        Response::from_parts(parts, body)
    }
}

/// Custom response type for refresh that optionally sets access_token cookie
pub struct RefreshResponse {
    json_body: serde_json::Value,
    access_cookie: Option<String>,
    from_cookie: bool,
}

impl IntoResponse for RefreshResponse {
    fn into_response(self) -> Response {
        let json_response = Json(self.json_body.clone()).into_response();
        let (mut parts, body) = json_response.into_parts();

        // Only set cookie if refresh token came from cookie (browser client)
        if self.from_cookie {
            if let Some(access_cookie) = self.access_cookie {
                if let Ok(cookie) = HeaderValue::from_str(&access_cookie) {
                    parts.headers.append(SET_COOKIE, cookie);
                }
            }
        }

        Response::from_parts(parts, body)
    }
}

/// POST /api/v1/auth/register
///
/// Registers a new user with email and password.
///
/// # Request Body
/// - `email`: User's email address (must be unique)
/// - `password`: User's password (minimum 8 characters)
/// - `confirm_password`: Password confirmation (must match password)
/// - `full_name`: Optional user's full name
///
/// # Returns
/// JSON response containing the created user object.
///
/// # HTTP Status Codes
/// - `200 OK`: User registered successfully
/// - `400 BAD_REQUEST`: Validation error (invalid email, weak password, passwords don't match)
/// - `409 CONFLICT`: Email already exists
/// - `500 INTERNAL_SERVER_ERROR`: Database error
pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterUser>,
) -> Result<Json<serde_json::Value>> {
    // Acquire database connection from pool
    let mut conn = state.pool.acquire().await.map_err(|e| {
        crate::error::Error::Internal(format!("Failed to acquire database connection: {}", e))
    })?;

    // Call service layer to register user
    let user = users::register_user(&mut conn, request).await?;

    // Return user data as JSON
    Ok(Json(serde_json::json!({
        "user": user
    })))
}

/// POST /api/v1/auth/login
///
/// Authenticates a user with email and password.
///
/// # Request Body
/// - `email`: User's email address
/// - `password`: User's password
///
/// # Returns
/// JSON response containing:
/// - `user`: User object
/// - `access_token`: JWT access token (15 minute expiration)
/// - `refresh_token`: Session refresh token (30 day expiration)
/// - `access_token_expires_at`: ISO 8601 timestamp of access token expiration
/// - `refresh_token_expires_at`: ISO 8601 timestamp of refresh token expiration
///
/// Also sets cookies:
/// - `access_token`: JWT access token cookie
/// - `refresh_token`: Session refresh token cookie
///
/// Both cookies have security flags: HttpOnly, SameSite=Lax, Secure (in production)
///
/// # HTTP Status Codes
/// - `200 OK`: Authentication successful
/// - `400 BAD_REQUEST`: Validation error (empty email/password)
/// - `401 UNAUTHORIZED`: Invalid email or password
/// - `500 INTERNAL_SERVER_ERROR`: Database error
///
/// # Client Compatibility
/// - **Browser clients**: Cookies are automatically sent with subsequent requests
/// - **API/Mobile clients**: Use tokens from JSON response in Authorization header
pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<LoginUser>,
) -> Result<LoginResponse> {
    // Acquire database connection from pool
    let mut conn = state.pool.acquire().await.map_err(|e| {
        crate::error::Error::Internal(format!("Failed to acquire database connection: {}", e))
    })?;

    // Call service layer to authenticate user
    let login_result = users::login_user(&mut conn, request).await?;

    // Build cookie configuration with security settings
    let config = CookieConfig::default();

    // Build Set-Cookie headers for both tokens
    let access_cookie = build_access_token_cookie(&login_result.access_token, &config);
    let refresh_cookie = build_refresh_token_cookie(&login_result.refresh_token, &config);

    // Build JSON response body - ALWAYS include tokens
    // (Login is the initial token grant, so clients need the tokens)
    let json_body = serde_json::json!({
        "user": login_result.user,
        "access_token": login_result.access_token,
        "refresh_token": login_result.refresh_token,
        "access_token_expires_at": login_result.access_token_expires_at,
        "refresh_token_expires_at": login_result.refresh_token_expires_at
    });

    // Return custom response with cookies
    Ok(LoginResponse {
        json_body,
        access_cookie,
        refresh_cookie,
    })
}

/// POST /api/v1/auth/refresh
///
/// Refreshes an access token using a valid refresh token.
///
/// Accepts refresh token from:
/// - Authorization header (API/mobile clients): `Bearer <token>`
/// - Cookie (browser clients): `refresh_token=<token>`
///
/// Returns new JWT access token and optionally sets cookie for browser clients.
///
/// # HTTP Status Codes
/// - `200 OK`: Token refreshed successfully
/// - `401 UNAUTHORIZED`: Invalid or expired refresh token
/// - `500 INTERNAL_SERVER_ERROR`: Database error
///
/// # Client Compatibility
/// - **Browser clients**: Receives new access_token in cookie automatically
/// - **API/Mobile clients**: Extract new access_token from JSON response
pub async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<RefreshResponse> {
    // Extract refresh token from Authorization header or cookie
    let (token, from_cookie) = extract_refresh_token(&headers)?;

    // Acquire database connection from pool
    let mut conn = state.pool.acquire().await.map_err(|e| {
        crate::error::Error::Internal(format!("Failed to acquire database connection: {}", e))
    })?;

    // Call service layer to refresh access token
    let refresh_result = users::refresh_access_token(&mut conn, &token).await?;

    // Build response (only set cookie if request came from cookie)
    let config = CookieConfig::default();
    let access_cookie = if from_cookie {
        Some(build_access_token_cookie(&refresh_result.access_token, &config))
    } else {
        None
    };

    let json_body = serde_json::json!({
        "access_token": refresh_result.access_token,
        "expires_at": refresh_result.expires_at
    });

    Ok(RefreshResponse {
        json_body,
        access_cookie,
        from_cookie,
    })
}

/// Extract refresh token from Authorization header or cookie
fn extract_refresh_token(headers: &HeaderMap) -> Result<(String, bool)> {
    let config = CookieConfig::default();

    // Priority 1: Authorization header (API/mobile clients)
    if let Some(auth_header) = headers.get(AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            let token = if auth_str.starts_with("Bearer ") {
                auth_str[7..].trim()
            } else {
                auth_str.trim()
            };

            if !token.is_empty() {
                return Ok((token.to_string(), false));
            }
        }
    }

    // Priority 2: Cookie (browser clients)
    if let Some(cookie_header) = headers.get(COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            let cookie_pattern = format!("{}=", config.refresh_token_name);

            for cookie_pair in cookie_str.split(';') {
                let cookie_pair = cookie_pair.trim();
                if cookie_pair.starts_with(&cookie_pattern) {
                    let token = cookie_pair[cookie_pattern.len()..].trim();
                    if !token.is_empty() {
                        return Ok((token.to_string(), true));
                    }
                }
            }
        }
    }

    Err(crate::error::Error::Authentication(
        "No valid refresh token found in Authorization header or cookie".to_string()
    ))
}
