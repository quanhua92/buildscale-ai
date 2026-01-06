//! Cache Management Example
//!
//! This example demonstrates the usage of the generic async cache with TTL support.
//! It covers:
//! - Basic CRUD operations
//! - TTL operations (set_ex, ttl, expire, persist)
//! - Existence checks (exists, keys, clear)
//! - Atomic operations (set_nx, get_and_set)
//! - Batch operations (mget, mset, mdelete)
//! - Thread safety and concurrent access
//! - Background cleanup
//! - Health metrics monitoring

use backend::cache::{Cache, CacheConfig};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

impl User {
    fn new(id: u32, name: &str, email: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            email: email.to_string(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Cache Management Example ===\n");

    // ============================================================================
    // Basic CRUD Operations
    // ============================================================================
    println!("1. Basic CRUD Operations");
    println!("-------------------------");

    let cache: Cache<String> = Cache::new_local(CacheConfig::default());

    // Set and Get
    cache.set("key1", "value1".to_string()).await?;
    let value = cache.get("key1").await?;
    println!("Set 'key1' = 'value1', Got: {:?}", value);

    // Overwrite
    cache.set("key1", "value2".to_string()).await?;
    let value = cache.get("key1").await?;
    println!("Overwrite 'key1' = 'value2', Got: {:?}", value);

    // Delete
    let deleted = cache.delete("key1").await?;
    println!("Delete 'key1': {}, Exists: {}", deleted, cache.exists("key1").await?);

    println!();

    // ============================================================================
    // TTL Operations
    // ============================================================================
    println!("2. TTL Operations");
    println!("------------------");

    // Set with expiration
    cache.set_ex("session:123", "active".to_string(), 5).await?;
    println!("Set 'session:123' with 5 second TTL");

    let value = cache.get("session:123").await?;
    println!("Immediately after: {:?}", value);

    // Check TTL
    let ttl = cache.ttl("session:123").await?;
    println!("TTL remaining: {:?} seconds", ttl);

    // Update expiration
    cache.expire("session:123", 10).await?;
    let ttl = cache.ttl("session:123").await?;
    println!("Updated TTL to 10s, remaining: {:?} seconds", ttl);

    // Make persistent (remove expiration)
    cache.persist("session:123").await?;
    let ttl = cache.ttl("session:123").await?;
    println!("Persisted (removed TTL), TTL: {:?}", ttl);

    println!();

    // ============================================================================
    // Existence Checks
    // ============================================================================
    println!("3. Existence Checks");
    println!("-------------------");

    cache.set("user:1", "Alice".to_string()).await?;
    cache.set("user:2", "Bob".to_string()).await?;
    cache.set("user:3", "Charlie".to_string()).await?;

    println!("Exists 'user:1': {}", cache.exists("user:1").await?);
    println!("Exists 'user:999': {}", cache.exists("user:999").await?);

    let keys = cache.keys().await?;
    println!("All keys: {:?}", keys);

    let count = cache.clear().await?;
    println!("Cleared {} entries", count);
    println!("Keys after clear: {:?}", cache.keys().await?);

    println!();

    // ============================================================================
    // Atomic Operations
    // ============================================================================
    println!("4. Atomic Operations");
    println!("--------------------");

    // Set if not exists (set_nx)
    let inserted = cache.set_nx("lock:resource", "locked".to_string()).await?;
    println!("set_nx 'lock:resource': {}", inserted);

    let inserted = cache.set_nx("lock:resource", "locked_again".to_string()).await?;
    println!("set_nx 'lock:resource' again: {}", inserted);

    let value = cache.get("lock:resource").await?;
    println!("Value should still be 'locked': {:?}", value);

    // Get and set atomically
    let old_value = cache.get_and_set("lock:resource", "locked_v2".to_string()).await?;
    println!("get_and_set returned: {:?}", old_value);

    let new_value = cache.get("lock:resource").await?;
    println!("New value: {:?}", new_value);

    println!();

    // ============================================================================
    // Batch Operations
    // ============================================================================
    println!("5. Batch Operations");
    println!("-------------------");

    // Multiple set
    cache
        .mset(vec![
            ("batch:1", "value1".to_string()),
            ("batch:2", "value2".to_string()),
            ("batch:3", "value3".to_string()),
        ])
        .await?;
    println!("mset 3 keys");

    // Multiple get
    let values = cache.mget(&["batch:1", "batch:2", "batch:3", "batch:999"]).await?;
    println!("mget results: {:?}", values);

    // Multiple delete
    let count = cache.mdelete(&["batch:1", "batch:2", "batch:3"]).await?;
    println!("mdelete {} keys", count);

    println!();

    // ============================================================================
    // Complex Types
    // ============================================================================
    println!("6. Complex Types");
    println!("----------------");

    let user_cache: Cache<User> = Cache::new_local(CacheConfig::default());

    let user1 = User::new(1, "Alice", "alice@example.com");
    let user2 = User::new(2, "Bob", "bob@example.com");

    user_cache.set("user:1", user1.clone()).await?;
    user_cache.set("user:2", user2.clone()).await?;

    let retrieved = user_cache.get("user:1").await?;
    println!("Stored and retrieved user: {:?}", retrieved);

    println!();

    // ============================================================================
    // Thread Safety and Concurrent Access
    // ============================================================================
    println!("7. Thread Safety");
    println!("----------------");

    let cache: Cache<String> = Cache::new_local(CacheConfig::default());

    let cache_clone1 = cache.clone();
    let handle1 = tokio::spawn(async move {
        for i in 0..100 {
            cache_clone1
                .set(&format!("thread1:key{}", i), format!("value{}", i))
                .await
                .unwrap();
        }
    });

    let cache_clone2 = cache.clone();
    let handle2 = tokio::spawn(async move {
        for i in 0..100 {
            cache_clone2
                .set(&format!("thread2:key{}", i), format!("value{}", i))
                .await
                .unwrap();
        }
    });

    handle1.await?;
    handle2.await?;

    let keys = cache.keys().await?;
    println!("Concurrent writes: {} keys created", keys.len());

    println!();

    // ============================================================================
    // Background Cleanup
    // ============================================================================
    println!("8. Background Cleanup");
    println!("---------------------");

    let cache = Cache::new_local(CacheConfig {
        cleanup_interval_seconds: 2,
        default_ttl_seconds: None,
    });

    // Set entries with different TTLs
    for i in 0..5 {
        cache
            .set_ex(&format!("temp:{}", i), format!("value{}", i), 3)
            .await?;
        println!("Set 'temp:{}' with 3 second TTL", i);
    }

    // Set some persistent entries
    for i in 0..3 {
        cache
            .set(&format!("perm:{}", i), format!("persistent{}", i))
            .await?;
        println!("Set 'perm:{}' without TTL", i);
    }

    let keys_before = cache.keys().await?;
    println!("\nTotal keys before expiration: {}", keys_before.len());

    println!("\nWaiting 5 seconds for entries to expire...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    println!("Waiting 3 more seconds for background cleanup...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    let keys_after = cache.keys().await?;
    println!("Total keys after cleanup: {}", keys_after.len());
    println!("Remaining keys: {:?}", keys_after);

    println!();

    // ============================================================================
    // Cache with Default TTL
    // ============================================================================
    println!("9. Default TTL Configuration");
    println!("------------------------------");

    let cache = Cache::new_local(CacheConfig {
        cleanup_interval_seconds: 60,
        default_ttl_seconds: Some(10), // All entries expire in 10s by default
    });

    cache.set("auto:1", "value1".to_string()).await?;
    cache.set("auto:2", "value2".to_string()).await?;

    println!("Set keys with default TTL of 10s");
    let ttl1 = cache.ttl("auto:1").await?;
    let ttl2 = cache.ttl("auto:2").await?;
    println!("TTL for 'auto:1': {:?}", ttl1);
    println!("TTL for 'auto:2': {:?}", ttl2);

    println!();

    // ============================================================================
    // Practical Example: User Session Cache
    // ============================================================================
    println!("10. Practical Example: User Session Cache");
    println!("------------------------------------------");

    let session_cache: Cache<User> = Cache::new_local(CacheConfig {
        cleanup_interval_seconds: 60,
        default_ttl_seconds: Some(30), // Sessions expire after 30 minutes
    });

    // Simulate user login
    let user = User::new(1, "Alice", "alice@example.com");
    let session_id = "session_abc123";

    session_cache.set_ex(session_id, user.clone(), 5).await?; // 5s for demo
    println!("User logged in, session cached");

    // Simulate API request with session validation
    if let Some(cached_user) = session_cache.get(session_id).await? {
        println!("Session valid, user: {}", cached_user.name);
    }

    // Check session TTL
    let ttl = session_cache.ttl(session_id).await?;
    println!("Session expires in: {:?} seconds", ttl);

    // Refresh session (user activity)
    session_cache.expire(session_id, 5).await?;
    println!("Session refreshed, TTL: {:?}", session_cache.ttl(session_id).await?);

    println!();

    // ============================================================================
    // Health Metrics
    // ============================================================================
    println!("11. Health Metrics");
    println!("-------------------");

    use backend::cache::CacheHealthMetrics;

    let cache = Cache::new_local(CacheConfig {
        cleanup_interval_seconds: 1,
        default_ttl_seconds: None,
    });

    // Add some entries
    for i in 0..10 {
        cache.set(&format!("key:{}", i), format!("value{}", i)).await?;
    }

    // Get health metrics
    let metrics: CacheHealthMetrics = cache.get_health_metrics().await?;
    println!("Current cache health:");
    println!("  Total keys: {}", metrics.num_keys);
    println!("  Memory usage: {} bytes", metrics.size_bytes);
    println!("  Last cleanup: {:?}", metrics.last_worker_time);
    println!("  Entries cleaned (lifetime): {}", metrics.cleaned_count);

    // Add some temporary entries
    for i in 0..5 {
        cache
            .set_ex(&format!("temp:{}", i), format!("temp{}", i), 2)
            .await?;
    }
    println!("\nAdded 5 temporary entries with 2s TTL");

    let before = cache.get_health_metrics().await?;
    println!("Before expiration: {} keys", before.num_keys);

    println!("\nWaiting for expiration and cleanup...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    let after = cache.get_health_metrics().await?;
    println!("After cleanup:");
    println!("  Total keys: {}", after.num_keys);
    println!("  Entries cleaned: {}", after.cleaned_count);
    println!("  Last cleanup: {:?}", after.last_worker_time);

    // JSON serialization example (for API responses)
    let json = serde_json::to_string_pretty(&after)?;
    println!("\nJSON output for API response:");
    println!("{}", json);

    println!("\n=== Example Complete ===");

    Ok(())
}
