use buildscale::{
    services::users::{login_user, logout_user, validate_session, refresh_session, register_user},
    services::sessions::cleanup_expired_sessions,
    models::users::LoginUser,
};
use crate::common::database::TestApp;
use chrono::{Duration, Utc};

#[tokio::test]
async fn test_login_success() {
    let test_app = TestApp::new("test_login_success").await;
    let mut conn = test_app.get_connection().await;

    // Register a test user first
    let register_user_data = test_app.generate_test_user();
    let password = register_user_data.password.clone();
    let email = register_user_data.email.clone();

    let registered_user = register_user(&mut conn, register_user_data).await.unwrap();

    // Test successful login
    let login_user_data = LoginUser {
        email: email.clone(),
        password: password.clone(),
    };

    let login_result = login_user(&mut conn, login_user_data).await.unwrap();

    // Verify login result
    assert_eq!(login_result.user.id, registered_user.id);
    assert_eq!(login_result.user.email, registered_user.email);

    // Verify access token (JWT, 15 minutes)
    assert!(!login_result.access_token.is_empty());
    assert!(login_result.access_token_expires_at > Utc::now());
    assert!(login_result.access_token_expires_at <= Utc::now() + Duration::minutes(16)); // 15 min + 1 min margin

    // Verify refresh token (Session, 30 days)
    assert!(!login_result.refresh_token.is_empty());
    assert!(login_result.refresh_token_expires_at > Utc::now());
    assert!(login_result.refresh_token_expires_at <= Utc::now() + Duration::hours(721)); // 30 days + 1 hour margin
}

#[tokio::test]
async fn test_login_invalid_email() {
    let test_app = TestApp::new("test_login_invalid_email").await;
    let mut conn = test_app.get_connection().await;

    // Try to login with non-existent email
    let login_user_data = LoginUser {
        email: "nonexistent@example.com".to_string(),
        password: "SomeSecurePass123!".to_string(),
    };

    let result = login_user(&mut conn, login_user_data).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        buildscale::error::Error::Authentication(msg) => {
            assert_eq!(msg, "Invalid email or password");
        }
        _ => panic!("Expected Authentication error"),
    }
}

#[tokio::test]
async fn test_login_invalid_password() {
    let test_app = TestApp::new("test_login_invalid_password").await;
    let mut conn = test_app.get_connection().await;

    // Register a test user first
    let register_user_data = test_app.generate_test_user();
    let email = register_user_data.email.clone();

    register_user(&mut conn, register_user_data).await.unwrap();

    // Try to login with wrong password
    let login_user_data = LoginUser {
        email: email.clone(),
        password: "WrongSecurePass123!".to_string(),
    };

    let result = login_user(&mut conn, login_user_data).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        buildscale::error::Error::Authentication(msg) => {
            assert_eq!(msg, "Invalid email or password");
        }
        _ => panic!("Expected Authentication error"),
    }
}

#[tokio::test]
async fn test_login_empty_credentials() {
    let test_app = TestApp::new("test_login_empty_credentials").await;
    let mut conn = test_app.get_connection().await;

    // Test empty email
    let login_user_data = LoginUser {
        email: "".to_string(),
        password: "SomeSecurePass123!".to_string(),
    };

    let result = login_user(&mut conn, login_user_data).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), buildscale::error::Error::Validation(_)));

    // Test empty password
    let login_user_data = LoginUser {
        email: "test@example.com".to_string(),
        password: "".to_string(),
    };

    let result = login_user(&mut conn, login_user_data).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), buildscale::error::Error::Validation(_)));
}

#[tokio::test]
async fn test_session_validation() {
    let test_app = TestApp::new("test_session_validation").await;
    let mut conn = test_app.get_connection().await;

    // Register and login a user
    let register_user_data = test_app.generate_test_user();
    let password = register_user_data.password.clone();
    let email = register_user_data.email.clone();

    register_user(&mut conn, register_user_data).await.unwrap();

    let login_user_data = LoginUser {
        email: email.clone(),
        password: password.clone(),
    };

    let login_result = login_user(&mut conn, login_user_data).await.unwrap();

    // Test valid session token
    let validated_user = validate_session(&mut conn, &login_result.refresh_token).await.unwrap();
    assert_eq!(validated_user.id, login_result.user.id);
    assert_eq!(validated_user.email, login_result.user.email);
}

#[tokio::test]
async fn test_session_validation_invalid_token() {
    let test_app = TestApp::new("test_session_validation_invalid_token").await;
    let mut conn = test_app.get_connection().await;

    // Test invalid session token
    let result = validate_session(&mut conn, "invalid_session_token").await;
    assert!(result.is_err());

    match result.unwrap_err() {
        buildscale::error::Error::InvalidToken(msg) => {
            assert!(msg.contains("Invalid token") || msg.contains("session"));
        }
        _ => panic!("Expected InvalidToken error"),
    }
}

#[tokio::test]
async fn test_session_validation_empty_token() {
    let test_app = TestApp::new("test_session_validation_empty_token").await;
    let mut conn = test_app.get_connection().await;

    // Test empty session token
    let result = validate_session(&mut conn, "").await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), buildscale::error::Error::InvalidToken(_)));
}

