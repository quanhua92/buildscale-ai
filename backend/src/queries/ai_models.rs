//! Database queries for AI models and workspace model access control

use crate::models::ai_models::{
    AiModel, NewAiModel, UpdateAiModel, WorkspaceAiModel, NewWorkspaceAiModel,
    UpdateWorkspaceAiModel,
};
use crate::error::Result;
use sqlx::{PgPool, Postgres, Row};
use uuid::Uuid;

// ============================================================================
// AI Models Queries
// ============================================================================

/// Create a new AI model
pub async fn create_model(
    pool: &PgPool,
    new_model: &NewAiModel,
) -> Result<AiModel> {
    let model = sqlx::query_as::<Postgres, AiModel>(
        r#"
        INSERT INTO ai_models (provider, model_name, display_name, description, context_window, is_enabled)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#
    )
    .bind(&new_model.provider)
    .bind(&new_model.model_name)
    .bind(&new_model.display_name)
    .bind(&new_model.description)
    .bind(new_model.context_window)
    .bind(new_model.is_enabled)
    .fetch_one(pool)
    .await?;

    Ok(model)
}

/// Get all AI models
pub async fn get_all_models(pool: &PgPool) -> Result<Vec<AiModel>> {
    let models = sqlx::query_as::<Postgres, AiModel>(
        "SELECT * FROM ai_models ORDER BY provider, display_name"
    )
    .fetch_all(pool)
    .await?;

    Ok(models)
}

/// Get all enabled AI models
pub async fn get_enabled_models(pool: &PgPool) -> Result<Vec<AiModel>> {
    let models = sqlx::query_as::<Postgres, AiModel>(
        "SELECT * FROM ai_models WHERE is_enabled = true ORDER BY provider, display_name"
    )
    .fetch_all(pool)
    .await?;

    Ok(models)
}

/// Get models by provider
pub async fn get_models_by_provider(pool: &PgPool, provider: &str) -> Result<Vec<AiModel>> {
    let models = sqlx::query_as::<Postgres, AiModel>(
        "SELECT * FROM ai_models WHERE provider = $1 ORDER BY display_name"
    )
    .bind(provider)
    .fetch_all(pool)
    .await?;

    Ok(models)
}

/// Get a specific model by provider and model name
pub async fn get_model_by_provider_and_name(
    pool: &PgPool,
    provider: &str,
    model_name: &str,
) -> Result<Option<AiModel>> {
    let model = sqlx::query_as::<Postgres, AiModel>(
        "SELECT * FROM ai_models WHERE provider = $1 AND model_name = $2"
    )
    .bind(provider)
    .bind(model_name)
    .fetch_optional(pool)
    .await?;

    Ok(model)
}

/// Get a model by ID
pub async fn get_model_by_id(pool: &PgPool, model_id: Uuid) -> Result<Option<AiModel>> {
    let model = sqlx::query_as::<Postgres, AiModel>(
        "SELECT * FROM ai_models WHERE id = $1"
    )
    .bind(model_id)
    .fetch_optional(pool)
    .await?;

    Ok(model)
}

/// Update a model
pub async fn update_model(
    pool: &PgPool,
    model_id: Uuid,
    updates: &UpdateAiModel,
) -> Result<AiModel> {
    let model = sqlx::query_as::<Postgres, AiModel>(
        r#"
        UPDATE ai_models
        SET
            display_name = COALESCE($1, display_name),
            description = COALESCE($2, description),
            context_window = COALESCE($3, context_window),
            is_enabled = COALESCE($4, is_enabled),
            updated_at = NOW()
        WHERE id = $5
        RETURNING *
        "#
    )
    .bind(updates.display_name.as_ref())
    .bind(updates.description.as_ref())
    .bind(updates.context_window)
    .bind(updates.is_enabled)
    .bind(model_id)
    .fetch_one(pool)
    .await?;

    Ok(model)
}

