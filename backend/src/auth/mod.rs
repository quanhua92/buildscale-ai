//! Auth module - Authentication and session management
//!
//! This module provides a layered architecture for authentication:
//! - `models`: Session and token types
//! - `queries`: Session database operations
//! - `services`: JWT, cookies, refresh tokens, and session management

pub mod models;
pub mod queries;
pub mod services;

// Re-export commonly used types at module level for convenience
pub use models::{
    NewUserSession, RevokedRefreshToken, UpdateUserSession, UserSession,
};

pub use services::{
    // JWT functions
    generate_jwt, verify_jwt, get_user_id_from_token, authenticate_jwt_token,
    authenticate_jwt_token_from_anywhere, Claims,
    // Cookie functions
    build_access_token_cookie, build_clear_token_cookie, build_refresh_token_cookie,
    extract_jwt_token, extract_refresh_token, authenticate_jwt_token_multi_source,
    CookieConfig, SameSite, ACCESS_TOKEN_COOKIE, REFRESH_TOKEN_COOKIE,
    // Refresh token functions
    generate_refresh_token, verify_refresh_token,
    // Session management functions
    cleanup_expired_sessions, extend_all_user_sessions, get_user_active_sessions,
    revoke_all_user_sessions, revoke_session_by_token, user_has_active_sessions,
};

pub use queries::{
    hash_session_token,
};
