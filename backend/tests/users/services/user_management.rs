use buildscale::{
    queries::users::{get_user_by_id, list_users},
    queries::sessions::hash_session_token,
    services::users::{register_user, verify_password, update_password, get_session_info, is_email_available, get_user_active_sessions, revoke_all_user_sessions},
    models::users::LoginUser,
};
use crate::common::database::TestApp;

#[tokio::test]
async fn test_list_users() {
    let test_app = TestApp::new("test_list_users").await;
    let mut conn = test_app.get_connection().await;

    // Get initial test user count
    let initial_test_count = test_app.count_test_users().await.unwrap();

    // Register multiple users using TestApp helper
    let test_users = test_app.generate_list_test_users(3);
    let mut registered_emails = Vec::new();
    for user_data in test_users {
        registered_emails.push(user_data.email.clone());
        register_user(&mut conn, user_data).await.unwrap();
    }

    // Check that users are listed
    let all_users = list_users(&mut conn).await.unwrap();
    let final_test_count = test_app.count_test_users().await.unwrap();

    // Assert that we have exactly 3 more test users than before
    assert_eq!(
        final_test_count,
        initial_test_count + 3,
        "Should have 3 more test users"
    );

    // Verify our test users are in the list
    for email in registered_emails {
        let found = all_users.iter().any(|u| u.email == email);
        assert!(found, "Registered user {} should be in the list", email);
    }

    // Users should be ordered by created_at DESC (newest first)
    for i in 0..all_users.len() - 1 {
        assert!(
            all_users[i].created_at >= all_users[i + 1].created_at,
            "Users should be ordered by created_at DESC"
        );
    }
}

#[tokio::test]
async fn test_multiple_users_different_passwords() {
    let test_app = TestApp::new("test_multiple_users_different_passwords").await;
    let mut conn = test_app.get_connection().await;

    // Register users with different passwords
    let passwords = vec![
        "SecurePass123!",
        "AnotherPass456!",
        "complex!@#$%^789",
        "VeryVeryVeryLongSecure123!",
    ];

    let mut user_ids = Vec::new();

    for password in passwords {
        let register_user_data = test_app.generate_test_user_with_password(password);
        let created_user = register_user(&mut conn, register_user_data).await.unwrap();
        user_ids.push(created_user.id);

        // Verify password works for each user
        let is_valid = verify_password(password, created_user.password_hash.as_deref().unwrap()).unwrap();
        assert!(
            is_valid,
            "Password should verify for user {}",
            created_user.id
        );

        // Verify passwords don't match each other
        let is_different = !verify_password("wrongpassword", created_user.password_hash.as_deref().unwrap()).unwrap();
        assert!(is_different, "Wrong password should not verify");
    }

    // Verify all users have different password hashes
    let mut password_hashes = Vec::new();
    for user_id in user_ids {
        // Look up user by finding a user with matching ID in our test prefix
        let all_users = list_users(&mut conn).await.unwrap();
        if let Some(user) = all_users.iter().find(|u| u.id == user_id) {
            password_hashes.push(user.password_hash.clone());
        }
    }

    // All password hashes should be unique (even with same password, different salts)
    let unique_hashes: std::collections::HashSet<_> = password_hashes.iter().collect();
    assert_eq!(
        unique_hashes.len(),
        password_hashes.len(),
        "All password hashes should be unique"
    );
}

// Tests for get_user_by_id service method
#[tokio::test]
async fn test_get_user_by_id_success() {
    let test_app = TestApp::new("test_get_user_by_id_success").await;
    let mut conn = test_app.get_connection().await;

    // Register a test user
    let register_user_data = test_app.generate_test_user();
    let registered_user = register_user(&mut conn, register_user_data).await.unwrap();

    // Get user by ID
    let found_user = get_user_by_id(&mut conn, registered_user.id).await.unwrap();
    assert!(found_user.is_some());

    let found_user = found_user.unwrap();
    assert_eq!(found_user.id, registered_user.id);
    assert_eq!(found_user.email, registered_user.email);
    assert_eq!(found_user.full_name, registered_user.full_name);
}

