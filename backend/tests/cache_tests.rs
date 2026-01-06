//! Comprehensive cache tests.
//!
//! This test suite covers all cache operations:
//! - Basic CRUD operations
//! - TTL operations
//! - Existence checks
//! - Atomic operations
//! - Batch operations
//! - Thread safety
//! - Background cleanup

use backend::cache::{Cache, CacheConfig};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestUser {
    id: u32,
    name: String,
    email: String,
}

impl TestUser {
    fn new(id: u32, name: &str, email: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            email: email.to_string(),
        }
    }
}

// Test helper to create a unique cache for each test
fn create_test_cache() -> Cache<String> {
    Cache::new_local(CacheConfig {
        cleanup_interval_seconds: 1,
        default_ttl_seconds: None,
    })
}

fn create_test_cache_with_default_ttl(default_ttl: u64) -> Cache<String> {
    Cache::new_local(CacheConfig {
        cleanup_interval_seconds: 1,
        default_ttl_seconds: Some(default_ttl),
    })
}

// ============================================================================
// Basic CRUD Operations Tests
// ============================================================================

#[tokio::test]
async fn test_basic_set_get() {
    let cache = create_test_cache();

    cache.set("key1", "value1".to_string()).await.unwrap();
    let value = cache.get("key1").await.unwrap();

    assert_eq!(value, Some("value1".to_string()));
}

#[tokio::test]
async fn test_get_nonexistent_key() {
    let cache = create_test_cache();

    let value = cache.get("nonexistent").await.unwrap();
    assert_eq!(value, None);
}

#[tokio::test]
async fn test_set_overwrites_existing() {
    let cache = create_test_cache();

    cache.set("key1", "value1".to_string()).await.unwrap();
    cache.set("key1", "value2".to_string()).await.unwrap();

    let value = cache.get("key1").await.unwrap();
    assert_eq!(value, Some("value2".to_string()));
}

#[tokio::test]
async fn test_delete_existing_key() {
    let cache = create_test_cache();

    cache.set("key1", "value1".to_string()).await.unwrap();
    let deleted = cache.delete("key1").await.unwrap();

    assert!(deleted);
    assert_eq!(cache.get("key1").await.unwrap(), None);
}

#[tokio::test]
async fn test_delete_nonexistent_key() {
    let cache = create_test_cache();

    let deleted = cache.delete("nonexistent").await.unwrap();
    assert!(!deleted);
}

#[tokio::test]
async fn test_complex_type() {
    let cache: Cache<TestUser> = Cache::new_local(CacheConfig::default());

    let user = TestUser::new(1, "Alice", "alice@example.com");
    cache.set("user:1", user.clone()).await.unwrap();

    let retrieved = cache.get("user:1").await.unwrap();
    assert_eq!(retrieved, Some(user));
}

// ============================================================================
// TTL Operations Tests
// ============================================================================

#[tokio::test]
async fn test_set_with_expiration() {
    let cache = create_test_cache();

    cache
        .set_ex("key1", "value1".to_string(), 2)
        .await
        .unwrap();

    let value = cache.get("key1").await.unwrap();
    assert_eq!(value, Some("value1".to_string()));

    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(3)).await;

    let value = cache.get("key1").await.unwrap();
    assert_eq!(value, None);
}

#[tokio::test]
async fn test_ttl_retrieval() {
    let cache = create_test_cache();

    cache.set("key1", "value1".to_string()).await.unwrap();
    let ttl = cache.ttl("key1").await.unwrap();
    assert_eq!(ttl, None); // No expiration

    cache
        .set_ex("key2", "value2".to_string(), 10)
        .await
        .unwrap();
    let ttl = cache.ttl("key2").await.unwrap();
    assert!(ttl.is_some());
    assert!(ttl.unwrap() > 0 && ttl.unwrap() <= 10);
}

#[tokio::test]
async fn test_expire_existing_key() {
    let cache = create_test_cache();

    cache.set("key1", "value1".to_string()).await.unwrap();
    let updated = cache.expire("key1", 2).await.unwrap();
    assert!(updated);

    let ttl = cache.ttl("key1").await.unwrap();
    assert!(ttl.is_some());

    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(3)).await;

    let value = cache.get("key1").await.unwrap();
    assert_eq!(value, None);
}

