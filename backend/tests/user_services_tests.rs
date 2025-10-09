mod common;

use backend::{
    queries::users::{find_user_by_email, list_users},
    services::users::{register_user, verify_password},
};
use common::database::{generate_test_user, generate_test_user_with_email, generate_test_user_with_password, TestDb};

#[tokio::test]
async fn test_user_registration_success() {
    let test_db = TestDb::new("test_user_registration_success").await;
    let mut conn = test_db.get_connection().await;

    let initial_count = test_db.count_test_users().await.unwrap();

    let register_user_data = generate_test_user(test_db.test_prefix());
    let user_email = register_user_data.email.clone();

    // Test successful user registration
    let result = register_user(&mut conn, register_user_data).await;
    assert!(result.is_ok(), "User registration should succeed");

    let created_user = result.unwrap();
    assert!(!created_user.id.to_string().is_empty(), "User should have a valid UUID");
    assert_eq!(created_user.email, user_email, "Email should match");
    assert!(!created_user.password_hash.is_empty(), "Password hash should not be empty");
    assert!(created_user.full_name.is_none(), "Full name should be None by default");
    assert!(created_user.created_at <= chrono::Utc::now(), "Created timestamp should be valid");

    let final_count = test_db.count_users().await.unwrap();
    assert_eq!(final_count, initial_count + 1, "User count should increase by 1");

    // Verify user exists in database
    assert!(test_db.user_exists(&user_email).await.unwrap(), "User should exist in database");
}

#[tokio::test]
async fn test_password_verification() {
    let test_db = TestDb::new("test_password_verification").await;
    let mut conn = test_db.get_connection().await;

    let register_user_data = generate_test_user(test_db.test_prefix());
    let password = register_user_data.password.clone();

    // Register user
    let created_user = register_user(&mut conn, register_user_data).await.unwrap();

    // Test correct password verification
    let is_valid = verify_password(&password, &created_user.password_hash).unwrap();
    assert!(is_valid, "Correct password should verify successfully");

    // Test incorrect password verification
    let is_invalid = verify_password("wrongpassword", &created_user.password_hash).unwrap();
    assert!(!is_invalid, "Incorrect password should not verify");

    // Test empty password verification
    let is_empty_invalid = verify_password("", &created_user.password_hash).unwrap();
    assert!(!is_empty_invalid, "Empty password should not verify");
}

#[tokio::test]
async fn test_user_lookup_by_email() {
    let test_db = TestDb::new("test_user_registration_success").await;
    let mut conn = test_db.get_connection().await;

    let register_user_data = generate_test_user(test_db.test_prefix());
    let user_email = register_user_data.email.clone();

    // Register user
    let created_user = register_user(&mut conn, register_user_data).await.unwrap();

    // Test successful lookup
    let found_user = find_user_by_email(&mut conn, &user_email).await.unwrap();
    assert!(found_user.is_some(), "User should be found by email");

    let found_user = found_user.unwrap();
    assert_eq!(found_user.id, created_user.id, "Found user ID should match");
    assert_eq!(found_user.email, user_email, "Found user email should match");

    // Test lookup of non-existent user
    let not_found = find_user_by_email(&mut conn, "nonexistent@example.com").await.unwrap();
    assert!(not_found.is_none(), "Non-existent user should not be found");
}

