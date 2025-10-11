use backend::{
    models::users::NewUser,
    queries::users::{create_user, get_user_by_id},
};
use crate::common::database::TestDb;

#[tokio::test]
async fn test_user_field_constraints() {
    let test_db = TestDb::new("test_user_field_constraints").await;
    let mut conn = test_db.get_connection().await;

    // Test that email constraint works (unique constraint)
    let email = format!("{}_constraint@example.com", test_db.test_prefix());
    let new_user1 = NewUser {
        email: email.clone(),
        password_hash: "hash1".to_string(),
        full_name: None,
    };

    let new_user2 = NewUser {
        email: email.clone(),
        password_hash: "hash2".to_string(),
        full_name: None,
    };

    // First user should succeed
    create_user(&mut conn, new_user1).await.unwrap();

    // Second user with same email should fail
    let result = create_user(&mut conn, new_user2).await;
    assert!(
        result.is_err(),
        "Duplicate email should violate unique constraint"
    );
}

#[tokio::test]
async fn test_query_error_handling() {
    let test_db = TestDb::new("test_query_error_handling").await;
    let mut conn = test_db.get_connection().await;

    // Test getting non-existent user by ID
    let fake_id = uuid::Uuid::now_v7();
    let result = get_user_by_id(&mut conn, fake_id).await;

    assert!(result.is_err(), "Non-existent user should cause error");
}