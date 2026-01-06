//! Generic async cache with TTL support.
//!
//! This module provides a Redis-like caching interface with:
//! - Async API using scc HashMap for safe concurrent access
//! - TTL (Time To Live) with background cleanup
//! - Thread-safe (Clone + Send + Sync) for use in async contexts
//! - Serialization support for future Redis migration
//!
//! # Why scc HashMap instead of DashMap?
//!
//! scc HashMap is async-first and designed to prevent deadlocks in async contexts:
//! - **Async methods**: All operations have async variants that properly yield
//! - **Lock-free resizing**: No blocking operations during resize
//! - **No iterator deadlocks**: Uses `iter_async` instead of blocking iterators
//! - **Safe background cleanup**: `retain_async` yields instead of blocking
//!
//! DashMap can deadlock when locks are held across `.await` points, especially in
//! single-threaded executors. See the Tobira application incident (PR #1141) for
//! a real-world example of this issue.

use crate::error::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::task::JoinHandle;

/// A cache entry with optional expiration time.
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    /// The cached value
    value: V,
    /// Optional expiration timestamp (None means no expiration)
    expires_at: Option<DateTime<Utc>>,
}

impl<V> CacheEntry<V> {
    /// Create a new cache entry without expiration.
    fn new(value: V) -> Self {
        Self {
            value,
            expires_at: None,
        }
    }

    /// Create a new cache entry with expiration.
    fn with_expiration(value: V, ttl_seconds: i64) -> Self {
        Self {
            value,
            expires_at: Some(Utc::now() + Duration::seconds(ttl_seconds)),
        }
    }

    /// Check if the entry has expired.
    fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| exp < Utc::now())
            .unwrap_or(false)
    }

    /// Get remaining TTL in seconds (None if no expiration).
    fn remaining_ttl(&self) -> Option<i64> {
        self.expires_at.map(|exp| {
            let remaining = exp - Utc::now();
            remaining.num_seconds().max(0)
        })
    }
}

/// Cache configuration options.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Background cleanup interval in seconds (default: 60)
    pub cleanup_interval_seconds: u64,
    /// Default TTL in seconds for entries (None means no expiration)
    pub default_ttl_seconds: Option<u64>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cleanup_interval_seconds: 60,
            default_ttl_seconds: None,
        }
    }
}

/// Health metrics for cache monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheHealthMetrics {
    /// Current number of entries in cache
    pub num_keys: usize,
    /// ISO8601 UTC timestamp of last cleanup completion (None if never run)
    pub last_worker_time: Option<String>,
    /// Number of entries removed by last cleanup (0 if never run)
    pub cleaned_count: u64,
    /// Estimated memory usage in bytes
    pub size_bytes: u64,
}

/// Internal metrics storage (thread-safe, separate from cache data).
#[derive(Debug, Default)]
struct CacheMetrics {
    /// Current number of entries
    num_keys: AtomicUsize,
    /// Last cleanup completion time
    last_worker_time: tokio::sync::RwLock<Option<DateTime<Utc>>>,
    /// Number of entries removed by last cleanup
    cleaned_count: AtomicU64,
    /// Estimated memory usage in bytes
    size_bytes: AtomicU64,
}

/// Local backend implementation using scc HashMap.
#[derive(Debug)]
pub struct LocalBackend<V> {
    /// Thread-safe storage for cache entries
    storage: Arc<scc::HashMap<String, CacheEntry<V>>>,
    /// Background cleanup task handle
    cleanup_task: Option<JoinHandle<()>>,
    /// Cache configuration
    config: CacheConfig,
    /// Health metrics (separate from cache data for type safety)
    metrics: Arc<CacheMetrics>,
}

impl<V> LocalBackend<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
{
    /// Create a new local backend with the given configuration.
    fn new(config: CacheConfig) -> Self {
        let storage = Arc::new(scc::HashMap::new());
        let metrics = Arc::new(CacheMetrics::default());
        let cleanup_task = Some(Self::spawn_cleanup_task(
            Arc::clone(&storage),
            Arc::clone(&metrics),
            config.cleanup_interval_seconds,
        ));

        Self {
            storage,
            cleanup_task,
            config,
            metrics,
        }
    }

