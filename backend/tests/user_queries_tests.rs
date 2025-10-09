mod common;

use backend::{
    models::users::UpdateUser,
    queries::users::{create_user, delete_user, get_user_by_id, list_users, update_user},
};
use common::database::{TestApp, TestDb};

#[tokio::test]
async fn test_create_user_query() {
    let test_app = TestApp::new("test_create_user_query").await;
    let mut conn = test_app.get_connection().await;
    let register_user_data = test_app.generate_test_user();
    let email = register_user_data.email.clone();

    // Create a NewUser manually (bypassing service layer)
    let new_user = backend::models::users::NewUser {
        email: email.clone(),
        password_hash: "test_hash_12345".to_string(),
        full_name: Some("Test User".to_string()),
    };

    // Test direct database insertion
    let created_user = create_user(&mut conn, new_user).await.unwrap();

    assert_eq!(created_user.email, email, "Email should match");
    assert_eq!(created_user.password_hash, "test_hash_12345", "Password hash should match");
    assert_eq!(created_user.full_name, Some("Test User".to_string()), "Full name should match");
    assert!(created_user.id.to_string().len() > 0, "ID should be populated");
}

#[tokio::test]
async fn test_get_user_by_id_query() {
    let test_db = TestDb::new("test_get_user_by_id_query").await;
    let mut conn = test_db.get_connection().await;

    // Create a user first
    let new_user = backend::models::users::NewUser {
        email: format!("{}_get_by_id@example.com", test_db.test_prefix()),
        password_hash: "test_hash".to_string(),
        full_name: Some("Test User".to_string()),
    };

    let created_user = create_user(&mut conn, new_user).await.unwrap();
    let user_id = created_user.id;

    // Test getting user by ID
    let found_user = get_user_by_id(&mut conn, user_id).await.unwrap();

    assert_eq!(found_user.id, user_id, "User ID should match");
    assert_eq!(found_user.email, format!("{}_get_by_id@example.com", test_db.test_prefix()), "Email should match");
    assert_eq!(found_user.full_name, Some("Test User".to_string()), "Full name should match");
}

#[tokio::test]
async fn test_get_user_by_id_not_found() {
    let test_db = TestDb::new("test_get_user_by_id_not_found").await;
    let mut conn = test_db.get_connection().await;

    // Test with non-existent UUID
    let fake_id = uuid::Uuid::now_v7();
    let result = get_user_by_id(&mut conn, fake_id).await;

    assert!(result.is_err(), "Should return error for non-existent user");
    let error = result.unwrap_err();
    let error_message = error.to_string();
    assert!(error_message.contains("no rows") || error_message.contains("found"),
           "Error should indicate user not found: {}", error_message);
}

