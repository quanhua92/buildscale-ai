use buildscale::{load_config, Cache, CacheConfig, run_api_server, run_cache_cleanup};
use secrecy::ExposeSecret;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello from the binary!");

    // Load configuration using lib.rs method
    let config = load_config()?;

    // Connect to database and optionally run migrations
    let pool = buildscale::DbPool::connect(config.database.connection_string().expose_secret())
        .await
        .expect("Failed to connect to database");

    // Check if migrations folder exists
    let migrations_path = std::path::Path::new("migrations");
    if migrations_path.exists() && migrations_path.is_dir() {
        println!("Running database migrations...");
        match sqlx::migrate::Migrator::new(migrations_path).await {
            Ok(migrator) => {
                match migrator.run(&pool).await {
                    Ok(_) => println!("✓ Database migrations completed successfully"),
                    Err(e) => {
                        eprintln!("⚠ Warning: Failed to run database migrations: {}", e);
                        eprintln!("  Continuing without migrations (database may not be up-to-date)");
                    }
                }
            }
            Err(e) => {
                eprintln!("⚠ Warning: Failed to load migrations: {}", e);
                eprintln!("  Continuing without migrations (database may not be up-to-date)");
            }
        }
    } else {
        eprintln!("⚠ Warning: Migrations folder not found at: {}", migrations_path.display());
        eprintln!("  Continuing without migrations (database may not be up-to-date)");
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