#[tokio::test]
async fn test_logout() {
    let test_app = TestApp::new("test_logout").await;
    let mut conn = test_app.get_connection().await;

    // Register and login a user
    let register_user_data = test_app.generate_test_user();
    let password = register_user_data.password.clone();
    let email = register_user_data.email.clone();

    register_user(&mut conn, register_user_data).await.unwrap();

    let login_user_data = LoginUser {
        email: email.clone(),
        password: password.clone(),
    };

    let login_result = login_user(&mut conn, login_user_data).await.unwrap();

    // Logout the user
    logout_user(&mut conn, &login_result.refresh_token).await.unwrap();

    // Try to validate the session - should fail
    let result = validate_session(&mut conn, &login_result.refresh_token).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), buildscale::error::Error::InvalidToken(_)));
}

#[tokio::test]
async fn test_logout_invalid_token() {
    let test_app = TestApp::new("test_logout_invalid_token").await;
    let mut conn = test_app.get_connection().await;

    // Try to logout with invalid token
    let result = logout_user(&mut conn, "invalid_session_token").await;
    assert!(result.is_err());
    // Invalid format tokens return Validation error from validate_session_token
    assert!(matches!(result.unwrap_err(), buildscale::error::Error::Validation(_)));
}

#[tokio::test]
async fn test_session_refresh() {
    let test_app = TestApp::new("test_session_refresh").await;
    let mut conn = test_app.get_connection().await;

    // Register and login a user
    let register_user_data = test_app.generate_test_user();
    let password = register_user_data.password.clone();
    let email = register_user_data.email.clone();

    register_user(&mut conn, register_user_data).await.unwrap();

    let login_user_data = LoginUser {
        email: email.clone(),
        password: password.clone(),
    };

    let login_result = login_user(&mut conn, login_user_data).await.unwrap();

    // Refresh the session for 24 more hours
    let refreshed_token = refresh_session(&mut conn, &login_result.refresh_token, 24).await.unwrap();

    // The refreshed token should be the same (we refresh the same session)
    assert_eq!(refreshed_token, login_result.refresh_token);

    // Verify the session is still valid
    let validated_user = validate_session(&mut conn, &refreshed_token).await.unwrap();
    assert_eq!(validated_user.id, login_result.user.id);
}

#[tokio::test]
async fn test_session_refresh_invalid_token() {
    let test_app = TestApp::new("test_session_refresh_invalid_token").await;
    let mut conn = test_app.get_connection().await;

    // Try to refresh with invalid token
    let result = refresh_session(&mut conn, "invalid_session_token", 24).await;
    assert!(result.is_err());
    // Invalid format tokens return Validation error from validate_session_token
    assert!(matches!(result.unwrap_err(), buildscale::error::Error::Validation(_)));
}

#[tokio::test]
async fn test_session_refresh_empty_token() {
    let test_app = TestApp::new("test_session_refresh_empty_token").await;
    let mut conn = test_app.get_connection().await;

    // Try to refresh with empty token
    let result = refresh_session(&mut conn, "", 24).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), buildscale::error::Error::Validation(_)));
}

#[tokio::test]
async fn test_login_case_insensitive_email() {
    let test_app = TestApp::new("test_login_case_insensitive_email").await;
    let mut conn = test_app.get_connection().await;

    // Register a test user with lowercase email
    let register_user_data = test_app.generate_test_user();
    let password = register_user_data.password.clone();
    let email = register_user_data.email.clone();

    register_user(&mut conn, register_user_data).await.unwrap();

    // Try to login with uppercase email
    let login_user_data = LoginUser {
        email: email.to_uppercase(),
        password: password.clone(),
    };

    let login_result = login_user(&mut conn, login_user_data).await.unwrap();
    assert_eq!(login_result.user.email, email); // Should return the original case
}

#[tokio::test]
async fn test_cleanup_expired_sessions() {
    let test_app = TestApp::new("test_cleanup_expired_sessions").await;
    let mut conn = test_app.get_connection().await;

    // Register and login a user
    let register_user_data = test_app.generate_test_user();
    let password = register_user_data.password.clone();
    let email = register_user_data.email.clone();

    register_user(&mut conn, register_user_data).await.unwrap();

    let login_user_data = LoginUser {
        email: email.clone(),
        password: password.clone(),
    };

    let login_result = login_user(&mut conn, login_user_data).await.unwrap();

    // Manually expire the session by updating expires_at to past time
    let expired_time = Utc::now() - Duration::hours(1);
    use buildscale::queries::sessions::hash_session_token;
    let token_hash = hash_session_token(&login_result.refresh_token);
    sqlx::query!(
        "UPDATE user_sessions SET expires_at = $1 WHERE token_hash = $2",
        expired_time,
        token_hash
    )
    .execute(&mut *conn)
    .await
    .unwrap();

    // Run cleanup
    let cleaned_count = cleanup_expired_sessions(&mut conn).await.unwrap();
    assert!(cleaned_count >= 1);

    // Try to validate the session - should fail
    let result = validate_session(&mut conn, &login_result.refresh_token).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), buildscale::error::Error::InvalidToken(_)));
}