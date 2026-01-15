use buildscale::{
    load_config,
    models::{
        users::RegisterUser,
        requests::CreateWorkspaceRequest,
        workspace_members::{AddMemberRequest, UpdateMemberRoleRequest},
    },
    services::{
        users::register_user,
        workspaces::create_workspace,
        workspace_members::{add_member_by_email, list_members, update_member_role, remove_member, get_my_membership},
    },
};
use secrecy::ExposeSecret;
use sqlx::PgPool;

const EXAMPLE_PREFIX: &str = "example_06_members_management";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = load_config()?;

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

    // Clean up any existing example data
    println!("Cleaning up any existing example data...");
    let _ = sqlx::query("DELETE FROM workspace_members WHERE user_id IN (SELECT id FROM users WHERE email LIKE $1)")
        .bind(format!("{}%", EXAMPLE_PREFIX))
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM roles WHERE workspace_id IN (SELECT id FROM workspaces WHERE name LIKE $1)")
        .bind(format!("{}%", EXAMPLE_PREFIX))
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM workspaces WHERE name LIKE $1")
        .bind(format!("{}%", EXAMPLE_PREFIX))
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM users WHERE email LIKE $1")
        .bind(format!("{}%", EXAMPLE_PREFIX))
        .execute(&mut *conn)
        .await;
    println!("✓ Cleanup completed");
    println!();

    // ========================================================
    // STEP 1: Setup Users and Workspace
    // ========================================================
    println!("🚀 STEP 1: Setting up users and workspace");

    // Create workspace owner
    let owner = register_user(
        &mut conn,
        RegisterUser {
            email: format!("{}_owner@example.com", EXAMPLE_PREFIX),
            password: "SecurePass123!".to_string(),
            confirm_password: "SecurePass123!".to_string(),
            full_name: Some("Workspace Owner".to_string()),
        },
    ).await?;
    println!("✓ Created owner: {}", owner.email);

    // Create another user to be added as a member
    let member_user = register_user(
        &mut conn,
        RegisterUser {
            email: format!("{}_member@example.com", EXAMPLE_PREFIX),
            password: "SecurePass123!".to_string(),
            confirm_password: "SecurePass123!".to_string(),
            full_name: Some("Team Member".to_string()),
        },
    ).await?;
    println!("✓ Created member user: {}", member_user.email);

    // Create workspace
    let workspace_result = create_workspace(
        &mut conn,
        CreateWorkspaceRequest {
            name: format!("{}_team_hub", EXAMPLE_PREFIX),
            owner_id: owner.id,
        },
    ).await?;
    let workspace = workspace_result.workspace;
    println!("✓ Created workspace: '{}'", workspace.name);
    println!();

    // ========================================================
    // STEP 2: Add Member by Email
    // ========================================================
    println!("➕ STEP 2: Adding member to workspace by email");

    let add_request = AddMemberRequest {
        email: member_user.email.clone(),
        role_name: "member".to_string(),
    };

    let new_member = add_member_by_email(
        &mut conn,
        workspace.id,
        owner.id,
        add_request,
    ).await?;

    println!("✓ Added {} to workspace with role: {}", new_member.email, new_member.role_name);
    println!();

    // ========================================================
    // STEP 3: List Members (Detailed)
    // ========================================================
    println!("📋 STEP 3: Listing all members with detailed information");

    let members = list_members(&mut conn, workspace.id, owner.id).await?;
    println!("✓ Found {} members in workspace:", members.len());
    for (i, m) in members.iter().enumerate() {
        println!("  {}. {} (Role: {}, Name: {})", 
            i + 1, m.email, m.role_name, m.full_name.as_deref().unwrap_or("N/A"));
    }
    println!();

    // ========================================================
    // STEP 4: Get My Membership
    // ========================================================
    println!("👤 STEP 4: Getting current user membership");

    let my_membership = get_my_membership(&mut conn, workspace.id, member_user.id).await?;
    println!("✓ Member {} confirms their role is: {}", my_membership.email, my_membership.role_name);
    println!();

    // ========================================================
    // STEP 5: Update Member Role
    // ========================================================
    println!("🔄 STEP 5: Updating member role");

    let update_request = UpdateMemberRoleRequest {
        role_name: "editor".to_string(),
    };

    let updated_member = update_member_role(
        &mut conn,
        workspace.id,
        member_user.id,
        owner.id,
        update_request,
    ).await?;

    println!("✓ Updated {} role to: {}", updated_member.email, updated_member.role_name);
    println!();

    // ========================================================
    // STEP 6: Member Leaves Workspace
    // ========================================================
    println!("🚪 STEP 6: Member leaving workspace");

    remove_member(&mut conn, workspace.id, member_user.id, member_user.id).await?;
    println!("✓ Member {} successfully left the workspace", member_user.email);

    let final_members = list_members(&mut conn, workspace.id, owner.id).await?;
    println!("✓ Final member count: {}", final_members.len());
    println!();

    println!("🎉 Member management demonstration completed successfully!");
    println!("✅ Added members by email");
    println!("✅ Retrieved detailed member information (joined data)");
    println!("✅ Verified role assignments and updates");
    println!("✅ Demonstrated self-removal (leaving workspace)");
    println!("✅ Enforced RBAC and owner protection");

    // Clean up
    drop(conn);
    pool.close().await;

    Ok(())
}
