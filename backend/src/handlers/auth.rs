use axum::{
    extract::State,
    http::{
        header::{AUTHORIZATION, COOKIE, SET_COOKIE},
        HeaderMap, HeaderValue,
    },
    response::{IntoResponse, Json, Response},
    Extension,
};
use crate::{
    error::{Error, Result},
    middleware::auth::AuthenticatedUser,
    models::users::{LoginUser, RegisterUser},
    services::{
        cookies::{
            build_access_token_cookie,
            build_clear_token_cookie,
            build_refresh_token_cookie,
            CookieConfig,
        },
        users,
    },
    state::AppState,
};

/// Macro to reduce boilerplate in error handling for auth handlers
/// Logs errors at appropriate level based on error type
macro_rules! handle_auth_error {
    ($operation:expr, $e:expr, $error_type_map:tt) => {
        match &$e {
            Error::Validation(_) => {
                tracing::warn!(
                    operation = $operation,
                    error = %$e,
                    concat!("User ", $operation, " failed: validation error"),
                );
            }
            Error::Conflict(_) => {
                tracing::warn!(
                    operation = $operation,
                    error = %$e,
                    concat!("User ", $operation, " failed: conflict"),
                );
            }
            Error::Authentication(_) => {
                tracing::warn!(
                    operation = $operation,
                    error = "authentication_failed",
                    concat!("User ", $operation, " failed: invalid credentials"),
                );
            }
            Error::InvalidToken(_) | Error::SessionExpired(_) => {
                tracing::warn!(
                    operation = $operation,
                    error = "invalid_token",
                    concat!("Token ", $operation, " failed: invalid or expired token"),
                );
            }
            _ => {
                tracing::error!(
                    operation = $operation,
                    error = %$e,
                    concat!("User ", $operation, " failed: internal error"),
                );
            }
        }
    };
}

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

/// Custom response type for refresh that optionally sets access_token AND refresh_token cookies
pub struct RefreshResponse {
    json_body: serde_json::Value,
    access_cookie: Option<String>,
    refresh_cookie: Option<String>,  // NEW: Support refresh token cookie
    from_cookie: bool,
}

impl IntoResponse for RefreshResponse {
    fn into_response(self) -> Response {
        let json_response = Json(self.json_body.clone()).into_response();
        let (mut parts, body) = json_response.into_parts();

        // Only set cookies if refresh token came from cookie (browser client)
        if self.from_cookie {
            // Set access_token cookie
            if let Some(access_cookie) = self.access_cookie {
                if let Ok(cookie) = HeaderValue::from_str(&access_cookie) {
                    parts.headers.append(SET_COOKIE, cookie);
                }
            }
            // Set refresh_token cookie (rotation)
            if let Some(refresh_cookie) = self.refresh_cookie {
                if let Ok(cookie) = HeaderValue::from_str(&refresh_cookie) {
                    parts.headers.append(SET_COOKIE, cookie);
                }
            }
        }

        Response::from_parts(parts, body)
    }
}

/// Custom response type for logout that clears both access and refresh token cookies
pub struct LogoutResponse {
    clear_access_cookie: String,
    clear_refresh_cookie: String,
}

