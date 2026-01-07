use buildscale::{
    queries::users::get_user_by_email,
    services::users::register_user,
};
use crate::common::database::TestApp;

#[tokio::test]
async fn test_user_registration_success() {
    let test_app = TestApp::new("test_user_registration_success").await;
    let mut conn = test_app.get_connection().await;

    let initial_count = test_app.count_test_users().await.unwrap();

    let register_user_data = test_app.generate_test_user();
    let user_email = register_user_data.email.clone();

    // Test successful user registration
    let result = register_user(&mut conn, register_user_data).await;
    assert!(result.is_ok(), "User registration should succeed");

    let created_user = result.unwrap();
    assert!(
        !created_user.id.to_string().is_empty(),
        "User should have a valid UUID"
    );
    assert_eq!(created_user.email, user_email.to_lowercase(), "Email should be stored in lowercase");
    assert!(
        created_user.password_hash.is_some(),
        "Password hash should exist"
    );
    if let Some(hash) = &created_user.password_hash {
        assert!(!hash.is_empty(), "Password hash should not be empty");
    }
    assert!(
        created_user.full_name.is_none(),
        "Full name should be None by default"
    );
    assert!(
        created_user.created_at <= chrono::Utc::now(),
        "Created timestamp should be valid"
    );

    let final_count = test_app.count_test_users().await.unwrap();
    assert_eq!(
        final_count,
        initial_count + 1,
        "User count should increase by 1"
    );

    // Verify user exists in database
    assert!(
        test_app.user_exists(&user_email).await.unwrap(),
        "User should exist in database"
    );
}

#[tokio::test]
async fn test_password_mismatch_validation() {
    let test_app = TestApp::new("test_password_mismatch_validation").await;
    let mut conn = test_app.get_connection().await;

    let mut register_user_data = test_app.generate_test_user();
    register_user_data.confirm_password = "differentpassword".to_string();

    // Test password mismatch validation
    let result = register_user(&mut conn, register_user_data).await;
    assert!(
        result.is_err(),
        "Password mismatch should cause registration to fail"
    );

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
    let test_app = TestApp::new("test_short_password_validation").await;
    let mut conn = test_app.get_connection().await;

    let register_user_data = test_app.generate_test_user_with_password("short");

    // Test short password validation
    let result = register_user(&mut conn, register_user_data).await;
    assert!(
        result.is_err(),
        "Short password should cause registration to fail"
    );

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
    let test_app = TestApp::new("test_minimum_valid_password_length").await;
    let mut conn = test_app.get_connection().await;

    // Test exactly 8 characters (should succeed)
    let register_user_data = test_app.generate_test_user_with_password("valid123");
    let result = register_user(&mut conn, register_user_data).await;
    assert!(result.is_ok(), "8-character password should be valid");
}

#[tokio::test]
async fn test_duplicate_email_validation() {
    let test_app = TestApp::new("test_duplicate_email_validation").await;
    let mut conn = test_app.get_connection().await;

    let email = test_app.generate_test_user().email;
    let mut register_user_data = test_app.generate_test_user();
    register_user_data.email = email.clone();

    // Register first user
    register_user(&mut conn, register_user_data).await.unwrap();

    // Try to register second user with same email
    let mut duplicate_user_data = test_app.generate_test_user();
    duplicate_user_data.email = email.clone();
    let result = register_user(&mut conn, duplicate_user_data).await;
    assert!(
        result.is_err(),
        "Duplicate email should cause registration to fail"
    );

    let error = result.unwrap_err();
    let error_message = error.to_string();
    // Should be a Conflict error (409), not a generic database error (500)
    assert!(
        error_message.contains("Conflict")
            || error_message.contains("already exists"),
        "Error should be a Conflict error (409), not generic database error: {}",
        error_message
    );
}

#[tokio::test]
async fn test_user_fields_are_populated() {
    let test_app = TestApp::new("test_user_fields_are_populated").await;
    let mut conn = test_app.get_connection().await;

    let register_user_data = test_app.generate_test_user();
    let email = register_user_data.email.clone();

    // Register user
    let created_user = register_user(&mut conn, register_user_data).await.unwrap();

    // Verify all required fields are populated
    assert!(
        !created_user.id.to_string().is_empty(),
        "ID should be populated"
    );
    assert_eq!(created_user.email, email.to_lowercase(), "Email should be stored in lowercase");
    assert!(
        created_user.password_hash.is_some(),
        "Password hash should be populated"
    );
    if let Some(hash) = &created_user.password_hash {
        assert!(!hash.is_empty(), "Password hash should not be empty");
    }
    assert!(
        created_user.full_name.is_none(),
        "Full name should be None by default"
    );

    // Verify timestamps
    assert!(
        created_user.created_at.naive_utc().and_utc().timestamp() > 0,
        "Created timestamp should be valid"
    );
    assert!(
        created_user.updated_at.naive_utc().and_utc().timestamp() > 0,
        "Updated timestamp should be valid"
    );

    // Verify password hash format (should be Argon2 format)
    let hash = created_user.password_hash.as_ref().unwrap();
    assert!(
        hash.starts_with("$argon2"),
        "Password hash should be Argon2 format"
    );
    assert!(
        hash.len() > 50,
        "Password hash should be substantial length"
    );
}

#[tokio::test]
async fn test_edge_case_email_addresses() {
    let test_app = TestApp::new("test_edge_case_email_addresses").await;
    let mut conn = test_app.get_connection().await;

    // Test various email formats using TestApp helper
    let edge_case_users = test_app.generate_edge_case_users();

    for user_data in edge_case_users {
        let email = user_data.email.clone();
        let result = register_user(&mut conn, user_data).await;
        assert!(result.is_ok(), "Email '{}' should be valid", email);

        let created_user = result.unwrap();
        // Email should be stored in lowercase (email addresses are case-insensitive)
        assert_eq!(
            created_user.email, email.to_lowercase(),
            "Email should be stored in lowercase"
        );
    }
}

#[tokio::test]
async fn test_database_transaction_isolation() {
    let test_app = TestApp::new("test_database_transaction_isolation").await;
    let mut conn = test_app.get_connection().await;

    let initial_count = test_app.count_test_users().await.unwrap();

    // Start a transaction
    let mut tx = test_app.test_db.pool.begin().await.unwrap();

    // Register user within transaction
    let register_user_data = test_app.generate_test_user();
    let user_email = register_user_data.email.clone();

    // Use transaction connection
    let _user = register_user(tx.as_mut(), register_user_data)
        .await
        .unwrap();

    // User should exist within transaction
    let found_in_tx = get_user_by_email(tx.as_mut(), &user_email).await.unwrap();
    assert!(
        found_in_tx.is_some(),
        "User should exist within transaction"
    );

    // User should NOT exist outside transaction yet
    let found_outside = get_user_by_email(&mut conn, &user_email).await.unwrap();
    assert!(
        found_outside.is_none(),
        "User should not exist outside transaction before commit"
    );

    // Commit transaction
    tx.commit().await.unwrap();

    // Now user should exist outside transaction
    let found_after_commit = get_user_by_email(&mut conn, &user_email).await.unwrap();
    assert!(
        found_after_commit.is_some(),
        "User should exist after transaction commit"
    );

    let final_count = test_app.count_test_users().await.unwrap();
    assert_eq!(
        final_count,
        initial_count + 1,
        "User count should increase after commit"
    );
}