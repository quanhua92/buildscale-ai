use buildscale::{
    models::users::NewUserSession,
    queries::sessions::{
        create_session, get_session_by_token_hash, get_sessions_by_user,
        delete_session, delete_session_by_token_hash, delete_sessions_by_user, delete_expired_sessions,
        is_session_valid, get_valid_session_by_token_hash, refresh_session, hash_session_token
    },
    services::users::register_user,
};
use crate::common::database::TestApp;
use chrono::{Duration, Utc};
use uuid::Uuid;

#[tokio::test]
async fn test_create_session() {
    let test_app = TestApp::new("test_create_session").await;
    let mut conn = test_app.get_connection().await;

    // First create a user
    let register_user_data = test_app.generate_test_user();
    let user = register_user(&mut conn, register_user_data).await.unwrap();

    let token = Uuid::now_v7().to_string();
    let expires_at = Utc::now() + Duration::hours(24);

    let new_session = NewUserSession {
        user_id: user.id,
        token_hash: hash_session_token(&token),
        expires_at,
    };

    let created_session = create_session(&mut conn, new_session).await.unwrap();

    assert_eq!(created_session.user_id, user.id);
    assert_eq!(created_session.token_hash, hash_session_token(&token));
    assert_eq!(created_session.expires_at, expires_at);
    assert!(!created_session.id.to_string().is_empty());
}

#[tokio::test]
async fn test_get_session_by_token_hash() {
    let test_app = TestApp::new("test_get_session_by_token_hash").await;
    let mut conn = test_app.get_connection().await;

    // First create a user
    let register_user_data = test_app.generate_test_user();
    let user = register_user(&mut conn, register_user_data).await.unwrap();

    let token = Uuid::now_v7().to_string();
    let expires_at = Utc::now() + Duration::hours(24);

    let new_session = NewUserSession {
        user_id: user.id,
        token_hash: hash_session_token(&token),
        expires_at,
    };

    let created_session = create_session(&mut conn, new_session).await.unwrap();

    // Test getting session by token
    let found_session = get_session_by_token_hash(&mut conn, &hash_session_token(&token)).await.unwrap();
    assert!(found_session.is_some());

    let found_session = found_session.unwrap();
    assert_eq!(found_session.id, created_session.id);
    assert_eq!(found_session.user_id, user.id);
    assert_eq!(found_session.token_hash, hash_session_token(&token));

    // Test getting non-existent session
    let not_found = get_session_by_token_hash(&mut conn, &hash_session_token("non_existent_token")).await.unwrap();
    assert!(not_found.is_none());
}

#[tokio::test]
async fn test_get_sessions_by_user() {
    let test_app = TestApp::new("test_get_sessions_by_user").await;
    let mut conn = test_app.get_connection().await;

    // First create a user
    let register_user_data = test_app.generate_test_user();
    let user = register_user(&mut conn, register_user_data).await.unwrap();

    let expires_at = Utc::now() + Duration::hours(24);

    // Create multiple sessions for the same user
    let session1_token = Uuid::now_v7().to_string();
    let session2_token = Uuid::now_v7().to_string();

    let new_session1 = NewUserSession {
        user_id: user.id,
        token_hash: hash_session_token(&session1_token),
        expires_at,
    };

    let new_session2 = NewUserSession {
        user_id: user.id,
        token_hash: hash_session_token(&session2_token),
        expires_at,
    };

    create_session(&mut conn, new_session1).await.unwrap();
    create_session(&mut conn, new_session2).await.unwrap();

    // Get all sessions for the user
    let user_sessions = get_sessions_by_user(&mut conn, user.id).await.unwrap();
    assert_eq!(user_sessions.len(), 2);

    // Verify sessions are ordered by created_at DESC
    assert_eq!(user_sessions[0].token_hash, hash_session_token(&session2_token));
    assert_eq!(user_sessions[1].token_hash, hash_session_token(&session1_token));

    // Test with non-existent user
    let no_sessions = get_sessions_by_user(&mut conn, Uuid::now_v7()).await.unwrap();
    assert!(no_sessions.is_empty());
}