#[tokio::test]
async fn test_expire_nonexistent_key() {
    let cache = create_test_cache();

    let updated = cache.expire("nonexistent", 10).await.unwrap();
    assert!(!updated);
}

#[tokio::test]
async fn test_persist_removes_expiration() {
    let cache = create_test_cache();

    cache
        .set_ex("key1", "value1".to_string(), 10)
        .await
        .unwrap();

    let ttl_before = cache.ttl("key1").await.unwrap();
    assert!(ttl_before.is_some());

    let persisted = cache.persist("key1").await.unwrap();
    assert!(persisted);

    let ttl_after = cache.ttl("key1").await.unwrap();
    assert_eq!(ttl_after, None);

    // Wait to ensure it doesn't expire
    tokio::time::sleep(Duration::from_secs(2)).await;

    let value = cache.get("key1").await.unwrap();
    assert_eq!(value, Some("value1".to_string()));
}

#[tokio::test]
async fn test_persist_nonexistent_key() {
    let cache = create_test_cache();

    let persisted = cache.persist("nonexistent").await.unwrap();
    assert!(!persisted);
}

#[tokio::test]
async fn test_default_ttl() {
    let cache = create_test_cache_with_default_ttl(2);

    cache.set("key1", "value1".to_string()).await.unwrap();

    let value = cache.get("key1").await.unwrap();
    assert_eq!(value, Some("value1".to_string()));

    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(3)).await;

    let value = cache.get("key1").await.unwrap();
    assert_eq!(value, None);
}

// ============================================================================
// Existence Checks Tests
// ============================================================================

#[tokio::test]
async fn test_exists() {
    let cache = create_test_cache();

    assert!(!cache.exists("key1").await.unwrap());

    cache.set("key1", "value1".to_string()).await.unwrap();
    assert!(cache.exists("key1").await.unwrap());

    cache.delete("key1").await.unwrap();
    assert!(!cache.exists("key1").await.unwrap());
}

#[tokio::test]
async fn test_exists_respects_expiration() {
    let cache = create_test_cache();

    cache
        .set_ex("key1", "value1".to_string(), 1)
        .await
        .unwrap();

    assert!(cache.exists("key1").await.unwrap());

    tokio::time::sleep(Duration::from_secs(2)).await;

    assert!(!cache.exists("key1").await.unwrap());
}

#[tokio::test]
async fn test_keys() {
    let cache = create_test_cache();

    cache.set("key1", "value1".to_string()).await.unwrap();
    cache.set("key2", "value2".to_string()).await.unwrap();
    cache.set("key3", "value3".to_string()).await.unwrap();

    let keys = cache.keys().await.unwrap();
    assert_eq!(keys.len(), 3);
    assert!(keys.contains(&"key1".to_string()));
    assert!(keys.contains(&"key2".to_string()));
    assert!(keys.contains(&"key3".to_string()));
}

#[tokio::test]
async fn test_keys_excludes_expired() {
    let cache = create_test_cache();

    cache.set("key1", "value1".to_string()).await.unwrap();
    cache
        .set_ex("key2", "value2".to_string(), 1)
        .await
        .unwrap();
    cache.set("key3", "value3".to_string()).await.unwrap();

    let keys_before = cache.keys().await.unwrap();
    assert_eq!(keys_before.len(), 3);

    tokio::time::sleep(Duration::from_secs(2)).await;

    let keys_after = cache.keys().await.unwrap();
    assert_eq!(keys_after.len(), 2);
    assert!(keys_after.contains(&"key1".to_string()));
    assert!(keys_after.contains(&"key3".to_string()));
}

#[tokio::test]
async fn test_clear() {
    let cache = create_test_cache();

    cache.set("key1", "value1".to_string()).await.unwrap();
    cache.set("key2", "value2".to_string()).await.unwrap();
    cache.set("key3", "value3".to_string()).await.unwrap();

    let count = cache.clear().await.unwrap();
    assert_eq!(count, 3);

    assert_eq!(cache.keys().await.unwrap().len(), 0);
}

// ============================================================================
// Atomic Operations Tests
// ============================================================================

#[tokio::test]
async fn test_set_nx_when_key_doesnt_exist() {
    let cache = create_test_cache();

    let inserted = cache.set_nx("key1", "value1".to_string()).await.unwrap();
    assert!(inserted);

    let value = cache.get("key1").await.unwrap();
    assert_eq!(value, Some("value1".to_string()));
}

