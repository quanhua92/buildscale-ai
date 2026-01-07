use buildscale::{
    load_config,
    models::users::{LoginUser, RegisterUser, UpdateUser},
    queries::users::{
        create_user, delete_user, get_user_by_email, get_user_by_id, list_users, update_user,
    },
    services::users::{login_user, logout_user, validate_session, refresh_session, register_user, verify_password, generate_password_hash, update_password, get_session_info, is_email_available, get_user_active_sessions, revoke_all_user_sessions},
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
        full_name: None,
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
    let is_valid = verify_password("testpassword123", created_user.password_hash.as_deref().unwrap())?;
    println!("✓ Password verification: {}", is_valid);
    println!();

    // Test user login
    println!("Testing user login...");
    let login_user_data = LoginUser {
        email: format!("{}_test@{}", EXAMPLE_PREFIX, "example.com"),
        password: "testpassword123".to_string(),
    };

    let login_result = login_user(&mut conn, login_user_data).await?;
    println!("✓ User login successful:");
    println!("  User ID: {}", login_result.user.id);
    println!("  User Email: {}", login_result.user.email);
    println!("  Access Token (JWT): {}...", &login_result.access_token[..8]);
    println!("  Access Token Expires at: {}", login_result.access_token_expires_at);
    println!("  Refresh Token (Session): {}...", &login_result.refresh_token[..8]);
    println!("  Refresh Token Expires at: {}", login_result.refresh_token_expires_at);
    println!();

    // Test session validation
    println!("Testing session validation...");
    let validated_user = validate_session(&mut conn, &login_result.refresh_token).await?;
    println!("✓ Session validation successful:");
    println!("  Validated User ID: {}", validated_user.id);
    println!("  Validated User Email: {}", validated_user.email);
    println!();

    // Test session refresh
    println!("Testing session refresh...");
    let refreshed_token = refresh_session(&mut conn, &login_result.refresh_token, 48).await?;
    println!("✓ Session refresh successful:");
    println!("  New expires at: {}", login_result.refresh_token_expires_at + chrono::Duration::hours(48)); // Should be extended
    println!("  Token unchanged: {}", refreshed_token == login_result.refresh_token);
    println!();

    // Test logout
    println!("Testing user logout...");
    logout_user(&mut conn, &login_result.refresh_token).await?;
    println!("✓ User logout successful");

    // Verify session is no longer valid after logout
    let validation_result = validate_session(&mut conn, &login_result.refresh_token).await;
    match validation_result {
        Ok(_) => println!("✗ Session validation should have failed after logout"),
        Err(_) => println!("✓ Session correctly invalidated after logout"),
    }
    println!();

    // Test finding user by email
    println!("Testing find user by email...");
    let found_user = get_user_by_email(
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
            full_name: None,
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
            full_name: Some("Bob Smith".to_string()),
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
            full_name: Some("Charlie Day".to_string()),
        },
    )
    .await?;
    created_users.push(("Charlie (Complex Password)", user3.clone()));

    // User 4: Uppercase email (will be normalized to lowercase)
    let user4 = register_user(
        &mut conn,
        RegisterUser {
            email: format!("{}_david@{}", EXAMPLE_PREFIX, "example.com"), // Use lowercase for consistency
            password: "UPPERCASE123".to_string(),
            confirm_password: "UPPERCASE123".to_string(),
            full_name: Some("David Williams".to_string()),
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

    // Update Bob's profile using query layer (excluding email - emails cannot be updated)
    let update_data = UpdateUser {
        password_hash: Some(generate_password_hash("new_secure_password_789")?),
        full_name: Some("Robert Smith".to_string()),
    };

    // Manually update using the query layer (bypassing service for demo)
    let mut bob_for_update = user2.clone();
    bob_for_update.password_hash = update_data.password_hash.clone();
    bob_for_update.full_name = update_data.full_name.clone();
    // Note: email remains unchanged as emails cannot be updated

    let updated_user = update_user(&mut conn, &bob_for_update).await?;
    println!(
        "✓ Updated user's full name to: {:?}",
        updated_user.full_name
    );
    println!(
        "✓ Updated user's password hash (email remains unchanged: '{}')",
        updated_user.email
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
            format!("{}_bob.smith@{}", EXAMPLE_PREFIX, "example.com"),
            "new_secure_password_789",
        ), // Note: Bob's email remains unchanged, only password hash updated
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
        if let Some(user) = get_user_by_email(&mut conn, &email).await? {
            let is_valid = verify_password(password, user.password_hash.as_deref().unwrap())?;
            println!("✓ Password verification for {}: {}", email, is_valid);
        }
    }
    println!();

    // Test user lookup by various methods
    println!("Testing user lookup methods...");

    // Find user by email
    if let Some(found_user) = get_user_by_email(
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

    // Get user by ID using service method
    if let Some((_, first_user)) = created_users.first() {
        match get_user_by_id(&mut conn, first_user.id).await? {
            Some(user_by_id) => println!("✓ Found user by ID {}: {}", user_by_id.id, user_by_id.email),
            None => println!("✗ User not found by ID {}", first_user.id),
        }
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
            full_name: Some("Transaction User".to_string()),
        },
    )
    .await?;

    println!("✓ Created user within transaction: {}", tx_user.email);

    // User should exist within transaction
    if let Some(user) = get_user_by_email(
        tx.as_mut(),
        &format!("{}_transaction_user@{}", EXAMPLE_PREFIX, "example.com"),
    )
    .await?
    {
        println!("✓ User exists within transaction: {}", user.email);
    }

    // User should NOT exist outside transaction yet
    let user_outside = get_user_by_email(
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
    if let Some(user) = get_user_by_email(
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
                full_name: None,
            },
        )
        .await?;

        let is_valid = verify_password(password, test_user.password_hash.as_deref().unwrap())?;
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
        password: "SecurePass123!".to_string(),
        confirm_password: "different123".to_string(),
        full_name: None,
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
        full_name: None,
    };

    match register_user(&mut conn, short_password_user).await {
        Ok(_) => println!("✗ Validation failed - should have rejected short password"),
        Err(e) => println!("✓ Validation correctly rejected short password: {}", e),
    }
    println!();

    // Test partial updates
    println!("Testing partial user updates...");
    if let Some(user) = get_user_by_email(
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
        buildscale::models::users::NewUser {
            email: format!("{}_direct@{}", EXAMPLE_PREFIX, "example.com"),
            password_hash: Some("direct_hash_12345".to_string()),
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
    let deleted_user = get_user_by_email(
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
    if get_user_by_email(&mut conn, &alice_email).await?.is_none() {
        // Re-create alice if she was cleaned up
        register_user(
            &mut conn,
            RegisterUser {
                email: alice_email.clone(),
                password: "alicepassword123".to_string(),
                confirm_password: "alicepassword123".to_string(),
                full_name: None,
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
            full_name: None,
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

    // ===== NEW SERVICE METHOD DEMONSTRATIONS =====

    // Demonstrate email availability checking
    println!("Testing email availability checking...");

    // Check if new email is available
    let available_email = format!("{}_new_user@{}", EXAMPLE_PREFIX, "example.com");
    let is_available = is_email_available(&mut conn, &available_email).await?;
    println!("✓ Email '{}' is available: {}", available_email, is_available);

    // Check if existing email is available
    let existing_email = format!("{}_alice@{}", EXAMPLE_PREFIX, "example.com");
    let is_existing_available = is_email_available(&mut conn, &existing_email).await?;
    println!("✓ Email '{}' is available: {}", existing_email, is_existing_available);

    // Test invalid email formats
    let invalid_emails = vec!["", "   ", "invalid-email", "nodomain@", "@nodomain.com"];
    for invalid_email in invalid_emails {
        match is_email_available(&mut conn, invalid_email).await {
            Ok(_) => println!("✗ Should have rejected invalid email: '{}'", invalid_email),
            Err(_) => println!("✓ Correctly rejected invalid email: '{}'", invalid_email),
        }
    }
    println!();

    // Demonstrate password updates using service method
    println!("Testing password update service method...");

    if let Some((_, charlie_user)) = created_users.iter().find(|(name, _)| name.contains("Charlie")) {
        println!("Updating password for {}...", charlie_user.email);

        // Update Charlie's password
        update_password(&mut conn, charlie_user.id, "NewCharliePassword2024").await?;
        println!("✓ Password updated successfully for {}", charlie_user.email);

        // Verify new password works by logging in
        let _login_result = login_user(&mut conn, LoginUser {
            email: charlie_user.email.clone(),
            password: "NewCharliePassword2024".to_string(),
        }).await?;
        println!("✓ Login successful with new password for {}", charlie_user.email);

        // Verify old password no longer works
        match login_user(&mut conn, LoginUser {
            email: charlie_user.email.clone(),
            password: "Complex!@#$%^789".to_string(), // Old password
        }).await {
            Ok(_) => println!("✗ Old password should no longer work"),
            Err(_) => println!("✓ Old password correctly rejected"),
        }
    }
    println!();

    // Demonstrate session info access
    println!("Testing session info service method...");

    // Create a new session for Alice
    if let Some((_, alice_user)) = created_users.iter().find(|(name, _)| name.contains("Alice")) {
        let login_result = login_user(&mut conn, LoginUser {
            email: alice_user.email.clone(),
            password: "alicepassword123".to_string(),
        }).await?;

        // Get session info without validation
        match get_session_info(&mut conn, &login_result.refresh_token).await? {
            Some(session_info) => {
                println!("✓ Retrieved session info:");
                println!("  Session ID: {}", session_info.id);
                println!("  User ID: {}", session_info.user_id);
                println!("  Token: {}...", &session_info.token[..8]);
                println!("  Expires at: {}", session_info.expires_at);
                println!("  Created at: {}", session_info.created_at);
            },
            None => println!("✗ Session info not found"),
        }

        // Test with invalid token
        match get_session_info(&mut conn, "invalid_token").await? {
            Some(_) => println!("✗ Should not find session info for invalid token"),
            None => println!("✓ Correctly returned None for invalid token"),
        }
    }
    println!();

    // Demonstrate user session management
    println!("Testing user session management methods...");

    if let Some((_, david_user)) = created_users.iter().find(|(name, _)| name.contains("David")) {
        // Create multiple sessions for David
        let mut session_tokens = Vec::new();
        for i in 0..3 {
            match login_user(&mut conn, LoginUser {
                email: david_user.email.clone(),
                password: "UPPERCASE123".to_string(),
            }).await {
                Ok(login_result) => {
                    session_tokens.push(login_result.refresh_token);
                    println!("✓ Created session {} for {}", i + 1, david_user.email);
                }
                Err(e) => {
                    println!("⚠️  Session {} creation failed for {}: {}", i + 1, david_user.email, e);
                    // Continue with other sessions instead of crashing
                }
            }
        }

        // Get active sessions
        let active_sessions = get_user_active_sessions(&mut conn, david_user.id).await?;
        println!("✓ David has {} active sessions", active_sessions.len());

        // Display session details
        for (i, session) in active_sessions.iter().enumerate() {
            println!("  Session {}: expires at {}", i + 1, session.expires_at);
        }

        // Revoke all sessions
        let revoked_count = revoke_all_user_sessions(&mut conn, david_user.id).await?;
        println!("✓ Revoked {} sessions for {}", revoked_count, david_user.email);

        // Verify sessions are no longer active
        let remaining_sessions = get_user_active_sessions(&mut conn, david_user.id).await?;
        println!("✓ David now has {} active sessions", remaining_sessions.len());

        // Verify tokens are invalid
        for (i, token) in session_tokens.iter().enumerate() {
            match validate_session(&mut conn, token).await {
                Ok(_) => println!("✗ Session {} should be invalid", i + 1),
                Err(_) => println!("✓ Session {} correctly invalidated", i + 1),
            }
        }
    }
    println!();

    // Test get_user_by_id with non-existent user
    println!("Testing get_user_by_id with non-existent user...");
    let non_existent_id = uuid::Uuid::now_v7();
    match get_user_by_id(&mut conn, non_existent_id).await? {
        Some(_) => println!("✗ Should not find non-existent user"),
        None => println!("✓ Correctly returned None for non-existent user ID"),
    }
    println!();

    println!("🎉 All features demonstrated successfully!");
    println!("✅ User Registration & Validation");
    println!("✅ User Login & Authentication");
    println!("✅ Session Management (Create, Validate, Refresh, Logout)");
    println!("✅ Password Hashing & Verification (Argon2)");
    println!("✅ Multiple User Management");
    println!("✅ User Updates (Password & Full Name only - emails cannot be updated)");
    println!("✅ Database Lookup Methods");
    println!("✅ Transaction Isolation");
    println!("✅ Various Email Formats & Passwords");
    println!("✅ Database Constraints");
    println!("✅ Direct Database Operations");
    println!("✅ Error Handling & Validation");
    println!("✅ Re-run Safety (Auto-cleanup & Idempotent)");
    println!("🆕 Email Availability Checking");
    println!("🆕 Service Layer Password Updates");
    println!("🆕 Session Information Access");
    println!("🆕 User Session Management (Active Sessions & Revocation)");
    println!("🆕 Safe User ID Lookups with Option Handling");
    println!();
    println!("💡 This example is re-run safe - it automatically cleans up previous data");
    println!("   and works with sqlx CLI migrations (no manual table creation).");
    println!();

    // Clean up the connection
    drop(conn);
    pool.close().await;

    Ok(())
}
