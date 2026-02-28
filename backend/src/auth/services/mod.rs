//! Auth services - JWT, cookies, refresh tokens, and session management

pub mod cookies;
pub mod jwt;
pub mod refresh_tokens;
pub mod sessions;

// Re-export main public APIs
pub use cookies::{
    build_access_token_cookie, build_clear_token_cookie, build_refresh_token_cookie,
    extract_jwt_token, extract_refresh_token, authenticate_jwt_token_multi_source,
    CookieConfig, SameSite, ACCESS_TOKEN_COOKIE, REFRESH_TOKEN_COOKIE,
};
pub use jwt::{
    generate_jwt, verify_jwt, get_user_id_from_token, authenticate_jwt_token,
    authenticate_jwt_token_from_anywhere, Claims,
};
pub use refresh_tokens::{generate_refresh_token, verify_refresh_token};
pub use sessions::{
    cleanup_expired_sessions, extend_all_user_sessions, get_user_active_sessions,
    revoke_all_user_sessions, revoke_session_by_token, user_has_active_sessions,
};