#[tokio::test]
async fn test_delete_session() {
    let test_app = TestApp::new("test_delete_session").await;
    let mut conn = test_app.get_connection().await;

    // First create a user
    let register_user_data = test_app.generate_test_user();
    let user = register_user(&mut conn, register_user_data).await.unwrap();

    let token = Uuid::now_v7().to_string();
    let expires_at = Utc::now() + Duration::hours(24);

    let new_session = NewUserSession {
        user_id: user.id,
        token_hash: hash_session_token(&token),
        expires_at,
    };

    let created_session = create_session(&mut conn, new_session).await.unwrap();

    // Delete the session
    let rows_affected = delete_session(&mut conn, created_session.id).await.unwrap();
    assert_eq!(rows_affected, 1);

    // Verify session is deleted
    let found_session = get_session_by_token_hash(&mut conn, &hash_session_token(&token)).await.unwrap();
    assert!(found_session.is_none());

    // Try to delete non-existent session
    let rows_affected = delete_session(&mut conn, Uuid::now_v7()).await.unwrap();
    assert_eq!(rows_affected, 0);
}

#[tokio::test]
async fn test_delete_session_by_token_hash() {
    let test_app = TestApp::new("test_delete_session_by_token_hash").await;
    let mut conn = test_app.get_connection().await;

    // First create a user
    let register_user_data = test_app.generate_test_user();
    let user = register_user(&mut conn, register_user_data).await.unwrap();

    let token = Uuid::now_v7().to_string();
    let expires_at = Utc::now() + Duration::hours(24);

    let new_session = NewUserSession {
        user_id: user.id,
        token_hash: hash_session_token(&token),
        expires_at,
    };

    create_session(&mut conn, new_session).await.unwrap();

    // Delete the session by token
    let rows_affected = delete_session_by_token_hash(&mut conn, &hash_session_token(&token)).await.unwrap();
    assert_eq!(rows_affected, 1);

    // Verify session is deleted
    let found_session = get_session_by_token_hash(&mut conn, &hash_session_token(&token)).await.unwrap();
    assert!(found_session.is_none());

    // Try to delete non-existent session by token
    let rows_affected = delete_session_by_token_hash(&mut conn, &hash_session_token("non_existent_token")).await.unwrap();
    assert_eq!(rows_affected, 0);
}

#[tokio::test]
async fn test_delete_sessions_by_user() {
    let test_app = TestApp::new("test_delete_sessions_by_user").await;
    let mut conn = test_app.get_connection().await;

    // Create two users
    let register_user_data1 = test_app.generate_test_user();
    let user1 = register_user(&mut conn, register_user_data1).await.unwrap();

    let register_user_data2 = test_app.generate_test_user();
    let user2 = register_user(&mut conn, register_user_data2).await.unwrap();

    let expires_at = Utc::now() + Duration::hours(24);

    // Create sessions for two different users
    let session1_token = Uuid::now_v7().to_string();
    let session2_token = Uuid::now_v7().to_string();
    let other_user_token = Uuid::now_v7().to_string();

    let new_session1 = NewUserSession {
        user_id: user1.id,
        token_hash: hash_session_token(&session1_token),
        expires_at,
    };

    let new_session2 = NewUserSession {
        user_id: user1.id,
        token_hash: hash_session_token(&session2_token),
        expires_at,
    };

    let other_user_session = NewUserSession {
        user_id: user2.id,
        token_hash: hash_session_token(&other_user_token),
        expires_at,
    };

    create_session(&mut conn, new_session1).await.unwrap();
    create_session(&mut conn, new_session2).await.unwrap();
    create_session(&mut conn, other_user_session).await.unwrap();

    // Delete all sessions for the first user
    let rows_affected = delete_sessions_by_user(&mut conn, user1.id).await.unwrap();
    assert_eq!(rows_affected, 2);

    // Verify first user's sessions are deleted
    let user_sessions = get_sessions_by_user(&mut conn, user1.id).await.unwrap();
    assert!(user_sessions.is_empty());

    // Verify other user's session is not deleted
    let other_user_sessions = get_sessions_by_user(&mut conn, user2.id).await.unwrap();
    assert_eq!(other_user_sessions.len(), 1);
}

