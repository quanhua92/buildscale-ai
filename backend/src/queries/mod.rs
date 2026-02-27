pub mod ai_models;
pub mod invitations;
pub mod roles;
pub mod sessions;
pub mod users;
pub mod workspaces;
pub mod workspace_members;

// Re-export file queries from fs module for backward compatibility
pub mod files {
    pub use crate::fs::queries::*;
}

// Re-export agent queries from agent module for backward compatibility
pub mod agent_sessions {
    pub use crate::agent::queries::*;
}

// Re-export chat queries from chat module for backward compatibility
pub mod chat {
    pub use crate::chat::queries::*;
}
