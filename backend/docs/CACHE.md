# Cache Implementation Guidelines

This document provides guidelines for future enhancements to the cache implementation, including Axum integration and Redis backend support.

## Table of Contents

1. [Usage](#usage)
2. [Current Implementation](#current-implementation)
3. [Axum Integration](#axum-integration)
4. [Redis Backend](#redis-backend)
5. [Migration Path](#migration-path)
6. [Best Practices](#best-practices)

## Usage

This section demonstrates how to use the cache module with practical examples. The cache provides a Redis-like API with TTL support, thread-safe operations, and automatic cleanup.

### Basic Setup

```rust
use backend::cache::{Cache, CacheConfig};

// Create a cache with default configuration
let cache: Cache<String> = Cache::new_local(CacheConfig::default());

// Or with custom configuration
let cache = Cache::new_local(CacheConfig {
    cleanup_interval_seconds: 60,  // Run cleanup every minute
    default_ttl_seconds: Some(3600), // Default 1 hour TTL for all entries
});
```

### 1. Basic CRUD Operations

The simplest operations - set, get, and delete values:

```rust
// Set a value
cache.set("user:123", "Alice").await?;

// Get a value (returns Option<String>)
if let Some(name) = cache.get("user:123").await? {
    println!("User name: {}", name);
}

// Delete a value (returns true if existed)
cache.delete("user:123").await?;
```

**Output:**
```
Set 'user:123' = 'Alice', Got: Some("Alice")
Delete 'user:123': true
```

**Use Cases:**
- Caching computed results
- Temporary data storage
- Session data

### 2. TTL (Time To Live) Operations

Control when entries expire automatically:

```rust
// Set with 5 second expiration
cache.set_ex("session:abc", "active", 5).await?;

// Check remaining time to live
let ttl = cache.ttl("session:abc").await?;
println!("Session expires in: {:?} seconds", ttl);

// Extend expiration (refresh session)
cache.expire("session:abc", 10).await?;

// Make entry permanent (remove expiration)
cache.persist("session:abc").await?;
```

**Output:**
```
Set 'session:abc' with 5 second TTL
Immediately after: Some("active")
TTL remaining: Some(4) seconds
Updated TTL to 10s, remaining: Some(10) seconds
Persisted (removed TTL), TTL: None
```

**Use Cases:**
- User sessions with auto-logout
- Rate limiting windows
- Cached API responses with automatic refresh

### 3. Existence Checks

Check if keys exist without retrieving values:

```rust
// Check if key exists
let exists = cache.exists("user:123").await?;
println!("User exists: {}", exists);

// Get all keys (excluding expired)
let keys = cache.keys().await?;
println!("All keys: {:?}", keys);

// Clear all entries
let count = cache.clear().await?;
println!("Cleared {} entries", count);
```

**Output:**
```
Exists 'user:123': true
All keys: ["user:1", "user:2", "session:abc"]
Cleared 3 entries
```

**Use Cases:**
- Cache invalidation
- Memory management
- Debugging and inspection

### 4. Atomic Operations

Perform operations safely without race conditions:

```rust
// Set only if key doesn't exist (useful for distributed locking)
let inserted = cache.set_nx("lock:resource", "locked").await?;
if inserted {
    println!("Lock acquired!");
}

// Get old value and set new value atomically
let old_value = cache.get_and_set("lock:resource", "locked_v2").await?;
println!("Previous value: {:?}", old_value);
```

**Output:**
```
set_nx 'lock:resource': true
set_nx 'lock:resource' again: false
Value should still be 'locked': Some("locked")
get_and_set returned: Some("locked")
New value: Some("locked_v2")
```

**Use Cases:**
- Distributed locking (prevent duplicate work)
- Atomic updates (compare-and-swap)
- Leader election

### 5. Batch Operations

Perform multiple operations efficiently:

```rust
// Set multiple keys at once
cache.mset(vec![
    ("batch:1", "value1"),
    ("batch:2", "value2"),
    ("batch:3", "value3"),
]).await?;

// Get multiple keys at once
let values = cache.mget(&["batch:1", "batch:2", "batch:3", "batch:999"]).await?;
println!("Results: {:?}", values);

// Delete multiple keys at once
let count = cache.mdelete(&["batch:1", "batch:2", "batch:3"]).await?;
println!("Deleted {} keys", count);
```

**Output:**
```
mset 3 keys
mget results: [Some("value1"), Some("value2"), Some("value3"), None]
mdelete 3 keys
```

**Use Cases:**
- Bulk data loading
- Multi-key retrieval
- Cache warming

### 6. Complex Types

Cache any serializable type:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

let user_cache: Cache<User> = Cache::new_local(CacheConfig::default());

// Store complex object
let user = User { id: 1, name: "Alice".to_string(), email: "alice@example.com".to_string() };
user_cache.set("user:1", user.clone()).await?;

// Retrieve complex object
if let Some(cached_user) = user_cache.get("user:1").await? {
    println!("Retrieved: {:?}", cached_user);
}
```

**Output:**
```
Stored and retrieved user: Some(User { id: 1, name: "Alice", email: "alice@example.com" })
```

**Use Cases:**
- Database row caching
- API response caching
- Configuration data

### 7. Thread Safety

Share cache across concurrent tasks:

```rust
let cache: Cache<String> = Cache::new_local(CacheConfig::default());
let cache_clone = cache.clone();

// Spawn concurrent tasks
tokio::spawn(async move {
    cache_clone.set("key1", "value1").await.unwrap();
});

// Original cache can still be used
cache.set("key2", "value2").await?;
```

**Output:**
```
Concurrent writes: 200 keys created
```

**Why this works:**
- Cache uses `Arc<DashMap>` internally
- DashMap provides lock-free concurrent access
- Clone is cheap (just copies the Arc pointer)

**Use Cases:**
- Web servers with concurrent requests
- Parallel data processing
- Shared state across async tasks

### 8. Background Cleanup

Automatic removal of expired entries:

```rust
let cache = Cache::new_local(CacheConfig {
    cleanup_interval_seconds: 2,  // Cleanup runs every 2 seconds
    default_ttl_seconds: None,
});

// Set temporary entries
cache.set_ex("temp:1", "value1", 3).await?;
cache.set("perm:1", "value2").await?; // No expiration

// After 5 seconds + cleanup interval
// temp:1 is automatically removed
// perm:1 still exists
```

**Output:**
```
Total keys before expiration: 8
Waiting 5 seconds for entries to expire...
Waiting 3 more seconds for background cleanup...
Total keys after cleanup: 3
Remaining keys: ["perm:0", "perm:1", "perm:2"]
```

**How it works:**
1. Entries expire after their TTL
2. Expired entries are removed on next read (lazy cleanup)
3. Background task runs every N seconds to remove expired entries (proactive cleanup)
4. Prevents memory bloat from unread expired keys

**Use Cases:**
- Long-running servers
- Memory-constrained environments
- High-volume caching

### 9. Default TTL Configuration

Set a global expiration policy:

```rust
let cache = Cache::new_local(CacheConfig {
    cleanup_interval_seconds: 60,
    default_ttl_seconds: Some(3600), // All entries expire in 1 hour by default
});

// Regular set() uses default TTL
cache.set("key", "value").await?;

// Check TTL (shows ~3600 seconds)
let ttl = cache.ttl("key").await?;
println!("Default TTL: {:?}", ttl);
```

**Output:**
```
Set keys with default TTL of 10s
TTL for 'auto:1': Some(9)
TTL for 'auto:2': Some(9)
```

**Use Cases:**
- Session caches (default 30 min)
- API response caches (default 5 min)
- Rate limit counters (default 1s)

### 10. Practical Example: User Session Cache

Real-world session management:

```rust
let session_cache: Cache<User> = Cache::new_local(CacheConfig {
    cleanup_interval_seconds: 60,
    default_ttl_seconds: Some(1800), // 30 minute sessions
});

// User logs in
let user = fetch_user_from_db("user:123").await?;
let session_id = generate_session_id();
session_cache.set_ex(&session_id, user.clone(), 300).await?; // 5 min for demo

// API request - validate session
if let Some(user) = session_cache.get(&session_id).await? {
    println!("Session valid for: {}", user.name);

    // User is active - refresh session
    session_cache.expire(&session_id, 300).await?;
} else {
    return Err("Invalid session".into());
}
```

**Output:**
```
User logged in, session cached
Session valid, user: Alice
Session expires in: Some(4) seconds
Session refreshed, TTL: Some(5)
```

**Real-world flow:**
1. User logs in → create session with 30 min TTL
2. User makes API request → validate from cache (fast, no DB query)
3. User is active → refresh TTL (extend session)
4. User inactive for 30 min → session auto-expires

### Running the Example

See all these features in action:

```bash
cargo run --example 04_cache_management
```

This runs through all 10 examples above with detailed output explanations.

## Current Implementation

The cache module (`src/cache.rs`) provides:

- **Local in-memory cache** using DashMap for concurrent access
- **TTL (Time To Live)** with background cleanup task
- **Redis-like API** with comprehensive operations
- **Thread-safe** (Clone + Send + Sync) for async contexts
- **Serialization support** (Serialize + Deserialize) for future Redis compatibility

### Architecture

```rust
pub enum Cache<V> {
    LocalCache(LocalBackend<V>),
}
```

The enum-based design allows easy extension with additional backends (Redis, Memcached, etc.) without changing the public API.

## Axum Integration

The current implementation is ready for Axum integration. Here's how to use it:

### 1. Add Axum Dependency

```toml
# Cargo.toml
[dependencies]
axum = "0.8.8"
tokio = { version = "1", features = ["full"] }
tower = "0.5"
```

### 2. Create Cache as Axum State

```rust
use axum::{Router, routing::get, extract::State};
use backend::cache::{Cache, CacheConfig};

#[derive(Clone)]
struct AppState {
    cache: Cache<String>,
}

#[tokio::main]
async fn main() {
    // Initialize cache
    let cache: Cache<String> = Cache::new_local(CacheConfig {
        cleanup_interval_seconds: 60,
        default_ttl_seconds: Some(3600), // 1 hour default TTL
    });

    let state = AppState { cache };

    let app = Router::new()
        .route("/users/{id}", get(get_user))
        .route("/users/{id}/clear", get(clear_user_cache))
        .with_state(state);

    // Run server...
}
```

### 3. Use Cache in Handlers

```rust
use axum::{extract::{Path, State}, Json, http::StatusCode};

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<User>, StatusCode> {
    // Try to get from cache
    if let Some(user) = state.cache.get(&format!("user:{}", id)).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Ok(Json(user));
    }

    // Cache miss - fetch from database
    let user = fetch_user_from_db(&id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Store in cache
    state.cache.set_ex(
        &format!("user:{}", id),
        user.clone(),
        300 // 5 minutes TTL
    ).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(user))
}

async fn clear_user_cache(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<String, StatusCode> {
    state.cache.delete(&format!("user:{}", id)).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(format!("Cleared cache for user {}", id))
}
```

### 4. Cache Middleware Pattern

For automatic caching of API responses:

```rust
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

async fn cache_middleware<B>(
    State(state): State<AppState>,
    req: Request<B>,
    next: Next<B>,
) -> Response {
    let cache_key = format!(
        "{}:{}",
        req.method(),
        req.uri().path()
    );

    // Try to get cached response
    if let Some(cached) = state.cache.get(&cache_key).await.unwrap() {
        return Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(cached.into())
            .unwrap();
    }

    // Continue to handler
    let response = next.run(req).await;

    // Cache successful GET responses
    if req.method() == axum::http::Method::GET
        && response.status().is_success()
    {
        // Cache for 60 seconds
        // Note: You'd need to extract response body here
        // This is simplified
    }

    response
}
```

## Redis Backend

When you're ready to add Redis support, follow these steps:

### 1. Add Fred Dependency

```toml
# Cargo.toml
[dependencies]
fred = "10"
serde_json = "1"
```

### 2. Create Redis Backend

```rust
// src/cache.rs (add to existing file)

use fred::prelude::*;

/// Redis backend implementation.
pub struct RedisBackend<V> {
    /// Fred Redis client
    client: RedisClient,
    /// Cache configuration
    config: CacheConfig,
    /// Serialization format (JSON, MessagePack, etc.)
    _phantom: std::marker::PhantomData<V>,
}

impl<V> RedisBackend<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    /// Create a new Redis backend.
    ///
    /// # Arguments
    /// * `redis_url` - Redis connection URL (e.g., "redis://127.0.0.1:6379")
    /// * `config` - Cache configuration
    pub async fn new(redis_url: &str, config: CacheConfig) -> Result<Self, Error> {
        let redis_config = Config::from_url(redis_url)
            .map_err(|e| Error::Cache(format!("Redis config error: {}", e)))?;

        let client = Builder::from_config(redis_config)
            .build()
            .map_err(|e| Error::Cache(format!("Redis client error: {}", e)))?;

        client
            .init()
            .await
            .map_err(|e| Error::Cache(format!("Redis init error: {}", e)))?;

        Ok(Self {
            client,
            config,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Serialize value to JSON bytes.
    fn serialize(value: &V) -> Result<Vec<u8>, Error> {
        serde_json::to_vec(value)
            .map_err(|e| Error::CacheSerialization(format!("Serialization error: {}", e)))
    }

    /// Deserialize value from JSON bytes.
    fn deserialize(bytes: &[u8]) -> Result<V, Error> {
        serde_json::from_slice(bytes)
            .map_err(|e| Error::CacheSerialization(format!("Deserialization error: {}", e)))
    }

    async fn get(&self, key: &str) -> Result<Option<V>> {
        let bytes: Option<Vec<u8>> = self
            .client
            .get(key)
            .await
            .map_err(|e| Error::Cache(format!("Redis GET error: {}", e)))?;

        bytes.map(|b| Self::deserialize(&b)).transpose()
    }

    async fn set(&self, key: &str, value: V) -> Result<()> {
        let bytes = Self::serialize(&value)?;

        // Fred uses None for no expiration
        let expiry: Option<Expiration> = self
            .config
            .default_ttl_seconds
            .map(|ttl| Expiration::EX(ttl as u64));

        self.client
            .set(key, bytes, expiry, None, false)
            .await
            .map_err(|e| Error::Cache(format!("Redis SET error: {}", e)))?;

        Ok(())
    }

    async fn set_ex(&self, key: &str, value: V, ttl_seconds: u64) -> Result<()> {
        let bytes = Self::serialize(&value)?;
        let expiry = Expiration::EX(ttl_seconds);

        self.client
            .set(key, bytes, Some(expiry), None, false)
            .await
            .map_err(|e| Error::Cache(format!("Redis SET_EX error: {}", e)))?;

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        let count: u8 = self
            .client
            .del(key)
            .await
            .map_err(|e| Error::Cache(format!("Redis DEL error: {}", e)))?;

        Ok(count > 0)
    }

    async fn ttl(&self, key: &str) -> Result<Option<i64>> {
        let ttl: i64 = self
            .client
            .ttl(key)
            .await
            .map_err(|e| Error::Cache(format!("Redis TTL error: {}", e)))?;

        // Fred returns -2 if key doesn't exist, -1 if no expiration
        match ttl {
            -2 => Ok(None),
            -1 => Ok(None), // No expiration
            ttl => Ok(Some(ttl)),
        }
    }

    // ... Implement all other cache methods similarly
}
```

### 3. Update Cache Enum

```rust
// src/cache.rs

pub enum Cache<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    LocalCache(LocalBackend<V>),
    #[allow(dead_code)]
    RedisCache(RedisBackend<V>), // Add Redis variant
}

impl<V> Cache<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
{
    /// Create a new Redis cache.
    pub async fn new_redis(redis_url: &str, config: CacheConfig) -> Result<Self, Error> {
        Ok(Self::RedisCache(RedisBackend::new(redis_url, config).await?))
    }

    /// Create a cache from environment variable.
    ///
    /// If BUILDSCALE_CACHE_REDIS_URL is set, uses Redis backend.
    /// Otherwise, uses local cache.
    pub async fn from_env(config: CacheConfig) -> Result<Self, Error> {
        if let Ok(redis_url) = std::env::var("BUILDSCALE_CACHE_REDIS_URL") {
            Self::new_redis(&redis_url, config).await
        } else {
            Ok(Self::new_local(config))
        }
    }
}
```

### 4. Update Enum Dispatch

```rust
// Update all cache methods to include Redis dispatch

impl<V> Cache<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
{
    pub async fn get(&self, key: &str) -> Result<Option<V>> {
        match self {
            Self::LocalCache(backend) => backend.get(key).await,
            Self::RedisCache(backend) => backend.get(key).await,
        }
    }

    pub async fn set(&self, key: &str, value: V) -> Result<()> {
        match self {
            Self::LocalCache(backend) => {
                backend.set(key, value).await;
                Ok(())
            }
            Self::RedisCache(backend) => backend.set(key, value).await,
        }
    }

    // ... Update all other methods similarly
}
```

## Why Fred?

[Fred](https://github.com/aembke/fred.rs) is a modern, high-performance Redis client for Rust with several advantages over the `redis` crate:

### Key Features of Fred:
- **Modern async/await design** - Built for tokio from the ground up
- **Better performance** - Zero-copy frame parsing, connection pooling
- **More features** - RESP2/RESP3, clustering, sentinel, TLS, pub/sub, transactions
- **Active maintenance** - Frequently updated with latest Redis features
- **Better documentation** - Comprehensive examples and API docs
- **Type-safe responses** - Converts Redis types to Rust types automatically

### Comparison with redis crate:
| Feature | Fred | redis-rs |
|---------|------|----------|
| RESP3 Support | ✅ | ❌ |
| Connection Pooling | ✅ Built-in | ⚠️ Limited |
| Zero-copy Parsing | ✅ | ❌ |
| Cluster Support | ✅ Native | ⚠️ Basic |
| Sentinel Support | ✅ Native | ⚠️ Basic |
| Async-only Design | ✅ | ⚠️ Sync/Async mixed |
| Active Development | ✅ | ⚠️ Slower |

## Migration Path

### Phase 1: Development (Current)
- Use local cache only
- Perfect for single-server deployments
- Fast and simple

### Phase 2: Production - Single Server
- Continue using local cache
- Add metrics (hit rate, memory usage)
- Monitor cache performance

### Phase 3: Production - Multiple Servers
- Add Redis backend support
- Use environment variable to switch between local/Redis
- Deploy Redis (hosted or self-hosted)

### Phase 4: Production - Distributed Cache
- Configure Redis with persistence
- Add cache cluster if needed
- Implement cache warming strategies

### Migration Strategy

```rust
// No code changes needed to switch from local to Redis

// Development
let cache: Cache<User> = Cache::new_local(CacheConfig::default());

// Production (just change this line!)
let cache: Cache<User> = Cache::new_redis(
    "redis://redis.example.com:6379",
    CacheConfig::default()
).await?;

// Or use environment variable
let cache: Cache<User> = Cache::from_env(CacheConfig::default()).await?;
```

## Best Practices

### 1. Cache Key Design

Use consistent, hierarchical key naming:

```rust
// Good: Namespaced keys
let user_cache_key = format!("user:{}", user_id);
let session_key = format!("session:{}", session_id);
let search_results_key = format!("search:{}:page:{}", query, page);

// Bad: Inconsistent naming
let key1 = format!("user-{}", user_id);
let key2 = format!("user_{}", user_id);
```

### 2. TTL Selection

Choose appropriate TTL based on data volatility:

```rust
// Very volatile (real-time data) - 30 seconds
cache.set_ex("stock:price:AAPL", price, 30).await?;

// Semi-volatile (user activity) - 5 minutes
cache.set_ex("user:online:123", status, 300).await?;

// Static (user profiles) - 1 hour
cache.set_ex("user:profile:123", profile, 3600).await?;

// Rarely changes (config data) - 24 hours
cache.set_ex("config:features", features, 86400).await?;
```

### 3. Cache Invalidation

```rust
// Invalidate related caches when data changes
async fn update_user(user_id: &str, new_data: User) -> Result<()> {
    // Update database
    db.update_user(user_id, &new_data).await?;

    // Clear user profile cache
    cache.delete(&format!("user:profile:{}", user_id)).await?;

    // Clear user list cache
    cache.delete("users:list").await?;

    Ok(())
}
```

### 4. Avoiding Stampede

Use atomic operations to prevent cache stampede:

```rust
async fn get_expensive_result(key: &str) -> Result<Data> {
    // Try to get from cache
    if let Some(data) = cache.get(key).await? {
        return Ok(data);
    }

    // Use set_nx to ensure only one thread computes
    let lock_key = format!("lock:{}", key);
    cache.set_nx(&lock_key, "computing".to_string()).await?;

    if cache.get(&lock_key).await?.is_some() {
        // We got the lock, compute the result
        let data = compute_expensively().await?;

        // Store result
        cache.set_ex(key, data.clone(), 3600).await?;

        // Release lock
        cache.delete(&lock_key).await?;

        Ok(data)
    } else {
        // Another thread is computing, wait and retry
        tokio::time::sleep(Duration::from_millis(100)).await;
        get_expensive_result(key).await
    }
}
```

### 5. Monitoring

Track cache metrics:

```rust
#[derive(Clone)]
struct MonitoredCache<V> {
    cache: Cache<V>,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
}

impl<V> MonitoredCache<V> {
    async fn get(&self, key: &str) -> Result<Option<V>> {
        match self.cache.get(key).await? {
            Some(value) => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                Ok(Some(value))
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            }
        }
    }

    fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed) as f64;
        let misses = self.misses.load(Ordering::Relaxed) as f64;
        hits / (hits + misses)
    }
}
```

### 6. Configuration

```rust
// .env
BUILDSCALE_CACHE_REDIS_URL=redis://localhost:6379
BUILDSCALE_CACHE_DEFAULT_TTL=3600
BUILDSCALE_CACHE_CLEANUP_INTERVAL=60

// Load from environment
let config = CacheConfig {
    cleanup_interval_seconds: std::env::var("BUILDSCALE_CACHE_CLEANUP_INTERVAL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60),
    default_ttl_seconds: std::env::var("BUILDSCALE_CACHE_DEFAULT_TTL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(None),
};
```

## Performance Considerations

### Local Cache
- **Pros**: Fast (nanoseconds), no network latency
- **Cons**: Limited by server memory, not shared across servers
- **Use case**: Single-server deployments, small datasets

### Redis Cache
- **Pros**: Shared across servers, persistent options, distributed
- **Cons**: Network latency (milliseconds), additional infrastructure
- **Use case**: Multi-server deployments, large datasets, production scale

### Hybrid Approach
```rust
// Use local cache as L1, Redis as L2
async fn get_with_l2(&self, key: &str) -> Result<Option<V>> {
    // Try L1 (local) first
    if let Some(value) = self.local_cache.get(key).await? {
        return Ok(Some(value));
    }

    // Try L2 (Redis)
    if let Some(value) = self.redis_cache.get(key).await? {
        // Populate L1
        self.local_cache.set(key, value.clone()).await?;
        return Ok(Some(value));
    }

    Ok(None)
}
```

## Testing

### Unit Tests
```rust
#[tokio::test]
async fn test_cache_hit_rate() {
    let cache = Cache::new_local(CacheConfig::default());

    cache.set("key", "value").await.unwrap();
    assert!(cache.get("key").await.unwrap().is_some());
}
```

### Integration Tests with Redis
```rust
#[tokio::test]
#[ignore] // Only run when Redis is available
async fn test_redis_cache() {
    let cache = Cache::new_redis("redis://localhost:6379", CacheConfig::default())
        .await
        .unwrap();

    cache.set("key", "value").await.unwrap();
    assert_eq!(cache.get("key").await.unwrap(), Some("value".to_string()));
}
```

## Troubleshooting

### High Memory Usage
```rust
// Monitor cache size
let keys = cache.keys().await?;
println!("Cache entries: {}", keys.len());

// Reduce TTL or add cleanup
cache.clear().await?;
```

### Low Hit Rate
```rust
// Increase TTL for frequently accessed data
cache.set_ex("popular_key", data, 3600).await?;

// Use cache warming on startup
warmup_cache().await?;
```

### Stale Data
```rust
// Reduce TTL for volatile data
cache.set_ex("volatile_key", data, 30).await?;

// Implement explicit invalidation
cache.delete("key").await?;
```

## Future Enhancements

1. **Cache partitioning** - Separate caches for different data types
2. **Cache warming** - Pre-populate cache on startup
3. **Metrics** - Hit rate, memory usage, operation counts
4. **Compression** - Compress large values before storing
5. **Sharding** - Distribute cache across multiple Redis instances
6. **Replication** - Redis replication for high availability
7. **Pub/Sub** - Cache invalidation across servers
8. **Transactions** - Multi-key operations with ACID guarantees

## Resources

- [Axum Documentation](https://docs.rs/axum/)
- [Fred Documentation](https://docs.rs/fred/)
- [Fred GitHub](https://github.com/aembke/fred.rs)
- [Redis Documentation](https://redis.io/docs/)
- [DashMap Documentation](https://docs.rs/dashmap/)
- [Serde Documentation](https://docs.rs/serde/)