#[tokio::test]
async fn test_is_session_valid() {
    let test_app = TestApp::new("test_is_session_valid").await;
    let mut conn = test_app.get_connection().await;

    // First create a user
    let register_user_data = test_app.generate_test_user();
    let user = register_user(&mut conn, register_user_data).await.unwrap();

    let valid_token = Uuid::now_v7().to_string();
    let expired_token = Uuid::now_v7().to_string();

    // Create a valid session (expires in future)
    let valid_session = NewUserSession {
        user_id: user.id,
        token_hash: hash_session_token(&valid_token),
        expires_at: Utc::now() + Duration::hours(1),
    };

    // Create an expired session (expired in past)
    let expired_session = NewUserSession {
        user_id: user.id,
        token_hash: hash_session_token(&expired_token),
        expires_at: Utc::now() - Duration::hours(1),
    };

    create_session(&mut conn, valid_session).await.unwrap();
    create_session(&mut conn, expired_session).await.unwrap();

    // Test valid session
    let is_valid = is_session_valid(&mut conn, &hash_session_token(&valid_token)).await.unwrap();
    assert!(is_valid);

    // Test expired session
    let is_valid = is_session_valid(&mut conn, &hash_session_token(&expired_token)).await.unwrap();
    assert!(!is_valid);

    // Test non-existent session
    let is_valid = is_session_valid(&mut conn, &hash_session_token("non_existent_token")).await.unwrap();
    assert!(!is_valid);
}

#[tokio::test]
async fn test_get_valid_session_by_token_hash() {
    let test_app = TestApp::new("test_get_valid_session_by_token_hash").await;
    let mut conn = test_app.get_connection().await;

    // First create a user
    let register_user_data = test_app.generate_test_user();
    let user = register_user(&mut conn, register_user_data).await.unwrap();

    let valid_token = Uuid::now_v7().to_string();
    let expired_token = Uuid::now_v7().to_string();

    // Create a valid session (expires in future)
    let valid_session = NewUserSession {
        user_id: user.id,
        token_hash: hash_session_token(&valid_token),
        expires_at: Utc::now() + Duration::hours(1),
    };

    // Create an expired session (expired in past)
    let expired_session = NewUserSession {
        user_id: user.id,
        token_hash: hash_session_token(&expired_token),
        expires_at: Utc::now() - Duration::hours(1),
    };

    create_session(&mut conn, valid_session).await.unwrap();
    create_session(&mut conn, expired_session).await.unwrap();

    // Test getting valid session
    let found_session = get_valid_session_by_token_hash(&mut conn, &hash_session_token(&valid_token)).await.unwrap();
    assert!(found_session.is_some());
    assert_eq!(found_session.unwrap().token_hash, hash_session_token(&valid_token));

    // Test getting expired session
    let found_session = get_valid_session_by_token_hash(&mut conn, &hash_session_token(&expired_token)).await.unwrap();
    assert!(found_session.is_none());

    // Test getting non-existent session
    let found_session = get_valid_session_by_token_hash(&mut conn, &hash_session_token("non_existent_token")).await.unwrap();
    assert!(found_session.is_none());
}