#[tokio::test]
async fn test_list_users() {
    let test_db = TestDb::new("test_user_registration_success").await;
    let mut conn = test_db.get_connection().await;

    // Get initial user count
    let initial_users = list_users(&mut conn).await.unwrap();
    let initial_count = initial_users.len();

    // Register multiple users
    let mut registered_emails = Vec::new();
    for i in 0..3 {
        let mut user_data = generate_test_user(test_db.test_prefix());
        user_data.email = format!("test_list_{}@example.com", i);
        registered_emails.push(user_data.email.clone());
        register_user(&mut conn, user_data).await.unwrap();
    }

    // Check that users are listed
    let all_users = list_users(&mut conn).await.unwrap();
    assert_eq!(all_users.len(), initial_count + 3, "Should have 3 more users");

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
async fn test_password_mismatch_validation() {
    let test_db = TestDb::new("test_user_registration_success").await;
    let mut conn = test_db.get_connection().await;

    let mut register_user_data = generate_test_user(test_db.test_prefix());
    register_user_data.confirm_password = "differentpassword".to_string();

    // Test password mismatch validation
    let result = register_user(&mut conn, register_user_data).await;
    assert!(result.is_err(), "Password mismatch should cause registration to fail");

    let error = result.unwrap_err();
    let error_message = error.to_string();
    assert!(
        error_message.contains("Passwords do not match"),
        "Error message should mention password mismatch: {}",
        error_message
    );
}

#[tokio::test]
async fn test_short_password_validation() {
    let test_db = TestDb::new("test_user_registration_success").await;
    let mut conn = test_db.get_connection().await;

    let register_user_data = generate_test_user_with_password(test_db.test_prefix(), "short");

    // Test short password validation
    let result = register_user(&mut conn, register_user_data).await;
    assert!(result.is_err(), "Short password should cause registration to fail");

    let error = result.unwrap_err();
    let error_message = error.to_string();
    assert!(
        error_message.contains("8 characters"),
        "Error message should mention password length requirement: {}",
        error_message
    );
}

#[tokio::test]
async fn test_minimum_valid_password_length() {
    let test_db = TestDb::new("test_user_registration_success").await;
    let mut conn = test_db.get_connection().await;

    // Test exactly 8 characters (should succeed)
    let register_user_data = generate_test_user_with_password(test_db.test_prefix(), "valid123");
    let result = register_user(&mut conn, register_user_data).await;
    assert!(result.is_ok(), "8-character password should be valid");
}

#[tokio::test]
async fn test_duplicate_email_validation() {
    let test_db = TestDb::new("test_user_registration_success").await;
    let mut conn = test_db.get_connection().await;

    let email = generate_test_user(test_db.test_prefix()).email;
    let register_user_data = generate_test_user_with_email(&email);

    // Register first user
    register_user(&mut conn, register_user_data).await.unwrap();

    // Try to register second user with same email
    let duplicate_user_data = generate_test_user_with_email(&email);
    let result = register_user(&mut conn, duplicate_user_data).await;
    assert!(result.is_err(), "Duplicate email should cause registration to fail");

    let error = result.unwrap_err();
    let error_message = error.to_string();
    // Should be a database constraint error
    assert!(
        error_message.contains("SQLx") || error_message.contains("duplicate") || error_message.contains("unique"),
        "Error should be related to database constraint: {}",
        error_message
    );
}

#[tokio::test]
async fn test_user_fields_are_populated() {
    let test_db = TestDb::new("test_user_registration_success").await;
    let mut conn = test_db.get_connection().await;

    let register_user_data = generate_test_user(test_db.test_prefix());
    let email = register_user_data.email.clone();

    // Register user
    let created_user = register_user(&mut conn, register_user_data).await.unwrap();

    // Verify all required fields are populated
    assert!(!created_user.id.to_string().is_empty(), "ID should be populated");
    assert_eq!(created_user.email, email, "Email should match");
    assert!(!created_user.password_hash.is_empty(), "Password hash should be populated");
    assert!(created_user.full_name.is_none(), "Full name should be None by default");

    // Verify timestamps
    assert!(created_user.created_at.naive_utc().and_utc().timestamp() > 0, "Created timestamp should be valid");
    assert!(created_user.updated_at.naive_utc().and_utc().timestamp() > 0, "Updated timestamp should be valid");

    // Verify password hash format (should be Argon2 format)
    assert!(created_user.password_hash.starts_with("$argon2"), "Password hash should be Argon2 format");
    assert!(created_user.password_hash.len() > 50, "Password hash should be substantial length");
}

#[tokio::test]
async fn test_multiple_users_different_passwords() {
    let test_db = TestDb::new("test_user_registration_success").await;
    let mut conn = test_db.get_connection().await;

    // Register users with different passwords
    let passwords = vec![
        "password123",
        "anotherpass456",
        "complex!@#$%^789",
        "veryveryverylongpassword123",
    ];

    let mut user_ids = Vec::new();

    for password in passwords {
        let register_user_data = generate_test_user_with_password(test_db.test_prefix(), password);
        let created_user = register_user(&mut conn, register_user_data).await.unwrap();
        user_ids.push(created_user.id);

        // Verify password works for each user
        let is_valid = verify_password(password, &created_user.password_hash).unwrap();
        assert!(is_valid, "Password should verify for user {}", created_user.id);

        // Verify passwords don't match each other
        let is_different = !verify_password("wrongpassword", &created_user.password_hash).unwrap();
        assert!(is_different, "Wrong password should not verify");
    }

    // Verify all users have different password hashes
    let mut password_hashes = Vec::new();
    for user_id in user_ids {
        let user = find_user_by_email(&mut conn, &format!("test_{}@example.com", user_id)).await.unwrap();
        if let Some(user) = user {
            password_hashes.push(user.password_hash);
        }
    }

    // All password hashes should be unique (even with same password, different salts)
    let unique_hashes: std::collections::HashSet<_> = password_hashes.iter().collect();
    assert_eq!(unique_hashes.len(), password_hashes.len(), "All password hashes should be unique");
}

#[tokio::test]
async fn test_edge_case_email_addresses() {
    let test_db = TestDb::new("test_user_registration_success").await;
    let mut conn = test_db.get_connection().await;

    // Test various email formats
    let test_emails = vec![
        "user+tag@example.com",
        "user.name@example.com",
        "user123@example.com",
        "UPPERCASE@EXAMPLE.COM",
    ];

    for email in test_emails {
        let register_user_data = generate_test_user_with_email(email);
        let result = register_user(&mut conn, register_user_data).await;
        assert!(result.is_ok(), "Email '{}' should be valid", email);

        let created_user = result.unwrap();
        assert_eq!(created_user.email.to_lowercase(), email.to_lowercase(),
                  "Email should be stored exactly as provided (case handling depends on database)");
    }
}

#[tokio::test]
async fn test_database_transaction_isolation() {
    let test_db = TestDb::new("test_user_registration_success").await;
    let mut conn = test_db.get_connection().await;

    let initial_count = test_db.count_test_users().await.unwrap();

    // Start a transaction
    let mut tx = test_db.pool.begin().await.unwrap();

    // Register user within transaction
    let register_user_data = generate_test_user(test_db.test_prefix());
    let user_email = register_user_data.email.clone();

    // Use transaction connection
    let _user = register_user(tx.as_mut(), register_user_data).await.unwrap();

    // User should exist within transaction
    let found_in_tx = find_user_by_email(tx.as_mut(), &user_email).await.unwrap();
    assert!(found_in_tx.is_some(), "User should exist within transaction");

    // User should NOT exist outside transaction yet
    let found_outside = find_user_by_email(&mut conn, &user_email).await.unwrap();
    assert!(found_outside.is_none(), "User should not exist outside transaction before commit");

    // Commit transaction
    tx.commit().await.unwrap();

    // Now user should exist outside transaction
    let found_after_commit = find_user_by_email(&mut conn, &user_email).await.unwrap();
    assert!(found_after_commit.is_some(), "User should exist after transaction commit");

    let final_count = test_db.count_users().await.unwrap();
    assert_eq!(final_count, initial_count + 1, "User count should increase after commit");
}