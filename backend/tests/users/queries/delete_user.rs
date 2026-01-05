use backend::{
    models::users::NewUser,
    queries::users::{create_user, delete_user},
};
use crate::common::database::TestDb;

#[tokio::test]
async fn test_delete_user_query() {
    let test_db = TestDb::new("test_delete_user_query").await;
    let mut conn = test_db.get_connection().await;

    // Use a different email to avoid cleanup conflicts
    let test_email = format!("{}_delete@example.com", test_db.test_prefix());

    // Create a user first
    let new_user = NewUser {
        email: test_email.to_string(),
        password_hash: Some("delete_hash".to_string()),
        full_name: None,
    };

    let created_user = create_user(&mut conn, new_user).await.unwrap();
    let user_id = created_user.id;

    // Verify user exists before deletion
    assert!(
        test_db.user_exists(&test_email).await.unwrap(),
        "User should exist before deletion"
    );

    let initial_count = test_db.count_test_users().await.unwrap();

    // Delete the user
    let rows_affected = delete_user(&mut conn, user_id).await.unwrap();
    assert_eq!(rows_affected, 1, "Should delete exactly 1 row");

    // Verify user no longer exists
    assert!(
        !test_db.user_exists(&test_email).await.unwrap(),
        "User should not exist after deletion"
    );

    let final_count = test_db.count_test_users().await.unwrap();
    assert_eq!(
        final_count,
        initial_count - 1,
        "Test user count should decrease by 1 after deletion"
    );
}

#[tokio::test]
async fn test_delete_nonexistent_user() {
    let test_db = TestDb::new("test_delete_nonexistent_user").await;
    let mut conn = test_db.get_connection().await;

    let fake_id = uuid::Uuid::now_v7();
    let rows_affected = delete_user(&mut conn, fake_id).await.unwrap();
    assert_eq!(
        rows_affected, 0,
        "Should delete 0 rows for non-existent user"
    );
}