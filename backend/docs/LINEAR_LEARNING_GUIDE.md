# BuildScale.ai Backend: Linear Learning Guide

**A comprehensive guide from first principles to implementation details**

---

## Table of Contents

1. [Part 1: First Principles - The Core Vision](#part-1-first-principles---the-core-vision)
2. [Part 2: System Architecture - The Big Picture](#part-2-system-architecture---the-big-picture)
3. [Part 3: Application Startup & State](#part-3-application-startup--state)
4. [Part 4: Authentication & Security Deep Dive](#part-4-authentication--security-deep-dive)
5. [Part 5: File System Architecture](#part-5-file-system-architecture)
6. [Part 6: AI System Architecture](#part-6-ai-system-architecture)
7. [Part 7: Plan Mode & Build Mode Workflow](#part-7-plan-mode--build-mode-workflow)
8. [Part 8: Tool System Architecture](#part-8-tool-system-architecture)
9. [Part 9: Memory System](#part-9-memory-system)
10. [Part 10: Error Handling System](#part-10-error-handling-system)
11. [Part 11: Background Workers](#part-11-background-workers)
12. [Part 12: Configuration System](#part-12-configuration-system)
13. [Part 13: Key Implementation Files Reference](#part-13-key-implementation-files-reference)
14. [Part 14: Quick Reference & Big Picture Summary](#part-14-quick-reference--big-picture-summary)

---

## Part 1: First Principles - The Core Vision

### What is BuildScale.ai?

BuildScale.ai is a **Distributed Operating System for AI**. It transforms a standard file system into an agentic computing environment where AI agents can live, plan, and execute within a structured workspace.

**The Core Problem It Solves:**

Traditional AI chatbots are limited because:
- They have no persistent memory beyond the current conversation
- They can't explore a codebase before making changes
- Every conversation starts from scratch
- There's no separation between planning and execution
- Tool access is all-or-nothing with no context

**BuildScale's Solution:**
- **Stateful Context**: Every workspace maintains persistent AI state
- **Explore-Then-Build**: AI explores the project before modifying files
- **Plan-Then-Execute**: Structured planning phase with user approval
- **Everything is a File**: Universal interface for all workspace resources

### The "Everything is a File" Philosophy

This concept comes from Unix: *everything* in the system can be accessed through file-like operations.

**Why This Matters:**
- **Universal Interface**: One API for all resources (documents, chats, agents, skills, plans)
- **Composability**: Tools can be combined in powerful ways
- **Simplicity**: Learn one pattern, apply everywhere

**Examples:**
- `/system/skills/email/SKILL.md` - A tool definition
- `/system/agents/researcher/AGENT.md` - An agent persona
- `/chats/chat-123.chat` - A conversation with YAML frontmatter
- `/plans/project-alpha.plan` - An implementation plan
- `/memories/user-preferences.md` - Persistent AI memory

### The Standardized Folder Taxonomy

Every workspace shares this consistent root structure:

```
/                          (Root - Container for entire workspace)
├── system/                (The "Toolbox" - Capabilities)
│   ├── skills/            (Skill definitions)
│   └── agents/            (Agent persona definitions)
├── chats/                 (The "Memory" - Conversation logs)
├── data/                  (The "Knowledge" - Raw documents)
├── users/                 (The "Home" - User-specific state)
├── projects/              (The "Work" - Codebases and files)
├── memories/              (Global memories shared across workspace)
└── plans/                 (Implementation plans)
```

**Purpose of Each Directory:**

| Path | Purpose | Examples |
|------|---------|----------|
| `/system/skills/<name>/SKILL.md` | Tool/capability definitions | Email, Git, Docker |
| `/system/agents/<name>/AGENT.md` | Agent personas | Planner, Builder, Coder |
| `/chats/chat-{id}.chat` | Conversation logs | All AI interactions |
| `/data/` | Ingested documents | PDFs, videos, CSVs |
| `/users/<user_id>/` | User workspace state | User-specific files |
| `/projects/<name>/` | Active projects | Source code, configs |
| `/memories/` | Global shared memories | Project decisions, preferences |
| `/plans/` | Implementation plans | Structured workflows |

### Multi-Tenancy: Workspaces as Isolation Units

**Workspace = Tenant**

Each workspace is:
- **Completely isolated**: Data never leaks between workspaces
- **Self-contained**: Has its own files, users, roles, AI sessions
- **Role-governed**: Four-tier RBAC system (Admin > Editor > Member > Viewer)

**Why This Matters:**
- Single backend can serve multiple teams/companies
- Users can belong to multiple workspaces with different roles
- Clean data separation for security and compliance

---

## Part 2: System Architecture - The Big Picture

### Technology Stack

| Component | Technology | Why It Was Chosen |
|-----------|-----------|-------------------|
| **Language** | Rust | Performance, memory safety, concurrency |
| **Web Framework** | Axum 0.8 | Async, type-safe, Tower middleware |
| **Database** | PostgreSQL 13+ | Relational integrity, JSON support |
| **Database Driver** | SQLx | Compile-time checked queries, async |
| **AI Framework** | Rig.rs | Agent abstraction, tool integration |
| **Runtime** | Tokio | Async runtime, extensive ecosystem |
| **Serialization** | Serde | Compile-time type-safe JSON |
| **Password Hashing** | Argon2 | Memory-hard, resistant to GPU attacks |

### The Layered Architecture Pattern

```
┌─────────────────────────────────────────────────────────┐
│                     Handlers                             │
│  (HTTP API endpoints - thin orchestration layer)         │
│  Responsibility: Extract → Validate → Call → Return      │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                     Services                             │
│  (Business logic, validation, authorization)             │
│  Responsibility: ALL business logic, reusable            │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                     Queries                              │
│  (Type-safe database operations, CRUD)                   │
│  Responsibility: SQL execution, transactions             │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                     Models                               │
│  (Data structures, validation rules, types)              │
│  Responsibility: Type definitions, validation            │
└─────────────────────────────────────────────────────────┘
```

### Layer Responsibilities

**Models Layer** (`models/`)
```rust
// Data structures with Serde serialization
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
}

// Validation rules
impl User {
    pub fn validate(&self) -> Result<()> {
        validate_email(&self.email)?;
        Ok(())
    }
}
```

**Queries Layer** (`queries/`)
```rust
// Type-safe database operations
pub async fn get_user_by_email(
    conn: &mut DbConn,
    email: &str,
) -> Result<Option<User>> {
    sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE email = $1",
        email
    )
    .fetch_optional(conn)
    .await
    .map_err(Error::from)
}
```

**Services Layer** (`services/`)
```rust
// Business logic and authorization
pub async fn authenticate_user(
    conn: &mut DbConn,
    email: &str,
    password: &str,
) -> Result<LoginResult> {
    validate_email(email)?;
    let user = get_user_by_email(conn, email).await?
        .ok_or(Error::Authentication("Invalid credentials".into()))?;
    verify_password(password, &user.password_hash)?;
    // ... create session, return tokens
}
```

**Handlers Layer** (`handlers/`)
```rust
// Thin orchestration: 3-5 lines maximum
pub async fn login(
    DbConnection(mut conn): DbConnection,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>> {
    let result = authenticate_user(&mut conn, &req.email, &req.password).await?;
    Ok(Json(result.into()))
}
```

### Data Flow Through Layers

**Request Processing Flow:**

```
HTTP Request (POST /login)
       │
       ▼
┌─────────────────┐
│ Handler Layer   │  1. Extract JSON body
│                 │  2. Call service
│ login()         │  3. Return JSON response
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Service Layer   │  1. Validate input
│                 │  2. Check password
│ authenticate()  │  3. Create session
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Query Layer     │  1. Execute SQL
│                 │  2. Parse results
│ get_user()      │  3. Return User
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Database        │  PostgreSQL
└─────────────────┘
```

### Module Organization Overview

```
src/
├── lib.rs           # Public exports and configuration
├── main.rs          # Entry point
├── config.rs        # Environment configuration
├── database.rs      # Connection pooling
├── error.rs         # Error types
├── validation.rs    # Input validation
├── state.rs         # Application state
│
├── agent/           # AI personas and sessions
├── chat/            # Conversations and streaming
├── tools/           # 27+ tools for AI
├── fs/              # File system core
├── workers/         # Background jobs
└── middleware/      # HTTP middleware
```

---

## Part 3: Application Startup & State

### Entry Point Walkthrough (`main.rs`)

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize tracing (logging)
    init_tracing();

    // 2. Load configuration from environment
    let config = load_config()?;

    // 3. Connect to database
    let pool = DbPool::connect(config.database.connection_string().expose_secret())
        .await?;

    // 4. Run migrations if present
    sqlx::migrate::Migrator::new("migrations")?
        .run(&pool).await?;

    // 5. Initialize cache
    let cache = Cache::new_local(CacheConfig {
        cleanup_interval_seconds: 60,
        default_ttl_seconds: Some(3600),
    });

    // 6. Initialize AI service (Rig.rs)
    let rig_service = Arc::new(RigService::from_config(&config.ai)?);

    // 7. Start API server (blocking)
    run_api_server(&config, cache, rig_service).await?;

    Ok(())
}
```

### Startup Sequence

```
1. Tracing (Logging)
   ├─ Read RUST_LOG environment variable
   ├─ Set up tracing subscriber
   └─ Configure log levels per module

2. Configuration
   ├─ Load .env file if present
   ├─ Parse BUILDSCALE__* environment variables
   └─ Validate JWT secrets (min 32 chars)

3. Database
   ├─ Create connection pool
   ├─ Run migrations if folder exists
   └─ Close migration pool

4. Cache
   ├─ Create in-memory cache
   └─ Spawn cleanup worker

5. AI Service
   ├─ Initialize providers (OpenAI, OpenRouter)
   └─ Create Rig service

6. API Server
   ├─ Create routes
   ├─ Add middleware
   ├─ Bind to address
   └─ Start serving
```

### Application State (`state.rs`)

The `AppState` struct holds shared resources accessed by all handlers:

```rust
#[derive(Clone)]
pub struct AppState {
    /// General cache (tokens, sessions)
    pub cache: Cache<String>,

    /// User cache (authenticated user data)
    pub user_cache: Cache<User>,

    /// Database connection pool
    pub pool: DbPool,

    /// Active AI agent registry
    pub agents: Arc<AgentRegistry>,

    /// Rig AI service
    pub rig_service: Arc<RigService>,

    /// File storage service (Disk I/O)
    pub storage: Arc<FileStorageService>,

    /// Application configuration
    pub config: Config,

    /// Worker communication channels
    pub archive_cleanup_tx: mpsc::UnboundedSender<ArchiveCleanupMessage>,
    pub tag_index_tx: mpsc::UnboundedSender<TagIndexMessage>,
    pub link_index_tx: mpsc::UnboundedSender<LinkIndexMessage>,
}
```

**Why These Are Shared:**
- `cache` / `user_cache`: Fast access without database hits
- `pool`: Expensive to create, reusable connections
- `agents`: Track active AI sessions across requests
- `rig_service`: AI providers are singleton resources
- `storage`: File I/O service needs config
- `channels`: Coordinate background workers

### Router Setup and API Structure

```rust
pub fn create_api_router(state: AppState) -> Router<AppState> {
    Router::new()
        // Public endpoints
        .route("/health", get(health_check))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))

        // Protected endpoints
        .route("/auth/me", get(me))
        .route("/providers", get(get_providers))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            jwt_auth_middleware,
        ))

        // Workspace-scoped endpoints
        .nest("/workspaces", create_workspace_router(state))
}
```

**Route Organization:**
- `/api/v1/health` - Public health check
- `/api/v1/auth/*` - Authentication (public)
- `/api/v1/auth/me` - Get current user (protected)
- `/api/v1/workspaces` - Workspace operations (protected)

### Middleware Layers

```
HTTP Request
     │
     ▼
┌─────────────────────────────┐
│ Request ID Middleware       │  Add x-request-id header
└────────┬────────────────────┘
         │
         ▼
┌─────────────────────────────┐
│ Trace Layer                 │  Log request/response
└────────┬────────────────────┘
         │
         ▼
┌─────────────────────────────┐
│ CORS Layer                  │  Handle cross-origin
└────────┬────────────────────┘
         │
         ▼
┌─────────────────────────────┐
│ Compression Layer           │  Gzip response
└────────┬────────────────────┘
         │
         ▼
┌─────────────────────────────┐
│ JWT Auth Middleware         │  Verify access token
└────────┬────────────────────┘
         │
         ▼
┌─────────────────────────────┐
│ Workspace Access Middleware │  Check membership
└────────┬────────────────────┘
         │
         ▼
   Route Handler
```

### Frontend Serving

**Admin Frontend** (`/admin`):
```rust
let admin_static_service = ServeDir::new(&config.server.admin_build_path)
    .not_found_service(ServeFile::new(admin_index_path));
app = app.nest_service("/admin", admin_static_service);
```

**Web Frontend** (`/`):
```rust
let web_static_service = ServeDir::new(&config.server.web_build_path)
    .not_found_service(ServeFile::new(web_index_path));
app = app.fallback_service(web_static_service);
```

---

## Part 4: Authentication & Security Deep Dive

### Dual-Token System

BuildScale uses **two complementary tokens** for security and usability:

| Token Type | Lifetime | Storage | Purpose |
|------------|----------|---------|---------|
| **JWT Access Token** | 15 minutes | Memory/Authorization header | API requests |
| **Session Refresh Token** | 30 days | Cookie/Database | Get new access tokens |

**Why Two Tokens?**
- **Security**: Short-lived access tokens limit damage if stolen
- **Usability**: Long-lived refresh tokens = fewer logins
- **Revocation**: Refresh tokens can be revoked on logout

### Token Lifetimes and Why

**JWT Access Token (15 minutes):**
- Short window for misuse if stolen
- User doesn't notice 15-minute re-auth
- Forces regular re-validation
- Stateless = fast verification (no DB lookup)

**Session Refresh Token (30 days):**
- Users stay logged in for reasonable time
- Can be revoked (unlike JWT)
- Database-backed = control
- HMAC signature prevents tampering

### Login Flow Step-by-Step

```rust
// 1. User sends credentials
POST /auth/login
{
  "email": "user@example.com",
  "password": "SecurePass123!"
}

// 2. Handler validates
login() -> authenticate_user()

// 3. Service checks credentials
// a. Find user by email (case-insensitive)
// b. Verify password with Argon2
let user = get_user_by_email(conn, email).await?;
verify_password(password, &user.password_hash)?;

// 4. Generate JWT access token (15 min)
let access_token = jwt::encode(
    &Header::default(),
    &Claims::new(user.id, expires_in_15_min),
    &EncodingKey::from_secret(jwt_secret)
)?;

// 5. Generate session refresh token (30 days)
let refresh_token = generate_session_token(); // 256-bit random
let signature = hmac_sign(&refresh_token, secret);
let stored_token = format!("{}:{}", refresh_token, signature);

// 6. Store in database
INSERT INTO user_sessions (user_id, token, expires_at)
VALUES ($1, $2, NOW() + INTERVAL '30 days')

// 7. Return both tokens
{
  "access_token": "eyJhbGc...",
  "refresh_token": "a1b2c3...",
  "expires_at": "2025-01-15T10:45:00Z",
  "user": {...}
}
```

### Registration Flow with Validation

```rust
POST /auth/register
{
  "email": "user@example.com",
  "password": "SecurePass123!",
  "full_name": "John Doe"
}

// Validation steps:
// 1. Email format (RFC 5321)
validate_email(&email)?;
// 2. Password strength (min 12 chars, 2 of 4 types)
validate_password(&password)?;
// 3. Full name (optional, max 100 chars)
validate_full_name(&full_name)?;

// 4. Hash password with Argon2
let password_hash = argon2::hash_password(password.as_bytes(), &salt)?;

// 5. Create user
INSERT INTO users (email, password_hash, full_name)
VALUES ($1, $2, $3)

// 6. Return created user
201 Created
{
  "id": "uuid-v7",
  "email": "user@example.com",
  "created_at": "2025-01-15T10:00:00Z"
}
```

### Password Hashing (Argon2)

```rust
// Argon2 config (memory-hard, resistant to GPU attacks)
use argon2::{
    Algorithm::Argon2id,
    Version::Version13,
    Params::{new, m_cost, t_cost, p_cost}
};

let params = Params::new(65536, 2, 1)?; // 64MB, 2 passes, 1 thread
let argon2 = Argon2::new(Algorithm::Argon2id, Version::Version13, params);

// Hash with unique salt per user
let salt = SaltString::generate(&mut OsRng);
let password_hash = argon2.hash_password(password.as_bytes(), &salt)?;

// Verify (constant-time comparison)
argon2.verify_password(password.as_bytes(), &hash).is_ok()
```

**Why Argon2:**
- Memory-hard = resistant to GPU/ASIC attacks
- Winner of Password Hashing Competition 2015
- Unique salt per user = rainbow table resistant
- Configurable cost parameters

### Token Refresh Mechanism

```rust
// When access token expires:
POST /auth/refresh
{
  "refresh_token": "a1b2c3..."
}

// 1. Validate refresh token
// a. Find session in database
// b. Check expiration
let session = get_session_by_token(conn, &token).await?;
if session.expires_at < now() {
    return Err(Error::SessionExpired("Token expired"));
}

// 2. Verify HMAC signature (tamper detection)
let (token_part, signature_part) = token.split(':').collect();
let expected_sig = hmac_sign(token_part, &secret);
if !constant_time_compare(signature_part, expected_sig) {
    return Err(Error::TokenTheftDetected("Token tampered"));
}

// 3. Generate new access token
let new_access_token = jwt::encode(...)?;

// 4. Rotate refresh token (optional)
let new_refresh_token = generate_session_token();
update_session_token(conn, session.id, &new_refresh_token).await?;

// 5. Return new tokens
{
  "access_token": "eyJhbGc...",
  "refresh_token": "d4e5f6...",  // New if rotated
  "expires_at": "2025-01-15T11:00:00Z"
}
```

### Cookie-Based Auth Option

For browser clients, tokens can be stored in cookies:

```rust
// Cookie configuration
pub struct CookieConfig {
    pub http_only: true,      // Prevent XSS access
    pub secure: true,         // HTTPS only (production)
    pub same_site: SameSite::Lax,  // CSRF protection
    pub path: "/",
    pub domain: None,
}

// Login sets cookies
Set-Cookie: access_token=eyJhbGc...; HttpOnly; Secure; SameSite=Lax; Path=/
Set-Cookie: refresh_token=a1b2c3...; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=2592000
```

**Priority Order** (for token extraction):
1. Authorization header (API/mobile)
2. Cookie fallback (browser)

### RBAC System (4-Tier Roles)

```rust
// Role hierarchy: Admin > Editor > Member > Viewer
pub const ADMIN_ROLE: &str = "admin";
pub const EDITOR_ROLE: &str = "editor";
pub const MEMBER_ROLE: &str = "member";
pub const VIEWER_ROLE: &str = "viewer";

// Permission check
pub fn role_has_permission(role: &str, permission: &str) -> bool {
    match role {
        "admin" => true,  // All permissions
        "editor" => matches!(permission,
            "workspace:read" | "workspace:write" |
            "content:create" | "content:update_all" | ...
        ),
        "member" => matches!(permission,
            "workspace:read" | "content:create" | ...
        ),
        "viewer" => matches!(permission,
            "workspace:read" | "content:read_own" | ...
        ),
        _ => false,
    }
}
```

**Permission Matrix:**

| Permission | Admin | Editor | Member | Viewer |
|------------|-------|--------|--------|--------|
| `workspace:read` | ✓ | ✓ | ✓ | ✓ |
| `workspace:write` | ✓ | ✓ | ✗ | ✗ |
| `workspace:delete` | ✓ | ✗ | ✗ | ✗ |
| `workspace:manage_members` | ✓ | ✗ | ✗ | ✗ |
| `content:create` | ✓ | ✓ | ✓ | ✗ |
| `content:update_all` | ✓ | ✓ | ✗ | ✗ |
| `members:add` | ✓ | ✗ | ✗ | ✗ |

### Workspace Invitations System

```rust
// 1. Create invitation with role
POST /workspaces/{id}/members/invite
{
  "emails": ["user@example.com"],
  "role_name": "editor",
  "expires_in_hours": 168  // 7 days
}

// 2. Generate UUID v7 token
let invitation_token = Uuid::now_v7();

// 3. Store in database
INSERT INTO workspace_invitations
(token, workspace_id, email, role_name, status, expires_at)
VALUES ($1, $2, $3, $4, 'pending', NOW() + INTERVAL '7 days')

// 4. Send email with link
https://app.buildscale.ai/invite/{invitation_token}

// 5. User accepts
POST /workspaces/invitations/accept
{
  "token": "...",
  "user_id": "..."
}

// 6. Create membership
INSERT INTO workspace_members (workspace_id, user_id, role_name)
VALUES ($1, $2, $3)
```

---

## Part 5: File System Architecture

### Hybrid Storage Model

BuildScale uses a **hybrid approach** combining database and disk:

```
┌─────────────────────────────────────────────────────┐
│                   Application Layer                  │
└─────────────────────────────────────────────────────┘
                       │
         ┌─────────────┴─────────────┐
         ▼                           ▼
┌─────────────────┐         ┌─────────────────┐
│   PostgreSQL    │         │   Disk Storage  │
│   (Metadata)    │         │   (Content)     │
│                 │         │                 │
│ • files table   │         │ • latest/       │
│ • versions[]    │         │ • archive/      │
│ • tags index    │         │ • trash/        │
│ • links index   │         │                 │
└─────────────────┘         └─────────────────┘
```

**Why Hybrid?**

| Component | Database | Disk |
|-----------|----------|------|
| **Identity** | ✓ Fast lookups, queries | ✗ No |
| **History** | ✓ Easy version tracking | ✗ Complex |
| **Content** | ✗ Blobs slow DB | ✓ Fast file I/O |
| **Tooling** | ✗ Requires DB access | ✓ Git, grep, etc. |

**Benefits:**
- Database for metadata = fast queries, indexing, joins
- Disk for content = standard tools, caching, streaming
- SHA-256 hashing = deduplication, integrity checks

### Directory Structure

```
/app/storage/workspaces/{workspace_id}/
├── latest/       # Current files (Source of Truth)
│   └── projects/backend/src/main.rs
├── archive/      # Hash-based backup copies
│   └── e3/b0/e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
└── trash/        # Soft-deleted files
    └── projects/backend/main.rs
```

### The `files` Table Schema

```sql
CREATE TABLE files (
    -- Identity
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    workspace_id UUID NOT NULL,
    parent_id UUID REFERENCES files(id),  -- NULL = Root
    file_type TEXT NOT NULL,  -- document, folder, chat, plan, memory, etc.

    -- Naming
    name TEXT NOT NULL,       -- Display name (supports spaces, emojis)
    path TEXT NOT NULL,       -- Materialized path for fast tree queries

    -- Content
    hash TEXT,               -- SHA-256 of current content (NULL for folders)
    versions TEXT[],          -- Historical hashes (newest first)

    -- Lifecycle
    deleted_at TIMESTAMPTZ,   -- Soft delete (NULL = active)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    UNIQUE(workspace_id, path) WHERE deleted_at IS NULL
);
```

**Column Purposes:**

| Column | Purpose | Example |
|--------|---------|---------|
| `id` | Permanent identifier | `01234567-89ab-cdef-0123-456789abcdef` |
| `workspace_id` | Tenant isolation | Same file in different workspaces |
| `parent_id` | Tree structure | `NULL` = root folder |
| `file_type` | Polymorphic behavior | Different handling per type |
| `name` | Human-readable | "My Document.md" |
| `path` | Fast queries | `/projects/backend/src/main.rs` |
| `hash` | Content addressing | `e3b0c442...` |
| `versions[]` | History | `["old_hash", "older_hash"]` |
| `deleted_at` | Soft delete | `2025-01-15T10:00:00Z` or `NULL` |

### File Types

| Type | Description | Content Location | Special Handling |
|------|-------------|------------------|------------------|
| `document` | Regular files | `latest/{path}` | None |
| `folder` | Directory | No content | `hash = NULL` |
| `chat` | Conversations | `latest/chats/chat-{id}.chat` | YAML frontmatter sync |
| `plan` | Implementation plans | `latest/plans/{name}.plan` | YAML frontmatter |
| `memory` | Persistent AI context | `latest/users/{id}/memories/` | Searchable |
| `agent` | Agent personas | `latest/system/agents/{name}/AGENT.md` | Registry lookup |
| `skill` | Tool definitions | `latest/system/skills/{name}/SKILL.md` | Registry lookup |

### Write Flow with Versioning

```
User: Update file /projects/backend/src/main.rs

1. Calculate hash = SHA-256(new_content)
   hash = "a1b2c3d4e5f6..."

2. Check if file exists
   SELECT id, hash FROM files WHERE path = '/projects/backend/src/main.rs'

3. If exists:
   a. Archive current content
      copy latest/projects/backend/src/main.rs
        to archive/a1/b2/a1b2c3d4e5f6...
   b. Append old hash to versions array
      UPDATE files
      SET versions = [old_hash, ...versions[]]
      WHERE id = file_id

4. Write new content to latest
   write_file("latest/projects/backend/src/main.rs", new_content)

5. Update database
   UPDATE files
   SET hash = 'new_hash', updated_at = NOW()
   WHERE id = file_id

Result: New version is live, old version safely archived
```

### Restore Flow

```
User: Restore file to version from yesterday

1. Get file with versions
   SELECT id, name, path, hash, versions
   FROM files
   WHERE id = file_id

2. Select target version
   target_hash = versions[3]  # 4th most recent

3. Read content from archive
   content = read_file("archive/ta/rget_hash...")

4. Archive current content
   (same as write flow step 3)

5. Restore target content
   write_file("latest/projects/backend/src/main.rs", content)

6. Update database
   UPDATE files
   SET hash = target_hash,
       versions = [current_hash, ...versions[0..2], versions[4..]]
   WHERE id = file_id
```

### Soft Delete vs Hard Delete

**Soft Delete** (`deleted_at` is set):
```
UPDATE files SET deleted_at = NOW() WHERE id = file_id

Result:
- File moved to trash/ directory
- Still queryable with WHERE deleted_at IS NOT NULL
- Can be restored: UPDATE files SET deleted_at = NULL
```

**Hard Delete (Purge)**:
```
DELETE FROM files WHERE id = file_id

Result:
- Permanent, irreversible
- Database record deleted
- Disk files remain (orphaned, cleaned by worker)
```

### Knowledge Graph (Tags and Wikilinks)

**Tag Extraction:**
```rust
// Extract hashtags from markdown content
pub fn extract_tags(content: &str) -> Vec<String> {
    Regex::new(r"#([\w/\-]+)")
        .captures_iter(content)
        .map(|c| c[1].to_lowercase())
        .collect()
}

// "#This is #important for #project/alpha"
// → ["this", "important", "project/alpha"]
```

**Wikilink Extraction:**
```rust
// Extract [[wikilinks]] from markdown
pub fn extract_links(content: &str) -> Vec<String> {
    Regex::new(r"\[\[([^\]|]+)(?:\|[^\]]+)?\]\]")
        .captures_iter(content)
        .map(|c| c[1].to_string())
        .collect()
}

// "See [[Project Alpha]] for details"
// → ["Project Alpha"]

// "See [[meetings/standup|Daily Standup]]"
// → ["meetings/standup"]
```

**Backlink Lookup:**
```sql
-- Find all files that link TO a file
SELECT DISTINCT f.name, f.path
FROM files f
INNER JOIN links l ON l.source_file_id = f.id
WHERE l.workspace_id = $1
  AND l.target_name = 'project-alpha'
  AND f.deleted_at IS NULL;
```

### Common Access Patterns

**Folder Navigation:**
```sql
-- List everything in a folder
SELECT * FROM files
WHERE parent_id = 'folder-uuid'
  AND deleted_at IS NULL
ORDER BY (file_type = 'folder') DESC, name ASC;
```

**Hierarchy Lookup:**
```sql
-- Get all files in /projects and subfolders (O(log N))
SELECT * FROM files
WHERE path = '/projects' OR path LIKE '/projects/%'
  AND deleted_at IS NULL
ORDER BY path ASC;
```

**Find by Tag:**
```sql
-- Find all files tagged #important
SELECT f.*
FROM files f
INNER JOIN tags t ON t.file_id = f.id
WHERE t.tag = 'important'
  AND f.deleted_at IS NULL;
```

---

## Part 6: AI System Architecture

### The Agentic Engine Concept

**BuildScale's Philosophy:**
> The Backend owns the "hands" (Tools) and "memory" (File System). The LLM is just the "processor" (swappable).

**Traditional Chatbot vs BuildScale:**

| Aspect | Traditional Chatbot | BuildScale |
|--------|-------------------|------------|
| Memory | Conversation only | Persistent files |
| Tools | None or hardcoded | 27+ filesystem tools |
| State | Stateless | Stateful sessions |
| Context | Fixed window | Dynamic engineering |
| Planning | No separation | Plan → Build modes |

### AI Providers (OpenAI, OpenRouter)

```rust
// Provider configuration
pub struct ProviderConfig {
    pub openai: Option<OpenAIConfig>,
    pub openrouter: Option<OpenRouterConfig>,
    pub default_provider: String,  // "openai" or "openrouter"
    pub default_model: String,     // "openai:gpt-4o"
}

// Model identifier format
"openai:gpt-4o"              // Explicit provider
"openrouter:anthropic/claude-3.5-sonnet"
"gpt-4o"                     // Uses default provider
```

**Why Multiple Providers?**
- **Redundancy**: If one is down, switch
- **Cost Optimization**: Use cheaper models for simple tasks
- **Feature Access**: Not all models on all providers
- **Geographic Distribution**: Reduced latency

### Agent Personas (Assistant, Planner, Builder)

```rust
pub fn get_persona(
    role: Option<&str>,      // "planner", "builder", or None
    mode: Option<&str>,      // "plan", "build", or None
    plan_content: Option<&str>  // Required for builder
) -> String
```

| Persona | Purpose | System Prompt Focus | When Used |
|---------|---------|-------------------|-----------|
| **Assistant** | General Q&A | Helpful, conversational | Default chat mode |
| **Planner** | Strategic planning | Exploration, questioning | Plan mode |
| **Builder** | Code generation | Precision, efficiency | Build mode |

**Persona Selection Priority:**
1. Explicit role parameter
2. Mode-based selection (plan → planner, build → builder)
3. Default to assistant

### Agent Sessions (Database Tracking)

```sql
CREATE TABLE agent_sessions (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    chat_id UUID NOT NULL,
    agent_type TEXT NOT NULL,  -- "assistant", "planner", "builder"
    status TEXT NOT NULL,      -- "idle", "running", "paused", "completed", "error"
    current_task TEXT,         -- What the agent is working on
    heartbeat TIMESTAMPTZ,     -- Last sign of life
    created_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ
);
```

**Session Lifecycle:**
1. **Create**: When chat starts → `idle`
2. **Running**: Processing user message → `running`
3. **Heartbeat**: Every 30 seconds while running
4. **Complete**: Interaction done → `idle`
5. **Terminal**: Error/cancel → `completed`/`error`

**Heartbeat Mechanism:**
- Frequency: Every 30 seconds
- Threshold: 120 seconds = stale
- Cleanup: Automatic background worker

### ChatActor State Machine

```rust
pub enum ActorState {
    Created,    // Initial state
    Idle,       // Waiting for input
    Running,    // Processing interaction
    Paused,     // Temporarily paused
    Completed,  // Finished naturally (terminal)
    Cancelled,  // User cancelled (terminal)
    Error,      // Unrecoverable error (terminal)
}

pub enum ActorEvent {
    ProcessInteraction,  // Start processing
    Pause,               // Pause current work
    Cancel,              // Cancel session
    Ping,                // Reset timeout
    Shutdown,            // Graceful shutdown
}
```

**State Transitions:**

```
┌─────────┐  ProcessInteraction  ┌──────────┐
│ Created │ ──────────────────>  │  Idle    │
└─────────┘                     └──────────┘
                                      │
                    ProcessInteraction│
                                      ▼
                                ┌──────────┐
                                │ Running  │
                                └──────────┘
                                      │
              ┌───────────────────────┼───────────────────────┐
              │ Pause                 │ Complete/Cancel      │
              ▼                       ▼                       ▼
        ┌──────────┐           ┌─────────────┐       ┌─────────────┐
        │  Paused  │ ─Resume──> │    Idle     │       │  Completed  │ (terminal)
        └──────────┘           └─────────────┘       └─────────────┘
                                      │                        │
                                      │ Cancel                │
                                      ▼                       ▼
                                ┌─────────────┐       ┌─────────────┐
                                │  Cancelled  │       │    Error    │ (terminal)
                                └─────────────┘       └─────────────┘
```

**Terminal States:**
Once an agent enters a terminal state (`Completed`, `Cancelled`, `Error`):
- No further state transitions possible
- Actor shuts down automatically
- Session preserved in database for audit
- New interaction spawns fresh actor

### Context Engineering (BuiltContext)

```rust
pub struct BuiltContext {
    /// System persona/instructions
    pub persona: String,

    /// Conversation history
    pub history: HistoryManager,

    /// File attachments with priority pruning
    pub attachment_manager: AttachmentManager,
}

pub struct AttachmentValue {
    pub content: String,
    pub priority: i32,      // Higher = dropped first
    pub tokens: usize,      // Estimated count
    pub is_essential: bool, // Never prune
}
```

**Priority Constants:**

| Constant | Value | When Dropped |
|----------|-------|--------------|
| `PRIORITY_ESSENTIAL` | 0 | Never |
| `PRIORITY_HIGH` | 3 | Last |
| `PRIORITY_MEDIUM` | 5 | Default for user attachments |
| `PRIORITY_LOW` | 10 | First |

**Context Building Flow:**
1. Load messages from database
2. Extract persona (from agents or chat config)
3. Split history (exclude last message)
4. Hydrate attachments (fetch content, estimate tokens)
5. Optimize (prune by priority if over limit)
6. Sort (consistent rendering order)

### AttachmentManager and Priority System

```rust
impl AttachmentManager {
    pub fn add_fragment(&mut self, content: String, priority: i32, tokens: usize) {
        self.attachments.push(AttachmentValue {
            content, priority, tokens, is_essential: priority == 0
        });
    }

    pub fn optimize_for_limit(&mut self, max_tokens: usize) {
        // Sort by priority (descending), then by tokens (ascending)
        self.attachments.sort_by(|a, b| {
            b.priority.cmp(&a.priority)
                .then_with(|| a.tokens.cmp(&b.tokens))
        });

        // Remove lowest priority until under limit
        while self.total_tokens() > max_tokens && !self.attachments.is_empty() {
            if self.attachments[0].is_essential {
                break; // Never drop essential
            }
            self.attachments.remove(0);
        }
    }
}
```

### Context Caching for Cost Optimization

**Cache-Optimized Architecture:**

```
[System Prompt] → [History + Attachments (oldest first)] → [Last Message]
                      ↑                                      ↑
                   Stable prefix                           Varies only
                   (cacheable)
```

**How It Works:**
1. **Chronological Sorting**: All items by timestamp (oldest first)
2. **Cacheable Prefix**: Older items don't change between requests
3. **Efficient Conversion**: `ContextItem` wrapper for consistent format

**Tool Result Optimization:**
- Keep full outputs: Most recent 5 tool results
- Truncate older: 1KB with `…[re-run]` suffix
- Tool calls: Always preserved (critical)

### SSE Event Protocol

**Server-Sent Events** stream AI responses to clients:

```rust
pub enum SseEvent {
    Thought { text: String },                    // AI reasoning
    Call { tool: String, arguments: Value },     // Tool invocation
    Observation { output: String, success: bool }, // Tool result
    FileUpdated { path: String, version: i32 },   // UI refresh
    PlanStep { step: i32, status: String },       // Progress
    Done { message: String },                     // Completion
    Stopped { reason: String },                   // Cancellation
    StateChanged { from: String, to: String },    // State transition
}
```

**Event Format:**
```
data: {"type":"thought","data":{"text":"Let me explore..."}}

data: {"type":"call","data":{"tool":"read","path":"/main.rs"}}

data: {"type":"observation","data":{"output":"...","success":true}}

data: {"type":"done","data":{"message":"Complete!"}}
```

---

## Part 7: Plan Mode & Build Mode Workflow

### Why Separate Planning from Execution?

**The Problem:** AI often makes changes without full understanding

**The Solution:** Two distinct modes

| Mode | Purpose | Allowed Actions |
|------|---------|-----------------|
| **Plan Mode** | Intent & Strategy | Read-only + write to `/plans/*.plan` |
| **Build Mode** | Execution & Implementation | Full access to all tools |

**Benefits:**
- AI explores before modifying
- User approval before destructive actions
- Clear audit trail of decisions
- Better results with upfront planning

### Two Modes Comparison

```
┌─────────────────────────────────────────────────────────────┐
│ PLAN MODE                                                   │
│━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━│
│ • Persona: Planner                                          │
│ • Allowed Tools: ls, read, grep, glob, find, cat           │
│ • Allowed Writes: Only to /plans/*.plan files              │
│ • Goal: Explore project, create plan, get approval         │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ BUILD MODE                                                  │
│━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━│
│ • Persona: Builder                                          │
│ • Allowed Tools: All 27+ tools                             │
│ • Allowed Writes: Any file in workspace                    │
│ • Goal: Execute approved plan                               │
└─────────────────────────────────────────────────────────────┘
```

### Agent Persona Switching

```rust
// Plan mode entry
let persona = get_persona(
    Some("planner"),  // Use planner persona
    Some("plan"),     // Mode is plan
    None
);

// Build mode entry
let persona = get_persona(
    Some("builder"),  // Use builder persona
    Some("build"),    // Mode is build
    Some(plan_content)  // Inject approved plan
);
```

**Planner Persona:**
```
You are the Planner. Your role is to:
1. EXPLORE the project using ls, read, grep
2. IDENTIFY all dependencies and potential issues
3. CREATE a detailed plan at /plans/{name}.plan
4. ASK the user for approval using ask_user tool
5. CALL exit_plan_mode when approved
```

**Builder Persona:**
```
You are the Builder. Your role is to:
1. READ the approved plan injected below
2. EXECUTE changes precisely as specified
3. USE appropriate tools for each task
4. VERIFY changes were successful

## APPROVED EXECUTION PLAN
{plan_content}
```

### Workflow Lifecycle (6 Steps)

```
┌─────────────────────────────────────────────────────────────┐
│ 1. NEW CHAT                                                 │
│    ├─ User starts new conversation                         │
│    └─ System enters Plan Mode, planner persona active     │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. EXPLORE                                                  │
│    ├─ AI uses discovery tools (grep, read, ls)             │
│    ├─ Reads existing code to understand patterns           │
│    └─ Identifies files that need modification              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. DRAFT                                                    │
│    ├─ AI writes plan to /plans/project-name.plan          │
│    ├─ Plan includes: Overview, Steps, Files affected       │
│    └─ YAML frontmatter with status: draft                  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│ 4. APPROVE                                                  │
│    ├─ AI calls ask_user tool                               │
│    ├─ User sees "Accept & Build" button                    │
│    └─ User clicks to approve                               │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│ 5. TRANSITION                                                │
│    ├─ Backend calls exit_plan_mode tool                     │
│    ├─ Chat metadata: mode → "build"                        │
│    ├─ Persona switches: planner → builder                  │
│    └─ Plan content injected into builder's context         │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│ 6. EXECUTE                                                  │
│    ├─ Builder persona now active                           │
│    ├─ Plan content pinned in prompt                        │
│    ├─ AI uses all tools to implement changes               │
│    └─ Completion when all steps done                       │
└─────────────────────────────────────────────────────────────┘
```

### The `ask_user` Tool

**Purpose:** Suspend AI generation to request structured user input

**JSON Schema:**
```json
{
  "questions": [
    {
      "name": "environment",
      "question": "Which deployment environment?",
      "schema": {"type": "string", "enum": ["dev", "staging", "prod"]},
      "buttons": [
        {"label": "Development", "value": "dev"},
        {"label": "Staging", "value": "staging"},
        {"label": "Production", "value": "prod", "variant": "danger"}
      ]
    }
  ]
}
```

**SSE Event:**
```json
{
  "type": "question_pending",
  "data": {
    "question_id": "uuid",
    "questions": [...],
    "created_at": "2025-01-15T10:30:00Z"
  }
}
```

**Answer Submission:**
```json
POST /api/v1/workspaces/{id}/chats/{chat_id}/messages
{
  "content": "[Answered: staging]",
  "metadata": {
    "question_answer": {
      "question_id": "uuid",
      "answers": {"environment": "staging"}
    }
  }
}
```

### The `exit_plan_mode` Tool

**Purpose:** Transition from Plan to Build mode

**Arguments:**
```json
{
  "plan_file_path": "/plans/project-alpha.plan"
}
```

**Logic:**
```rust
// 1. Verify plan file exists
let plan = get_file_by_path(conn, workspace_id, &plan_file_path)?;

// 2. Update chat metadata
UPDATE chat_messages
SET app_data = jsonb_set(
    app_data,
    '{mode}',
    '"build"'
)
WHERE chat_id = $1

// 3. Update YAML frontmatter on disk
sync_chat_metadata_to_disk(chat_id).await?;

// 4. Emit SSE event
emit(SseEvent::ModeChanged {
    from: "plan".into(),
    to: "build".into()
})?;

// 5. Reload context with builder persona
// Persona switched automatically on next interaction
```

### Chat Metadata (`app_data`)

```sql
-- The app_data column tracks workflow state
{
  "mode": "plan",              -- "plan" or "build"
  "plan_file": "/plans/..."    -- Active plan file path
}
```

**YAML Frontmatter Sync:**

```markdown
---
mode: plan
plan_file: /plans/project-alpha.plan
updated_at: 2025-01-15T10:30:00Z
---

# Conversation starts here...
```

---

## Part 8: Tool System Architecture

### Tool Trait Definition

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the name of this tool
    fn name(&self) -> &'static str;

    /// Returns a description of what this tool does
    fn description(&self) -> &'static str;

    /// Returns the JSON schema definition for this tool's arguments
    fn definition(&self) -> Value;

    /// Executes the tool with given arguments
    async fn execute(
        &self,
        conn: &mut DbConn,
        storage: &FileStorageService,
        workspace_id: Uuid,
        user_id: Uuid,
        config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse>;
}
```

### ToolConfig Struct

```rust
pub struct ToolConfig {
    /// Whether the system is in Plan Mode
    pub plan_mode: bool,

    /// Path to the active plan file (Build Mode only)
    pub active_plan_path: Option<String>,

    /// Chat ID for tools that update metadata
    pub chat_id: Option<Uuid>,

    /// Channel to signal tag indexer worker
    pub tag_index_tx: Option<mpsc::UnboundedSender<TagIndexMessage>>,

    /// Channel to signal link indexer worker
    pub link_index_tx: Option<mpsc::UnboundedSender<LinkIndexMessage>>,
}
```

### Tool Categories and Their Purposes

| Category | Tools | Purpose |
|----------|-------|---------|
| **File System** | 14 tools | File manipulation (ls, read, write, edit, rm, mv, touch, mkdir, grep, glob, find, cat, file_info, read_multiple_files) |
| **Memory** | 5 tools | Persistent AI context (set, get, search, delete, list) |
| **Plan** | 6 tools | Plan mode workflow (write, read, edit, list, ask_user, exit_plan_mode) |
| **Web** | 2 tools | External data (fetch, search) |

### All 27+ Tools Listed

**File Tools (14):**
| Tool | Description | Plan Mode |
|------|-------------|-----------|
| `ls` | List directory contents | ✓ Allowed |
| `read` | Read file content | ✓ Allowed |
| `write` | Create/update file | ✗ Blocked (unless .plan) |
| `edit` | Search and replace in file | ✗ Blocked (unless .plan) |
| `rm` | Delete file | ✗ Blocked |
| `mv` | Move/rename file | ✗ Blocked |
| `touch` | Create empty file | ✗ Blocked |
| `mkdir` | Create directory | ✗ Blocked (unless /plans/) |
| `grep` | Search file contents | ✓ Allowed |
| `glob` | Find files by pattern | ✓ Allowed |
| `find` | Search file metadata | ✓ Allowed |
| `cat` | Concatenate files | ✓ Allowed |
| `file_info` | Get file metadata | ✓ Allowed |
| `read_multiple_files` | Batch read | ✓ Allowed |

**Memory Tools (5):**
| Tool | Description |
|------|-------------|
| `memory_set` | Create/update memory |
| `memory_get` | Retrieve specific memory |
| `memory_search` | Search memories |
| `memory_delete` | Delete memory |
| `memory_list` | List categories/tags |

**Plan Tools (6):**
| Tool | Description |
|------|-------------|
| `ask_user` | Request user input |
| `exit_plan_mode` | Transition to Build mode |
| `plan_write` | Create plan file |
| `plan_read` | Read plan file |
| `plan_edit` | Edit plan file |
| `plan_list` | List plans |

**Web Tools (2):**
| Tool | Description |
|------|-------------|
| `web_fetch` | Fetch URL content |
| `web_search` | Search the web |

### Tool Executor Pattern (Enum Dispatch)

```rust
pub enum ToolExecutor {
    // File tools
    Ls, Read, Write, Edit, Rm, Mv, Touch,
    Grep, Mkdir, Glob, FileInfo, ReadMultipleFiles, Find, Cat,

    // Plan tools
    AskUser, ExitPlanMode,
    PlanWrite, PlanRead, PlanEdit, PlanList,

    // Memory tools
    MemorySet, MemoryGet, MemorySearch, MemoryDelete, MemoryList,

    // Web tools
    WebFetch, WebSearch,
}

impl ToolExecutor {
    pub async fn execute(
        &self,
        conn: &mut DbConn,
        storage: &FileStorageService,
        workspace_id: Uuid,
        user_id: Uuid,
        config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse> {
        match self {
            ToolExecutor::Write => {
                // Plan mode check
                if config.plan_mode {
                    return Err(Error::Validation(
                        ValidationErrors::Single {
                            field: "plan_mode".into(),
                            message: PLAN_MODE_ERROR.into(),
                        }
                    ));
                }
                // Execute write
                file::WriteTool.execute(conn, storage, workspace_id, user_id, config, args).await
            }
            // ... other tools
        }
    }
}
```

### Plan Mode Enforcement Mechanism

```rust
// In WriteTool::execute()
pub const PLAN_MODE_ERROR: &str =
    "System is in Plan Mode. To switch to Build Mode:
    1) Use ask_user with Accept/Reject buttons to request approval
    2) When user clicks Accept, call exit_plan_mode with your plan file path";

async fn execute(...) -> Result<ToolResponse> {
    // Check plan mode
    if config.plan_mode {
        // Only allow writing to .plan files
        if !args["path"].as_str().unwrap_or("").ends_with(".plan") {
            return Err(Error::Validation(
                ValidationErrors::Single {
                    field: "plan_mode".into(),
                    message: PLAN_MODE_ERROR.into(),
                }
            ));
        }
    }

    // Proceed with write
    // ...
}
```

### Tool Definitions for AI Context (JSON Schema)

```rust
pub fn get_all_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "read".into(),
            description: "Read the content of a file at a given path".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The absolute path to the file to read"
                    }
                },
                "required": ["path"]
            })
        },
        // ... 26 more tools
    ]
}
```

**Sent to LLM as system context:**
```json
{
  "type": "function",
  "function": {
    "name": "read",
    "description": "Read the content of a file at a given path",
    "parameters": {
      "type": "object",
      "properties": {
        "path": {"type": "string", "description": "The absolute path"}
      },
      "required": ["path"]
    }
  }
}
```

---

## Part 9: Memory System

### What Are Memories?

**Memories** are persistent, structured AI context stored as Markdown files.

**Why Use Memories?**
- AI forgets beyond conversation context window
- Important decisions/preferences shouldn't be lost
- Cross-session continuity improves user experience

### Memory Scopes

| Scope | Path Pattern | Visibility | Use Case |
|-------|--------------|------------|----------|
| `user` | `/users/{user_id}/memories/{category}/{key}.md` | Private to user | Personal preferences |
| `global` | `/memories/{category}/{key}.md` | Shared across workspace | Project decisions |

**Example Paths:**
```
/users/abc-123/memories/preferences/coding-style.md
/users/abc-123/memories/corrections/no-console-log.md
/memories/project/api-endpoints.md
/memories/project/database-schema.md
```

### Memory File Format

```markdown
---
title: "Coding Style Preferences"
tags: ["preferences", "code-style"]
category: "preferences"
created_at: "2025-01-15T10:30:00Z"
updated_at: "2025-01-15T10:30:00Z"
scope: "user"
---

# Coding Style Preferences

- Use 4 spaces for indentation
- Prefer `const` over `let` when possible
- Add JSDoc comments to all functions
- Maximum line length: 100 characters
```

### The 5 Memory Tools

| Tool | Description | Example |
|------|-------------|---------|
| `memory_set` | Create/update memory | Save coding style preference |
| `memory_get` | Retrieve specific memory | Get API endpoints reference |
| `memory_search` | Search across memories | Find all decisions about auth |
| `memory_delete` | Delete memory (soft delete) | Remove outdated preference |
| `memory_list` | List categories/tags/memories | Show all memory categories |

**Example: `memory_set`**
```json
{
  "category": "preferences",
  "key": "coding-style",
  "title": "My Coding Style",
  "content": "# Coding Style\n\n- Use 4 spaces\n- Prefer const",
  "scope": "user"
}
```

### Recommended Categories

| Category | Purpose | Example Keys |
|----------|---------|--------------|
| `preferences` | User preferences | `coding-style`, `editor-config` |
| `project` | Project-specific | `api-endpoints`, `tech-stack` |
| `decisions` | Architecture decisions | `auth-strategy`, `database-choice` |
| `context` | Work context | `current-focus`, `team-info` |
| `corrections` | User corrections | `no-console-log`, `prefer-async` |
| `patterns` | Discovered patterns | `error-handling`, `naming` |
| `references` | Quick references | `git-commands`, `docker-cheatsheet` |

### Use Cases

**1. Persistent Preferences:**
```json
// User: "Remember I prefer 4-space indentation"
memory_set({
  "category": "preferences",
  "key": "indentation",
  "content": "Use 4 spaces for indentation, not 2"
})
```

**2. Project Context:**
```json
// AI discovers project uses PostgreSQL
memory_set({
  "category": "project",
  "key": "database",
  "content": "This project uses PostgreSQL 13+ with SQLx"
})
```

**3. Learning Corrections:**
```json
// User: "Stop using console.log, use logger instead"
memory_set({
  "category": "corrections",
  "key": "no-console-log",
  "content": "Never use console.log. Use the logger module instead."
})
```

---

## Part 10: Error Handling System

### Error Enum (All Variants)

```rust
#[derive(Debug, Error)]
pub enum Error {
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Validation error: {0}")]
    Validation(ValidationErrors),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Access forbidden: {0}")]
    Forbidden(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Session expired: {0}")]
    SessionExpired(String),

    #[error("Invalid session token: {0}")]
    InvalidToken(String),

    #[error("Token theft detected: {0}")]
    TokenTheftDetected(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("AI provider error: {0}")]
    AiProvider(String),

    #[error("Provider '{0}' not configured")]
    ProviderNotConfigured(String),

    #[error("Invalid model format: {0}")]
    InvalidModelFormat(String),

    #[error("Model '{0}' not supported by provider '{1}'")]
    ModelNotSupported(String, String),

    #[error("API key not configured for provider '{0}'")]
    ApiKeyMissing(String),

    #[error("Model '{0}' is disabled")]
    ModelDisabled(String),
}
```

### ValidationErrors Struct

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ValidationErrors {
    Single { field: String, message: String },
    Multiple { fields: HashMap<String, String> },
}
```

**Single Error:**
```json
{
  "error": "Validation failed",
  "code": "VALIDATION_ERROR",
  "fields": {
    "email": "Email cannot be empty"
  }
}
```

**Multiple Errors:**
```json
{
  "error": "Validation failed",
  "code": "VALIDATION_ERROR",
  "fields": {
    "email": "Invalid email format",
    "password": "Password too short",
    "workspace_name": "Cannot be empty"
  }
}
```

### HTTP Status Code Mapping

| Error Variant | Status Code | Error Code |
|---------------|-------------|------------|
| `Validation` | 400 | VALIDATION_ERROR |
| `NotFound` | 404 | NOT_FOUND |
| `Forbidden` | 403 | FORBIDDEN |
| `Conflict` | 409 | CONFLICT |
| `Authentication` | 401 | AUTHENTICATION_FAILED |
| `InvalidToken` | 401 | INVALID_TOKEN |
| `SessionExpired` | 401 | SESSION_EXPIRED |
| `TokenTheftDetected` | 403 | TOKEN_THEFT |
| `Sqlx` | 500 | INTERNAL_ERROR |
| `Internal` | 500 | INTERNAL_ERROR |
| `Config` | 500 | CONFIG_ERROR |
| `Cache` | 500 | CACHE_ERROR |
| `Llm` | 500 | LLM_ERROR |
| `AiProvider` | 500 | AI_PROVIDER_ERROR |
| `ModelDisabled` | 403 | MODEL_DISABLED |

### Error Code System

```rust
impl Error {
    fn error_code(&self) -> &'static str {
        match self {
            Error::Validation(_) => "VALIDATION_ERROR",
            Error::NotFound(_) => "NOT_FOUND",
            Error::Forbidden(_) => "FORBIDDEN",
            Error::Conflict(_) => "CONFLICT",
            Error::Authentication(_) => "AUTHENTICATION_FAILED",
            Error::InvalidToken(_) => "INVALID_TOKEN",
            Error::SessionExpired(_) => "SESSION_EXPIRED",
            Error::TokenTheftDetected(_) => "TOKEN_THEFT",
            Error::Sqlx(_) => "INTERNAL_ERROR",
            Error::Internal(_) => "INTERNAL_ERROR",
            Error::Config(_) => "CONFIG_ERROR",
            Error::Cache(_) => "CACHE_ERROR",
            Error::Llm(_) => "LLM_ERROR",
            Error::AiProvider(_) => "AI_PROVIDER_ERROR",
            Error::ProviderNotConfigured(_) => "PROVIDER_NOT_CONFIGURED",
            Error::InvalidModelFormat(_) => "INVALID_MODEL_FORMAT",
            Error::ModelNotSupported(_, _) => "MODEL_NOT_SUPPORTED",
            Error::ApiKeyMissing(_) => "API_KEY_MISSING",
            Error::ModelDisabled(_) => "MODEL_DISABLED",
            // ... other variants
        }
    }
}
```

### JSON Response Format

```rust
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (body, status) = match self {
            Error::Validation(errors) => {
                let body = match errors {
                    ValidationErrors::Single { field, message } => {
                        json!({
                            "error": "Validation failed",
                            "code": "VALIDATION_ERROR",
                            "fields": { field: message }
                        })
                    }
                    ValidationErrors::Multiple { fields } => {
                        json!({
                            "error": "Validation failed",
                            "code": "VALIDATION_ERROR",
                            "fields": fields
                        })
                    }
                };
                (body, StatusCode::BAD_REQUEST)
            }
            Error::NotFound(msg) => (
                json!({"error": msg, "code": "NOT_FOUND"}),
                StatusCode::NOT_FOUND
            ),
            // ... other cases
        };

        (status, Json(body)).into_response()
    }
}
```

### Error Logging Approach

```rust
fn log_error(error: &Error, error_code: &str, status_code: u16) {
    if status_code >= 500 {
        // Server errors = error level
        tracing::error!(
            error_code,
            error = %error,
            status_code,
            "Error returned to client"
        );
    } else {
        // Client errors = warning level
        tracing::warn!(
            error_code,
            error = %error,
            status_code,
            "Error returned to client"
        );
    }
}
```

**Why Different Levels?**
- **5xx Errors**: Server fault, needs investigation
- **4xx Errors**: Client fault, expected and acceptable

### IntoResponse Implementation

All errors automatically convert to HTTP responses:

```rust
// In handlers, just return the error
pub async fn get_user(Path(id): Path<Uuid>) -> Result<Json<User>> {
    let user = get_user_by_id(&mut conn, id).await?;
    Ok(Json(user))  // Error automatically converts to 404 response
}
```

---

## Part 11: Background Workers

### Why Background Workers?

**Problems They Solve:**
- **Cleanup**: Remove expired sessions/archives without blocking requests
- **Indexing**: Update knowledge graph asynchronously
- **Maintenance**: Periodic tasks (health checks, compaction)

**Benefits:**
- Non-blocking for user requests
- Consistent execution intervals
- Graceful shutdown support

### Worker Types and Responsibilities

| Worker | Responsibility | Interval |
|--------|---------------|----------|
| **Token Cleanup** | Remove expired session tokens | 24 hours |
| **Archive Cleanup** | Remove unused archive blobs | 1 hour |
| **Tag Indexer** | Extract tags from markdown files | Event-driven |
| **Link Indexer** | Extract wikilinks from markdown | Event-driven |

### Channel-Based Communication

```rust
// In state.rs
pub struct AppState {
    /// Channel to notify archive cleanup worker
    pub archive_cleanup_tx: mpsc::UnboundedSender<ArchiveCleanupMessage>,

    /// Channel to notify tag indexer worker
    pub tag_index_tx: mpsc::UnboundedSender<TagIndexMessage>,

    /// Channel to notify link indexer worker
    pub link_index_tx: mpsc::UnboundedSender<LinkIndexMessage>,
}
```

**Usage:**
```rust
// When file is modified
archive_cleanup_tx.send(ArchiveCleanupMessage {
    workspace_id,
    hashes: vec![old_hash],
})?;

tag_index_tx.send(TagIndexMessage {
    workspace_id,
    file_id,
})?;
```

### Archive Cleanup Worker

```rust
pub async fn archive_cleanup_worker(
    pool: DbPool,
    mut shutdown: broadcast::Receiver<()>,
    mut rx: mpsc::UnboundedReceiver<ArchiveCleanupMessage>,
    config: StorageWorkerConfig,
    storage_config: StorageConfig,
) {
    let mut interval = tokio::time::interval(
        Duration::from_secs(config.cleanup_interval_seconds)
    );

    loop {
        select! {
            // Periodic cleanup
            _ = interval.tick() => {
                cleanup_unused_blobs(&pool, &storage_config).await;
            }

            // Immediate cleanup request
            Some(msg) = rx.recv() => {
                cleanup_specific_blobs(&pool, &storage_config, msg).await;
            }

            // Shutdown signal
            _ = shutdown.recv() => {
                tracing::info!("Archive cleanup worker shutting down");
                return;
            }
        }
    }
}
```

### Tag Indexer Worker

```rust
pub async fn tag_indexer_worker(
    pool: DbPool,
    mut shutdown: broadcast::Receiver<()>,
    mut rx: mpsc::UnboundedReceiver<TagIndexMessage>,
    storage_config: StorageConfig,
) {
    loop {
        select! {
            Some(msg) = rx.recv() => {
                // Read file content
                let content = read_file_content(&storage_config, &msg.file_id).await?;

                // Extract tags
                let tags = extract_tags(&content);

                // Update database
                update_file_tags(&pool, msg.file_id, &tags).await?;
            }

            _ = shutdown.recv() => {
                tracing::info!("Tag indexer worker shutting down");
                return;
            }
        }
    }
}
```

### Link Indexer Worker

```rust
pub async fn link_indexer_worker(
    pool: DbPool,
    mut shutdown: broadcast::Receiver<()>,
    mut rx: mpsc::UnboundedReceiver<LinkIndexMessage>,
    storage_config: StorageConfig,
) {
    loop {
        select! {
            Some(msg) = rx.recv() => {
                // Read file content
                let content = read_file_content(&storage_config, &msg.file_id).await?;

                // Extract wikilinks
                let links = extract_links(&content);

                // Update database
                update_file_links(&pool, msg.file_id, &links).await?;
            }

            _ = shutdown.recv() => {
                tracing::info!("Link indexer worker shutting down");
                return;
            }
        }
    }
}
```

### Token Cleanup Worker

```rust
pub async fn revoked_token_cleanup_worker(
    pool: DbPool,
    mut shutdown: broadcast::Receiver<()>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(86400)); // 24 hours

    loop {
        select! {
            _ = interval.tick() => {
                let mut conn = pool.acquire().await?;
                let deleted = cleanup_expired_sessions(&mut conn).await?;
                tracing::info!("Cleaned up {} expired sessions", deleted);
            }

            _ = shutdown.recv() => {
                tracing::info!("Token cleanup worker shutting down");
                return;
            }
        }
    }
}
```

### Graceful Shutdown Handling

```rust
// In main.rs
let (cleanup_shutdown_tx, _) = tokio::sync::broadcast::channel(1);

// Subscribe each worker
let shutdown_auth = cleanup_shutdown_tx.subscribe();
let shutdown_storage = cleanup_shutdown_tx.subscribe();
let shutdown_tags = cleanup_shutdown_tx.subscribe();
let shutdown_links = cleanup_shutdown_tx.subscribe();

// Spawn workers
tokio::spawn(async move {
    revoked_token_cleanup_worker(pool_auth, shutdown_auth).await;
});

// ...

// On shutdown signal
let shutdown_signal = async {
    tokio::signal::ctrl_c().await.expect("failed to install CTRL+C handler");
    tracing::info!("Shutdown signal received");
    cleanup_shutdown_tx.send(()).ok();
};

axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal)
    .await?;
```

---

## Part 12: Configuration System

### Environment Variable Naming Convention

**Pattern:** `BUILDSCALE__<SECTION>__<KEY>`

Examples:
```bash
BUILDSCALE__DATABASE__HOST=localhost
BUILDSCALE__DATABASE__PORT=5432
BUILDSCALE__JWT__SECRET=your-secret-key
BUILDSCALE__AI__PROVIDERS__OPENAI__API_KEY=sk-...
```

**Rules:**
- Prefix: `BUILDSCALE`
- Separator: Double underscore `__`
- Case: Uppercase
- Nesting: Reflects config struct hierarchy

### All Configuration Sections

#### Database Configuration

```rust
pub struct DatabaseConfig {
    pub user: String,
    pub password: SecretString,
    pub host: String,
    pub port: u16,
    pub database: String,
}
```

**Environment Variables:**
```bash
BUILDSCALE__DATABASE__USER=buildscale
BUILDSCALE__DATABASE__PASSWORD=secure_password
BUILDSCALE__DATABASE__HOST=localhost
BUILDSCALE__DATABASE__PORT=5432
BUILDSCALE__DATABASE__DATABASE=buildscale
```

**Defaults:**
| Key | Default |
|-----|---------|
| `user` | `"postgres"` |
| `password` | `"password"` |
| `host` | `"localhost"` |
| `port` | `5432` |
| `database` | `"postgres"` |

#### JWT Configuration

```rust
pub struct JwtConfig {
    pub secret: SecretString,                    // Min 32 chars
    pub access_token_expiration_minutes: i64,    // Default: 15
    pub refresh_token_secret: SecretString,      // Min 32 chars
}
```

**Environment Variables:**
```bash
BUILDSCALE__JWT__SECRET=at-least-32-chars-random
BUILDSCALE__JWT__ACCESS_TOKEN_EXPIRATION_MINUTES=15
BUILDSCALE__JWT__REFRESH_TOKEN_SECRET=another-32-chars-random
```

**Validation Rules:**
- Both secrets: Minimum 32 characters
- Rejected patterns: `change-this`, `secret`, `password`, `123456`, `example`

#### Sessions Configuration

```rust
pub struct SessionsConfig {
    pub expiration_hours: i64,                      // Default: 720 (30 days)
    pub revoked_token_retention_minutes: i64,       // Default: 1440 (1 day)
}
```

**Environment Variables:**
```bash
BUILDSCALE__SESSIONS__EXPIRATION_HOURS=720
BUILDSCALE__SESSIONS__REVOKED_TOKEN_RETENTION_MINUTES=1440
```

#### AI/Providers Configuration

```rust
pub struct AiConfig {
    pub chunk_window_size: usize,          // Default: 1000
    pub chunk_overlap: usize,              // Default: 200
    pub embedding_dimension: usize,        // Default: 1536
    pub default_persona: String,
    pub default_context_token_limit: usize,// Default: 128000
    pub actor_inactivity_timeout_seconds: u64, // Default: 600
    pub providers: ProviderConfig,
}

pub struct ProviderConfig {
    pub openai: Option<OpenAIConfig>,
    pub openrouter: Option<OpenRouterConfig>,
    pub default_provider: String,          // "openai"
    pub default_model: String,             // "openai:gpt-5-mini"
}

pub struct OpenAIConfig {
    pub api_key: SecretString,
    pub base_url: Option<String>,
    pub enable_reasoning_summaries: bool,  // Default: false
    pub reasoning_effort: String,         // "low", "medium", "high"
}
```

**Environment Variables:**
```bash
# Default provider
BUILDSCALE__AI__PROVIDERS__DEFAULT_PROVIDER=openai
BUILDSCALE__AI__PROVIDERS__DEFAULT_MODEL=openai:gpt-5-mini

# OpenAI
BUILDSCALE__AI__PROVIDERS__OPENAI__API_KEY=sk-...
BUILDSCALE__AI__PROVIDERS__OPENAI__ENABLE_REASONING_SUMMARIES=false
BUILDSCALE__AI__PROVIDERS__OPENAI__REASONING_EFFORT=low

# OpenRouter
BUILDSCALE__AI__PROVIDERS__OPENROUTER__API_KEY=sk-or-...

# Actor timeout
BUILDSCALE__AI__ACTOR_INACTIVITY_TIMEOUT_SECONDS=600
```

#### Storage Configuration

```rust
pub struct StorageConfig {
    pub base_path: String,  // Default: "./storage"
}

pub struct StorageWorkerConfig {
    pub cleanup_interval_seconds: u64,  // Default: 3600 (1 hour)
    pub cleanup_batch_size: i64,        // Default: 100
}
```

**Environment Variables:**
```bash
BUILDSCALE__STORAGE__BASE_PATH=./storage
BUILDSCALE__STORAGE_WORKER__CLEANUP_INTERVAL_SECONDS=3600
BUILDSCALE__STORAGE_WORKER__CLEANUP_BATCH_SIZE=100
```

#### Server Configuration

```rust
pub struct ServerConfig {
    pub host: String,              // Default: "0.0.0.0"
    pub port: u16,                 // Default: 3000
    pub admin_build_path: String,  // Default: "./admin"
    pub web_build_path: String,    // Default: "./web"
}
```

**Environment Variables:**
```bash
BUILDSCALE__SERVER__HOST=0.0.0.0
BUILDSCALE__SERVER__PORT=3000
BUILDSCALE__SERVER__ADMIN_BUILD_PATH=./admin
BUILDSCALE__SERVER__WEB_BUILD_PATH=./web
```

#### Cache Configuration

```rust
pub struct CacheConfig {
    pub user_cache_ttl_seconds: u64,  // Default: 900 (15 minutes)
}
```

**Environment Variables:**
```bash
BUILDSCALE__CACHE__USER_CACHE_TTL_SECONDS=900
```

### Validation Rules

**JWT Secrets:**
```rust
// Check minimum length
if secret.len() < 32 {
    return Err("BUILDSCALE__JWT__SECRET must be at least 32 characters");
}

// Check for weak patterns
let weak_patterns = vec!["change-this", "secret", "password", "123456", "example"];
for pattern in weak_patterns {
    if secret.to_lowercase().contains(pattern) {
        return Err(format!("JWT secret contains weak pattern '{}'", pattern));
    }
}
```

### Default Values

| Section | Key | Default |
|---------|-----|---------|
| Database | user | `"postgres"` |
| Database | host | `"localhost"` |
| Database | port | `5432` |
| Database | database | `"postgres"` |
| JWT | access_token_expiration_minutes | `15` |
| Sessions | expiration_hours | `720` (30 days) |
| Sessions | revoked_token_retention_minutes | `1440` (1 day) |
| Cache | user_cache_ttl_seconds | `900` (15 min) |
| Server | host | `"0.0.0.0"` |
| Server | port | `3000` |
| Server | admin_build_path | `"./admin"` |
| Server | web_build_path | `"./web"` |
| Storage | base_path | `"./storage"` |
| Storage Worker | cleanup_interval_seconds | `3600` (1 hour) |
| Storage Worker | cleanup_batch_size | `100` |
| AI | actor_inactivity_timeout_seconds | `600` (10 min) |
| Providers | default_provider | `"openai"` |
| Providers | default_model | `"openai:gpt-5-mini"` |
| OpenAI | enable_reasoning_summaries | `false` |
| OpenAI | reasoning_effort | `"low"` |

### Security Best Practices

1. **Never commit secrets to version control**
   - Use `.env` file (in `.gitignore`)
   - Use environment variables in production

2. **Use different secrets for access and refresh tokens**
   - Prevents single breach from compromising both

3. **Rotate secrets regularly**
   - Recommended: Every 90 days
   - Requires re-issuing all tokens

4. **Use strong, random secrets**
   - Minimum 32 characters
   - Cryptographically random
   - No dictionary words or patterns

5. **Configure HTTPS in production**
   - Set `BUILDSCALE__SERVER__HOST` appropriately
   - Use reverse proxy (nginx, Traefik) for SSL termination

---

## Part 13: Key Implementation Files Reference

### Core Application Files

| File | Purpose | Key Contents |
|------|---------|--------------|
| `src/main.rs` | Entry point | Startup sequence, server initialization |
| `src/lib.rs` | Module root | Exports, router setup, configuration loading |
| `src/state.rs` | Application state | `AppState` struct, worker channels |
| `src/config.rs` | Configuration | All config structs, validation |
| `src/error.rs` | Error handling | `Error` enum, `ValidationErrors`, HTTP mapping |
| `src/validation.rs` | Input validation | Email, password, workspace name validation |

### Authentication Module (`src/auth/`)

| File | Purpose | Key Types/Functions |
|------|---------|-------------------|
| `services/sessions.rs` | Session management | `create_session`, `validate_session`, `refresh_session` |
| `services/cookies.rs` | Cookie utilities | `build_access_token_cookie`, `extract_jwt_token` |
| `models/users.rs` | User model | `User`, `LoginUser`, `RegisterUser` |
| `handlers/auth.rs` | HTTP endpoints | `login`, `logout`, `register`, `refresh` |

### Chat Module (`src/chat/`)

| File | Purpose | Key Types/Functions |
|------|---------|-------------------|
| `services/mod.rs` | Core services | `ChatService`, `BuiltContext` |
| `services/context.rs` | Context management | `AttachmentManager`, `HistoryManager` |
| `services/sync.rs` | YAML sync | `ChatFrontmatter`, sync functions |
| `services/registry.rs` | Session registry | `AgentRegistry` |
| `services/actor/actor.rs` | Actor implementation | `ChatActor` |
| `services/engine/rig.rs` | Rig integration | `RigService`, agent creation |
| `services/state_machine/` | State machine | `ChatStateMachine`, states, events |
| `handlers/chat.rs` | HTTP endpoints | `create_chat`, `post_chat_message` |

### Tools Module (`src/tools/`)

| Directory | Tools |
|-----------|-------|
| `tools/file/` | 14 file system tools |
| `tools/memory/` | 5 memory tools |
| `tools/plan/` | 6 plan mode tools |
| `tools/web/` | 2 web tools |
| `tools/mod.rs` | `Tool` trait, `ToolConfig`, executor |

### File System Module (`src/fs/`)

| File | Purpose | Key Functions |
|------|---------|--------------|
| `services.rs` | File operations | `create_file`, `read_file`, `write_file`, `delete_file` |
| `storage.rs` | Disk I/O | `FileStorageService`, read/write operations |
| `queries.rs` | Database queries | File CRUD, version queries |
| `parsers/tags.rs` | Tag extraction | `extract_tags` |
| `parsers/links.rs` | Link extraction | `extract_links` |
| `workers/` | Indexer workers | `tag_indexer_worker`, `link_indexer_worker` |

### Workers Module (`src/workers/`)

| File | Purpose |
|------|---------|
| `auth/token_cleanup.rs` | Expired session cleanup |
| `storage/archive_cleanup.rs` | Unused blob cleanup |

### Middleware (`src/middleware/`)

| File | Purpose |
|------|---------|
| `auth.rs` | JWT authentication |
| `workspace_access.rs` | Workspace membership verification |

### Provider Integrations (`src/providers/`)

| File | Purpose |
|------|---------|
| `openai.rs` | OpenAI API client |
| `openrouter.rs` | OpenRouter API client |
| `common.rs` | Shared provider utilities |

### Agent Module (`src/agent/`)

| Directory | Purpose |
|-----------|---------|
| `core/assistant.rs` | Default assistant persona |
| `core/planner.rs` | Strategic planning persona |
| `core/builder.rs` | Plan execution persona |
| `models/session.rs` | `AgentSession` model |
| `services/sessions.rs` | Session lifecycle |
| `handlers/sessions.rs` | REST API endpoints |

### Cross-References Between Files

**Request Flow:**
```
HTTP Request
  → handlers/auth.rs::login()
  → auth/services/sessions.rs::create_session()
  → auth/queries.rs::create_user_session()
  → Database (user_sessions table)
  → Response (tokens)
```

**Chat Flow:**
```
POST /chats/{id}/messages
  → handlers/chat.rs::post_chat_message()
  → chat/services/mod.rs::ChatService::send_message()
  → chat/services/actor/actor.rs::ChatActor::process()
  → chat/services/engine/rig.rs::RigService::create_agent()
  → Rig + LLM
  → SSE stream
```

**File Write Flow:**
```
Tool: write
  → tools/file/write.rs::WriteTool::execute()
  → fs/services.rs::write_file()
  → fs/storage.rs::FileStorageService::write()
  → Disk write (latest/)
  → fs/queries.rs::update_file()
  → Database update (files table)
  → workers/ signal → tag_indexer_worker
```

---

## Part 14: Quick Reference & Big Picture Summary

### Common Commands

```bash
# Build
cargo build

# Run tests
cargo test

# Run specific test
cargo test test_user_registration_success

# Run examples
cargo run --example 01_hello

# Database migrations
sqlx migrate run
sqlx migrate info

# Run server
cargo run

# Development with auto-reload
cargo install cargo-watch
cargo watch -x run
```

### Request Flow Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     HTTP Request                            │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                  Middleware Layers                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │ Request ID  │→ │ Trace Layer │→ │    CORS     │        │
│  └─────────────┘  └─────────────┘  └─────────────┘        │
│  ┌─────────────┐  ┌─────────────┐                         │
│  │ Compression │→ │ JWT Auth    │                          │
│  └─────────────┘  └─────────────┘                         │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                     Handler Layer                           │
│  Extract params → Call service → Return response            │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                     Service Layer                           │
│  Business logic → Validation → Authorization                │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                      Query Layer                             │
│  SQL execution → Transaction management → Result mapping    │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    PostgreSQL Database                       │
└─────────────────────────────────────────────────────────────┘
```

### AI Flow Diagram

```
┌─────────────────────────────────────────────────────────────┐
│              POST /chats/{id}/messages                      │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                  ChatActor::process()                       │
│  • Create/update session in agent_sessions                  │
│  • Build context (persona + history + attachments)          │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                  RigService::create_agent()                 │
│  • Select provider (OpenAI/OpenRouter)                      │
│  • Select model (gpt-4o, claude-3.5-sonnet, etc.)          │
│  • Register tools (27+ tools with JSON schema)              │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                      LLM (via Rig)                          │
│  • Stream response (thought → tool call → result → ...)     │
│  • Each event sent via SSE                                  │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                   Tool Execution                            │
│  • Tool executor dispatches to appropriate tool            │
│  • File/memory/plan/web tools execute                      │
│  • Results returned to LLM                                 │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                   SSE Stream Response                       │
│  • Client receives events in real-time                     │
│  • Events: thought, call, observation, file_updated, done  │
└─────────────────────────────────────────────────────────────┘
```

### Key Concepts Summary

| Concept | Description | Location |
|---------|-------------|----------|
| **Workspace** | Tenant isolation unit | `workspaces/` module |
| **RBAC** | 4-tier role system | `models/roles.rs` |
| **Dual-Token Auth** | JWT + Session tokens | `AUTHENTICATION.md` |
| **Everything is a File** | Universal interface | `FILE_SYSTEM.md` |
| **Hybrid Storage** | Database + Disk | `fs/` module |
| **Plan → Build Mode** | Exploration before execution | `PLAN_SYSTEM.md` |
| **Agent Personas** | Assistant, Planner, Builder | `agent/core/` |
| **State Machine** | ChatActor lifecycle | `chat/services/state_machine/` |
| **Tool System** | 27+ composable tools | `tools/` module |
| **Context Engineering** | Dynamic pruning | `chat/services/context.rs` |

### Common Patterns Reference

**Handler Pattern** (3-5 lines max):
```rust
pub async fn some_handler(
    DbConnection(mut conn): DbConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<Request>,
) -> Result<Json<Response>> {
    let result = some_service(&mut conn, id, req).await?;
    Ok(Json(result))
}
```

**Service Pattern** (all business logic):
```rust
pub async fn some_service(
    conn: &mut DbConn,
    id: Uuid,
    req: Request,
) -> Result<Response> {
    // 1. Validate input
    validate_input(&req)?;

    // 2. Check permissions
    check_permission(conn, user_id, "resource:read").await?;

    // 3. Execute business logic
    let result = do_something(conn, id).await?;

    // 4. Return result
    Ok(result)
}
```

**Tool Pattern** (plan mode enforcement):
```rust
async fn execute(...) -> Result<ToolResponse> {
    // Check plan mode
    if config.plan_mode {
        return Err(Error::Validation(PLAN_MODE_ERROR));
    }

    // Execute logic
    let result = do_work(...).await?;

    Ok(ToolResponse {
        success: true,
        output: result,
    })
}
```

### Troubleshooting Quick Tips

| Problem | Solution |
|---------|----------|
| Database connection fails | Check `BUILDSCALE__DATABASE__*` variables |
| JWT validation fails | Verify `BUILDSCALE__JWT__SECRET` is set and ≥32 chars |
| Files not being indexed | Check tag/link indexer workers are running |
| AI requests failing | Verify API keys for OpenAI/OpenRouter |
| Plan mode blocked | Use `exit_plan_mode` tool to switch to build mode |
| Session expired | Call `/auth/refresh` with refresh token |
| Permission denied | Check user role in workspace members |
| CORS errors | Verify `CorsLayer` configuration in `lib.rs` |

---

## Appendix: File Path Reference

**Quick lookup for developers:**

| What You Want to Do | File to Edit |
|---------------------|--------------|
| Add a new validation rule | `src/validation.rs` |
| Add a new API endpoint | `src/handlers/` |
| Add a new tool | `src/tools/` (appropriate subdirectory) |
| Change JWT expiration | `src/config.rs` (`JwtConfig`) |
| Modify file storage behavior | `src/fs/storage.rs` |
| Add new agent persona | `src/agent/core/` |
| Change plan mode behavior | `src/tools/plan/` |
| Update worker intervals | `src/config.rs` (`StorageWorkerConfig`) |
| Add new permission | `src/models/permissions.rs` |
| Modify error responses | `src/error.rs` |

---

**End of LINEAR_LEARNING_GUIDE.md**

For questions or contributions, please refer to the main documentation in `docs/` or the codebase itself.
