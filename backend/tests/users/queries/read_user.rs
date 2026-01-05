use backend::{
    models::users::NewUser,
    queries::users::{create_user, get_user_by_id},
};
use crate::common::database::TestDb;

#[tokio::test]
async fn test_get_user_by_id_query() {
    let test_db = TestDb::new("test_get_user_by_id_query").await;
    let mut conn = test_db.get_connection().await;

    // Create a user first
    let new_user = NewUser {
        email: format!("{}_get_by_id@example.com", test_db.test_prefix()),
        password_hash: "test_hash".to_string(),
        full_name: Some("Test User".to_string()),
    };

    let created_user = create_user(&mut conn, new_user).await.unwrap();
    let user_id = created_user.id;

    // Test getting user by ID
    let found_user = get_user_by_id(&mut conn, user_id).await.unwrap().unwrap();

    assert_eq!(found_user.id, user_id, "User ID should match");
    assert_eq!(
        found_user.email,
        format!("{}_get_by_id@example.com", test_db.test_prefix()),
        "Email should match"
    );
    assert_eq!(
        found_user.full_name,
        Some("Test User".to_string()),
        "Full name should match"
    );
}

#[tokio::test]
async fn test_get_user_by_id_not_found() {
    let test_db = TestDb::new("test_get_user_by_id_not_found").await;
    let mut conn = test_db.get_connection().await;

    // Test with non-existent UUID
    let fake_id = uuid::Uuid::now_v7();
    let result = get_user_by_id(&mut conn, fake_id).await.unwrap();

    assert!(result.is_none(), "Should return None for non-existent user");
}