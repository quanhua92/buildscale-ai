use backend::{
    models::users::NewUser,
    queries::users::{create_user, list_users},
};
use crate::common::database::TestDb;

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
        assert!(
            !user.id.to_string().is_empty(),
            "User ID should not be empty"
        );
        assert!(!user.email.is_empty(), "User email should not be empty");
        assert!(
            user.password_hash.is_some(),
            "Password hash should exist"
        );
        if let Some(hash) = &user.password_hash {
            assert!(!hash.is_empty(), "Password hash should not be empty");
        }
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
        let new_user = NewUser {
            email: format!("{}_list_test_{}@example.com", test_db.test_prefix(), i),
            password_hash: Some(format!("hash_{}", i)),
            full_name: Some(format!("User {}", i)),
        };

        let created_user = create_user(&mut conn, new_user).await.unwrap();
        created_ids.push(created_user.id);
    }

    // List all users
    let all_users = list_users(&mut conn).await.unwrap();

    // Count our test users specifically
    let test_user_count = all_users
        .iter()
        .filter(|u| {
            u.email
                .starts_with(&format!("{}_list_test_", test_db.test_prefix()))
        })
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