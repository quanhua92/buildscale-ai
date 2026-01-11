//! Health check handlers
//!
//! This module provides health check endpoints for monitoring the API status
//! and cache performance.

use axum::{extract::{Extension, State}, Json};
use serde::Serialize;
use crate::{
    cache::CacheHealthMetrics,
    middleware::auth::AuthenticatedUser,
    state::AppState,
};

/// Public health check response
///
/// Simple status indicator for load balancers and health monitoring.
/// No sensitive information (commit hashes, build timestamps) is exposed.
#[derive(Debug, Serialize)]
pub struct HealthCheckResponse {
    /// Status indicator (always "ok")
    pub status: String,
}

/// Public health check handler that returns simple status
///
/// This endpoint provides basic health monitoring for load balancers
/// and infrastructure monitoring. It does not require authentication.
///
/// # Arguments
/// * `state` - Application state
///
/// # Returns
/// JSON response with status field
///
/// # Example
/// ```bash
/// curl http://localhost:3000/api/v1/health
/// # Returns: {"status":"ok"}
/// ```
pub async fn health_check(
    State(_state): State<AppState>,
) -> Json<HealthCheckResponse> {
    tracing::info!("Health check requested - system operational");
    Json(HealthCheckResponse {
        status: "ok".to_string(),
    })
}

/// Protected cache health check handler that returns cache metrics
///
/// This endpoint requires valid JWT authentication and returns detailed
/// cache metrics for monitoring and debugging.
///
/// # Authentication
/// Requires valid JWT token via:
/// - Authorization header (API clients): `Bearer <token>`
/// - Cookie (Browser clients): `access_token=<token>`
///
/// The JWT middleware validates the token, caches user details, and adds
/// `AuthenticatedUser` to request extensions automatically.
///
/// # Arguments
/// * `user` - Authenticated user (automatically extracted by JWT middleware)
/// * `state` - Application state containing the cache instance
///
/// # Returns
/// JSON response containing cache health metrics
///
/// # Example
/// ```bash
/// # With Authorization header
/// curl http://localhost:3000/api/v1/health/cache \
///   -H "Authorization: Bearer <access_token>"
///
/// # With cookie
/// curl http://localhost:3000/api/v1/health/cache \
///   -H "Cookie: access_token=<token>"
/// ```
///
/// # Response
/// ```json
/// {
///   "num_keys": 42,
///   "last_worker_time": "2026-01-08T10:00:00Z",
///   "cleaned_count": 5,
///   "size_bytes": 18432
/// }
/// ```
pub async fn health_cache(
    Extension(_user): Extension<AuthenticatedUser>, // From middleware
    State(state): State<AppState>,
) -> Json<CacheHealthMetrics> {
    #[cfg(debug_assertions)]
    tracing::debug!(operation = "health_cache", "Cache health metrics requested");

    // user.id, user.email, user.full_name are available if needed
    let metrics = state.cache.get_health_metrics().await
        .expect("Failed to get cache health metrics");

    #[cfg(debug_assertions)]
    tracing::debug!(
        operation = "health_cache",
        num_keys = metrics.num_keys,
        size_bytes = metrics.size_bytes,
        "Cache health metrics retrieved",
    );

    Json(metrics)
}