    /// Spawn a background task to clean up expired entries.
    ///
    /// Uses scc's `retain_async` which properly yields, preventing deadlocks
    /// that can occur with DashMap's blocking `retain` in async contexts.
    fn spawn_cleanup_task(
        storage: Arc<scc::HashMap<String, CacheEntry<V>>>,
        metrics: Arc<CacheMetrics>,
        interval_seconds: u64,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_seconds));
            loop {
                interval.tick().await;
                let now = Utc::now();

                // Count keys before cleanup
                let count_before = storage.len();

                // SAFE: retain_async yields properly, won't deadlock
                storage
                    .retain_async(|_, entry| {
                        entry
                            .expires_at
                            .map(|exp| exp > now)
                            .unwrap_or(true)
                    })
                    .await;

                // Calculate metrics
                let count_after = storage.len();
                let cleaned_count = count_before.saturating_sub(count_after) as u64;
                let num_keys = count_after;
                let size_bytes = Self::estimate_size(num_keys);

                // Update atomic metrics (accumulate cleaned_count over lifetime)
                metrics.num_keys.store(num_keys, Ordering::Relaxed);
                if cleaned_count > 0 {
                    metrics.cleaned_count.fetch_add(cleaned_count, Ordering::Relaxed);
                }
                metrics.size_bytes.store(size_bytes, Ordering::Relaxed);
                *metrics.last_worker_time.write().await = Some(now);
            }
        })
    }

    /// Check if a key exists and is not expired.
    async fn exists(&self, key: &str) -> bool {
        self.storage
            .read(key, |_, entry| !entry.is_expired())
            .unwrap_or(false)
    }

    /// Get a value by key (returns None if key doesn't exist or is expired).
    async fn get(&self, key: &str) -> Option<V> {
        self.storage
            .read(key, |_, entry| {
                if !entry.is_expired() {
                    Some(entry.value.clone())
                } else {
                    None
                }
            })
            .flatten()
    }

    /// Set a value without expiration (or with default TTL if configured).
    async fn set(&self, key: &str, value: V) {
        let entry = if let Some(default_ttl) = self.config.default_ttl_seconds {
            CacheEntry::with_expiration(value, default_ttl as i64)
        } else {
            CacheEntry::new(value)
        };
        let _ = self.storage.insert(key.to_string(), entry.clone());

        // If insert failed (key exists), update it
        if self.storage.read(key, |_, _| true).unwrap_or(false) {
            self.storage.update(key, |_, existing| *existing = entry);
        }
    }

    /// Set a value with expiration in seconds.
    async fn set_ex(&self, key: &str, value: V, ttl_seconds: u64) {
        let entry = CacheEntry::with_expiration(value, ttl_seconds as i64);
        let _ = self.storage.insert(key.to_string(), entry.clone());

        // If insert failed (key exists), update it
        if self.storage.read(key, |_, _| true).unwrap_or(false) {
            self.storage.update(key, |_, existing| *existing = entry);
        }
    }

    /// Delete a key (returns true if key existed).
    async fn delete(&self, key: &str) -> bool {
        self.storage.remove(key).is_some()
    }

    /// Get the remaining TTL for a key (None if no expiration).
    async fn ttl(&self, key: &str) -> Option<i64> {
        self.storage
            .read(key, |_, entry| entry.remaining_ttl())
            .flatten()
    }

    /// Set expiration on an existing key (returns true if key existed).
    async fn expire(&self, key: &str, ttl_seconds: u64) -> bool {
        self.storage
            .update(key, |_, entry| {
                entry.expires_at = Some(Utc::now() + Duration::seconds(ttl_seconds as i64));
                true
            })
            .is_some()
    }

    /// Remove expiration from a key (returns true if key existed).
    async fn persist(&self, key: &str) -> bool {
        self.storage
            .update(key, |_, entry| {
                entry.expires_at = None;
                true
            })
            .is_some()
    }

    /// Set a value only if the key doesn't exist (returns true if set).
    async fn set_nx(&self, key: &str, value: V) -> bool {
        let entry = if let Some(default_ttl) = self.config.default_ttl_seconds {
            CacheEntry::with_expiration(value, default_ttl as i64)
        } else {
            CacheEntry::new(value)
        };

        // Check if key exists first
        if self.storage.read(key, |_, _| true).unwrap_or(false) {
            return false;
        }

        // Key doesn't exist, insert it
        self.storage.insert(key.to_string(), entry).is_ok()
    }

    /// Get and set a value atomically (returns old value).
    async fn get_and_set(&self, key: &str, value: V) -> Option<V> {
        let entry = if let Some(default_ttl) = self.config.default_ttl_seconds {
            CacheEntry::with_expiration(value, default_ttl as i64)
        } else {
            CacheEntry::new(value)
        };

        // Read old value first (returns None if key doesn't exist or is expired)
        let old_value = self.storage.read(key, |_, entry| {
            if !entry.is_expired() {
                Some(entry.value.clone())
            } else {
                None
            }
        }).flatten();

        // Check if key exists (even if expired)
        let key_exists = self.storage.read(key, |_, _| true).unwrap_or(false);

        // Insert or update
        if key_exists {
            // Key exists, update it
            self.storage.update(key, |_, existing| *existing = entry);
        } else {
            // Key doesn't exist, insert it
            let _ = self.storage.insert(key.to_string(), entry);
        }

        old_value
    }

    /// Get all non-expired keys.
    async fn keys(&self) -> Vec<String> {
        let now = Utc::now();
        let mut keys = Vec::new();

        // Scan all entries and collect non-expired keys
        self.storage
            .scan_async(|key, entry| {
                if entry.expires_at.map(|exp| exp > now).unwrap_or(true) {
                    keys.push(key.clone());
                }
            })
            .await;

        keys
    }

    /// Clear all entries (returns count of cleared entries).
    async fn clear(&self) -> usize {
        let count = self.storage.len();
        self.storage.clear();
        count
    }

    /// Get multiple keys.
    async fn mget(&self, keys: &[&str]) -> Vec<Option<V>> {
        keys.iter()
            .map(|&key| {
                self.storage
                    .read(key, |_, entry| {
                        if !entry.is_expired() {
                            Some(entry.value.clone())
                        } else {
                            None
                        }
                    })
                    .flatten()
            })
            .collect()
    }

    /// Set multiple keys.
    async fn mset(&self, items: Vec<(&str, V)>) {
        for (key, value) in items {
            let entry = if let Some(default_ttl) = self.config.default_ttl_seconds {
                CacheEntry::with_expiration(value, default_ttl as i64)
            } else {
                CacheEntry::new(value)
            };
            let _ = self.storage.insert(key.to_string(), entry.clone());

            // If insert failed (key exists), update it
            if self.storage.read(key, |_, _| true).unwrap_or(false) {
                self.storage.update(key, |_, existing| *existing = entry);
            }
        }
    }

    /// Delete multiple keys (returns count of deleted keys).
    async fn mdelete(&self, keys: &[&str]) -> u64 {
        keys.iter()
            .filter(|&&key| self.storage.remove(key).is_some())
            .count() as u64
    }

    /// Get health metrics from atomic fields.
    async fn get_health_metrics_impl(&self) -> CacheHealthMetrics {
        // Calculate current metrics in real-time
        let num_keys = self.storage.len();
        let size_bytes = Self::estimate_size(num_keys);

        // Read metrics from last cleanup (these are only updated by cleanup task)
        let cleaned_count = self.metrics.cleaned_count.load(Ordering::Relaxed);

        // Read timestamp (requires async lock)
        let last_worker_time = self.metrics.last_worker_time.read().await.map(|ts| {
            ts.to_rfc3339()
        });

        CacheHealthMetrics {
            num_keys,
            last_worker_time,
            cleaned_count,
            size_bytes,
        }
    }

    /// Estimate memory usage in bytes using average entry size.
    ///
    /// Uses average key size (String struct + heap allocation) for O(1) calculation
    /// instead of O(n) scan_async operation.
    fn estimate_size(num_entries: usize) -> u64 {
        // Average key size: 32 bytes (24 byte String struct + ~8 byte heap allocation)
        // Value size: CacheEntry struct size
        let avg_key_size = 32u64;
        let value_size = std::mem::size_of::<CacheEntry<V>>() as u64;
        let entry_size = avg_key_size + value_size;

        num_entries as u64 * entry_size
    }
}

