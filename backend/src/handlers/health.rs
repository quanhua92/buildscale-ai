use axum::{extract::State, Json};
use crate::state::AppState;
use crate::cache::CacheHealthMetrics;

/// Health check handler that returns cache metrics
///
/// This endpoint provides health monitoring by returning cache statistics
/// including the number of keys, last cleanup time, entries cleaned, and memory usage.
///
/// # Arguments
/// * `state` - Application state containing the cache instance
///
/// # Returns
/// JSON response containing cache health metrics
pub async fn health_check(
    State(state): State<AppState>,
) -> Json<CacheHealthMetrics> {
    let metrics = state.cache.get_health_metrics().await
        .expect("Failed to get cache health metrics");
    Json(metrics)
}