#[tokio::test]
async fn test_get_user_by_id_not_found() {
    let test_app = TestApp::new("test_get_user_by_id_not_found").await;
    let mut conn = test_app.get_connection().await;

    // Try to get non-existent user
    let non_existent_id = uuid::Uuid::now_v7();
    let found_user = get_user_by_id(&mut conn, non_existent_id).await.unwrap();
    assert!(found_user.is_none());
}

// Tests for update_password service method
#[tokio::test]
async fn test_update_password_success() {
    let test_app = TestApp::new("test_update_password_success").await;
    let mut conn = test_app.get_connection().await;

    // Register a test user
    let register_user_data = test_app.generate_test_user();
    let original_password = register_user_data.password.clone();
    let registered_user = register_user(&mut conn, register_user_data).await.unwrap();

    // Update password
    let new_password = "newpassword123";
    update_password(&mut conn, registered_user.id, new_password).await.unwrap();

    // Verify the new password works by logging in
    let login_data = LoginUser {
        email: registered_user.email.clone(),
        password: new_password.to_string(),
    };

    let login_result = buildscale::services::users::login_user(&mut conn, login_data).await.unwrap();
    assert_eq!(login_result.user.id, registered_user.id);

    // Verify the old password no longer works
    let old_login_data = LoginUser {
        email: registered_user.email,
        password: original_password,
    };

    let result = buildscale::services::users::login_user(&mut conn, old_login_data).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_password_too_short() {
    let test_app = TestApp::new("test_update_password_too_short").await;
    let mut conn = test_app.get_connection().await;

    // Register a test user
    let register_user_data = test_app.generate_test_user();
    let registered_user = register_user(&mut conn, register_user_data).await.unwrap();

    // Try to update with too short password
    let result = update_password(&mut conn, registered_user.id, "short").await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), buildscale::error::Error::Validation(_)));
}

#[tokio::test]
async fn test_update_password_non_existent_user() {
    let test_app = TestApp::new("test_update_password_non_existent_user").await;
    let mut conn = test_app.get_connection().await;

    // Try to update password for non-existent user
    let non_existent_id = uuid::Uuid::now_v7();
    let result = update_password(&mut conn, non_existent_id, "newpassword123").await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), buildscale::error::Error::NotFound(_)));
}

// Tests for get_session_info service method
#[tokio::test]
async fn test_get_session_info_success() {
    let test_app = TestApp::new("test_get_session_info_success").await;
    let mut conn = test_app.get_connection().await;

    // Register and login a user
    let register_user_data = test_app.generate_test_user();
    let password = register_user_data.password.clone();
    let email = register_user_data.email.clone();

    register_user(&mut conn, register_user_data).await.unwrap();

    let login_data = LoginUser {
        email,
        password,
    };

    let login_result = buildscale::services::users::login_user(&mut conn, login_data).await.unwrap();

    // Get session info
    let session_info = get_session_info(&mut conn, &login_result.refresh_token).await.unwrap();
    assert!(session_info.is_some());

    let session_info = session_info.unwrap();
    // Cannot compare token_hash with login_result.refresh_token (different types)
    assert_eq!(session_info.user_id, login_result.user.id);
    assert!(session_info.expires_at > chrono::Utc::now());
}

#[tokio::test]
async fn test_get_session_info_not_found() {
    let test_app = TestApp::new("test_get_session_info_not_found").await;
    let mut conn = test_app.get_connection().await;

    // Try to get info for non-existent session
    let session_info = get_session_info(&mut conn, "non_existent_token").await.unwrap();
    assert!(session_info.is_none());
}

#[tokio::test]
async fn test_get_session_info_empty_token() {
    let test_app = TestApp::new("test_get_session_info_empty_token").await;
    let mut conn = test_app.get_connection().await;

    // Try to get info with empty token
    let result = get_session_info(&mut conn, "").await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), buildscale::error::Error::Validation(_)));
}

// Tests for is_email_available service method
#[tokio::test]
async fn test_is_email_available_true() {
    let test_app = TestApp::new("test_is_email_available_true").await;
    let mut conn = test_app.get_connection().await;

    // Check if email is available (should be true)
    let available = is_email_available(&mut conn, "newemail@example.com").await.unwrap();
    assert!(available);
}

