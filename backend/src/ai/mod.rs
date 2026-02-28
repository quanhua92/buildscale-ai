//! AI module - AI model management and workspace access control
//!
//! This module provides a layered architecture for AI model management:
//! - `models`: AiModel, NewAiModel, UpdateAiModel, WorkspaceAiModel, etc.
//! - `queries`: AI model database operations

pub mod models;
pub mod queries;

// Re-export commonly used types at module level
pub use models::{
    AiModel, NewAiModel, UpdateAiModel,
    WorkspaceAiModel, NewWorkspaceAiModel, UpdateWorkspaceAiModel,
    ModelAccessStatus,
};

pub use queries::{
    create_model, get_all_models, get_enabled_models,
    get_models_by_provider, get_model_by_provider_and_name, get_model_by_id,
    update_model, delete_model,
    grant_workspace_model_access, get_workspace_models,
    get_workspace_enabled_models, get_workspace_model_details,
    update_workspace_model_status, revoke_workspace_model_access,
    check_workspace_model_access, get_workspace_models_by_provider,
    get_model_by_provider_and_name_conn,
};
