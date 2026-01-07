use axum::{
    extract::State,
    http::{
        header::SET_COOKIE,
        HeaderValue,
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

    // Build JSON response body
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