#[tokio::test]
async fn test_set_nx_when_key_exists() {
    let cache = create_test_cache();

    cache.set("key1", "value1".to_string()).await.unwrap();

    let inserted = cache.set_nx("key1", "value2".to_string()).await.unwrap();
    assert!(!inserted);

    let value = cache.get("key1").await.unwrap();
    assert_eq!(value, Some("value1".to_string()));
}

#[tokio::test]
async fn test_get_and_set() {
    let cache = create_test_cache();

    cache.set("key1", "value1".to_string()).await.unwrap();

    let old_value = cache
        .get_and_set("key1", "value2".to_string())
        .await
        .unwrap();
    assert_eq!(old_value, Some("value1".to_string()));

    let new_value = cache.get("key1").await.unwrap();
    assert_eq!(new_value, Some("value2".to_string()));
}

#[tokio::test]
async fn test_get_and_set_nonexistent_key() {
    let cache = create_test_cache();

    let old_value = cache
        .get_and_set("key1", "value1".to_string())
        .await
        .unwrap();
    assert_eq!(old_value, None);

    let value = cache.get("key1").await.unwrap();
    assert_eq!(value, Some("value1".to_string()));
}

#[tokio::test]
async fn test_get_and_set_respects_expiration() {
    let cache = create_test_cache();

    cache
        .set_ex("key1", "value1".to_string(), 1)
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(2)).await;

    let old_value = cache
        .get_and_set("key1", "value2".to_string())
        .await
        .unwrap();
    // Old value was expired, so returns None
    assert_eq!(old_value, None);

    let new_value = cache.get("key1").await.unwrap();
    assert_eq!(new_value, Some("value2".to_string()));
}

// ============================================================================
// Batch Operations Tests
// ============================================================================

#[tokio::test]
async fn test_mget() {
    let cache = create_test_cache();

    cache.set("key1", "value1".to_string()).await.unwrap();
    cache.set("key2", "value2".to_string()).await.unwrap();
    cache.set("key3", "value3".to_string()).await.unwrap();

    let values = cache.mget(&["key1", "key2", "key3", "key4"]).await.unwrap();
    assert_eq!(values.len(), 4);
    assert_eq!(values[0], Some("value1".to_string()));
    assert_eq!(values[1], Some("value2".to_string()));
    assert_eq!(values[2], Some("value3".to_string()));
    assert_eq!(values[3], None);
}

#[tokio::test]
async fn test_mget_empty() {
    let cache = create_test_cache();

    let values = cache.mget(&[]).await.unwrap();
    assert!(values.is_empty());
}

#[tokio::test]
async fn test_mset() {
    let cache = create_test_cache();

    cache
        .mset(vec![
            ("key1", "value1".to_string()),
            ("key2", "value2".to_string()),
            ("key3", "value3".to_string()),
        ])
        .await
        .unwrap();

    assert_eq!(
        cache.get("key1").await.unwrap(),
        Some("value1".to_string())
    );
    assert_eq!(
        cache.get("key2").await.unwrap(),
        Some("value2".to_string())
    );
    assert_eq!(
        cache.get("key3").await.unwrap(),
        Some("value3".to_string())
    );
}

#[tokio::test]
async fn test_mset_empty() {
    let cache = create_test_cache();

    cache.mset(vec![]).await.unwrap();
    // Should not panic or error
}

#[tokio::test]
async fn test_mdelete() {
    let cache = create_test_cache();

    cache.set("key1", "value1".to_string()).await.unwrap();
    cache.set("key2", "value2".to_string()).await.unwrap();
    cache.set("key3", "value3".to_string()).await.unwrap();

    let count = cache
        .mdelete(&["key1", "key2", "key3", "key4"])
        .await
        .unwrap();
    assert_eq!(count, 3);

    assert!(!cache.exists("key1").await.unwrap());
    assert!(!cache.exists("key2").await.unwrap());
    assert!(!cache.exists("key3").await.unwrap());
}

#[tokio::test]
async fn test_mdelete_empty() {
    let cache = create_test_cache();

    let count = cache.mdelete(&[]).await.unwrap();
    assert_eq!(count, 0);
}

