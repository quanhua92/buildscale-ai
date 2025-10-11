use backend::{
    load_config,
    models::{
        users::RegisterUser,
        workspaces::NewWorkspace,
        roles::NewRole,
        workspace_members::NewWorkspaceMember,
    },
    queries::{
        users::{get_user_by_email, get_user_by_id, list_users},
        workspaces::{create_workspace, get_workspace_by_id, list_workspaces},
        roles::{create_role, get_role_by_id, get_role_by_id_optional, list_roles_by_workspace},
        workspace_members::{list_workspace_members},
    },
    services::{
        users::register_user,
        workspaces::{create_workspace as create_workspace_service, delete_workspace as delete_workspace_service},
        roles::{create_role as create_role_service},
        workspace_members::{
            create_workspace_member as add_member,
            remove_workspace_member as remove_member,
            is_workspace_member as check_membership,
            add_user_to_workspace,
        },
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
        format!("{}_viewer@{}", EXAMPLE_PREFIX, "example.com"),
        format!("{}_member1@{}", EXAMPLE_PREFIX, "example.com"),
        format!("{}_member2@{}", EXAMPLE_PREFIX, "example.com"),
        format!("{}_temp@{}", EXAMPLE_PREFIX, "example.com"),
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
    // STEP 2: Create workspaces with owners
    // ========================================================
    println!("🏢 STEP 2: Creating workspaces");

    // Create first workspace using service layer
    let workspace1_data = NewWorkspace {
        name: format!("{}_marketing_team", EXAMPLE_PREFIX),
        owner_id: owner_user.id,
    };
    let workspace1 = create_workspace_service(&mut conn, workspace1_data).await?;
    println!("✓ Created workspace1: '{}' (ID: {}, Owner: {})",
        workspace1.name, workspace1.id, owner_user.email);

    // Create second workspace using direct query layer
    let workspace2_data = NewWorkspace {
        name: format!("{}_engineering_team", EXAMPLE_PREFIX),
        owner_id: owner_user.id,
    };
    let workspace2 = create_workspace(&mut conn, workspace2_data).await?;
    println!("✓ Created workspace2: '{}' (ID: {}, Owner: {})",
        workspace2.name, workspace2.id, owner_user.email);

    // Create third workspace with different owner
    let temp_user = register_user(
        &mut conn,
        RegisterUser {
            email: format!("{}_temp@{}", EXAMPLE_PREFIX, "example.com"),
            password: "temppass123".to_string(),
            confirm_password: "temppass123".to_string(),
            full_name: Some("Temp Owner".to_string()),
        },
    )
    .await?;
    let workspace3_data = NewWorkspace {
        name: format!("{}_research_team", EXAMPLE_PREFIX),
        owner_id: temp_user.id,
    };
    let workspace3 = create_workspace(&mut conn, workspace3_data).await?;
    println!("✓ Created workspace3: '{}' (ID: {}, Owner: {})",
        workspace3.name, workspace3.id, temp_user.email);
    println!();

    // ========================================================
    // STEP 3: Create roles in workspaces
    // ========================================================
    println!("👥 STEP 3: Creating roles in workspaces");

    // Create roles in workspace1
    let admin_role1 = create_role_service(&mut conn, NewRole {
        workspace_id: workspace1.id,
        name: format!("{}_admin", EXAMPLE_PREFIX),
        description: Some("Full administrative access".to_string()),
    }).await?;
    println!("✓ Created admin role in workspace1: '{}' (ID: {})", admin_role1.name, admin_role1.id);

    let editor_role1 = create_role_service(&mut conn, NewRole {
        workspace_id: workspace1.id,
        name: format!("{}_editor", EXAMPLE_PREFIX),
        description: Some("Can edit and create content".to_string()),
    }).await?;
    println!("✓ Created editor role in workspace1: '{}' (ID: {})", editor_role1.name, editor_role1.id);

    let viewer_role1 = create_role_service(&mut conn, NewRole {
        workspace_id: workspace1.id,
        name: format!("{}_viewer", EXAMPLE_PREFIX),
        description: Some("Read-only access".to_string()),
    }).await?;
    println!("✓ Created viewer role in workspace1: '{}' (ID: {})", viewer_role1.name, viewer_role1.id);

    // Create roles in workspace2
    let developer_role2 = create_role_service(&mut conn, NewRole {
        workspace_id: workspace2.id,
        name: format!("{}_developer", EXAMPLE_PREFIX),
        description: Some("Can develop and deploy code".to_string()),
    }).await?;
    println!("✓ Created developer role in workspace2: '{}' (ID: {})", developer_role2.name, developer_role2.id);

    let tester_role2 = create_role_service(&mut conn, NewRole {
        workspace_id: workspace2.id,
        name: format!("{}_tester", EXAMPLE_PREFIX),
        description: Some("Can test and review code".to_string()),
    }).await?;
    println!("✓ Created tester role in workspace2: '{}' (ID: {})", tester_role2.name, tester_role2.id);
    println!();

    // ========================================================
    // STEP 4: Add members to workspaces with roles
    // ========================================================
    println!("🔗 STEP 4: Adding members to workspaces");

    // Add admin to workspace1 as admin
    let admin_member1 = add_member(&mut conn, NewWorkspaceMember {
        workspace_id: workspace1.id,
        user_id: admin_user.id,
        role_id: admin_role1.id,
    }).await?;
    println!("✓ Added {} to workspace1 as admin", admin_user.email);

    // Add editor to workspace1 as editor
    let editor_member1 = add_member(&mut conn, NewWorkspaceMember {
        workspace_id: workspace1.id,
        user_id: editor_user.id,
        role_id: editor_role1.id,
    }).await?;
    println!("✓ Added {} to workspace1 as editor", editor_user.email);

    // Add viewer to workspace1 as viewer
    let viewer_member1 = add_member(&mut conn, NewWorkspaceMember {
        workspace_id: workspace1.id,
        user_id: viewer_user.id,
        role_id: viewer_role1.id,
    }).await?;
    println!("✓ Added {} to workspace1 as viewer", viewer_user.email);

    // Add member1 and member2 to workspace2 with different roles
    let member1_developer = add_member(&mut conn, NewWorkspaceMember {
        workspace_id: workspace2.id,
        user_id: member1_user.id,
        role_id: developer_role2.id,
    }).await?;
    println!("✓ Added {} to workspace2 as developer", member1_user.email);

    let member2_tester = add_member(&mut conn, NewWorkspaceMember {
        workspace_id: workspace2.id,
        user_id: member2_user.id,
        role_id: tester_role2.id,
    }).await?;
    println!("✓ Added {} to workspace2 as tester", member2_user.email);

    // Add admin user also to workspace2 as developer (show multiple workspace membership)
    let admin_developer2 = add_member(&mut conn, NewWorkspaceMember {
        workspace_id: workspace2.id,
        user_id: admin_user.id,
        role_id: developer_role2.id,
    }).await?;
    println!("✓ Added {} to workspace2 as developer (multiple membership)", admin_user.email);
    println!();

    // ========================================================
    // STEP 5: List and verify workspace memberships
    // ========================================================
    println!("📋 STEP 5: Listing and verifying workspace memberships");

    // List all workspaces
    let all_workspaces = list_workspaces(&mut conn).await?;
    println!("✓ Found {} workspaces:", all_workspaces.len());
    for (i, workspace) in all_workspaces.iter().enumerate() {
        println!("  {}. {} (ID: {}, Owner: {})",
            i + 1, workspace.name, workspace.id, workspace.owner_id);
    }
    println!();

    // List members of workspace1
    let workspace1_members = list_workspace_members(&mut conn, workspace1.id).await?;
    println!("✓ Workspace1 '{}' has {} members:", workspace1.name, workspace1_members.len());
    for (i, member) in workspace1_members.iter().enumerate() {
        // Get user details
        if let Ok(user) = get_user_by_id(&mut conn, member.user_id).await {
            // Get role details
            if let Ok(Some(role)) = get_role_by_id_optional(&mut conn, member.role_id).await {
                println!("  {}. {} - Role: {}", i + 1, user.email, role.name);
            }
        }
    }
    println!();

    // List roles in workspace1
    let workspace1_roles = list_roles_by_workspace(&mut conn, workspace1.id).await?;
    println!("✓ Workspace1 '{}' has {} roles:", workspace1.name, workspace1_roles.len());
    for (i, role) in workspace1_roles.iter().enumerate() {
        println!("  {}. {} - Description: {:?}", i + 1, role.name, role.description);
    }
    println!();

    // ========================================================
    // STEP 6: Test membership validation and constraints
    // ========================================================
    println!("🔍 STEP 6: Testing membership validation and constraints");

    // Check if specific users are members
    let is_admin_member1 = check_membership(&mut conn, workspace1.id, admin_user.id).await?;
    println!("✓ Is {} a member of workspace1? {}", admin_user.email, is_admin_member1);

    let is_member1_workspace1 = check_membership(&mut conn, workspace1.id, member1_user.id).await?;
    println!("✓ Is {} a member of workspace1? {}", member1_user.email, is_member1_workspace1);

    // Test adding duplicate member (should fail)
    println!("Testing duplicate member addition (should fail)...");
    match add_member(&mut conn, NewWorkspaceMember {
        workspace_id: workspace1.id,
        user_id: admin_user.id,
        role_id: admin_role1.id,
    }).await {
        Ok(_) => println!("✗ Constraint failed - should not allow duplicate membership"),
        Err(e) => println!("✓ Correctly prevented duplicate membership: {}", e),
    }

    // Test adding member with non-existent workspace (should fail)
    println!("Testing invalid workspace membership (should fail)...");
    match add_member(&mut conn, NewWorkspaceMember {
        workspace_id: uuid::Uuid::now_v7(),
        user_id: viewer_user.id,
        role_id: viewer_role1.id,
    }).await {
        Ok(_) => println!("✗ Constraint failed - should not allow membership in non-existent workspace"),
        Err(e) => println!("✓ Correctly prevented membership in non-existent workspace: {}", e),
    }

    // Test adding member with non-existent role (should fail)
    println!("Testing invalid role membership (should fail)...");
    match add_member(&mut conn, NewWorkspaceMember {
        workspace_id: workspace1.id,
        user_id: viewer_user.id,
        role_id: uuid::Uuid::now_v7(),
    }).await {
        Ok(_) => println!("✗ Constraint failed - should not allow membership with non-existent role"),
        Err(e) => println!("✓ Correctly prevented membership with non-existent role: {}", e),
    }
    println!();

    // ========================================================
    // STEP 7: Test role management
    // ========================================================
    println!("⚙️ STEP 7: Testing role management");

    // Create a new role in workspace1
    let moderator_role1 = create_role_service(&mut conn, NewRole {
        workspace_id: workspace1.id,
        name: format!("{}_moderator", EXAMPLE_PREFIX),
        description: Some("Can moderate content and users".to_string()),
    }).await?;
    println!("✓ Created moderator role in workspace1: '{}' (ID: {})", moderator_role1.name, moderator_role1.id);

    // Verify role was created
    let workspace1_roles_updated = list_roles_by_workspace(&mut conn, workspace1.id).await?;
    println!("✓ Workspace1 now has {} roles", workspace1_roles_updated.len());

    // Test duplicate role creation (should fail)
    println!("Testing duplicate role creation (should fail)...");
    match create_role_service(&mut conn, NewRole {
        workspace_id: workspace1.id,
        name: format!("{}_admin", EXAMPLE_PREFIX), // Same name as existing
        description: Some("Duplicate admin role".to_string()),
    }).await {
        Ok(_) => println!("✗ Constraint failed - should not allow duplicate role names"),
        Err(e) => println!("✓ Correctly prevented duplicate role creation: {}", e),
    }
    println!();

    // ========================================================
    // STEP 8: Test member role changes
    // ========================================================
    println!("🔄 STEP 8: Testing member role changes");

    // Change viewer's role to moderator in workspace1
    println!("Changing {}'s role from viewer to moderator...", viewer_user.email);

    // First remove current membership
    remove_member(&mut conn, workspace1.id, viewer_user.id).await?;
    println!("✓ Removed {} from workspace1", viewer_user.email);

    // Add with new role
    let viewer_moderator = add_member(&mut conn, NewWorkspaceMember {
        workspace_id: workspace1.id,
        user_id: viewer_user.id,
        role_id: moderator_role1.id,
    }).await?;
    println!("✓ Added {} back to workspace1 as moderator", viewer_user.email);

    // Verify the change
    let updated_workspace1_members = list_workspace_members(&mut conn, workspace1.id).await?;
    println!("✓ Workspace1 now has {} members", updated_workspace1_members.len());
    println!();

    // ========================================================
    // STEP 9: Test member removal and workspace deletion
    // ========================================================
    println!("🗑️ STEP 9: Testing member removal and workspace deletion");

    // Remove member2 from workspace2
    println!("Removing {} from workspace2...", member2_user.email);
    let removed_rows = remove_member(&mut conn, workspace2.id, member2_user.id).await?;
    println!("✓ Removed {} member(s) from workspace2", removed_rows);

    // Try to remove workspace owner (should fail)
    println!("Testing owner removal prevention (should fail)...");
    match remove_member(&mut conn, workspace1.id, owner_user.id).await {
        Ok(_) => println!("✗ Security failed - should not allow removing workspace owner"),
        Err(e) => println!("✓ Correctly prevented owner removal: {}", e),
    }

    // Delete workspace3 (owned by temp user)
    println!("Deleting workspace3...");
    let deleted_workspace3 = delete_workspace_service(&mut conn, workspace3.id).await?;
    println!("✓ Deleted workspace3: {}", deleted_workspace3);

    // Try to delete non-existent workspace (should fail)
    println!("Testing non-existent workspace deletion (should fail)...");
    match delete_workspace_service(&mut conn, uuid::Uuid::now_v7()).await {
        Ok(_) => println!("✗ Should not succeed with non-existent workspace"),
        Err(e) => println!("✓ Correctly failed for non-existent workspace: {}", e),
    }
    println!();

    // ========================================================
    // STEP 10: Test advanced scenarios and transactions
    // ========================================================
    println!("🔬 STEP 10: Testing advanced scenarios and transactions");

    // Test adding user to workspace using service method
    println!("Testing service method: add_user_to_workspace...");
    let new_user = register_user(
        &mut conn,
        RegisterUser {
            email: format!("{}_advanced@{}", EXAMPLE_PREFIX, "example.com"),
            password: "advancedpass123".to_string(),
            confirm_password: "advancedpass123".to_string(),
            full_name: Some("Advanced User".to_string()),
        },
    )
    .await?;

    let _new_member = add_user_to_workspace(
        &mut conn,
        workspace1.id,
        new_user.id,
        &format!("{}_editor", EXAMPLE_PREFIX) // Role name
    ).await?;
    println!("✓ Added {} to workspace1 using role name 'editor'", new_user.email);

    // Test transaction isolation for workspace creation
    println!("Testing transaction isolation for workspace creation...");
    let mut tx = pool.begin().await?;

    // Create workspace in transaction
    let tx_workspace = create_workspace(&mut *tx, NewWorkspace {
        name: format!("{}_transaction_workspace", EXAMPLE_PREFIX),
        owner_id: owner_user.id,
    }).await?;
    println!("✓ Created workspace within transaction: {}", tx_workspace.name);

    // Create role in transaction
    let tx_role = create_role(&mut *tx, NewRole {
        workspace_id: tx_workspace.id,
        name: format!("{}_transaction_role", EXAMPLE_PREFIX),
        description: Some("Transaction test role".to_string()),
    }).await?;
    println!("✓ Created role within transaction: {}", tx_role.name);

    // Workspace should exist within transaction
    let tx_workspace_check = get_workspace_by_id(&mut *tx, tx_workspace.id).await?;
    println!("✓ Workspace exists within transaction: {}", tx_workspace_check.name);

    // But NOT outside transaction yet
    let outside_check = get_workspace_by_id(&mut conn, tx_workspace.id).await;
    assert!(outside_check.is_err(), "Workspace should not exist outside transaction before commit");
    println!("✓ Workspace correctly not visible outside transaction before commit");

    // Commit transaction
    tx.commit().await?;
    println!("✓ Transaction committed");

    // Now workspace should exist outside transaction
    let committed_workspace = get_workspace_by_id(&mut conn, tx_workspace.id).await?;
    println!("✓ Workspace now exists after commit: {}", committed_workspace.name);
    println!();

    // ========================================================
    // STEP 11: Final summary and cleanup
    // ========================================================
    println!("📊 STEP 11: Final summary");

    // List final workspaces
    let final_workspaces = list_workspaces(&mut conn).await?;
    println!("✓ Final workspaces count: {}", final_workspaces.len());

    // List final users
    let final_users = list_users(&mut conn).await?;
    println!("✓ Final users count: {}", final_users.len());

    // List all roles across all workspaces
    let mut total_roles = 0;
    for workspace in &final_workspaces {
        let roles = list_roles_by_workspace(&mut conn, workspace.id).await.unwrap_or_default();
        total_roles += roles.len();
        println!("  - Workspace '{}' has {} roles", workspace.name, roles.len());
    }
    println!("✓ Total roles across all workspaces: {}", total_roles);

    // List all members across all workspaces
    let mut total_members = 0;
    for workspace in &final_workspaces {
        let members = list_workspace_members(&mut conn, workspace.id).await.unwrap_or_default();
        total_members += members.len();
        println!("  - Workspace '{}' has {} members", workspace.name, members.len());
    }
    println!("✓ Total workspace memberships: {}", total_members);
    println!();

    println!("🎉 All workspace management features demonstrated successfully!");
    println!("✅ User Management & Multi-tenant Architecture");
    println!("✅ Workspace Creation & Management");
    println!("✅ Role-Based Access Control (RBAC)");
    println!("✅ Workspace Member Management");
    println!("✅ Role Assignment & Changes");
    println!("✅ Database Constraints & Validation");
    println!("✅ Service Layer Business Logic");
    println!("✅ Transaction Isolation");
    println!("✅ Owner Removal Prevention");
    println!("✅ Cross-Entity Relationships");
    println!("✅ Error Handling & Edge Cases");
    println!("✅ Re-run Safety (Auto-cleanup & Idempotent)");
    println!();
    println!("💡 This example demonstrates a complete multi-tenant workspace system");
    println!("   with proper RBAC, constraints, and business logic validation.");
    println!("   It's re-run safe and works with sqlx CLI migrations.");
    println!();

    // Clean up the connection
    drop(conn);
    pool.close().await;

    Ok(())
}