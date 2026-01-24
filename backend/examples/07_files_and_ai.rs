use buildscale::{
    load_config,
    models::{
        files::FileType,
        requests::{CreateFileRequest, CreateVersionRequest, SemanticSearchHttp, UpdateFileRequest},
        users::RegisterUser,
    },
    services::{
        files::{
            add_tag, create_file_with_content, create_version, get_file_network, link_files,
            update_file, process_file_for_ai, semantic_search,
        },
        users::register_user,
        workspaces::create_workspace,
    },
};
use secrecy::ExposeSecret;
use sqlx::PgPool;

const EXAMPLE_PREFIX: &str = "example_07_files_and_ai";

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

    // ========================================================
    // CLEANUP
    // ========================================================
    println!("Cleaning up any existing example data...");
    let email = format!("{}@example.com", EXAMPLE_PREFIX);
    
    // Cleanup: In a production environment with proper ON DELETE CASCADE/SET NULL,
    // deleting the workspace (or user) would automatically clean up all associated data.
    // Here we leverage our newly added ON DELETE SET NULL for author_id and existing
    // cascades for workspace_id.
    sqlx::query("DELETE FROM workspaces WHERE owner_id IN (SELECT id FROM users WHERE email = $1)")
        .bind(&email).execute(&mut *conn).await.ok();
    sqlx::query("DELETE FROM users WHERE email = $1")
        .bind(&email).execute(&mut *conn).await.ok();

    println!("✓ Cleanup completed");
    println!();

    // ========================================================
    // STEP 1: Setup User and Workspace
    // ========================================================
    println!("🚀 STEP 1: Setting up User and Workspace");
    let user = register_user(
        &mut conn,
        RegisterUser {
            email: email.clone(),
            password: "BuildScaleAI!SuperSecure!2026".to_string(),
            confirm_password: "BuildScaleAI!SuperSecure!2026".to_string(),
            full_name: Some("AI Demo User".to_string()),
        },
    )
    .await?;

    let workspace_result = create_workspace(
        &mut conn,
        buildscale::models::requests::CreateWorkspaceRequest {
            name: "AI Knowledge Base".to_string(),
            owner_id: user.id,
        },
    )
    .await?;
    let workspace_id = workspace_result.workspace.id;
    println!("✓ Created Workspace: {} (ID: {})", workspace_result.workspace.name, workspace_id);
    println!();

    // ========================================================
    // STEP 2: Create Files and Folders (The "Write" Tool)
    // ========================================================
    println!("📂 STEP 2: Creating file hierarchy");

    // 2.1 Create a Folder
    let folder_request = CreateFileRequest {
        workspace_id,
        parent_id: None,
        author_id: user.id,
        name: "Research".to_string(),
        slug: None,
        path: None,
        is_virtual: None,
        permission: None,
        file_type: FileType::Folder,
        content: serde_json::json!({}),
        app_data: None,
    };
    let folder = create_file_with_content(&mut conn, folder_request).await?.file;
    println!("✓ Created Folder: {} (Slug: /{})", folder.name, folder.slug);

    // 2.2 Create a Document inside the folder
    let doc1_request = CreateFileRequest {
        workspace_id,
        parent_id: Some(folder.id),
        author_id: user.id,
        name: "RAG Guide".to_string(),
        slug: Some("rag_guide.md".to_string()),
        path: None,
        is_virtual: None,
        permission: None,
        file_type: FileType::Document,
        content: serde_json::json!("Retrieval-Augmented Generation (RAG) is a technique used to give LLMs access to external data."),
        app_data: Some(serde_json::json!({"tags": ["ai", "guide"]})),
    };
    let doc1 = create_file_with_content(&mut conn, doc1_request).await?;
    println!("✓ Created Document: {} (Slug: /{}/{})", doc1.file.name, folder.slug, doc1.file.slug);

    // 2.3 Create another Document
    let doc2_request = CreateFileRequest {
        workspace_id,
        parent_id: Some(folder.id),
        author_id: user.id,
        name: "Agents".to_string(),
        slug: Some("agents.md".to_string()),
        path: None,
        is_virtual: None,
        permission: None,
        file_type: FileType::Document,
        content: serde_json::json!("Autonomous agents use files as their toolbox to perform actions and remember context."),
        app_data: None,
    };
    let doc2 = create_file_with_content(&mut conn, doc2_request).await?;
    println!("✓ Created Document: {} (Slug: /{}/{})", doc2.file.name, folder.slug, doc2.file.slug);
    println!();

    // ========================================================
    // STEP 3: Versioning (Immutable History)
    // ========================================================
    println!("⏱️ STEP 3: Demonstrating Versioning");
    
    let update_request = CreateVersionRequest {
        author_id: Some(user.id),
        branch: Some("main".to_string()),
        content: serde_json::json!("Retrieval-Augmented Generation (RAG) is a technique used to give LLMs access to external data. v2 adds re-ranking support."),
        app_data: None,
    };
    let v2 = create_version(&mut conn, doc1.file.id, update_request).await?;
    println!("✓ Updated 'rag_guide.md' to Version 2");
    println!("  - Original Hash: {}", doc1.latest_version.hash);
    println!("  - New Hash:      {}", v2.hash);
    println!();

    // ========================================================
    // STEP 4: Organization (Move & Rename)
    // ========================================================
    println!("🔄 STEP 4: Moving and Renaming");
    
    let move_request = UpdateFileRequest {
        parent_id: Some(None), // Move to root
        name: Some("AI Agents Handbook".to_string()),
        slug: None,
        is_virtual: None,
        permission: None,
    };
    let doc2_updated = update_file(&mut conn, doc2.file.id, move_request).await?;
    println!("✓ Moved and Renamed: {} (Slug: /{})", doc2_updated.name, doc2_updated.slug);
    println!();

    // ========================================================
    // STEP 5: Knowledge Graph (Tags & Links)
    // ========================================================
    println!("🕸️ STEP 5: Building the Knowledge Graph");
    
    // Add a tag
    add_tag(&mut conn, doc1.file.id, "Research").await?;
    println!("✓ Tagged 'rag_guide.md' with #research");

    // Link files (doc2 -> doc1)
    link_files(&mut conn, doc2.file.id, doc1.file.id).await?;
    println!("✓ Linked 'ai_agents_handbook.md' -> 'rag_guide.md'");

    // Fetch network
    let network = get_file_network(&mut conn, doc1.file.id).await?;
    println!("✓ Local Network for 'rag_guide.md':");
    println!("  - Tags: {:?}", network.tags);
    println!("  - Backlinks: {}", network.backlinks.len());
    for link in network.backlinks {
        println!("    <- Linked from: {}", link.slug);
    }
    println!();

    // ========================================================
    // STEP 6: AI Engine (Semantic Search)
    // ========================================================
    println!("🧠 STEP 6: AI Semantic Search");
    
    // Manually trigger ingestion (simulating background worker)
    println!("Triggering AI ingestion for documents...");
    process_file_for_ai(&mut conn, doc1.file.id, &config.ai).await?;
    process_file_for_ai(&mut conn, doc2.file.id, &config.ai).await?;
    
    // Perform search
    // Using a non-zero vector to ensure similarity scores are calculated correctly
    println!("Searching for documents related to 'autonomous context'...");
    let search_results = semantic_search(
        &mut conn,
        workspace_id,
        SemanticSearchHttp {
            query_vector: vec![0.1; 1536],
            limit: Some(5),
        },
    )
    .await?;

    println!("✓ Found {} results:", search_results.len());
    for (i, res) in search_results.iter().enumerate() {
        println!("  {}. File: {} (Similarity: {:.2})", i + 1, res.file.slug, res.similarity);
        println!("     Snippet: \"{}\"", res.chunk_content);
    }
    println!();

    println!("🎉 'Everything is a File' example completed successfully!");
    println!("✅ Centralized identity with immutable content");
    println!("✅ Safe hierarchy management with move/rename");
    println!("✅ Knowledge graph with bidirectional backlinks");
    println!("✅ AI-ready semantic search foundation");

    // Cleanup connection
    drop(conn);
    pool.close().await;

    Ok(())
}