impl<V> Drop for LocalBackend<V> {
    fn drop(&mut self) {
        // Abort the cleanup task when the backend is dropped
        if let Some(task) = self.cleanup_task.take() {
            task.abort();
        }
    }
}

/// Generic cache enum with extensible backend variants.
///
/// Currently only supports `LocalCache` variant, but designed to be
/// extended with `RedisCache` or other backends in the future.
///
/// # Type Parameters
/// * `V` - The value type, must be serializable for future Redis compatibility
///
/// # Example
/// ```rust,no_run
/// use backend::cache::Cache;
///
/// // Create a local cache
/// # #[tokio::main]
/// # async fn main() {
/// let cache: Cache<String> = Cache::new_local(Default::default());
/// # }
/// ```
#[derive(Debug)]
pub enum Cache<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    /// Local in-memory cache using scc HashMap (async-safe, no deadlocks)
    LocalCache(LocalBackend<V>),
}

impl<V> Cache<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
{
    /// Create a new local cache with the given configuration.
    ///
    /// # Arguments
    /// * `config` - Cache configuration options
    ///
    /// # Example
    /// ```rust,no_run
    /// use backend::cache::{Cache, CacheConfig};
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let cache: Cache<String> = Cache::new_local(CacheConfig::default());
    /// # }
    /// ```
    pub fn new_local(config: CacheConfig) -> Self {
        Self::LocalCache(LocalBackend::new(config))
    }