impl IntoResponse for LogoutResponse {
    fn into_response(self) -> Response {
        let json_response = Json(serde_json::json!({
            "message": "Logout successful"
        }))
        .into_response();

        let (mut parts, body) = json_response.into_parts();

        // Clear both cookies by setting them with Max-Age=0
        if let Ok(clear_access) = HeaderValue::from_str(&self.clear_access_cookie) {
            parts.headers.append(SET_COOKIE, clear_access);
        }
        if let Ok(clear_refresh) = HeaderValue::from_str(&self.clear_refresh_cookie) {
            parts.headers.append(SET_COOKIE, clear_refresh);
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
    tracing::info!(operation = "register", "User registration initiated");

    #[cfg(debug_assertions)]
    tracing::debug!(
        email_provided = !request.email.is_empty(),
        password_length = request.password.len(),
        has_full_name = request.full_name.is_some(),
        "Request payload details",
    );

    // Acquire database connection from pool
    let mut conn = state.pool.acquire().await.map_err(|e| {
        tracing::error!(
            operation = "register",
            error_code = "DATABASE_ACQUISITION_FAILED",
            error = %e,
            "Failed to acquire database connection",
        );
        crate::error::Error::Internal(format!("Failed to acquire database connection: {}", e))
    })?;

    // Call service layer to register user
    let user = match users::register_user(&mut conn, request).await {
        Ok(user) => user,
        Err(e) => {
            handle_auth_error!("register", e, {});
            return Err(e);
        }
    };

    tracing::info!(
        operation = "register",
        user_id = %user.id,
        "User registered successfully",
    );

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
    tracing::info!(operation = "login", "User login initiated");

    #[cfg(debug_assertions)]
    tracing::debug!(
        password_length = request.password.len(),
        "Login request details",
    );

    // Acquire database connection from pool
    let mut conn = state.pool.acquire().await.map_err(|e| {
        tracing::error!(
            operation = "login",
            error_code = "DATABASE_ACQUISITION_FAILED",
            error = %e,
            "Failed to acquire database connection",
        );
        crate::error::Error::Internal(format!("Failed to acquire database connection: {}", e))
    })?;

    // Call service layer to authenticate user
    let login_result = match users::login_user(&mut conn, request).await {
        Ok(result) => result,
        Err(e) => {
            handle_auth_error!("login", e, {});
            return Err(e);
        }
    };

    tracing::info!(
        operation = "login",
        user_id = %login_result.user.id,
        access_expires_at = %login_result.access_token_expires_at,
        refresh_expires_at = %login_result.refresh_token_expires_at,
        "User login successful",
    );

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
/// Returns new JWT access token and rotated refresh token.
///
/// Accepts refresh token from:
/// - Authorization header (API/mobile clients): `Bearer <token>`
/// - Cookie (Browser clients): `refresh_token=<token>`
///
/// **Token Rotation**: Each refresh generates a NEW refresh token and invalidates the old one.
/// This is an OAuth 2.0 security best practice that prevents token theft replay attacks.
///
/// Returns new JWT access token and new refresh token.
/// Browser clients receive both as cookies; API clients receive both in JSON.
///
/// # HTTP Status Codes
/// - `200 OK`: Token refreshed successfully
/// - `401 UNAUTHORIZED`: Invalid or expired refresh token
/// - `500 INTERNAL_SERVER_ERROR`: Database error
///
/// # Client Compatibility
/// - **Browser clients**: Receives new access_token and refresh_token in cookies automatically
/// - **API/Mobile clients**: Extract new access_token and refresh_token from JSON response
///
/// # Breaking Change from Previous Version
/// The response now includes `refresh_token` field. API clients must update to store the new token.
pub async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<RefreshResponse> {
    // Extract refresh token from Authorization header or cookie
    let (token, from_cookie) = extract_refresh_token(&headers)?;

    let token_source = if from_cookie { "cookie" } else { "header" };
    tracing::info!(
        operation = "refresh",
        token_source = token_source,
        "Token refresh initiated",
    );

    // Acquire database connection from pool
    let mut conn = state.pool.acquire().await.map_err(|e| {
        tracing::error!(
            operation = "refresh",
            error_code = "DATABASE_ACQUISITION_FAILED",
            error = %e,
            "Failed to acquire database connection",
        );
        crate::error::Error::Internal(format!("Failed to acquire database connection: {}", e))
    })?;

    // Call service layer to refresh access token with rotation
    let refresh_result = match users::refresh_access_token(&mut conn, &token).await {
        Ok(result) => result,
        Err(e) => {
            handle_auth_error!("refresh", e, {});
            return Err(e);
        }
    };

    let token_rotated = refresh_result.refresh_token.is_some();
    tracing::info!(
        operation = "refresh",
        token_rotated = token_rotated,
        expires_at = %refresh_result.expires_at,
        "Token refresh successful",
    );

    // Build response (only set cookies if request came from cookie)
    let config = CookieConfig::default();
    let access_cookie = if from_cookie {
        Some(build_access_token_cookie(&refresh_result.access_token, &config))
    } else {
        None
    };

    // Build refresh token cookie (only for browser clients, if token exists)
    let refresh_cookie = if from_cookie {
        refresh_result.refresh_token.as_ref().map(|token| {
            build_refresh_token_cookie(token, &config)
        })
    } else {
        None
    };

    let json_body = serde_json::json!({
        "access_token": refresh_result.access_token,
        "refresh_token": refresh_result.refresh_token,  // None if within grace period
        "expires_at": refresh_result.expires_at
    });

    Ok(RefreshResponse {
        json_body,
        access_cookie,
        refresh_cookie,  // NEW: Pass refresh cookie
        from_cookie,
    })
}

/// POST /api/v1/auth/logout
///
/// Logs out a user by invalidating their refresh token session.
///
/// Accepts refresh token from:
/// - Authorization header (API/mobile clients): `Bearer <token>`
/// - Cookie (browser clients): `refresh_token=<token>`
///
/// Clears both access_token and refresh_token cookies.
///
/// # HTTP Status Codes
/// - `200 OK`: Logout successful
/// - `401 UNAUTHORIZED`: Invalid or expired refresh token
/// - `500 INTERNAL_SERVER_ERROR`: Database error
///
/// # Client Compatibility
/// - **Browser clients**: Cookies are cleared automatically via Set-Cookie headers
/// - **API/Mobile clients**: Tokens remain in client storage but are invalidated server-side
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<LogoutResponse> {
    tracing::info!(operation = "logout", "User logout initiated");

    // Extract refresh token from Authorization header or cookie
    let (token, _from_cookie) = match extract_refresh_token(&headers) {
        Ok(result) => result,
        Err(e) => {
            tracing::warn!(
                operation = "logout",
                error = "invalid_token",
                "Logout attempted with invalid or missing token",
            );
            return Err(e);
        }
    };

    // Acquire database connection from pool
    let mut conn = state.pool.acquire().await.map_err(|e| {
        tracing::error!(
            operation = "logout",
            error_code = "DATABASE_ACQUISITION_FAILED",
            error = %e,
            "Failed to acquire database connection",
        );
        crate::error::Error::Internal(format!("Failed to acquire database connection: {}", e))
    })?;

    // Call service layer to logout user (invalidate session)
    match users::logout_user(&mut conn, &token).await {
        Ok(_) => {
            tracing::info!(operation = "logout", "User logout successful");
        }
        Err(e) => {
            tracing::error!(
                operation = "logout",
                error = %e,
                "User logout failed: internal error",
            );
            return Err(e);
        }
    }

    // Build clear cookie headers for both tokens
    let config = CookieConfig::default();
    let clear_access_cookie = build_clear_token_cookie(&config.access_token_name);
    let clear_refresh_cookie = build_clear_token_cookie(&config.refresh_token_name);

    // Return response that clears both cookies
    Ok(LogoutResponse {
        clear_access_cookie,
        clear_refresh_cookie,
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

/// GET /api/v1/auth/me
///
/// Returns the currently authenticated user's profile.
///
/// This endpoint requires a valid JWT access token via:
/// - Authorization header (API/mobile clients): `Bearer <token>`
/// - Cookie (browser clients): `access_token=<token>`
///
/// # Returns
/// JSON response containing the authenticated user object.
///
/// # HTTP Status Codes
/// - `200 OK`: Successfully retrieved user profile
/// - `401 UNAUTHORIZED`: Invalid or expired JWT token
pub async fn me(
    Extension(auth_user): Extension<AuthenticatedUser>,
) -> Result<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "user": auth_user
    })))
}
