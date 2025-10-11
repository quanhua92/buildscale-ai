use backend::{
    queries::users::get_user_by_email,
    services::users::{register_user, verify_password},
};
use crate::common::database::TestApp;

#[tokio::test]
async fn test_password_verification() {
    let test_app = TestApp::new("test_password_verification").await;
    let mut conn = test_app.get_connection().await;

    let register_user_data = test_app.generate_test_user();
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
    let test_app = TestApp::new("test_user_lookup_by_email").await;
    let mut conn = test_app.get_connection().await;

    let register_user_data = test_app.generate_test_user();
    let user_email = register_user_data.email.clone();

    // Register user
    let created_user = register_user(&mut conn, register_user_data).await.unwrap();

    // Test successful lookup
    let found_user = get_user_by_email(&mut conn, &user_email).await.unwrap();
    assert!(found_user.is_some(), "User should be found by email");

    let found_user = found_user.unwrap();
    assert_eq!(found_user.id, created_user.id, "Found user ID should match");
    assert_eq!(
        found_user.email, user_email,
        "Found user email should match"
    );

    // Test lookup of non-existent user
    let not_found = get_user_by_email(&mut conn, "nonexistent@example.com")
        .await
        .unwrap();
    assert!(not_found.is_none(), "Non-existent user should not be found");
}