    /// Check if a key exists and is not expired.
    ///
    /// # Arguments
    /// * `key` - The key to check
    ///
    /// # Returns
    /// * `Ok(true)` if key exists and is not expired
    /// * `Ok(false)` otherwise
    pub async fn exists(&self, key: &str) -> Result<bool> {
        match self {
            Self::LocalCache(backend) => Ok(backend.exists(key).await),
        }
    }

    /// Get a value by key.
    ///
    /// # Arguments
    /// * `key` - The key to retrieve
    ///
    /// # Returns
    /// * `Ok(Some(value))` if key exists and is not expired
    /// * `Ok(None)` if key doesn't exist or is expired
    pub async fn get(&self, key: &str) -> Result<Option<V>> {
        match self {
            Self::LocalCache(backend) => Ok(backend.get(key).await),
        }
    }

    /// Set a value without expiration (or with default TTL if configured).
    ///
    /// # Arguments
    /// * `key` - The key to set
    /// * `value` - The value to store
    pub async fn set(&self, key: &str, value: V) -> Result<()> {
        match self {
            Self::LocalCache(backend) => {
                backend.set(key, value).await;
                Ok(())
            }
        }
    }

    /// Set a value with expiration in seconds.
    ///
    /// # Arguments
    /// * `key` - The key to set
    /// * `value` - The value to store
    /// * `ttl_seconds` - Time to live in seconds
    pub async fn set_ex(&self, key: &str, value: V, ttl_seconds: u64) -> Result<()> {
        match self {
            Self::LocalCache(backend) => {
                backend.set_ex(key, value, ttl_seconds).await;
                Ok(())
            }
        }
    }

    /// Delete a key.
    ///
    /// # Arguments
    /// * `key` - The key to delete
    ///
    /// # Returns
    /// * `Ok(true)` if key existed and was deleted
    /// * `Ok(false)` if key didn't exist
    pub async fn delete(&self, key: &str) -> Result<bool> {
        match self {
            Self::LocalCache(backend) => Ok(backend.delete(key).await),
        }
    }

    /// Get the remaining TTL for a key in seconds.
    ///
    /// # Arguments
    /// * `key` - The key to check
    ///
    /// # Returns
    /// * `Ok(Some(seconds))` - Remaining TTL if key has expiration
    /// * `Ok(None)` - If key exists but has no expiration
    pub async fn ttl(&self, key: &str) -> Result<Option<i64>> {
        match self {
            Self::LocalCache(backend) => Ok(backend.ttl(key).await),
        }
    }

    /// Set expiration on an existing key.
    ///
    /// # Arguments
    /// * `key` - The key to set expiration on
    /// * `ttl_seconds` - Time to live in seconds
    ///
    /// # Returns
    /// * `Ok(true)` if key existed and expiration was set
    /// * `Ok(false)` if key didn't exist
    pub async fn expire(&self, key: &str, ttl_seconds: u64) -> Result<bool> {
        match self {
            Self::LocalCache(backend) => Ok(backend.expire(key, ttl_seconds).await),
        }
    }

    /// Remove expiration from a key (make it persistent).
    ///
    /// # Arguments
    /// * `key` - The key to make persistent
    ///
    /// # Returns
    /// * `Ok(true)` if key existed and expiration was removed
    /// * `Ok(false)` if key didn't exist
    pub async fn persist(&self, key: &str) -> Result<bool> {
        match self {
            Self::LocalCache(backend) => Ok(backend.persist(key).await),
        }
    }

    /// Set a value only if the key doesn't exist.
    ///
    /// # Arguments
    /// * `key` - The key to set
    /// * `value` - The value to store
    ///
    /// # Returns
    /// * `Ok(true)` if value was set (key didn't exist)
    /// * `Ok(false)` if key already exists
    pub async fn set_nx(&self, key: &str, value: V) -> Result<bool> {
        match self {
            Self::LocalCache(backend) => Ok(backend.set_nx(key, value).await),
        }
    }

    /// Get and set a value atomically.
    ///
    /// # Arguments
    /// * `key` - The key to get and set
    /// * `value` - The new value to set
    ///
    /// # Returns
    /// * `Ok(Some(old_value))` if key existed and was not expired
    /// * `Ok(None)` if key didn't exist or was expired
    pub async fn get_and_set(&self, key: &str, value: V) -> Result<Option<V>> {
        match self {
            Self::LocalCache(backend) => Ok(backend.get_and_set(key, value).await),
        }
    }

