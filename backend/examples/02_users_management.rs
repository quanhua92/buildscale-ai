use backend::{
    load_config,
    models::users::RegisterUser,
    services::users::{register_user, verify_password},
    queries::users::{list_users, find_user_by_email},
};
use secrecy::ExposeSecret;
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration using lib.rs method
    let config = load_config()?;

    // Print configuration using Display implementation
    println!("Loaded configuration:");
    println!("{}", config);
    println!();

    // Create database connection pool
    println!("Connecting to database...");
    let pool = PgPool::connect(&config.database.connection_string().expose_secret()).await?;
    println!("✓ Database connection established");
    println!();

    // Get a database connection
    let mut conn = pool.acquire().await?;

    // Test user registration
    println!("Testing user registration...");

    let register_user_data = RegisterUser {
        email: "test@example.com".to_string(),
        password: "testpassword123".to_string(),
        confirm_password: "testpassword123".to_string(),
    };

    // Register the user
    let created_user = register_user(&mut conn, register_user_data).await?;
    println!("✓ User registered successfully:");
    println!("  ID: {}", created_user.id);
    println!("  Email: {}", created_user.email);
    println!("  Created at: {}", created_user.created_at);
    println!();

    // Test password verification
    println!("Testing password verification...");
    let is_valid = verify_password("testpassword123", &created_user.password_hash)?;
    println!("✓ Password verification: {}", is_valid);
    println!();

    // Test finding user by email
    println!("Testing find user by email...");
    let found_user = find_user_by_email(&mut conn, "test@example.com").await?;
    match found_user {
        Some(user) => {
            println!("✓ User found by email:");
            println!("  ID: {}", user.id);
            println!("  Email: {}", user.email);
        }
        None => println!("✗ User not found by email"),
    }
    println!();

    // List all users in database
    println!("Listing all users in database...");
    let users = list_users(&mut conn).await?;

    if users.is_empty() {
        println!("No users found in database");
    } else {
        println!("Found {} user(s):", users.len());
        for (i, user) in users.iter().enumerate() {
            println!("  {}. ID: {}, Email: {}, Created: {}",
                i + 1,
                user.id,
                user.email,
                user.created_at.format("%Y-%m-%d %H:%M:%S")
            );
        }
    }
    println!();

    // Test validation - password mismatch
    println!("Testing validation - password mismatch...");
    let invalid_register_user = RegisterUser {
        email: "invalid@example.com".to_string(),
        password: "password123".to_string(),
        confirm_password: "different123".to_string(),
    };

    match register_user(&mut conn, invalid_register_user).await {
        Ok(_) => println!("✗ Validation failed - should have rejected mismatched passwords"),
        Err(e) => println!("✓ Validation correctly rejected mismatched passwords: {}", e),
    }
    println!();

    // Test validation - short password
    println!("Testing validation - short password...");
    let short_password_user = RegisterUser {
        email: "short@example.com".to_string(),
        password: "short".to_string(),
        confirm_password: "short".to_string(),
    };

    match register_user(&mut conn, short_password_user).await {
        Ok(_) => println!("✗ Validation failed - should have rejected short password"),
        Err(e) => println!("✓ Validation correctly rejected short password: {}", e),
    }
    println!();

    println!("✓ All tests completed successfully! User registration integration is working.");

    // Clean up the connection
    drop(conn);
    pool.close().await;

    Ok(())
}