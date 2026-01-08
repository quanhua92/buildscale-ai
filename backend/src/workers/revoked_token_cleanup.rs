use crate::queries::sessions;
use crate::Config;
use std::time::Duration;
use tokio::time::interval;
use tracing::{info, warn, error};

/// Background worker that periodically cleans up expired revoked tokens
///
/// Runs every 5 minutes to remove revoked tokens based on retention period
/// This keeps the revoked_refresh_tokens table size manageable
pub async fn revoked_token_cleanup_worker(
    pool: sqlx::PgPool,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let mut cleanup_interval = interval(Duration::from_secs(300)); // Every 5 minutes
    info!("Revoked token cleanup worker started (runs every 5 minutes)");

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("Revoked token cleanup worker shutting down");
                break;
            }
            _ = cleanup_interval.tick() => {
                let mut conn = match pool.acquire().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        error!("Failed to acquire database connection for cleanup: {}", e);
                        continue;
                    }
                };

                // Load config to get retention period
                let config = match Config::load() {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        error!("Failed to load config for cleanup: {}", e);
                        continue;
                    }
                };

                // Use configured retention period (default: 1440 minutes = 1 day)
                let retention_minutes = config.sessions.revoked_token_retention_minutes;

                match sessions::delete_expired_revoked_tokens(&mut conn, retention_minutes).await {
                    Ok(count) => {
                        if count > 0 {
                            info!("Cleaned up {} expired revoked tokens (older than {} minutes)", count, retention_minutes);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to cleanup expired revoked tokens: {}", e);
                    }
                }
            }
        }
    }

    info!("Revoked token cleanup worker stopped");
}
