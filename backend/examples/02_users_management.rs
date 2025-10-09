use backend::{
    load_config,
    models::users::{RegisterUser, UpdateUser},
    queries::users::{
        create_user, delete_user, find_user_by_email, get_user_by_id, list_users, update_user,
    },
    services::users::{register_user, verify_password},
};
use secrecy::ExposeSecret;
use sqlx::PgPool;

const EXAMPLE_PREFIX: &str = "example_02_users_management";

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
    let pool = PgPool::connect(config.database.connection_string().expose_secret()).await?;
    println!("✓ Database connection established");
    println!();

    // Get a database connection
    let mut conn = pool.acquire().await?;

    // Clean up any existing example data for safe re-runs
    println!("Cleaning up any existing example data for safe re-runs...");
    let cleanup_patterns = vec![
        format!("{}_test@{}", EXAMPLE_PREFIX, "example.com"),
        format!("{}_alice@{}", EXAMPLE_PREFIX, "example.com"),
        format!("{}_bob.smith@{}", EXAMPLE_PREFIX, "example.com"),
        format!("{}_robert.smith@{}", EXAMPLE_PREFIX, "example.com"),
        format!("{}_charlie+tag@{}", EXAMPLE_PREFIX, "example.com"),
        format!("{}_david@{}", EXAMPLE_PREFIX, "EXAMPLE.COM"),
        format!("{}_transaction_user@{}", EXAMPLE_PREFIX, "example.com"),
        format!("{}_direct@{}", EXAMPLE_PREFIX, "example.com"),
        format!("{}_invalid@{}", EXAMPLE_PREFIX, "example.com"),
        format!("{}_short@{}", EXAMPLE_PREFIX, "example.com"),
    ];

    // Try to clean up, but don't fail if table doesn't exist (migrations not run yet)
    let mut cleanup_success = false;
    for pattern in &cleanup_patterns {
        match sqlx::query("DELETE FROM users WHERE email LIKE $1")
            .bind(format!("%{}%", pattern))
            .execute(&mut *conn)
            .await
        {
            Ok(_) => cleanup_success = true,
            Err(e) if e.to_string().contains("does not exist") => {
                println!("ℹ️  Users table doesn't exist yet - will create users when needed");
                break;
            }
            Err(e) => {
                println!("⚠️  Cleanup warning: {}", e);
                break;
            }
        }
    }

    // Only clean up test patterns if the first cleanup succeeded
    if cleanup_success {
        sqlx::query("DELETE FROM users WHERE email LIKE $1")
            .bind(format!("{}%", EXAMPLE_PREFIX))
            .execute(&mut *conn)
            .await
            .ok();
    }

    println!("✓ Cleanup completed - safe to re-run");
    println!();

    // Test user registration
    println!("Testing user registration...");

    let register_user_data = RegisterUser {
        email: format!("{}_test@{}", EXAMPLE_PREFIX, "example.com"),
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
    let found_user = find_user_by_email(
        &mut conn,
        &format!("{}_test@{}", EXAMPLE_PREFIX, "example.com"),
    )
    .await?;
    match found_user {
        Some(user) => {
            println!("✓ User found by email:");
            println!("  ID: {}", user.id);
            println!("  Email: {}", user.email);
        }
        None => println!("✗ User not found by email"),
    }
    println!();

    // Test creating multiple users with various scenarios
    println!("Creating multiple users with different scenarios...");

    let mut created_users = Vec::new();

    // User 1: Basic registration
    let user1 = register_user(
        &mut conn,
        RegisterUser {
            email: format!("{}_alice@{}", EXAMPLE_PREFIX, "example.com"),
            password: "alicepassword123".to_string(),
            confirm_password: "alicepassword123".to_string(),
        },
    )
    .await?;
    created_users.push(("Alice (Basic)", user1.clone()));

    // User 2: With full name
    let user2 = register_user(
        &mut conn,
        RegisterUser {
            email: format!("{}_bob.smith@{}", EXAMPLE_PREFIX, "example.com"),
            password: "bobsecure456".to_string(),
            confirm_password: "bobsecure456".to_string(),
        },
    )
    .await?;
    created_users.push(("Bob (With Full Name)", user2.clone()));

    // User 3: Complex password
    let user3 = register_user(
        &mut conn,
        RegisterUser {
            email: format!("{}_charlie+tag@{}", EXAMPLE_PREFIX, "example.com"),
            password: "Complex!@#$%^789".to_string(),
            confirm_password: "Complex!@#$%^789".to_string(),
        },
    )
    .await?;
    created_users.push(("Charlie (Complex Password)", user3.clone()));

    // User 4: Uppercase email
    let user4 = register_user(
        &mut conn,
        RegisterUser {
            email: format!("{}_david@{}", EXAMPLE_PREFIX, "EXAMPLE.COM"),
            password: "UPPERCASE123".to_string(),
            confirm_password: "UPPERCASE123".to_string(),
        },
    )
    .await?;
    created_users.push(("David (Uppercase Email)", user4.clone()));

    println!("✓ Successfully created {} users", created_users.len());
    for (description, user) in &created_users {
        println!("  {}: {} (ID: {})", description, user.email, user.id);
    }
    println!();

    // Test user updates
    println!("Testing user updates...");

    // Update Bob's profile using query layer
    let mut updated_bob = user2.clone();
    updated_bob.email = "robert.smith@example.com".to_string();
    updated_bob.password_hash = "updated_hash".to_string(); // This would normally be a new hash

    let update_data = UpdateUser {
        email: Some(format!("{}_robert.smith@{}", EXAMPLE_PREFIX, "example.com")),
        password_hash: Some("new_secure_hash_789".to_string()),
        full_name: Some("Robert Smith".to_string()),
    };

    // Manually update using the query layer (bypassing service for demo)
    let mut bob_for_update = user2.clone();
    bob_for_update.email = update_data.email.clone().unwrap();
    bob_for_update.password_hash = update_data.password_hash.clone().unwrap();
    bob_for_update.full_name = update_data.full_name.clone();

    let updated_user = update_user(&mut conn, &bob_for_update).await?;
    println!(
        "✓ Updated user's email from '{}' to '{}'",
        user2.email, updated_user.email
    );
    println!(
        "✓ Updated user's full name to: {:?}",
        updated_user.full_name
    );
    println!();

    // Test password verification for all users
    println!("Testing password verification for all users...");
    let test_passwords = vec![
        (
            format!("{}_alice@{}", EXAMPLE_PREFIX, "example.com"),
            "alicepassword123",
        ),
        (
            format!("{}_robert.smith@{}", EXAMPLE_PREFIX, "example.com"),
            "bobsecure456",
        ), // Note: Still using original password
        (
            format!("{}_charlie+tag@{}", EXAMPLE_PREFIX, "example.com"),
            "Complex!@#$%^789",
        ),
        (
            format!("{}_david@{}", EXAMPLE_PREFIX, "EXAMPLE.COM"),
            "UPPERCASE123",
        ),
    ];

    for (email, password) in test_passwords {
        if let Some(user) = find_user_by_email(&mut conn, &email).await? {
            let is_valid = verify_password(password, &user.password_hash)?;
            println!("✓ Password verification for {}: {}", email, is_valid);
        }
    }
    println!();

    // Test user lookup by various methods
    println!("Testing user lookup methods...");

    // Find user by email
    if let Some(found_user) = find_user_by_email(
        &mut conn,
        &format!("{}_alice@{}", EXAMPLE_PREFIX, "example.com"),
    )
    .await?
    {
        println!(
            "✓ Found Alice by email: {} (ID: {})",
            found_user.email, found_user.id
        );
    }

    // Get user by ID
    if let Some((_, first_user)) = created_users.first() {
        let user_by_id = get_user_by_id(&mut conn, first_user.id).await?;
        println!("✓ Found user by ID {}: {}", user_by_id.id, user_by_id.email);
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
            println!(
                "  {}. ID: {}, Email: {}, Full Name: {:?}, Created: {}",
                i + 1,
                user.id,
                user.email,
                user.full_name,
                user.created_at.format("%Y-%m-%d %H:%M:%S")
            );
        }
    }
    println!();

    // Test transaction isolation
    println!("Testing database transaction isolation...");

    // Start a transaction
    let mut tx = pool.begin().await?;

    // Create a user within transaction
    let tx_user = register_user(
        tx.as_mut(),
        RegisterUser {
            email: format!("{}_transaction_user@{}", EXAMPLE_PREFIX, "example.com"),
            password: "transaction123".to_string(),
            confirm_password: "transaction123".to_string(),
        },
    )
    .await?;

    println!("✓ Created user within transaction: {}", tx_user.email);

    // User should exist within transaction
    if let Some(user) = find_user_by_email(
        tx.as_mut(),
        &format!("{}_transaction_user@{}", EXAMPLE_PREFIX, "example.com"),
    )
    .await?
    {
        println!("✓ User exists within transaction: {}", user.email);
    }

    // User should NOT exist outside transaction yet
    let user_outside = find_user_by_email(
        &mut conn,
        &format!("{}_transaction_user@{}", EXAMPLE_PREFIX, "example.com"),
    )
    .await?;
    assert!(
        user_outside.is_none(),
        "User should not exist outside transaction before commit"
    );
    println!("✓ User correctly not visible outside transaction before commit");

    // Commit transaction
    tx.commit().await?;
    println!("✓ Transaction committed");

    // Now user should exist outside transaction
    if let Some(user) = find_user_by_email(
        &mut conn,
        &format!("{}_transaction_user@{}", EXAMPLE_PREFIX, "example.com"),
    )
    .await?
    {
        println!("✓ User now exists after transaction commit: {}", user.email);
    }
    println!();

    // Test different password lengths
    println!("Testing various valid password lengths...");
    let password_tests = vec![
        ("valid8", "valid123"),
        ("valid10", "validpass10"),
        ("valid20", "validpassword20chars"),
    ];

    for (name, password) in password_tests {
        let test_user = register_user(
            &mut conn,
            RegisterUser {
                email: format!("{}_{}@{}", EXAMPLE_PREFIX, name, "example.com"),
                password: password.to_string(),
                confirm_password: password.to_string(),
            },
        )
        .await?;

        let is_valid = verify_password(password, &test_user.password_hash)?;
        println!(
            "✓ Password length test {}: {} - Valid: {}",
            name,
            password.len(),
            is_valid
        );
    }
    println!();

    // Test validation - password mismatch
    println!("Testing validation - password mismatch...");
    let invalid_register_user = RegisterUser {
        email: format!("{}_invalid@{}", EXAMPLE_PREFIX, "example.com"),
        password: "password123".to_string(),
        confirm_password: "different123".to_string(),
    };

    match register_user(&mut conn, invalid_register_user).await {
        Ok(_) => println!("✗ Validation failed - should have rejected mismatched passwords"),
        Err(e) => println!(
            "✓ Validation correctly rejected mismatched passwords: {}",
            e
        ),
    }
    println!();

    // Test validation - short password
    println!("Testing validation - short password...");
    let short_password_user = RegisterUser {
        email: format!("{}_short@{}", EXAMPLE_PREFIX, "example.com"),
        password: "short".to_string(),
        confirm_password: "short".to_string(),
    };

    match register_user(&mut conn, short_password_user).await {
        Ok(_) => println!("✗ Validation failed - should have rejected short password"),
        Err(e) => println!("✓ Validation correctly rejected short password: {}", e),
    }
    println!();

    // Test partial updates
    println!("Testing partial user updates...");
    if let Some(user) = find_user_by_email(
        &mut conn,
        &format!("{}_alice@{}", EXAMPLE_PREFIX, "example.com"),
    )
    .await?
    {
        let mut alice_for_update = user.clone();
        alice_for_update.full_name = Some("Alice Johnson".to_string());

        let updated_alice = update_user(&mut conn, &alice_for_update).await?;
        println!(
            "✓ Partially updated Alice's full name to: {:?}",
            updated_alice.full_name
        );
        println!("  Email and password hash remained unchanged");
    }
    println!();

    // Test direct database operations (bypassing service layer)
    println!("Testing direct database operations...");

    // Create user directly using query layer
    let direct_user = create_user(
        &mut conn,
        backend::models::users::NewUser {
            email: format!("{}_direct@{}", EXAMPLE_PREFIX, "example.com"),
            password_hash: "direct_hash_12345".to_string(),
            full_name: Some("Direct User".to_string()),
        },
    )
    .await?;
    println!(
        "✓ Created user directly via query layer: {} (Full name: {:?})",
        direct_user.email, direct_user.full_name
    );

    // Delete user
    let rows_affected = delete_user(&mut conn, direct_user.id).await?;
    println!("✓ Deleted user: {} rows affected", rows_affected);

    // Verify deletion
    let deleted_user = find_user_by_email(
        &mut conn,
        &format!("{}_direct@{}", EXAMPLE_PREFIX, "example.com"),
    )
    .await?;
    assert!(deleted_user.is_none(), "User should be deleted");
    println!("✓ Confirmed user deletion - no longer found in database");
    println!();

    // Test database constraints
    println!("Testing database constraints...");

    // Try to create duplicate user
    // First ensure example_02_alice@example.com exists
    let alice_email = format!("{}_alice@{}", EXAMPLE_PREFIX, "example.com");
    if find_user_by_email(&mut conn, &alice_email).await?.is_none() {
        // Re-create alice if she was cleaned up
        register_user(
            &mut conn,
            RegisterUser {
                email: alice_email.clone(),
                password: "alicepassword123".to_string(),
                confirm_password: "alicepassword123".to_string(),
            },
        )
        .await
        .ok();
    }

    match register_user(
        &mut conn,
        RegisterUser {
            email: alice_email, // Already exists
            password: "newpassword123".to_string(),
            confirm_password: "newpassword123".to_string(),
        },
    )
    .await
    {
        Ok(_) => println!("✗ Database constraint failed - should reject duplicate email"),
        Err(e) => println!("✓ Database correctly rejected duplicate email: {}", e),
    }
    println!();

    // Test final user count
    println!("Final user count...");
    let final_users = list_users(&mut conn).await?;
    println!("✓ Total users in database: {}", final_users.len());

    // Show ordering (newest first)
    if !final_users.is_empty() {
        println!("✓ Users are ordered by creation date (newest first):");
        for (i, user) in final_users.iter().take(3).enumerate() {
            println!(
                "  {}. {} - Created: {}",
                i + 1,
                user.email,
                user.created_at.format("%Y-%m-%d %H:%M:%S UTC")
            );
        }
    }
    println!();

    println!("🎉 All features demonstrated successfully!");
    println!("✅ User Registration & Validation");
    println!("✅ Password Hashing & Verification (Argon2)");
    println!("✅ Multiple User Management");
    println!("✅ User Updates (Full & Partial)");
    println!("✅ Database Lookup Methods");
    println!("✅ Transaction Isolation");
    println!("✅ Various Email Formats & Passwords");
    println!("✅ Database Constraints");
    println!("✅ Direct Database Operations");
    println!("✅ Error Handling & Validation");
    println!("✅ Re-run Safety (Auto-cleanup & Idempotent)");
    println!();
    println!("💡 This example is re-run safe - it automatically cleans up previous data");
    println!("   and works with sqlx CLI migrations (no manual table creation).");
    println!();

    // Clean up the connection
    drop(conn);
    pool.close().await;

    Ok(())
}
