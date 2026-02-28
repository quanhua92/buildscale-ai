//! Agent module for BuildScale AI
//!
//! Organized into layers:
//! - `core/` - Agent personas and system prompts
//! - `models/` - Data structures (AgentSession, AgentType)
//! - `queries/` - Database operations
//! - `services/` - Business logic
//! - `handlers/` - HTTP API endpoints

pub mod core;
pub mod models;
pub mod queries;
pub mod services;
pub mod handlers;

// Re-export commonly used types
pub use models::{
    AgentSession, AgentType, SessionStatus, NewAgentSession, UpdateAgentSession,
    AgentSessionInfo, AgentSessionsListResponse, PauseSessionRequest,
    ResumeSessionRequest, SessionActionResponse, SessionHeartbeat,
};
pub use services::{
    get_or_create_session, get_session, pause_session,
    resume_session, cancel_session, cleanup_stale_sessions,
    create_session, update_session_status, update_session_task,
    update_session_metadata, update_heartbeat, delete_session,
    list_workspace_sessions, list_user_sessions,
};
pub use handlers::{
    list_workspace_sessions as list_workspace_sessions_handler,
    get_session as get_session_handler,
    pause_session as pause_session_handler,
    resume_session as resume_session_handler,
    cancel_session as cancel_session_handler,
};

/// Central registry for agent personas.
///
/// Returns the system prompt for a given role, falling back to the
/// default Assistant persona if the role is not recognized or provided.
///
/// # Arguments
/// * `role` - Optional role identifier (e.g., "planner", "builder")
/// * `mode` - Optional chat mode ("plan" or "build") to auto-select persona
/// * `plan_content` - Optional plan content for Build Mode (required when role="builder")
///
/// # Returns
/// System prompt string for the requested persona
///
/// # Examples
/// ```
/// use buildscale::agent;
///
/// // Get planner persona
/// let prompt = agent::get_persona(Some("planner"), None, None);
///
/// // Get builder persona with plan content
/// let prompt = agent::get_persona(Some("builder"), None, Some("Plan content here"));
///
/// // Auto-select based on mode
/// let prompt = agent::get_persona(None, Some("plan"), None);
///
/// // Default to assistant
/// let prompt = agent::get_persona(Some("unknown"), None, None);
/// ```
pub fn get_persona(role: Option<&str>, mode: Option<&str>, plan_content: Option<&str>) -> String {
    // Priority: explicit role > mode-based selection > default
    match role {
        Some("builder") => {
            // Builder requires plan content
            let plan = plan_content.unwrap_or("# No Plan Provided\n\nError: Builder agent requires a plan.");
            core::builder::get_system_prompt(plan)
        }
        Some("planner") => core::planner::get_system_prompt(),
        None => {
            // Mode-based selection
            match mode {
                Some("plan") => core::planner::get_system_prompt(),
                Some("build") => {
                    // Builder persona requires plan content
                    // This is handled by ChatService::build_context which reads the plan file
                    let plan = plan_content.unwrap_or("# No Plan Provided\n\nError: Builder agent requires a plan.");
                    core::builder::get_system_prompt(plan)
                }
                None => core::assistant::get_system_prompt(),
                _ => core::assistant::get_system_prompt(),
            }
        }
        _ => core::assistant::get_system_prompt(),
    }
}
