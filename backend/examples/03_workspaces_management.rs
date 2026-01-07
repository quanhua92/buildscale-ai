use buildscale::{
    load_config,
    models::{
        users::RegisterUser,
        requests::{
            CreateWorkspaceRequest, CreateWorkspaceWithMembersRequest,
            WorkspaceMemberRequest, UserWorkspaceRegistrationRequest
        },
        roles::{ADMIN_ROLE, EDITOR_ROLE, MEMBER_ROLE, VIEWER_ROLE},
    },
    queries::{
        users::list_users,
        workspaces::{list_workspaces},
          workspace_members::{list_workspace_members},
    },
    services::{
        users::{register_user, register_user_with_workspace},
        workspaces::{create_workspace, create_workspace_with_members, update_workspace_owner, delete_workspace},
        roles::list_workspace_roles,
    },
};
use secrecy::ExposeSecret;
use sqlx::PgPool;

const EXAMPLE_PREFIX: &str = "example_03_workspaces_management";

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
        format!("{}_owner@{}", EXAMPLE_PREFIX, "example.com"),
        format!("{}_admin@{}", EXAMPLE_PREFIX, "example.com"),
        format!("{}_editor@{}", EXAMPLE_PREFIX, "example.com"),
        format!("{}_member@{}", EXAMPLE_PREFIX, "example.com"),
        format!("{}_viewer@{}", EXAMPLE_PREFIX, "example.com"),
        format!("{}_member1@{}", EXAMPLE_PREFIX, "example.com"),
        format!("{}_member2@{}", EXAMPLE_PREFIX, "example.com"),
        format!("{}_new_user@{}", EXAMPLE_PREFIX, "example.com"),
    ];

    // Try to clean up all related tables in proper order (respecting foreign keys)
    let mut cleanup_success = false;

    // Clean up workspace_members first
    if let Ok(_) = sqlx::query("DELETE FROM workspace_members WHERE user_id IN (SELECT id FROM users WHERE email LIKE $1)")
        .bind(format!("{}%", EXAMPLE_PREFIX))
        .execute(&mut *conn)
        .await
    {
        cleanup_success = true;
    }

    // Clean up roles
    if cleanup_success {
        sqlx::query("DELETE FROM roles WHERE name LIKE $1")
            .bind(format!("{}%", EXAMPLE_PREFIX))
            .execute(&mut *conn)
            .await
            .ok();
    }

    // Clean up workspaces
    if cleanup_success {
        sqlx::query("DELETE FROM workspaces WHERE name LIKE $1")
            .bind(format!("{}%", EXAMPLE_PREFIX))
            .execute(&mut *conn)
            .await
            .ok();
    }

    // Clean up users
    for pattern in &cleanup_patterns {
        match sqlx::query("DELETE FROM users WHERE email LIKE $1")
            .bind(format!("%{}%", pattern))
            .execute(&mut *conn)
            .await
        {
            Ok(_) => cleanup_success = true,
            Err(e) if e.to_string().contains("does not exist") => {
                println!("ℹ️  Tables don't exist yet - will create data when needed");
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

    // ========================================================
    // STEP 1: Create users who will participate in workspaces
    // ========================================================
    println!("🚀 STEP 1: Creating users for workspace management demo");

    // Create workspace owner
    let owner_user = register_user(
        &mut conn,
        RegisterUser {
            email: format!("{}_owner@{}", EXAMPLE_PREFIX, "example.com"),
            password: "ownerpass123".to_string(),
            confirm_password: "ownerpass123".to_string(),
            full_name: Some("Workspace Owner".to_string()),
        },
    )
    .await?;
    println!("✓ Created workspace owner: {} (ID: {})", owner_user.email, owner_user.id);

    // Create additional users for different roles
    let admin_user = register_user(
        &mut conn,
        RegisterUser {
            email: format!("{}_admin@{}", EXAMPLE_PREFIX, "example.com"),
            password: "adminpass123".to_string(),
            confirm_password: "adminpass123".to_string(),
            full_name: Some("Admin User".to_string()),
        },
    )
    .await?;
    println!("✓ Created admin user: {} (ID: {})", admin_user.email, admin_user.id);

    let editor_user = register_user(
        &mut conn,
        RegisterUser {
            email: format!("{}_editor@{}", EXAMPLE_PREFIX, "example.com"),
            password: "editorpass123".to_string(),
            confirm_password: "editorpass123".to_string(),
            full_name: Some("Editor User".to_string()),
        },
    )
    .await?;
    println!("✓ Created editor user: {} (ID: {})", editor_user.email, editor_user.id);

    let viewer_user = register_user(
        &mut conn,
        RegisterUser {
            email: format!("{}_viewer@{}", EXAMPLE_PREFIX, "example.com"),
            password: "viewerpass123".to_string(),
            confirm_password: "viewerpass123".to_string(),
            full_name: Some("Viewer User".to_string()),
        },
    )
    .await?;
    println!("✓ Created viewer user: {} (ID: {})", viewer_user.email, viewer_user.id);

    let member_user = register_user(
        &mut conn,
        RegisterUser {
            email: format!("{}_member@{}", EXAMPLE_PREFIX, "example.com"),
            password: "memberpass123".to_string(),
            confirm_password: "memberpass123".to_string(),
            full_name: Some("Member User".to_string()),
        },
    )
    .await?;
    println!("✓ Created member user: {} (ID: {})", member_user.email, member_user.id);

    let member1_user = register_user(
        &mut conn,
        RegisterUser {
            email: format!("{}_member1@{}", EXAMPLE_PREFIX, "example.com"),
            password: "member1pass123".to_string(),
            confirm_password: "member1pass123".to_string(),
            full_name: Some("Member One".to_string()),
        },
    )
    .await?;
    println!("✓ Created member1 user: {} (ID: {})", member1_user.email, member1_user.id);

    let member2_user = register_user(
        &mut conn,
        RegisterUser {
            email: format!("{}_member2@{}", EXAMPLE_PREFIX, "example.com"),
            password: "member2pass123".to_string(),
            confirm_password: "member2pass123".to_string(),
            full_name: Some("Member Two".to_string()),
        },
    )
    .await?;
    println!("✓ Created member2 user: {} (ID: {})", member2_user.email, member2_user.id);
    println!();

    // ========================================================
    // STEP 2: Demonstrate simplified workspace creation
    // ========================================================
    println!("🏢 STEP 2: Creating workspaces with simplified workflows");

    // Create first workspace using new comprehensive method
    let workspace1_request = CreateWorkspaceRequest {
        name: format!("{}_marketing_team", EXAMPLE_PREFIX),
        owner_id: owner_user.id,
    };
    let workspace1_result = create_workspace(&mut conn, workspace1_request).await?;
    println!("✓ Created workspace1: '{}' with {} default roles",
        workspace1_result.workspace.name, workspace1_result.roles.len());
    println!("  - Owner automatically added as admin member");

    // Create second workspace with multiple initial members
    let workspace2_request = CreateWorkspaceWithMembersRequest {
        name: format!("{}_engineering_team", EXAMPLE_PREFIX),
        owner_id: owner_user.id,
        members: vec![
            WorkspaceMemberRequest {
                user_id: admin_user.id,
                role_name: ADMIN_ROLE.to_string(),
            },
            WorkspaceMemberRequest {
                user_id: editor_user.id,
                role_name: EDITOR_ROLE.to_string(),
            },
            WorkspaceMemberRequest {
                user_id: member_user.id,
                role_name: MEMBER_ROLE.to_string(),
            },
            WorkspaceMemberRequest {
                user_id: viewer_user.id,
                role_name: VIEWER_ROLE.to_string(),
            },
        ],
    };
    let workspace2_result = create_workspace_with_members(&mut conn, workspace2_request).await?;
    println!("✓ Created workspace2: '{}' with {} initial members",
        workspace2_result.workspace.name, workspace2_result.members.len());
    println!("  - Owner + 4 members added with their roles in one operation");
    println!();

    // ========================================================
    // STEP 3: Demonstrate user registration with workspace
    // ========================================================
    println!("👤 STEP 3: Registering user with workspace creation");

    let new_user_request = UserWorkspaceRegistrationRequest {
        email: format!("{}_new_user@{}", EXAMPLE_PREFIX, "example.com"),
        password: "newuserpass123".to_string(),
        confirm_password: "newuserpass123".to_string(),
        full_name: Some("New User with Workspace".to_string()),
        workspace_name: format!("{}_personal_workspace", EXAMPLE_PREFIX),
    };
    let new_user_result = register_user_with_workspace(&mut conn, new_user_request).await?;
    println!("✓ Registered new user: {}", new_user_result.user.email);
    println!("✓ Created personal workspace: '{}' with default setup",
        new_user_result.workspace.workspace.name);
    println!("  - User registration + workspace creation in one transaction");
    println!();

    // ========================================================
    // STEP 4: Verify the simplified setup
    // ========================================================
    println!("🔍 STEP 4: Verifying workspace setup");

    // List all workspaces
    let all_workspaces = list_workspaces(&mut conn).await?;
    println!("✓ Found {} workspaces:", all_workspaces.len());
    for (i, workspace) in all_workspaces.iter().enumerate() {
        println!("  {}. {} (ID: {}, Owner: {})",
            i + 1, workspace.name, workspace.id, workspace.owner_id);
    }
    println!();

    // Verify workspace1 has default roles
    let workspace1_roles = list_workspace_roles(&mut conn, workspace1_result.workspace.id).await?;
    println!("✓ Workspace1 '{}' has {} default roles:", workspace1_result.workspace.name, workspace1_roles.len());
    for role in &workspace1_roles {
        println!("  - {} ({})", role.name, role.description.as_ref().unwrap_or(&"No description".to_string()));
    }
    println!();

    // Verify workspace2 has members with their roles
    let workspace2_members = list_workspace_members(&mut conn, workspace2_result.workspace.id).await?;
    println!("✓ Workspace2 '{}' has {} members:", workspace2_result.workspace.name, workspace2_members.len());
    for member in &workspace2_members {
        // Get user details (simplified for demo)
        println!("  - User ID: {} with role ID: {}", member.user_id, member.role_id);
    }
    println!();

    // ========================================================
    // STEP 5: Demonstrate workspace ownership transfer
    // ========================================================
    println!("🔄 STEP 5: Testing workspace ownership transfer");

    // Transfer ownership of workspace1 to admin_user
    println!("Transferring workspace1 ownership from {} to {}...",
        owner_user.email, admin_user.email);

    let updated_workspace1 = update_workspace_owner(
        &mut conn,
        workspace1_result.workspace.id,
        owner_user.id,
        admin_user.id,
    ).await?;

    println!("✓ Ownership transferred successfully");
    println!("  - New owner: {}", updated_workspace1.owner_id);
    println!("  - Previous owner automatically added as admin member");
    println!();

    // ========================================================
    // STEP 6: Test edge cases and validation
    // ========================================================
    println!("⚠️ STEP 6: Testing validation and edge cases");

    // Test creating workspace with empty name
    println!("Testing empty workspace name validation...");
    let empty_workspace_request = CreateWorkspaceRequest {
        name: "".to_string(),
        owner_id: member1_user.id,
    };
    match create_workspace(&mut conn, empty_workspace_request).await {
        Ok(_) => println!("✗ Validation failed - should not allow empty workspace name"),
        Err(e) => println!("✓ Correctly prevented empty workspace name: {}", e),
    }

    // Test creating workspace with name too long
    println!("Testing workspace name length validation...");
    let long_name_request = CreateWorkspaceRequest {
        name: "a".repeat(101),
        owner_id: member1_user.id,
    };
    match create_workspace(&mut conn, long_name_request).await {
        Ok(_) => println!("✗ Validation failed - should not allow workspace name > 100 chars"),
        Err(e) => println!("✓ Correctly prevented long workspace name: {}", e),
    }

    // Test transferring ownership to same user
    println!("Testing ownership transfer to same user...");
    match update_workspace_owner(
        &mut conn,
        workspace2_result.workspace.id,
        owner_user.id,
        owner_user.id,
    ).await {
        Ok(_) => println!("✗ Validation failed - should not allow transfer to same user"),
        Err(e) => println!("✓ Correctly prevented transfer to same user: {}", e),
    }
    println!();

    // ========================================================
    // STEP 7: Demonstrate workspace deletion
    // ========================================================
    println!("🗑️ STEP 7: Testing workspace deletion");

    // Delete workspace3 (new user's personal workspace)
    println!("Deleting personal workspace...");
    let deleted_count = delete_workspace(&mut conn, new_user_result.workspace.workspace.id).await?;
    println!("✓ Deleted workspace ({} rows affected)", deleted_count);

    // Verify workspace no longer exists
    let final_workspaces = list_workspaces(&mut conn).await?;
    println!("✓ Final workspace count: {}", final_workspaces.len());
    println!();

    // ========================================================
    // STEP 8: Final summary
    // ========================================================
    println!("📊 STEP 8: Final summary");

    // List final workspaces and their setup
    let final_workspaces = list_workspaces(&mut conn).await?;
    println!("✓ Final workspaces and their complete setup:");

    for workspace in &final_workspaces {
        let roles = list_workspace_roles(&mut conn, workspace.id).await.unwrap_or_default();
        let members = list_workspace_members(&mut conn, workspace.id).await.unwrap_or_default();

        println!("  📁 Workspace: {}", workspace.name);
        println!("     Owner: {}", workspace.owner_id);
        println!("     Roles: {} (default: admin, editor, member, viewer)", roles.len());
        println!("     Members: {}", members.len());

        for (i, member) in members.iter().enumerate() {
            // Get role name by checking against default roles
            let role_name = if member.role_id == roles[0].id { ADMIN_ROLE }
                           else if roles.len() > 1 && member.role_id == roles[1].id { EDITOR_ROLE }
                           else if roles.len() > 2 && member.role_id == roles[2].id { MEMBER_ROLE }
                           else if roles.len() > 3 && member.role_id == roles[3].id { VIEWER_ROLE }
                           else { "unknown" };
            println!("       {}. User ID: {} - {}", i + 1, member.user_id, role_name);
        }
        println!();
    }

    // List final users
    let final_users = list_users(&mut conn).await?;
    println!("✓ Total users created: {}", final_users.len());
    println!();

    println!("🎉 Simplified workspace management demonstrated successfully!");
    println!("✅ Single-method workspace creation with automatic setup");
    println!("✅ Workspace creation with multiple initial members");
    println!("✅ User registration with workspace in one transaction");
    println!("✅ Automatic default roles creation (admin, editor, member, viewer)");
    println!("✅ Automatic owner as admin member assignment");
    println!("✅ Streamlined ownership transfer with role management");
    println!("✅ Comprehensive validation and error handling");
    println!("✅ Simplified API - no more fragmented multi-step operations");
    println!();
    println!("💡 The new simplified approach reduces complexity from 10+ methods to 3 core methods:");
    println!("   - create_workspace() - creates workspace + default roles + owner as admin");
    println!("   - create_workspace_with_members() - above + multiple members with roles");
    println!("   - register_user_with_workspace() - user registration + workspace creation");
    println!();
    println!("🚀 This makes it much easier for REST APIs and frontend applications!");

    // Clean up the connection
    drop(conn);
    pool.close().await;

    Ok(())
}