    /// Get all non-expired keys.
    ///
    /// # Returns
    /// * `Ok(keys)` - Vector of all non-expired keys
    pub async fn keys(&self) -> Result<Vec<String>> {
        match self {
            Self::LocalCache(backend) => Ok(backend.keys().await),
        }
    }

    /// Clear all entries from the cache.
    ///
    /// # Returns
    /// * `Ok(count)` - Number of entries cleared
    pub async fn clear(&self) -> Result<usize> {
        match self {
            Self::LocalCache(backend) => Ok(backend.clear().await),
        }
    }

    /// Get multiple keys.
    ///
    /// # Arguments
    /// * `keys` - Slice of keys to retrieve
    ///
    /// # Returns
    /// * `Ok(values)` - Vector of Options corresponding to each key
    pub async fn mget(&self, keys: &[&str]) -> Result<Vec<Option<V>>> {
        match self {
            Self::LocalCache(backend) => Ok(backend.mget(keys).await),
        }
    }

    /// Set multiple keys.
    ///
    /// # Arguments
    /// * `items` - Vector of (key, value) pairs to set
    pub async fn mset(&self, items: Vec<(&str, V)>) -> Result<()> {
        match self {
            Self::LocalCache(backend) => {
                backend.mset(items).await;
                Ok(())
            }
        }
    }

    /// Delete multiple keys.
    ///
    /// # Arguments
    /// * `keys` - Slice of keys to delete
    ///
    /// # Returns
    /// * `Ok(count)` - Number of keys deleted
    pub async fn mdelete(&self, keys: &[&str]) -> Result<u64> {
        match self {
            Self::LocalCache(backend) => Ok(backend.mdelete(keys).await),
        }
    }

    /// Get health metrics for monitoring cache state.
    ///
    /// Returns metrics about cache usage, cleanup operations, and memory.
    /// Metrics are updated by the background cleanup worker.
    ///
    /// # Returns
    /// * `Ok(CacheHealthMetrics)` - Current health metrics
    ///
    /// # Example
    /// ```rust
    /// use backend::cache::{Cache, CacheConfig};
    ///
    /// # async fn example() {
    /// let cache: Cache<String> = Cache::new_local(CacheConfig::default());
    /// let metrics = cache.get_health_metrics().await.unwrap();
    /// println!("Keys: {}", metrics.num_keys);
    /// println!("Last cleanup: {:?}", metrics.last_worker_time);
    /// # }
    /// ```
    pub async fn get_health_metrics(&self) -> Result<CacheHealthMetrics> {
        match self {
            Self::LocalCache(backend) => Ok(backend.get_health_metrics_impl().await),
        }
    }
}

// Implement Clone for Cache (shallow clone via Arc)
impl<V> Clone for Cache<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        match self {
            // Note: scc HashMap is wrapped in Arc, so cloning is cheap
            // We don't implement Clone on LocalBackend directly to avoid
            // accidentally cloning the cleanup task
            Self::LocalCache(backend) => {
                // Create a new LocalBackend that shares the same storage
                // but doesn't have its own cleanup task
                let storage = Arc::clone(&backend.storage);
                let metrics = Arc::clone(&backend.metrics);
                let config = backend.config.clone();

                // Create a new backend without cleanup task (shared storage)
                // This is safe because the original backend's cleanup task
                // will clean up entries for all shared references
                Self::LocalCache(LocalBackend {
                    storage,
                    cleanup_task: None,
                    config,
                    metrics,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_set_get() {
        let cache: Cache<String> = Cache::new_local(CacheConfig::default());

        cache.set("key1", "value1".to_string()).await.unwrap();
        let value = cache.get("key1").await.unwrap();

        assert_eq!(value, Some("value1".to_string()));
    }

    #[tokio::test]
    async fn test_delete() {
        let cache: Cache<String> = Cache::new_local(CacheConfig::default());

        cache.set("key1", "value1".to_string()).await.unwrap();
        assert!(cache.delete("key1").await.unwrap());
        assert!(!cache.delete("key1").await.unwrap());

        assert_eq!(cache.get("key1").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_exists() {
        let cache: Cache<String> = Cache::new_local(CacheConfig::default());

        assert!(!cache.exists("key1").await.unwrap());

        cache.set("key1", "value1".to_string()).await.unwrap();
        assert!(cache.exists("key1").await.unwrap());

        cache.delete("key1").await.unwrap();
        assert!(!cache.exists("key1").await.unwrap());
    }
}
