use buildscale::{
    models::users::NewUser,
    queries::users::create_user,
};
use crate::common::database::TestApp;

#[tokio::test]
async fn test_create_user_query() {
    let test_app = TestApp::new("test_create_user_query").await;
    let mut conn = test_app.get_connection().await;
    let register_user_data = test_app.generate_test_user();
    let email = register_user_data.email.clone();

    // Create a NewUser manually (bypassing service layer)
    let new_user = NewUser {
        email: email.clone(),
        password_hash: Some("test_hash_12345".to_string()),
        full_name: Some("Test User".to_string()),
    };

    // Test direct database insertion
    let created_user = create_user(&mut conn, new_user).await.unwrap();

    assert_eq!(created_user.email, email, "Email should match");
    assert_eq!(
        created_user.password_hash, Some("test_hash_12345".to_string()),
        "Password hash should match"
    );
    assert_eq!(
        created_user.full_name,
        Some("Test User".to_string()),
        "Full name should match"
    );
    assert!(
        !created_user.id.to_string().is_empty(),
        "ID should be populated"
    );
}