/// Delete a model
pub async fn delete_model(pool: &PgPool, model_id: Uuid) -> Result<u64> {
    let result = sqlx::query("DELETE FROM ai_models WHERE id = $1")
        .bind(model_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

// ============================================================================
// Workspace AI Models Queries
// ============================================================================

/// Grant a workspace access to a model
pub async fn grant_workspace_model_access(
    pool: &PgPool,
    new_mapping: &NewWorkspaceAiModel,
) -> Result<WorkspaceAiModel> {
    let mapping = sqlx::query_as::<Postgres, WorkspaceAiModel>(
        r#"
        INSERT INTO workspace_ai_models (workspace_id, model_id, status)
        VALUES ($1, $2, $3)
        ON CONFLICT (workspace_id, model_id)
        DO UPDATE SET status = $3, updated_at = NOW()
        RETURNING *
        "#
    )
    .bind(new_mapping.workspace_id)
    .bind(new_mapping.model_id)
    .bind(&new_mapping.status)
    .fetch_one(pool)
    .await?;

    Ok(mapping)
}

/// Get all models for a workspace
pub async fn get_workspace_models(pool: &PgPool, workspace_id: Uuid) -> Result<Vec<WorkspaceAiModel>> {
    let models = sqlx::query_as::<Postgres, WorkspaceAiModel>(
        "SELECT * FROM workspace_ai_models WHERE workspace_id = $1 ORDER BY created_at DESC"
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(models)
}

/// Get enabled models for a workspace (with status='active')
pub async fn get_workspace_enabled_models(
    pool: &PgPool,
    workspace_id: Uuid,
) -> Result<Vec<WorkspaceAiModel>> {
    let models = sqlx::query_as::<Postgres, WorkspaceAiModel>(
        r#"
        SELECT wm.*
        FROM workspace_ai_models wm
        JOIN ai_models m ON wm.model_id = m.id
        WHERE wm.workspace_id = $1
          AND wm.status = 'active'
          AND m.is_enabled = true
        ORDER BY m.provider, m.display_name
        "#
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(models)
}

/// Get workspace-model mapping with full model details
pub async fn get_workspace_model_details(
    pool: &PgPool,
    workspace_id: Uuid,
) -> Result<Vec<(AiModel, WorkspaceAiModel)>> {
    let rows = sqlx::query(
        r#"
        SELECT
            m.id as model_id, m.provider, m.model_name, m.display_name,
            m.description, m.context_window, m.is_enabled,
            m.created_at as model_created_at, m.updated_at as model_updated_at,
            wm.id as wm_id, wm.workspace_id, wm.model_id as wm_model_id,
            wm.status, wm.created_at as wm_created_at, wm.updated_at as wm_updated_at
        FROM workspace_ai_models wm
        JOIN ai_models m ON wm.model_id = m.id
        WHERE wm.workspace_id = $1
        ORDER BY m.provider, m.display_name
        "#
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;

    let mut results = Vec::new();
    for row in rows {
        let ai_model = AiModel {
            id: row.get("model_id"),
            provider: row.get("provider"),
            model_name: row.get("model_name"),
            display_name: row.get("display_name"),
            description: row.get("description"),
            context_window: row.get("context_window"),
            is_enabled: row.get("is_enabled"),
            created_at: row.get("model_created_at"),
            updated_at: row.get("model_updated_at"),
        };

        let workspace_model = WorkspaceAiModel {
            id: row.get("wm_id"),
            workspace_id: row.get("workspace_id"),
            model_id: row.get("wm_model_id"),
            status: row.get("status"),
            created_at: row.get("wm_created_at"),
            updated_at: row.get("wm_updated_at"),
        };

        results.push((ai_model, workspace_model));
    }

    Ok(results)
}

/// Update workspace-model access status
pub async fn update_workspace_model_status(
    pool: &PgPool,
    workspace_id: Uuid,
    model_id: Uuid,
    updates: &UpdateWorkspaceAiModel,
) -> Result<WorkspaceAiModel> {
    let mapping = sqlx::query_as::<Postgres, WorkspaceAiModel>(
        r#"
        UPDATE workspace_ai_models
        SET
            status = COALESCE($1, status),
            updated_at = NOW()
        WHERE workspace_id = $2 AND model_id = $3
        RETURNING *
        "#
    )
    .bind(updates.status.as_ref())
    .bind(workspace_id)
    .bind(model_id)
    .fetch_one(pool)
    .await?;

    Ok(mapping)
}

/// Revoke workspace access to a model
pub async fn revoke_workspace_model_access(
    pool: &PgPool,
    workspace_id: Uuid,
    model_id: Uuid,
) -> Result<u64> {
    let result = sqlx::query(
        "DELETE FROM workspace_ai_models WHERE workspace_id = $1 AND model_id = $2"
    )
    .bind(workspace_id)
    .bind(model_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

/// Check if a workspace has access to a specific model
pub async fn check_workspace_model_access(
    pool: &PgPool,
    workspace_id: Uuid,
    model_id: Uuid,
) -> Result<bool> {
    let exists: Option<bool> = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM workspace_ai_models
            WHERE workspace_id = $1 AND model_id = $2 AND status = 'active'
        )
        "#
    )
    .bind(workspace_id)
    .bind(model_id)
    .fetch_one(pool)
    .await?;

    Ok(exists.unwrap_or(false))
}
