//! Generic async cache with TTL support.
//!
//! This module provides a Redis-like caching interface with:
//! - Async API using DashMap for concurrent access
//! - TTL (Time To Live) with background cleanup
//! - Thread-safe (Clone + Send + Sync) for use in async contexts
//! - Serialization support for future Redis migration

use crate::error::Result;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use serde::{de::DeserializeOwned, Serialize};
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

/// Local backend implementation using DashMap.
#[derive(Debug)]
pub struct LocalBackend<V> {
    /// Thread-safe storage for cache entries
    storage: Arc<DashMap<String, CacheEntry<V>>>,
    /// Background cleanup task handle
    cleanup_task: Option<JoinHandle<()>>,
    /// Cache configuration
    config: CacheConfig,
}

impl<V> LocalBackend<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
{
    /// Create a new local backend with the given configuration.
    fn new(config: CacheConfig) -> Self {
        let storage = Arc::new(DashMap::new());
        let cleanup_task = Some(Self::spawn_cleanup_task(
            Arc::clone(&storage),
            config.cleanup_interval_seconds,
        ));

        Self {
            storage,
            cleanup_task,
            config,
        }
    }

    /// Spawn a background task to clean up expired entries.
    fn spawn_cleanup_task(
        storage: Arc<DashMap<String, CacheEntry<V>>>,
        interval_seconds: u64,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_seconds));
            loop {
                interval.tick().await;
                let now = Utc::now();
                storage.retain(|_, entry| {
                    entry
                        .expires_at
                        .map(|exp| exp > now)
                        .unwrap_or(true)
                });
            }
        })
    }

    /// Check if a key exists and is not expired.
    async fn exists(&self, key: &str) -> bool {
        if let Some(entry) = self.storage.get(key) {
            !entry.is_expired()
        } else {
            false
        }
    }

    /// Get a value by key (returns None if key doesn't exist or is expired).
    async fn get(&self, key: &str) -> Option<V> {
        if let Some(entry) = self.storage.get(key) {
            if !entry.is_expired() {
                return Some(entry.value.clone());
            }
        }
        None
    }

    /// Set a value without expiration.
    async fn set(&self, key: &str, value: V) {
        let entry = if let Some(default_ttl) = self.config.default_ttl_seconds {
            CacheEntry::with_expiration(value, default_ttl as i64)
        } else {
            CacheEntry::new(value)
        };
        self.storage.insert(key.to_string(), entry);
    }

    /// Set a value with expiration in seconds.
    async fn set_ex(&self, key: &str, value: V, ttl_seconds: u64) {
        let entry = CacheEntry::with_expiration(value, ttl_seconds as i64);
        self.storage.insert(key.to_string(), entry);
    }

    /// Delete a key (returns true if key existed).
    async fn delete(&self, key: &str) -> bool {
        self.storage.remove(key).is_some()
    }

    /// Get the remaining TTL for a key (None if no expiration).
    async fn ttl(&self, key: &str) -> Option<i64> {
        self.storage.get(key).and_then(|entry| entry.remaining_ttl())
    }

    /// Set expiration on an existing key (returns true if key existed).
    async fn expire(&self, key: &str, ttl_seconds: u64) -> bool {
        if let Some(mut entry) = self.storage.get_mut(key) {
            entry.expires_at = Some(Utc::now() + Duration::seconds(ttl_seconds as i64));
            true
        } else {
            false
        }
    }

    /// Remove expiration from a key (returns true if key existed).
    async fn persist(&self, key: &str) -> bool {
        if let Some(mut entry) = self.storage.get_mut(key) {
            entry.expires_at = None;
            true
        } else {
            false
        }
    }

    /// Set a value only if the key doesn't exist (returns true if set).
    async fn set_nx(&self, key: &str, value: V) -> bool {
        use dashmap::mapref::entry::Entry;
        match self.storage.entry(key.to_string()) {
            Entry::Vacant(entry) => {
                let cache_entry = if let Some(default_ttl) = self.config.default_ttl_seconds {
                    CacheEntry::with_expiration(value, default_ttl as i64)
                } else {
                    CacheEntry::new(value)
                };
                entry.insert(cache_entry);
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    /// Get and set a value atomically (returns old value).
    async fn get_and_set(&self, key: &str, value: V) -> Option<V> {
        let entry = if let Some(default_ttl) = self.config.default_ttl_seconds {
            CacheEntry::with_expiration(value, default_ttl as i64)
        } else {
            CacheEntry::new(value)
        };

        let old_entry = self.storage.insert(key.to_string(), entry)?;
        if !old_entry.is_expired() {
            Some(old_entry.value)
        } else {
            None
        }
    }

    /// Get all non-expired keys.
    async fn keys(&self) -> Vec<String> {
        let now = Utc::now();
        self.storage
            .iter()
            .filter(|entry| {
                entry
                    .expires_at
                    .map(|exp| exp > now)
                    .unwrap_or(true)
            })
            .map(|entry| entry.key().clone())
            .collect()
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
                self.storage.get(key).and_then(|entry| {
                    if !entry.is_expired() {
                        Some(entry.value.clone())
                    } else {
                        None
                    }
                })
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
            self.storage.insert(key.to_string(), entry);
        }
    }

    /// Delete multiple keys (returns count of deleted keys).
    async fn mdelete(&self, keys: &[&str]) -> u64 {
        keys.iter()
            .filter(|&&key| self.storage.remove(key).is_some())
            .count() as u64
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
/// ```rust
/// use backend::cache::Cache;
///
/// // Create a local cache
/// let cache: Cache<String> = Cache::new_local(Default::default());
/// ```
#[derive(Debug)]
pub enum Cache<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    /// Local in-memory cache using DashMap
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
    /// ```rust
    /// use backend::cache::{Cache, CacheConfig};
    ///
    /// let cache: Cache<String> = Cache::new_local(CacheConfig::default());
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
}

// Implement Clone for Cache (shallow clone via Arc)
impl<V> Clone for Cache<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        match self {
            // Note: LocalBackend stores Arc<DashMap>, so cloning is cheap
            // We don't implement Clone on LocalBackend directly to avoid
            // accidentally cloning the cleanup task
            Self::LocalCache(backend) => {
                // Create a new LocalBackend that shares the same storage
                // but doesn't have its own cleanup task
                let storage = Arc::clone(&backend.storage);
                let config = backend.config.clone();

                // Create a new backend without cleanup task (shared storage)
                // This is safe because the original backend's cleanup task
                // will clean up entries for all shared references
                Self::LocalCache(LocalBackend {
                    storage,
                    cleanup_task: None,
                    config,
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
