use backend::{
    models::users::{NewUser, UpdateUser},
    queries::users::{create_user, update_user},
};
use crate::common::database::TestDb;

#[tokio::test]
async fn test_update_user_query() {
    let test_db = TestDb::new("test_update_user_query").await;
    let mut conn = test_db.get_connection().await;

    // Create a user first
    let new_user = NewUser {
        email: format!("{}_update@example.com", test_db.test_prefix()),
        password_hash: "old_hash".to_string(),
        full_name: Some("Old Name".to_string()),
    };

    let created_user = create_user(&mut conn, new_user).await.unwrap();

    // Create update data
    let update_data = UpdateUser {
        email: Some(format!("{}_updated@example.com", test_db.test_prefix())),
        password_hash: Some("new_hash".to_string()),
        full_name: Some("New Name".to_string()),
    };

    // Update the user by modifying the created_user
    let mut user_to_update = created_user.clone();
    user_to_update.email = update_data.email.clone().unwrap();
    user_to_update.password_hash = update_data.password_hash.clone().unwrap();
    user_to_update.full_name = update_data.full_name.clone();

    let updated_user = update_user(&mut conn, &user_to_update).await.unwrap();

    assert_eq!(updated_user.id, created_user.id, "ID should not change");
    assert_eq!(
        updated_user.email,
        format!("{}_updated@example.com", test_db.test_prefix()),
        "Email should be updated"
    );
    assert_eq!(
        updated_user.password_hash, "new_hash",
        "Password hash should be updated"
    );
    assert_eq!(
        updated_user.full_name,
        Some("New Name".to_string()),
        "Full name should be updated"
    );
    assert!(
        updated_user.updated_at > created_user.updated_at,
        "Updated timestamp should be newer"
    );
}

#[tokio::test]
async fn test_update_user_partial() {
    let test_db = TestDb::new("test_update_user_partial").await;
    let mut conn = test_db.get_connection().await;

    // Create a user first
    let new_user = NewUser {
        email: format!("{}_partial_update@example.com", test_db.test_prefix()),
        password_hash: "hash123".to_string(),
        full_name: Some("Original Name".to_string()),
    };

    let created_user = create_user(&mut conn, new_user).await.unwrap();

    // Update only email
    let mut user_to_update = created_user.clone();
    user_to_update.email = format!("{}_new_email@example.com", test_db.test_prefix());

    let updated_user = update_user(&mut conn, &user_to_update).await.unwrap();

    assert_eq!(
        updated_user.email,
        format!("{}_new_email@example.com", test_db.test_prefix()),
        "Email should be updated"
    );
    assert_eq!(
        updated_user.password_hash, "hash123",
        "Password hash should remain unchanged"
    );
    assert_eq!(
        updated_user.full_name,
        Some("Original Name".to_string()),
        "Full name should remain unchanged"
    );
}