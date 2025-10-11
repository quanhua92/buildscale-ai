use backend::{
    queries::users::list_users,
    services::users::{register_user, verify_password},
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
        "password123",
        "anotherpass456",
        "complex!@#$%^789",
        "veryveryverylongpassword123",
    ];

    let mut user_ids = Vec::new();

    for password in passwords {
        let register_user_data = test_app.generate_test_user_with_password(password);
        let created_user = register_user(&mut conn, register_user_data).await.unwrap();
        user_ids.push(created_user.id);

        // Verify password works for each user
        let is_valid = verify_password(password, &created_user.password_hash).unwrap();
        assert!(
            is_valid,
            "Password should verify for user {}",
            created_user.id
        );

        // Verify passwords don't match each other
        let is_different = !verify_password("wrongpassword", &created_user.password_hash).unwrap();
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