// ============================================================================
// Thread Safety Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_reads() {
    let cache = create_test_cache();

    cache.set("key1", "value1".to_string()).await.unwrap();
    cache.set("key2", "value2".to_string()).await.unwrap();

    let cache_clone = cache.clone();

    let handle1 = tokio::spawn(async move {
        for _ in 0..100 {
            cache_clone.get("key1").await.unwrap();
        }
    });

    let cache_clone = cache.clone();
    let handle2 = tokio::spawn(async move {
        for _ in 0..100 {
            cache_clone.get("key2").await.unwrap();
        }
    });

    handle1.await.unwrap();
    handle2.await.unwrap();
}

#[tokio::test]
async fn test_concurrent_writes() {
    let cache = create_test_cache();

    let cache_clone = cache.clone();
    let handle1 = tokio::spawn(async move {
        for i in 0..50 {
            cache_clone
                .set(&format!("key1_{}", i), format!("value1_{}", i))
                .await
                .unwrap();
        }
    });

    let cache_clone = cache.clone();
    let handle2 = tokio::spawn(async move {
        for i in 0..50 {
            cache_clone
                .set(&format!("key2_{}", i), format!("value2_{}", i))
                .await
                .unwrap();
        }
    });

    handle1.await.unwrap();
    handle2.await.unwrap();

    // Verify all keys were written
    let keys = cache.keys().await.unwrap();
    assert_eq!(keys.len(), 100);
}

#[tokio::test]
async fn test_clone_shares_data() {
    let cache1 = create_test_cache();

    cache1.set("key1", "value1".to_string()).await.unwrap();

    let cache2 = cache1.clone();

    // cache2 should see the same data
    assert_eq!(
        cache2.get("key1").await.unwrap(),
        Some("value1".to_string())
    );

    // Changes to cache2 should be visible in cache1
    cache2.set("key2", "value2".to_string()).await.unwrap();
    assert_eq!(
        cache1.get("key2").await.unwrap(),
        Some("value2".to_string())
    );
}

// ============================================================================
// Background Cleanup Tests
// ============================================================================

#[tokio::test]
async fn test_background_cleanup_removes_expired_entries() {
    let cache = Cache::new_local(CacheConfig {
        cleanup_interval_seconds: 1,
        default_ttl_seconds: None,
    });

    // Set entries with 2 second TTL
    for i in 0..10 {
        cache
            .set_ex(&format!("key{}", i), format!("value{}", i), 2)
            .await
            .unwrap();
    }

    // Set some persistent entries
    for i in 10..20 {
        cache
            .set(&format!("key{}", i), format!("value{}", i))
            .await
            .unwrap();
    }

    let keys_before = cache.keys().await.unwrap();
    assert_eq!(keys_before.len(), 20);

    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Trigger background cleanup (wait for cleanup interval)
    tokio::time::sleep(Duration::from_secs(2)).await;

    let keys_after = cache.keys().await.unwrap();
    // Only persistent entries should remain
    assert_eq!(keys_after.len(), 10);
}

#[tokio::test]
async fn test_background_cleanup_with_default_ttl() {
    let cache = Cache::new_local(CacheConfig {
        cleanup_interval_seconds: 1,
        default_ttl_seconds: Some(2),
    });

    // Set entries (will use default TTL of 2 seconds)
    for i in 0..10 {
        cache
            .set(&format!("key{}", i), format!("value{}", i))
            .await
            .unwrap();
    }

    let keys_before = cache.keys().await.unwrap();
    assert_eq!(keys_before.len(), 10);

    // Wait for expiration + cleanup
    tokio::time::sleep(Duration::from_secs(5)).await;

    let keys_after = cache.keys().await.unwrap();
    assert_eq!(keys_after.len(), 0);
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[tokio::test]
async fn test_json_serialization_compatibility() {
    let cache: Cache<TestUser> = Cache::new_local(CacheConfig::default());

    let user = TestUser::new(1, "Alice", "alice@example.com");
    cache.set("user:1", user.clone()).await.unwrap();

    let retrieved = cache.get("user:1").await.unwrap();
    assert_eq!(retrieved, Some(user.clone()));

    // Verify we can serialize to JSON
    let json = serde_json::to_string(&user).unwrap();
    let deserialized: TestUser = serde_json::from_str(&json).unwrap();
    assert_eq!(user, deserialized);
}
