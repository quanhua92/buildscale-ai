//! Users module - User management
//!
//! This module provides a layered architecture for user management:
//! - `models`: User, NewUser, UpdateUser, RegisterUser, LoginUser, LoginResult, RefreshTokenResult
//! - `queries`: User database operations
//! - `services`: User business logic (registration, login, session management)

pub mod models;
pub mod queries;
pub mod services;

// Re-export commonly used types at module level for convenience
pub use models::{
    User, NewUser, UpdateUser, RegisterUser, LoginUser, LoginResult, RefreshTokenResult,
};

pub use services::{
    register_user, register_user_with_workspace,
    login_user, logout_user, validate_session,
    refresh_session, refresh_access_token,
    generate_password_hash, verify_password, generate_session_token,
    update_password, get_session_info, is_email_available,
    get_user_active_sessions, revoke_all_user_sessions,
};

pub use queries::{
    create_user, get_user_by_id, get_user_by_email,
    list_users, update_user, update_user_password, delete_user,
};