#[tokio::test]
async fn test_update_user_query() {
    let test_db = TestDb::new("test_update_user_query").await;
    let mut conn = test_db.get_connection().await;

    // Create a user first
    let new_user = backend::models::users::NewUser {
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
    assert_eq!(updated_user.email, format!("{}_updated@example.com", test_db.test_prefix()), "Email should be updated");
    assert_eq!(updated_user.password_hash, "new_hash", "Password hash should be updated");
    assert_eq!(updated_user.full_name, Some("New Name".to_string()), "Full name should be updated");
    assert!(updated_user.updated_at > created_user.updated_at, "Updated timestamp should be newer");
}

#[tokio::test]
async fn test_update_user_partial() {
    let test_db = TestDb::new("test_update_user_partial").await;
    let mut conn = test_db.get_connection().await;

    // Create a user first
    let new_user = backend::models::users::NewUser {
        email: format!("{}_partial_update@example.com", test_db.test_prefix()),
        password_hash: "hash123".to_string(),
        full_name: Some("Original Name".to_string()),
    };

    let created_user = create_user(&mut conn, new_user).await.unwrap();

    // Update only email
    let mut user_to_update = created_user.clone();
    user_to_update.email = format!("{}_new_email@example.com", test_db.test_prefix());

    let updated_user = update_user(&mut conn, &user_to_update).await.unwrap();

    assert_eq!(updated_user.email, format!("{}_new_email@example.com", test_db.test_prefix()), "Email should be updated");
    assert_eq!(updated_user.password_hash, "hash123", "Password hash should remain unchanged");
    assert_eq!(updated_user.full_name, Some("Original Name".to_string()), "Full name should remain unchanged");
}

#[tokio::test]
async fn test_delete_user_query() {
    let test_db = TestDb::new("test_delete_user_query").await;
    let mut conn = test_db.get_connection().await;

    // Use a different email to avoid cleanup conflicts
    let test_email = format!("{}_delete@example.com", test_db.test_prefix());

    // Create a user first
    let new_user = backend::models::users::NewUser {
        email: test_email.to_string(),
        password_hash: "delete_hash".to_string(),
        full_name: None,
    };

    let created_user = create_user(&mut conn, new_user).await.unwrap();
    let user_id = created_user.id;

    // Verify user exists before deletion
    assert!(test_db.user_exists(&test_email).await.unwrap(),
           "User should exist before deletion");

    let initial_count = test_db.count_test_users().await.unwrap();

    // Delete the user
    let rows_affected = delete_user(&mut conn, user_id).await.unwrap();
    assert_eq!(rows_affected, 1, "Should delete exactly 1 row");

    // Verify user no longer exists
    assert!(!test_db.user_exists(&test_email).await.unwrap(),
            "User should not exist after deletion");

    let final_count = test_db.count_test_users().await.unwrap();
    assert_eq!(final_count, initial_count - 1, "Test user count should decrease by 1 after deletion");
}

#[tokio::test]
async fn test_delete_nonexistent_user() {
    let test_db = TestDb::new("test_delete_nonexistent_user").await;
    let mut conn = test_db.get_connection().await;

    let fake_id = uuid::Uuid::now_v7();
    let rows_affected = delete_user(&mut conn, fake_id).await.unwrap();
    assert_eq!(rows_affected, 0, "Should delete 0 rows for non-existent user");
}

#[tokio::test]
async fn test_list_users_empty() {
    let test_db = TestDb::new("test_list_users_empty").await;
    let mut conn = test_db.get_connection().await;

    // Get current users count
    let current_users = list_users(&mut conn).await.unwrap();
    let _current_count = current_users.len();

    // Verify list_users works even when there are no test users
    assert!(!current_users.is_empty(), "Should be able to list users");

    // If there are users, they should have valid fields
    for user in current_users {
        assert!(!user.id.to_string().is_empty(), "User ID should not be empty");
        assert!(!user.email.is_empty(), "User email should not be empty");
        assert!(!user.password_hash.is_empty(), "Password hash should not be empty");
    }
}

#[tokio::test]
async fn test_list_users_with_multiple_users() {
    let test_db = TestDb::new("test_list_users_with_multiple_users").await;
    let mut conn = test_db.get_connection().await;

    // Get initial user count
    let initial_users = list_users(&mut conn).await.unwrap();
    let _initial_count = initial_users.len();

    // Create multiple users
    let mut created_ids = Vec::new();
    for i in 0..5 {
        let new_user = backend::models::users::NewUser {
            email: format!("{}_list_test_{}@example.com", test_db.test_prefix(), i),
            password_hash: format!("hash_{}", i),
            full_name: Some(format!("User {}", i)),
        };

        let created_user = create_user(&mut conn, new_user).await.unwrap();
        created_ids.push(created_user.id);
    }

    // List all users
    let all_users = list_users(&mut conn).await.unwrap();

    // Count our test users specifically
    let test_user_count = all_users.iter()
        .filter(|u| u.email.starts_with(&format!("{}_list_test_", test_db.test_prefix())))
        .count();

    assert_eq!(test_user_count, 5, "Should have 5 test users");

    // Verify our created users are in the list
    for id in created_ids {
        let found = all_users.iter().any(|u| u.id == id);
        assert!(found, "Created user {} should be in the list", id);
    }

    // Verify ordering (newest first)
    for i in 0..all_users.len() - 1 {
        assert!(
            all_users[i].created_at >= all_users[i + 1].created_at,
            "Users should be ordered by created_at DESC"
        );
    }
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

#[tokio::test]
async fn test_user_field_constraints() {
    let test_db = TestDb::new("test_user_field_constraints").await;
    let mut conn = test_db.get_connection().await;

    // Test that email constraint works (unique constraint)
    let email = format!("{}_constraint@example.com", test_db.test_prefix());
    let new_user1 = backend::models::users::NewUser {
        email: email.clone(),
        password_hash: "hash1".to_string(),
        full_name: None,
    };

    let new_user2 = backend::models::users::NewUser {
        email: email.clone(),
        password_hash: "hash2".to_string(),
        full_name: None,
    };

    // First user should succeed
    create_user(&mut conn, new_user1).await.unwrap();

    // Second user with same email should fail
    let result = create_user(&mut conn, new_user2).await;
    assert!(result.is_err(), "Duplicate email should violate unique constraint");
}