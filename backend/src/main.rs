use buildscale::{init_tracing, load_config, Cache, CacheConfig, run_api_server, run_cache_cleanup};
use secrecy::ExposeSecret;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    // Load configuration using lib.rs method
    let config = load_config()?;

    // Connect to database and optionally run migrations
    let pool = buildscale::DbPool::connect(config.database.connection_string().expose_secret())
        .await
        .expect("Failed to connect to database");

    // Check if migrations folder exists
    let migrations_path = std::path::Path::new("migrations");
    if migrations_path.exists() && migrations_path.is_dir() {
        tracing::info!("Running database migrations...");
        match sqlx::migrate::Migrator::new(migrations_path).await {
            Ok(migrator) => {
                match migrator.run(&pool).await {
                    Ok(_) => tracing::info!("✓ Database migrations completed successfully"),
                    Err(e) => {
                        tracing::warn!("Failed to run database migrations: {}", e);
                        tracing::warn!("Continuing without migrations (database may not be up-to-date)");
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to load migrations: {}", e);
                tracing::warn!("Continuing without migrations (database may not be up-to-date)");
            }
        }
    } else {
        tracing::warn!("Migrations folder not found at: {}", migrations_path.display());
        tracing::warn!("Continuing without migrations (database may not be up-to-date)");
    }

    // Close the migration pool (the API server will create its own)
    pool.close().await;

    // Initialize cache
    let cache: Cache<String> = Cache::new_local(CacheConfig {
        cleanup_interval_seconds: 60,
        default_ttl_seconds: Some(3600),
    });

    // Spawn cleanup worker in background
    let cache_clone = cache.clone();
    tokio::spawn(async move {
        run_cache_cleanup(cache_clone).await;
    });

    // Start API server (this will block)
    run_api_server(&config, cache).await?;

    Ok(())
}