#[tokio::test]
async fn test_is_email_available_false() {
    let test_app = TestApp::new("test_is_email_available_false").await;
    let mut conn = test_app.get_connection().await;

    // Register a test user
    let register_user_data = test_app.generate_test_user();
    let email = register_user_data.email.clone();
    register_user(&mut conn, register_user_data).await.unwrap();

    // Check if the same email is available (should be false)
    let available = is_email_available(&mut conn, &email).await.unwrap();
    assert!(!available);
}

#[tokio::test]
async fn test_is_email_available_case_insensitive() {
    let test_app = TestApp::new("test_is_email_available_case_insensitive").await;
    let mut conn = test_app.get_connection().await;

    // Register a test user with lowercase email
    let register_user_data = test_app.generate_test_user();
    let email = register_user_data.email.clone();
    register_user(&mut conn, register_user_data).await.unwrap();

    // Check if uppercase version is available (should be false)
    let available = is_email_available(&mut conn, &email.to_uppercase()).await.unwrap();
    assert!(!available);
}

#[tokio::test]
async fn test_is_email_available_invalid_format() {
    let test_app = TestApp::new("test_is_email_available_invalid_format").await;
    let mut conn = test_app.get_connection().await;

    // Check invalid email formats
    let invalid_emails = vec!["", "   ", "invalid-email", "nodomain@", "@nodomain.com"];

    for invalid_email in invalid_emails {
        let result = is_email_available(&mut conn, invalid_email).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), buildscale::error::Error::Validation(_)));
    }
}

// Tests for user service session management methods
#[tokio::test]
async fn test_get_user_active_sessions() {
    let test_app = TestApp::new("test_get_user_active_sessions").await;
    let mut conn = test_app.get_connection().await;

    // Register and login a user
    let register_user_data = test_app.generate_test_user();
    let password = register_user_data.password.clone();
    let email = register_user_data.email.clone();

    let registered_user = register_user(&mut conn, register_user_data).await.unwrap();

    // Create multiple sessions by logging in multiple times
    let mut session_tokens = Vec::new();
    for _i in 0..3 {
        let login_data = LoginUser {
            email: email.clone(),
            password: password.clone(),
        };

        let login_result = buildscale::services::users::login_user(&mut conn, login_data).await.unwrap();
        session_tokens.push(login_result.refresh_token);
    }

    // Get active sessions
    let active_sessions = get_user_active_sessions(&mut conn, registered_user.id).await.unwrap();
    assert_eq!(active_sessions.len(), 3);

    // Verify all our session tokens are in the list
    for token in &session_tokens {
        let found = active_sessions.iter().any(|s| s.token_hash == hash_session_token(token));
        assert!(found, "Session token {} should be in active sessions", token);
    }
}

#[tokio::test]
async fn test_revoke_all_user_sessions() {
    let test_app = TestApp::new("test_revoke_all_user_sessions").await;
    let mut conn = test_app.get_connection().await;

    // Register and login a user multiple times
    let register_user_data = test_app.generate_test_user();
    let password = register_user_data.password.clone();
    let email = register_user_data.email.clone();

    let registered_user = register_user(&mut conn, register_user_data).await.unwrap();

    // Create multiple sessions
    let mut session_tokens = Vec::new();
    for _ in 0..3 {
        let login_data = LoginUser {
            email: email.clone(),
            password: password.clone(),
        };

        let login_result = buildscale::services::users::login_user(&mut conn, login_data).await.unwrap();
        session_tokens.push(login_result.refresh_token);
    }

    // Verify sessions are active before revocation
    let active_sessions_before = get_user_active_sessions(&mut conn, registered_user.id).await.unwrap();
    assert_eq!(active_sessions_before.len(), 3);

    // Revoke all sessions
    let revoked_count = revoke_all_user_sessions(&mut conn, registered_user.id).await.unwrap();
    assert_eq!(revoked_count, 3);

    // Verify sessions are no longer active
    let active_sessions_after = get_user_active_sessions(&mut conn, registered_user.id).await.unwrap();
    assert_eq!(active_sessions_after.len(), 0);

    // Verify tokens are no longer valid
    for token in session_tokens {
        let result = buildscale::services::users::validate_session(&mut conn, &token).await;
        assert!(result.is_err());
    }
}