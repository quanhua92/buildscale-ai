use buildscale::{load_config, Cache, CacheConfig, run_api_server, run_cache_cleanup};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello from the binary!");

    // Load configuration using lib.rs method
    let config = load_config()?;

    // Print configuration using Display implementation
    println!("Loaded configuration:");
    println!("{}", config);

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
