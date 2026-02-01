← [Back to Index](./README.md) | **API Reference**: [REST API Guide](./REST_API_GUIDE.md)

# Developer API Guide

Service layer API reference and usage examples for the multi-tenant workspace-based RBAC system.

## Table of Contents
- [Quick API Reference](#quick-api-reference)
- [Core Service APIs](#core-service-apis)
  - [User Authentication](#user-authentication)
  - [Workspace Management](#workspace-management)
  - [Member Management](#member-management)
  - [File Management & AI](#file-management--ai)
  - [Permissions & RBAC](#permissions--rbac)
  - [Roles](#roles)
  - [AI Providers](#ai-providers)
  - [Invitations API](#invitations-api)
  - [Sessions API](#sessions-api)
  - [Refresh Token Service](#refresh-token-service)
  - [Chat & AI Services](#chat--ai-services)
  - [Validation Utilities](#validation-utilities)
- [Essential Usage Examples](#essential-usage-examples)
- [Development Best Practices](#development-best-practices)
- [Error Handling Guide](#error-handling-guide)
- [Key Architecture](#key-architecture)
- [Environment Setup](#environment-setup)

---

## Quick API Reference

| Area | Key Functions |
|------|--------------|
| **Users** | `register_user`, `register_user_with_workspace`, `login_user`, `validate_session`, `logout_user`, `get_user_by_id`, `update_password`, `is_email_available`, `verify_password`, `generate_session_token`, `get_session_info`, `get_user_active_sessions` |
| **Workspaces** | `create_workspace`, `get_workspace`, `list_user_workspaces`, `update_workspace_owner`, `can_access_workspace` |
| **Members** | `list_members`, `get_my_membership`, `add_member_by_email`, `update_member_role`, `remove_member` |
| **Files** | `create_file_with_content`, `create_version`, `get_file_with_content`, `move_or_rename_file`, `soft_delete_file`, `restore_file`, `purge_file`, `list_trash`, `hash_content`, `auto_wrap_document_content`, `slugify`, `calculate_path`, `ensure_path_exists`, `chunk_text`, `extract_text_recursively` |
| **Network** | `add_tag`, `remove_tag`, `list_files_by_tag`, `link_files`, `remove_link`, `get_file_network` |
| **AI Engine** | `process_file_for_ai`, `semantic_search` |
| **AI Providers** | `RigService::from_config`, `RigService::create_agent`, `ModelIdentifier::parse`, `AiProvider::from_str`, `get_models_by_provider`, `get_enabled_models`, `get_workspace_enabled_models` |
| **Chat & AI** | `save_message`, `get_chat_session`, `build_context`, `format_file_fragment`, `format_history_fragment`, `AttachmentManager`, `HistoryManager` |
| **Refresh Tokens** | `generate_refresh_token`, `verify_refresh_token` |
| **Permissions** | `validate_workspace_permission`, `validate_any_workspace_permission`, `require_workspace_permission`, `get_user_workspace_permissions` |
| **Roles** | `create_default_roles`, `get_role_by_name`, `list_workspace_roles`, `get_role` |
| **Invitations** | `create_invitation`, `accept_invitation`, `revoke_invitation`, `delete_invitation`, `bulk_create_invitations`, `get_invitation_by_token`, `list_workspace_invitations`, `list_user_sent_invitations`, `list_email_invitations`, `get_invitations_expiring_soon`, `get_workspace_invitation_stats`, `resend_invitation`, `cleanup_expired_invitations` |
| **Sessions** | `cleanup_expired_sessions`, `revoke_all_user_sessions`, `revoke_session_by_token`, `get_user_active_sessions`, `user_has_active_sessions`, `extend_all_user_sessions` |
| **Validation** | `validate_email`, `validate_password`, `validate_workspace_name`, `validate_session_token`, `validate_full_name`, `validate_uuid` |

---

## Core Service APIs

### User Authentication
```rust
// User registration (12+ char password, email validation)
pub async fn register_user(conn: &mut DbConn, register_user: RegisterUser) -> Result<User>

// Combined user + workspace creation
pub async fn register_user_with_workspace(
    conn: &mut DbConn,
    request: UserWorkspaceRegistrationRequest
) -> Result<UserWorkspaceResult>

// Authentication with dual-token generation (JWT + session)
pub async fn login_user(conn: &mut DbConn, login_user: LoginUser) -> Result<LoginResult>

// Session validation and management
pub async fn validate_session(conn: &mut DbConn, session_token: &str) -> Result<User>
pub async fn logout_user(conn: &mut DbConn, session_token: &str) -> Result<()>
pub async fn refresh_session(conn: &mut DbConn, session_token: &str, hours_to_extend: i64) -> Result<String>

// JWT access token refresh
pub async fn refresh_access_token(conn: &mut DbConn, refresh_token: &str) -> Result<RefreshTokenResult>

// Account security
pub async fn update_password(conn: &mut DbConn, user_id: Uuid, request: UpdatePasswordRequest) -> Result<()>
pub async fn is_email_available(conn: &mut DbConn, email: &str) -> Result<bool>

// Password and session token utilities
pub fn verify_password(password: &str, hash: &str) -> Result<bool>
pub fn generate_session_token() -> Result<String>

// Session information
pub async fn get_session_info(conn: &mut DbConn, session_token: &str) -> Result<Option<UserSession>>
pub async fn get_user_active_sessions(conn: &mut DbConn, user_id: Uuid) -> Result<Vec<UserSession>>
```

### Workspace Management
```rust
// Workspace creation with automatic setup (creates default roles + owner as admin)
pub async fn create_workspace(
    conn: &mut DbConn,
    request: CreateWorkspaceRequest
) -> Result<CompleteWorkspaceResult>

// Workspace creation with initial team
pub async fn create_workspace_with_members(
    conn: &mut DbConn,
    request: CreateWorkspaceWithMembersRequest
) -> Result<CompleteWorkspaceResult>

// Basic operations
pub async fn get_workspace(conn: &mut DbConn, id: Uuid) -> Result<Workspace>
pub async fn list_user_workspaces(conn: &mut DbConn, owner_id: Uuid) -> Result<Vec<Workspace>>
pub async fn list_workspaces(conn: &mut DbConn) -> Result<Vec<Workspace>>
pub async fn delete_workspace(conn: &mut DbConn, id: Uuid) -> Result<u64>

// Critical ownership and access functions
pub async fn update_workspace_owner(
    conn: &mut DbConn,
    workspace_id: Uuid,
    current_owner_id: Uuid,
    new_owner_id: Uuid,
) -> Result<Workspace>

pub async fn can_access_workspace(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<bool>

pub async fn validate_workspace_ownership(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<()>
```

### Member Management
```rust
// Member creation and assignment
pub async fn create_workspace_member(
    conn: &mut DbConn,
    new_member: NewWorkspaceMember,
) -> Result<WorkspaceMember>

// Member updates and role changes
pub async fn update_workspace_member(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
    update_member: UpdateWorkspaceMember,
) -> Result<WorkspaceMember>

pub async fn remove_workspace_member(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<u64>

// Member queries and lookups
pub async fn list_workspace_members(
    conn: &mut DbConn,
    workspace_id: Uuid,
) -> Result<Vec<WorkspaceMemberDetailed>>

pub async fn get_workspace_member(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<WorkspaceMember>

pub async fn get_my_membership(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<WorkspaceMemberDetailed>

pub async fn add_member_by_email(
    conn: &mut DbConn,
    workspace_id: Uuid,
    requester_user_id: Uuid,
    request: AddMemberRequest,
) -> Result<WorkspaceMemberDetailed>

pub async fn update_member_role(
    conn: &mut DbConn,
    workspace_id: Uuid,
    target_user_id: Uuid,
    requester_user_id: Uuid,
    request: UpdateMemberRoleRequest,
) -> Result<WorkspaceMemberDetailed>

pub async fn remove_member(
    conn: &mut DbConn,
    workspace_id: Uuid,
    target_user_id: Uuid,
    requester_user_id: Uuid,
) -> Result<()>
```

### File Management & AI
```rust
// Create file with initial content (Transactional)
pub async fn create_file_with_content(
    conn: &mut DbConn,
    request: CreateFileRequest,
) -> Result<FileWithContent>

// Append new version (Deduplicated)
pub async fn create_version(
    conn: &mut DbConn,
    file_id: Uuid,
    request: CreateVersionRequest,
) -> Result<FileVersion>

// File Lifecycle
pub async fn move_or_rename_file(conn: &mut DbConn, file_id: Uuid, request: UpdateFileHttp) -> Result<File>
pub async fn soft_delete_file(conn: &mut DbConn, file_id: Uuid) -> Result<()>
pub async fn restore_file(conn: &mut DbConn, file_id: Uuid) -> Result<File>
pub async fn purge_file(conn: &mut DbConn, file_id: Uuid) -> Result<()>
pub async fn list_trash(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<File>>

// Knowledge Graph
pub async fn add_tag(conn: &mut DbConn, file_id: Uuid, tag: &str) -> Result<()>
pub async fn remove_tag(conn: &mut DbConn, file_id: Uuid, tag: &str) -> Result<()>
pub async fn list_files_by_tag(conn: &mut DbConn, workspace_id: Uuid, tag: &str) -> Result<Vec<File>>
pub async fn link_files(conn: &mut DbConn, source_id: Uuid, target_id: Uuid) -> Result<()>
pub async fn remove_link(conn: &mut DbConn, source_id: Uuid, target_id: Uuid) -> Result<()>
pub async fn get_file_network(conn: &mut DbConn, file_id: Uuid) -> Result<FileNetworkSummary>

// AI Engine
pub async fn process_file_for_ai(conn: &mut DbConn, file_id: Uuid, config: &AiConfig) -> Result<()>
pub async fn semantic_search(
    conn: &mut DbConn,
    workspace_id: Uuid,
    request: SemanticSearchHttp
) -> Result<Vec<SearchResult>>

// File Utility Functions
pub fn hash_content(version_id: Uuid, content: &serde_json::Value) -> Result<String>
pub fn auto_wrap_document_content(file_type: FileType, content: serde_json::Value) -> serde_json::Value
pub fn slugify(name: &str) -> String
pub fn calculate_path(parent_path: Option<&str>, slug: &str) -> String
pub async fn ensure_path_exists(conn: &mut DbConn, workspace_id: Uuid, path: &str, author_id: Uuid) -> Result<Option<Uuid>>
pub fn chunk_text(text: &str, window_size: usize, overlap: usize) -> Vec<String>
pub fn extract_text_recursively(value: &serde_json::Value) -> String
```

### Permissions & RBAC
```rust
// Basic permission validation
pub async fn validate_workspace_permission(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
    permission: &str,
) -> Result<bool>

// Requirement check (Errors if permission missing)
pub async fn require_workspace_permission(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
    permission: &str,
) -> Result<()>

// Multi-permission validation
pub async fn validate_any_workspace_permission(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
    permissions: Vec<&str>,
) -> Result<bool>

pub async fn validate_all_workspace_permissions(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
    permissions: Vec<&str>,
) -> Result<bool>

// Metadata lookup
pub async fn get_user_workspace_permissions(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<Vec<String>>
```

### Roles
```rust
// Default setup for new workspaces
pub async fn create_default_roles(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<Role>>

// Role lookup
pub async fn get_role_by_name(conn: &mut DbConn, workspace_id: Uuid, name: &str) -> Result<Role>
pub async fn list_workspace_roles(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<Role>>
pub async fn get_role(conn: &mut DbConn, id: Uuid) -> Result<Role>
```

### AI Providers

The multi-provider AI system supports OpenAI and OpenRouter with flexible configuration and workspace-level provider overrides.

**Core Types**:

```rust
use backend::providers::{AiProvider, ModelIdentifier};

// Supported AI providers
pub enum AiProvider {
    OpenAi,
    OpenRouter,
}

impl AiProvider {
    pub fn as_str(&self) -> &'static str
}

// Parse provider from string
impl FromStr for AiProvider {
    type Err = String;
}

// Model identifier with provider
pub struct ModelIdentifier {
    pub provider: AiProvider,
    pub model: String,
}

impl ModelIdentifier {
    // Parse "provider:model" or legacy "model" format
    pub fn parse(input: &str, default_provider: AiProvider) -> Result<Self, String>
}
```

**RigService**:

```rust
use backend::services::chat::rig_engine::RigService;

// Multi-provider AI service
pub struct RigService {
    openai: Option<Arc<OpenAiProvider>>,
    openrouter: Option<Arc<OpenRouterProvider>>,
    default_provider: AiProvider,
}

impl RigService {
    // Create from AI configuration
    pub fn from_config(ai_config: &AiConfig) -> Result<Self, String>

    // Check if provider is configured
    pub fn is_provider_configured(&self, provider: AiProvider) -> bool

    // Get list of configured providers
    pub fn configured_providers(&self) -> Vec<AiProvider>

    // Create AI agent for chat session
    pub async fn create_agent(
        &self,
        pool: DbPool,
        storage: Arc<FileStorageService>,
        workspace_id: Uuid,
        chat_id: Uuid,
        user_id: Uuid,
        session: &ChatSession,
    ) -> Result<rig::agent::AgentBox, PromptError>
}
```

**Provider Query Functions**:

```rust
use backend::queries::ai_models;

// Get models by provider
pub async fn get_models_by_provider(
    pool: &PgPool,
    provider: &str,
) -> Result<Vec<AiModel>>

// Get all enabled models
pub async fn get_enabled_models(
    pool: &PgPool,
) -> Result<Vec<AiModel>>

// Get workspace enabled models
pub async fn get_workspace_enabled_models(
    pool: &PgPool,
    workspace_id: Uuid,
) -> Result<Vec<AiModel>>

// Check workspace model access
pub async fn check_workspace_model_access(
    pool: &PgPool,
    workspace_id: Uuid,
    model_id: Uuid,
) -> Result<bool>

// Grant workspace model access
pub async fn grant_workspace_model_access(
    pool: &PgPool,
    new_model: &NewWorkspaceAiModel,
) -> Result<WorkspaceAiModel>

// Revoke workspace model access
pub async fn revoke_workspace_model_access(
    pool: &PgPool,
    workspace_id: Uuid,
    model_id: Uuid,
) -> Result<u64>
```

**Configuration**:

```rust
use backend::config::{AiConfig, ProviderConfig, OpenAIConfig, OpenRouterConfig};

// Provider configuration structure
pub struct ProviderConfig {
    pub openai: Option<OpenAIConfig>,
    pub openrouter: Option<OpenRouterConfig>,
    pub default_provider: String,
}

// OpenAI-specific configuration
pub struct OpenAIConfig {
    pub api_key: SecretString,
    pub base_url: Option<String>,
    pub enable_reasoning_summaries: bool,
    pub reasoning_effort: String,  // "low", "medium", "high"
}

// OpenRouter configuration
pub struct OpenRouterConfig {
    pub api_key: SecretString,
    pub base_url: Option<String>,
}
```

**Model Identifier Format**:

The system supports both new and legacy model identifier formats:

```rust
// New format (recommended): "provider:model"
let model = ModelIdentifier::parse("openai:gpt-4o", AiProvider::OpenAi)?;
// Returns: ModelIdentifier { provider: OpenAi, model: "gpt-4o" }

let model = ModelIdentifier::parse("openrouter:anthropic/claude-3.5-sonnet", AiProvider::OpenRouter)?;
// Returns: ModelIdentifier { provider: OpenRouter, model: "anthropic/claude-3.5-sonnet" }

// Legacy format (backward compatible): "model"
let model = ModelIdentifier::parse("gpt-4o", AiProvider::OpenAi)?;
// Returns: ModelIdentifier { provider: OpenAi, model: "gpt-4o" }
```

**Usage Example**:

```rust
use backend::providers::{AiProvider, ModelIdentifier};
use backend::services::chat::rig_engine::RigService;
use backend::config::AiConfig;

// Load AI configuration
let ai_config = AiConfig::load()?;

// Create RigService from config
let rig_service = RigService::from_config(&ai_config)?;

// Check provider availability
if rig_service.is_provider_configured(AiProvider::OpenAi) {
    println!("OpenAI provider is configured");
}

// Parse model identifier
let model_id = ModelIdentifier::parse("openai:gpt-4o", AiProvider::OpenAi)?;

// Create AI agent for chat
let agent = rig_service.create_agent(
    pool,
    storage,
    workspace_id,
    chat_id,
    user_id,
    &chat_session,
).await?;

// Use agent with Rig.rs framework
let response = agent.prompt(&user_message).await?;
```

**Environment Variables**:

```bash
# OpenAI Provider
BUILDSCALE__AI__PROVIDERS__OPENAI__API_KEY=sk-...
BUILDSCALE__AI__PROVIDERS__OPENAI__ENABLE_REASONING_SUMMARIES=true
BUILDSCALE__AI__PROVIDERS__OPENAI__REASONING_EFFORT=medium

# OpenRouter Provider
BUILDSCALE__AI__PROVIDERS__OPENROUTER__API_KEY=sk-or-...

# Default Provider
BUILDSCALE__AI__PROVIDERS__DEFAULT_PROVIDER=openai

# Legacy (deprecated, auto-migrates to providers.openai.api_key)
BUILDSCALE__AI__OPENAI_API_KEY=sk-...
```

**Workspace Provider Override**:

Workspaces can override the default provider:

```rust
use backend::models::workspaces::UpdateWorkspace;

// Set workspace to use OpenRouter
let update = UpdateWorkspace {
    name: None,
    owner_id: None,
    ai_provider_override: Some(Some("openrouter".to_string())),
};

workspaces::update_workspace(&mut conn, workspace_id, update).await?;

// Clear override (use global default)
let update = UpdateWorkspace {
    name: None,
    owner_id: None,
    ai_provider_override: Some(None),  // Set to NULL
};
```

**Key Features**:
- **Multi-Provider Support**: OpenAI and OpenRouter with unified interface
- **Backward Compatible**: Legacy model strings auto-migrated to new format
- **Workspace Override**: Per-workspace provider configuration
- **OpenAI Reasoning**: Configurable reasoning effort for GPT-5 models
- **Model Access Control**: Global and workspace-level model availability
- **Runtime Migration**: Automatic legacy format detection and migration

### Invitations API
```rust
// Manage lifecycle of workspace invites
pub async fn create_invitation(
    conn: &mut DbConn,
    request: CreateInvitationRequest,
    inviter_id: Uuid,
) -> Result<CreateInvitationResponse>

pub async fn accept_invitation(
    conn: &mut DbConn,
    request: AcceptInvitationRequest,
    user_id: Uuid,
) -> Result<AcceptInvitationResponse>

pub async fn revoke_invitation(
    conn: &mut DbConn,
    request: RevokeInvitationRequest,
    revoker_id: Uuid,
) -> Result<WorkspaceInvitation>

pub async fn delete_invitation(
    conn: &mut DbConn,
    invitation_id: Uuid,
    deleter_id: Uuid,
) -> Result<u64>

pub async fn bulk_create_invitations(
    conn: &mut DbConn,
    workspace_id: Uuid,
    emails: Vec<String>,
    role_name: String,
    inviter_id: Uuid,
    expires_in_hours: Option<i64>,
) -> Result<Vec<CreateInvitationResponse>>

pub async fn get_invitation_by_token(conn: &mut DbConn, token: &str) -> Result<WorkspaceInvitation>

// Invitation listing and filtering
pub async fn list_workspace_invitations(
    conn: &mut DbConn,
    workspace_id: Uuid,
    requester_id: Uuid,
) -> Result<Vec<WorkspaceInvitation>>

pub async fn list_user_sent_invitations(
    conn: &mut DbConn,
    user_id: Uuid,
) -> Result<Vec<WorkspaceInvitation>>

pub async fn list_email_invitations(
    conn: &mut DbConn,
    email: &str,
) -> Result<Vec<WorkspaceInvitation>>

pub async fn get_invitations_expiring_soon(
    conn: &mut DbConn,
    hours: i32,
) -> Result<Vec<WorkspaceInvitation>>

pub async fn get_workspace_invitation_stats(
    conn: &mut DbConn,
    workspace_id: Uuid,
    requester_id: Uuid,
) -> Result<Vec<(String, i64)>>

// Invitation management
pub async fn resend_invitation(
    conn: &mut DbConn,
    invitation_id: Uuid,
    resender_id: Uuid,
    expires_in_hours: Option<i64>,
) -> Result<CreateInvitationResponse>

pub async fn cleanup_expired_invitations(conn: &mut DbConn) -> Result<u64>
```

### Sessions API
```rust
// Administrative session management
pub async fn cleanup_expired_sessions(conn: &mut DbConn) -> Result<u64>
pub async fn revoke_all_user_sessions(conn: &mut DbConn, user_id: Uuid) -> Result<u64>
pub async fn revoke_session_by_token(conn: &mut DbConn, session_token: &str) -> Result<()>
pub async fn get_user_active_sessions(conn: &mut DbConn, user_id: Uuid) -> Result<Vec<UserSession>>
pub async fn user_has_active_sessions(conn: &mut DbConn, user_id: Uuid) -> Result<bool>
pub async fn extend_all_user_sessions(conn: &mut DbConn, user_id: Uuid, hours_to_extend: i64) -> Result<u64>
```

### Refresh Token Service
```rust
// HMAC-signed refresh token generation (256-bit randomness)
// Format: "<random_64chars>:<signature_64chars>" (129 total characters)
pub fn generate_refresh_token(config: &Config) -> Result<String>

// Verify refresh token signature using constant-time comparison
pub fn verify_refresh_token(token: &str, config: &Config) -> Result<Vec<u8>>
```

**Security Features**:
- **HMAC-SHA256**: Tamper-evident signature prevents token manipulation
- **256-bit Randomness**: Cryptographically secure random bytes
- **Constant-Time Comparison**: Prevents timing attacks on verification
- **Format**: `<random_hex>:<signature_hex>` for easy inspection

### Chat & AI Services
```rust
// Chat message persistence with write-through caching
pub async fn save_message(
    conn: &mut DbConn,
    workspace_id: Uuid,
    new_msg: NewChatMessage,
) -> Result<ChatMessage>

// Complete chat session retrieval (config + message history)
pub async fn get_chat_session(
    conn: &mut DbConn,
    workspace_id: Uuid,
    chat_file_id: Uuid,
) -> Result<ChatSession>

// Update chat metadata (mode, plan_file)
pub async fn update_chat_metadata(
    conn: &mut DbConn,
    storage: &FileStorageService,
    workspace_id: Uuid,
    chat_file_id: Uuid,
    mode: String,
    plan_file: Option<String>,
) -> Result<()>

// Build AI context with attachments and history
pub async fn build_context(
    conn: &mut DbConn,
    workspace_id: Uuid,
    chat_file_id: Uuid,
    default_persona: &str,
    default_context_token_limit: usize,
) -> Result<BuiltContext>

// Context formatting utilities
pub fn format_file_fragment(path: &str, content: &str) -> String
pub fn format_history_fragment(messages: &[ChatMessage]) -> String
```

**Context Management**:

```rust
// Attachment manager for file attachments with priority-based pruning
pub struct AttachmentManager {
    pub map: AttachmentMap,  // IndexMap<AttachmentKey, AttachmentValue>
}

impl AttachmentManager {
    pub fn new() -> Self
    pub fn add_fragment(&mut self, key: AttachmentKey, value: AttachmentValue)
    pub fn optimize_for_limit(&mut self, max_tokens: usize)  // Prune low-priority files
    pub fn sort_by_position(&mut self)  // System → Skills → Files → Env → History → Request
    pub fn render(&self) -> String  // Wrap files in <file_context> markers
}

// History manager for conversation messages with token estimation
pub struct HistoryManager {
    pub messages: Vec<ChatMessage>,
}

impl HistoryManager {
    pub fn new(messages: Vec<ChatMessage>) -> Self
    pub fn estimate_tokens(&self) -> usize  // Uses 4 chars per token
    pub fn len(&self) -> usize
    pub fn is_empty(&self) -> bool
}
```

**Key Features**:
- **Write-Through Caching**: `save_message()` updates both chat_messages and file_versions tables
- **Priority-Based Pruning**: Drop low-priority files when over token limits
- **Token Estimation**: Automatic counting using `ESTIMATED_CHARS_PER_TOKEN` (4 chars/token)
- **Attachment Priorities**: ESSENTIAL(0) > HIGH(3) > MEDIUM(5) > LOW(10)
- **Position-Based Ordering**: Ensures consistent prompt structure
- **In-Place Update Optimization**: Reuses latest version to prevent history bloat

### Validation Utilities
```rust
// Centralized validation logic
pub fn validate_email(email: &str) -> Result<()>
pub fn validate_password(password: &str) -> Result<()>
pub fn validate_workspace_name(name: &str) -> Result<()>
pub fn validate_full_name(full_name: &Option<String>) -> Result<()>
pub fn validate_session_token(token: &str) -> Result<()>
pub fn validate_file_slug(slug: &str) -> Result<()>
pub fn validate_uuid(uuid_str: &str) -> Result<Uuid>
```

---

## Essential Usage Examples

### User Authentication
```rust
// Register + Login
let user = register_user(&mut conn, RegisterUser {
    email: "user@example.com".to_string(),
    password: "SecurePass123!".to_string(),
    confirm_password: "SecurePass123!".to_string(),
    full_name: Some("John Doe".to_string()),
}).await?;

let login_result = login_user(&mut conn, LoginUser {
    email: "user@example.com".to_string(),
    password: "SecurePass123!".to_string(),
}).await?;

// Returns both JWT access token (15 min) and refresh token (30 days)
// - Use login_result.access_token in API Authorization header
// - Use login_result.refresh_token to get new access tokens

// When access token expires, refresh it
let new_token = refresh_access_token(&mut conn, &login_result.refresh_token).await?;

// Validate session (uses refresh token)
let user = validate_session(&mut conn, &login_result.refresh_token).await?;

// Logout (invalidates refresh token)
logout_user(&mut conn, &login_result.refresh_token).await?;
```

### Cookie-Based Authentication (Browser Clients)

For web browser clients, use cookie utilities for seamless authentication:

```rust
use backend::services::cookies::{
    extract_jwt_token,
    extract_refresh_token,
    build_access_token_cookie,
    build_refresh_token_cookie,
    build_clear_token_cookie,
    CookieConfig,
};
use backend::services::jwt::authenticate_jwt_token_from_anywhere;

// Extract token from header or cookie (priority: header > cookie)
let token = extract_jwt_token(
    request.headers().get("authorization")
        .and_then(|h| h.to_str().ok()),
    request.cookies().get("access_token")
        .and_then(|c| Some(c.value()))
)?;

// Authenticate with multi-source support
let user_id = authenticate_jwt_token_from_anywhere(
    request.headers().get("authorization")
        .and_then(|h| h.to_str().ok()),
    request.cookies().get("access_token")
        .and_then(|c| Some(c.value())),
    &config.jwt.secret,
)?;

// Build cookies for login response
let config = CookieConfig::default();
let access_cookie = build_access_token_cookie(&login_result.access_token, &config);
let refresh_cookie = build_refresh_token_cookie(&login_result.refresh_token, &config);

// Set cookies in response
response.append_header("Set-Cookie", access_cookie);
response.append_header("Set-Cookie", refresh_cookie);

// Clear cookies for logout
let clear_access = build_clear_token_cookie("access_token");
let clear_refresh = build_clear_token_cookie("refresh_token");
response.append_header("Set-Cookie", clear_access);
response.append_header("Set-Cookie", clear_refresh);
```

**Cookie Security Flags**:
- `HttpOnly`: Prevents JavaScript access (XSS protection)
- `Secure`: HTTPS-only (set to `true` in production)
- `SameSite=Lax`: CSRF protection while allowing links from emails/OAuth
- `Max-Age`: Automatic expiration (15 min for access, 30 days for refresh)

### Workspace Setup
```rust
// Create workspace with automatic role setup
let workspace_result = create_workspace(&mut conn, CreateWorkspaceRequest {
    name: "Team Workspace".to_string(),
    owner_id: user.id,
}).await?;
// Creates: workspace + 4 default roles + owner as admin

// Create workspace with initial team
let workspace_result = create_workspace_with_members(&mut conn, CreateWorkspaceWithMembersRequest {
    name: "Project Workspace".to_string(),
    owner_id: user.id,
    members: vec![
        WorkspaceMemberRequest {
            user_id: editor_user.id,
            role_name: "editor".to_string(),
        },
        WorkspaceMemberRequest {
            user_id: member_user.id,
            role_name: "member".to_string(),
        },
    ],
}).await?;
```

### Member Management
```rust
// Add member with role
let member = add_workspace_member(&mut conn, workspace.id, user.id, editor_role.id).await?;

// Update member role
let updated = update_workspace_member_role(&mut conn, workspace.id, user.id, admin_role.id).await?;

// List members
let members = list_workspace_members(&mut conn, workspace.id).await?;
```

### Invitation Workflow
```rust
// Create invitation
let invitation_result = create_invitation(&mut conn, CreateInvitationRequest {
    workspace_id: workspace.id,
    invited_email: "teammate@example.com".to_string(),
    role_name: "member".to_string(),
    expires_in_hours: Some(168), // 7 days (note: this is for invitations, not sessions)
}, inviter.id).await?;

// Accept invitation (creates membership automatically)
let accept_result = accept_invitation(&mut conn, AcceptInvitationRequest {
    invitation_token: invitation_result.invitation.invitation_token,
}, new_user.id).await?;
```

### Bulk Operations
```rust
// Bulk invite team members
let emails = vec![
    "member1@example.com".to_string(),
    "member2@example.com".to_string(),
    "member3@example.com".to_string(),
];

let invitation_results = bulk_create_invitations(
    &mut conn,
    workspace.id,
    emails,
    "member".to_string(),
    inviter.id,
    Some(168), // 7 days (note: this is for invitations, not sessions)
).await?;

// Cleanup expired sessions/invitations
let cleaned_sessions = cleanup_expired_sessions(&mut conn).await?;
let cleaned_invitations = cleanup_expired_invitations(&mut conn).await?;
```

## Development Best Practices

### Use Type-Safe Role Constants
```rust
// ✅ Preferred: Use centralized constants
use backend::models::roles::{ADMIN_ROLE, EDITOR_ROLE, MEMBER_ROLE, VIEWER_ROLE};

let member_request = WorkspaceMemberRequest {
    user_id: user.id,
    role_name: ADMIN_ROLE.to_string(),
};
```

### Use Comprehensive Creation Methods
```rust
// ✅ Preferred: Automatic workspace setup
let result = create_workspace(&mut conn, workspace_request).await?;
// Creates: workspace + default roles + owner as admin

// ❌ Avoid: Manual multi-step creation
```

## Error Handling Guide

### Error Types

The system uses a comprehensive error hierarchy with specific error types for different scenarios:

```rust
#[derive(Debug, Error)]
pub enum Error {
    Sqlx(#[from] sqlx::Error),           // Database errors
    Validation(String),                   // Input validation errors
    NotFound(String),                      // Resource not found
    Forbidden(String),                    // Permission denied
    Conflict(String),                      // Resource conflicts
    Authentication(String),               // Invalid credentials
    InvalidToken(String),                 // Invalid/expired session tokens
    SessionExpired(String),               // Session expiration errors
    Internal(String),                      // System errors
}
```

### Error Handling Patterns

#### 1. Comprehensive Error Handling
```rust
match service_function(&mut conn, request).await {
    Ok(result) => handle_success(result),
    Err(Error::Validation(msg)) => {
        log::warn!("Validation error: {}", msg);
        return Err(create_api_error(400, msg));
    },
    Err(Error::Authentication(msg)) => {
        log::info!("Authentication failed: {}", msg);
        return Err(create_api_error(401, "Invalid credentials"));
    },
    Err(Error::InvalidToken(msg) | Error::SessionExpired(msg)) => {
        log::info!("Session error: {}", msg);
        return Err(create_api_error(401, "Session expired"));
    },
    Err(Error::Forbidden(msg)) => {
        log::warn!("Access forbidden: {}", msg);
        return Err(create_api_error(403, "Access denied"));
    },
    Err(Error::NotFound(msg)) => {
        log::info!("Resource not found: {}", msg);
        return Err(create_api_error(404, msg));
    },
    Err(Error::Conflict(msg)) => {
        log::warn!("Conflict error: {}", msg);
        return Err(create_api_error(409, msg));
    },
    Err(Error::Sqlx(db_error)) => {
        log::error!("Database error: {}", db_error);
        return Err(create_api_error(500, "Database error"));
    },
    Err(Error::Internal(msg)) => {
        log::error!("Internal error: {}", msg);
        return Err(create_api_error(500, "Internal server error"));
    }
}
```

#### 2. User Registration Error Handling
```rust
match register_user(&mut conn, register_request).await {
    Ok(user) => create_user_session(user),
    Err(Error::Validation(msg)) => {
        match msg.as_str() {
            "Password must be at least 12 characters long" =>
                show_field_error("password", "Password too short"),
            "Passwords do not match" =>
                show_field_error("confirm_password", "Passwords don't match"),
            "Email cannot be empty" =>
                show_field_error("email", "Email is required"),
            _ => show_general_error("Validation failed")
        }
    },
    Err(Error::Conflict(msg)) if msg.contains("duplicate key value violates unique constraint") => {
        show_field_error("email", "Email already registered");
    },
    Err(error) => {
        log::error!("Registration error: {}", error);
        show_general_error("Registration failed. Please try again.");
    }
}
```

#### 3. Authentication Error Handling
```rust
match login_user(&mut conn, login_request).await {
    Ok(login_result) => {
        create_user_session(login_result);
        redirect_to_dashboard();
    },
    Err(Error::Authentication(_)) => {
        show_error("Invalid email or password");
        increment_login_attempts();
    },
    Err(Error::Validation(msg)) => {
        show_error(&format!("Please fill in all fields: {}", msg));
    },
    Err(error) => {
        log::error!("Login error: {}", error);
        show_error("Login failed. Please try again.");
    }
}
```

#### 4. Workspace Access Error Handling
```rust
match can_access_workspace(&mut conn, workspace_id, user.id).await {
    Ok(true) => {
        // User has access - proceed
        handle_workspace_request();
    },
    Ok(false) => {
        log::warn!("User {} attempted to access workspace {}", user.id, workspace_id);
        return Err(create_api_error(403, "You don't have access to this workspace"));
    },
    Err(Error::NotFound(_)) => {
        return Err(create_api_error(404, "Workspace not found"));
    },
    Err(Error::InvalidToken(_) | Error::SessionExpired(_)) => {
        return Err(create_api_error(401, "Please log in to continue"));
    },
    Err(error) => {
        log::error!("Workspace access check failed: {}", error);
        return Err(create_api_error(500, "Access check failed"));
    }
}
```

#### 5. Session Management Error Handling
```rust
match validate_session(&mut conn, session_token).await {
    Ok(user) => {
        // Valid session - proceed with request
        handle_authenticated_request(user);
    },
    Err(Error::InvalidToken(_) | Error::SessionExpired(_)) => {
        // Clear invalid session cookie
        clear_session_cookie();
        redirect_to_login_with_message("Your session has expired. Please log in again.");
    },
    Err(error) => {
        log::error!("Session validation error: {}", error);
        clear_session_cookie();
        redirect_to_login_with_message("Authentication error. Please log in again.");
    }
}
```

### Error Response Format

#### API Error Response Structure
```rust
pub struct ApiError {
    pub error: String,
    pub message: String,
    pub details: Option<String>,
    pub code: Option<String>,
}

// Example error responses
{
    "error": "validation_error",
    "message": "Email is required",
    "details": "The email field cannot be empty",
    "code": "EMAIL_REQUIRED"
}

{
    "error": "authentication_error",
    "message": "Invalid credentials",
    "details": null,
    "code": "INVALID_CREDENTIALS"
}
```

### Common Error Scenarios

| Scenario | Error Type | HTTP Status | User Message |
|----------|------------|-------------|--------------|
| Invalid email format | `Validation` | 400 | "Invalid email format" |
| Password too short | `Validation` | 400 | "Password must be at least 12 characters long" |
| Email already exists | `Conflict` | 409 | "Email already registered" |
| Invalid login credentials | `Authentication` | 401 | "Invalid email or password" |
| Session expired | `SessionExpired` | 401 | "Session expired. Please log in again" |
| No workspace access | `Forbidden` | 403 | "You don't have access to this workspace" |
| Workspace not found | `NotFound` | 404 | "Workspace not found" |
| Database connection failed | `Sqlx` | 500 | "Database error. Please try again" |
| Internal system error | `Internal` | 500 | "Internal server error" |

## Key Architecture

- **Three-Layer**: Service → Query → Model architecture
- **RBAC System**: 4-tier roles (Admin > Editor > Member > Viewer)
- **Comprehensive Permissions**: Fine-grained permissions across workspace, content, and member categories
- **Multi-Tenant**: Complete workspace isolation with shared users
- **Session-Based**: Random HMAC-signed tokens with Argon2 password hashing

## Environment Setup

```bash
# Required environment variables
BUILDSCALE__DATABASE__USER=your_db_user
BUILDSCALE__DATABASE__PASSWORD=your_db_password
BUILDSCALE__DATABASE__HOST=localhost
BUILDSCALE__DATABASE__PORT=5432
BUILDSCALE__DATABASE__DATABASE=your_db_name

# Development commands
cargo build                    # Build project
cargo test                      # Run all tests
sqlx migrate run               # Run migrations
```