#[tokio::test]
async fn test_refresh_session() {
    let test_app = TestApp::new("test_refresh_session").await;
    let mut conn = test_app.get_connection().await;

    // First create a user
    let register_user_data = test_app.generate_test_user();
    let user = register_user(&mut conn, register_user_data).await.unwrap();

    let token = Uuid::now_v7().to_string();
    let original_expires_at = Utc::now() + Duration::hours(1);

    let new_session = NewUserSession {
        user_id: user.id,
        token_hash: hash_session_token(&token),
        expires_at: original_expires_at,
    };

    let created_session = create_session(&mut conn, new_session).await.unwrap();

    // Refresh the session with new expiration time
    let new_expires_at = Utc::now() + Duration::hours(24);
    let refreshed_session = refresh_session(&mut conn, created_session.id, new_expires_at).await.unwrap();

    assert_eq!(refreshed_session.id, created_session.id);
    assert_eq!(refreshed_session.user_id, user.id);
    assert_eq!(refreshed_session.token_hash, hash_session_token(&token));
    assert_eq!(refreshed_session.expires_at, new_expires_at);
    assert!(refreshed_session.updated_at > created_session.updated_at);
}

#[tokio::test]
async fn test_delete_expired_sessions() {
    let test_app = TestApp::new("test_delete_expired_sessions_unique").await;
    let mut conn = test_app.get_connection().await;

    // First create a user
    let register_user_data = test_app.generate_test_user();
    let user = register_user(&mut conn, register_user_data).await.unwrap();

    let expires_at = Utc::now() + Duration::hours(1);

    // Create some expired sessions with unique tokens
    let expired_time = Utc::now() - Duration::hours(1);
    let test_prefix = test_app.test_prefix();
    for i in 0..3 {
        let token = format!("{}_expired_token_{}_{}", test_prefix, i, Uuid::now_v7());
        let expired_session = NewUserSession {
            user_id: user.id,
            token_hash: hash_session_token(&token),
            expires_at: expired_time,
        };
        create_session(&mut conn, expired_session).await.unwrap();
    }

    // Create some valid sessions and store their hashes
    let mut valid_token_hashes = Vec::new();
    for i in 0..2 {
        let token = format!("{}_valid_token_{}_{}", test_prefix, i, Uuid::now_v7());
        let token_hash = hash_session_token(&token);
        valid_token_hashes.push(token_hash.clone());
        let valid_session = NewUserSession {
            user_id: user.id,
            token_hash,
            expires_at,
        };
        create_session(&mut conn, valid_session).await.unwrap();
    }

    // Delete expired sessions
    let rows_affected = delete_expired_sessions(&mut conn).await.unwrap();

    // We expect at least 3 to be deleted (our test sessions), but there might be more from other tests
    assert!(rows_affected >= 3);

    // Verify our test valid sessions remain
    let all_sessions = get_sessions_by_user(&mut conn, user.id).await.unwrap();

    // Count our valid test sessions by comparing hashes
    let our_valid_sessions = all_sessions.iter()
        .filter(|s| valid_token_hashes.contains(&s.token_hash))
        .count();

    assert_eq!(our_valid_sessions, 2);

    // Verify all our valid sessions are still present
    for hash in &valid_token_hashes {
        let found = all_sessions.iter().any(|s| &s.token_hash == hash);
        assert!(found, "Valid session should still exist");
    }
}

#[tokio::test]
async fn test_session_constraints() {
    let test_app = TestApp::new("test_session_constraints").await;
    let mut conn = test_app.get_connection().await;

    // First create a user
    let register_user_data = test_app.generate_test_user();
    let user = register_user(&mut conn, register_user_data).await.unwrap();

    let token = Uuid::now_v7().to_string();
    let expires_at = Utc::now() + Duration::hours(24);

    let new_session = NewUserSession {
        user_id: user.id,
        token_hash: hash_session_token(&token),
        expires_at,
    };

    // Create first session
    create_session(&mut conn, new_session.clone()).await.unwrap();

    // Try to create another session with the same token (should fail due to UNIQUE constraint)
    let result = create_session(&mut conn, new_session).await;
    assert!(result.is_err());
}