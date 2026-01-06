# Backend System Learning Path

A comprehensive, first-principles guide to understanding the BuildScale-AI backend architecture. Work through each section sequentially, completing all checkboxes before moving on.

---

## Table of Contents

1. [Multi-tenant Architecture & Workspace Isolation](#1-multi-tenant-architecture--workspace-isolation)
2. [Three-Layer Architecture (Service → Query → Model)](#2-three-layer-architecture-service--query--model)
3. [Database Schema Design](#3-database-schema-design)
4. [Authentication System](#4-authentication-system)
5. [Role-Based Access Control (RBAC)](#5-role-based-access-control-rbac)
6. [Workspace Invitation System](#6-workspace-invitation-system)
7. [Input Validation & Error Handling](#7-input-validation--error-handling)
8. [Entity Relationships](#8-entity-relationships)
9. [Session Management](#9-session-management)
10. [Configuration & Environment](#10-configuration--environment)
11. [Testing Strategy](#11-testing-strategy)
12. [Security Best Practices](#12-security-best-practices)

---

## 1. Multi-tenant Architecture & Workspace Isolation

### Core Concept

**Multi-tenancy** is an architecture where a single instance of software serves multiple customers (tenants), with each tenant's data isolated from others.

### First Principles

**Why Multi-tenancy?**
- **Cost Efficiency**: One deployment serves many customers
- **Maintenance**: Single codebase to update and maintain
- **Scalability**: Add new tenants without new infrastructure

**The Problem It Solves**:
```
Without multi-tenancy:
  Company A → Server A → Database A
  Company B → Server B → Database B
  Company C → Server C → Database C
  (Expensive, hard to maintain)

With multi-tenancy:
  Company A ─┐
  Company B ─┼─→ Single Server → Single Database (with isolation)
  Company C ─┘
  (Efficient, centralized)
```

### How BuildScale Implements It

**Workspace = Tenant**

Each workspace is a completely isolated environment:

```rust
// src/models/workspaces.rs
pub struct Workspace {
    pub id: Uuid,           // Unique tenant identifier
    pub name: String,       // Tenant name
    pub owner_id: Uuid,     // Single owner with full control
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**Key Isolation Mechanisms**:

1. **Data Separation**: All workspace-related data includes `workspace_id`
2. **Query Filtering**: Every query filters by `workspace_id`
3. **Foreign Key Cascades**: Deleting workspace removes all its data

```sql
-- Every query is scoped to workspace
SELECT * FROM roles WHERE workspace_id = $1;
SELECT * FROM workspace_members WHERE workspace_id = $1;
```

**Users Are Global, Memberships Are Scoped**:
```
┌─────────────────────────────────────────────────┐
│                  GLOBAL USERS                    │
│  user@example.com (id: uuid-123)                │
└──────────┬─────────────────────┬────────────────┘
           │                     │
           ▼                     ▼
┌──────────────────┐   ┌──────────────────┐
│  Workspace A     │   │  Workspace B     │
│  Role: Admin     │   │  Role: Viewer    │
│  (full access)   │   │  (read only)     │
└──────────────────┘   └──────────────────┘
```

### Key Code Locations

| File | Purpose |
|------|---------|
| `src/models/workspaces.rs` | Workspace data structures |
| `src/services/workspaces.rs` | Workspace business logic |
| `src/queries/workspaces.rs` | Workspace database operations |

### Understanding Checklist

- [ ] I understand why multi-tenancy is used (cost, maintenance, scalability)
- [ ] I understand that Workspace = Tenant in this system
- [ ] I understand users are global but memberships are workspace-scoped
- [ ] I understand all queries filter by `workspace_id` for isolation
- [ ] I understand cascade deletes clean up all workspace data

### Q&A

**Q1: Why not use separate databases per tenant?**
> A: Separate databases add operational complexity (backups, migrations, connections). Single database with workspace_id filtering provides isolation with simpler operations. Trade-off: slightly more complex queries, but much simpler infrastructure.

**Q2: How do we prevent data leaks between workspaces?**
> A: Every query that accesses workspace-scoped data MUST include `WHERE workspace_id = $1`. This is enforced at the query layer. Foreign key constraints ensure referential integrity.

**Q3: Can a user belong to multiple workspaces?**
> A: Yes! Users are global entities. They can be members of multiple workspaces with different roles in each. The `workspace_members` table tracks these relationships.

**Q4: What happens when a workspace is deleted?**
> A: Cascade deletes remove all related data:
> - All roles in that workspace → deleted
> - All memberships in that workspace → deleted
> - All invitations for that workspace → deleted
> - Users themselves are NOT deleted (they may belong to other workspaces)

**Q5: Why does each workspace have a single owner?**
> A: Single owner model provides clear accountability and prevents conflicts. The owner has ultimate control and cannot be removed. Ownership can be transferred but never shared.

---

## 2. Three-Layer Architecture (Service → Query → Model)

### Core Concept

**Layered Architecture** separates code into distinct layers with specific responsibilities. Each layer only communicates with adjacent layers.

### First Principles

**Why Layers?**
- **Separation of Concerns**: Each layer has one job
- **Testability**: Test each layer independently
- **Maintainability**: Changes in one layer don't ripple everywhere
- **Reusability**: Lower layers can be reused by multiple higher layers

**The Three Layers**:
```
┌─────────────────────────────────────────────────────────┐
│                    SERVICE LAYER                         │
│  "What to do" - Business logic, validation, workflows   │
├─────────────────────────────────────────────────────────┤
│                    QUERY LAYER                           │
│  "How to store" - Database CRUD operations              │
├─────────────────────────────────────────────────────────┤
│                    MODEL LAYER                           │
│  "What it looks like" - Data structures, types          │
└─────────────────────────────────────────────────────────┘
```

### Layer Responsibilities

#### Model Layer (`src/models/`)

**Purpose**: Define data structures and types

```rust
// src/models/users.rs

// What we store in the database
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,  // Never plain text!
    pub full_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// What we receive from user input
pub struct RegisterUser {
    pub email: String,
    pub password: String,           // Plain text (will be hashed)
    pub confirm_password: String,   // Must match password
    pub full_name: Option<String>,
}

// What we send back after login
pub struct LoginResult {
    pub user: User,
    pub session_token: String,
    pub expires_at: DateTime<Utc>,
}
```

**Key Insight**: Different structs for different purposes:
- `User` → Database entity
- `RegisterUser` → Input from client
- `LoginResult` → Response to client

#### Query Layer (`src/queries/`)

**Purpose**: Execute database operations (CRUD)

```rust
// src/queries/users.rs

// CREATE
pub async fn create_user(conn: &mut DbConn, new_user: NewUser) -> Result<User> {
    sqlx::query_as!(User,
        "INSERT INTO users (email, password_hash, full_name)
         VALUES ($1, $2, $3)
         RETURNING id, email, password_hash, full_name, created_at, updated_at",
        new_user.email,
        new_user.password_hash,
        new_user.full_name
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)
}

// READ
pub async fn get_user_by_email(conn: &mut DbConn, email: &str) -> Result<Option<User>> {
    sqlx::query_as!(User,
        "SELECT id, email, password_hash, full_name, created_at, updated_at
         FROM users WHERE LOWER(email) = LOWER($1)",
        email
    )
    .fetch_optional(conn)
    .await
    .map_err(Error::Sqlx)
}
```

**Key Insight**: Query layer is "dumb" - it just executes SQL. No business logic!

#### Service Layer (`src/services/`)

**Purpose**: Implement business logic and orchestrate operations

```rust
// src/services/users.rs

pub async fn register_user(conn: &mut DbConn, register: RegisterUser) -> Result<User> {
    // 1. VALIDATE (business rule)
    validate_email(&register.email)?;
    validate_password(&register.password)?;
    if register.password != register.confirm_password {
        return Err(Error::Validation("Passwords do not match".to_string()));
    }

    // 2. TRANSFORM (hash password - security requirement)
    let password_hash = generate_password_hash(&register.password)?;

    // 3. CREATE MODEL (prepare for storage)
    let new_user = NewUser {
        email: register.email.to_lowercase(),
        password_hash,
        full_name: register.full_name,
    };

    // 4. DELEGATE TO QUERY LAYER
    let user = users::create_user(conn, new_user).await?;

    Ok(user)
}
```

**Key Insight**: Service layer is where the "thinking" happens.

### Data Flow Example

```
HTTP Request: POST /register
{
    "email": "user@example.com",
    "password": "SecurePass123",
    "confirm_password": "SecurePass123"
}

    │
    ▼
┌─────────────────────────────────────────────────────────┐
│ SERVICE LAYER: register_user()                          │
│                                                         │
│ 1. validate_email("user@example.com")     ✓            │
│ 2. validate_password("SecurePass123")      ✓            │
│ 3. passwords match?                         ✓            │
│ 4. hash = generate_password_hash(...)                   │
│ 5. Call query layer...                                  │
└─────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────┐
│ QUERY LAYER: create_user()                              │
│                                                         │
│ INSERT INTO users (email, password_hash, full_name)     │
│ VALUES ($1, $2, $3) RETURNING *                         │
└─────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────┐
│ MODEL LAYER: User struct                                │
│                                                         │
│ User {                                                  │
│     id: "uuid-123",                                     │
│     email: "user@example.com",                          │
│     password_hash: "$argon2id$...",                     │
│     full_name: None,                                    │
│     created_at: "2024-11-25T10:00:00Z",                │
│     updated_at: "2024-11-25T10:00:00Z"                 │
│ }                                                       │
└─────────────────────────────────────────────────────────┘
    │
    ▼
HTTP Response: 201 Created
{
    "id": "uuid-123",
    "email": "user@example.com",
    "full_name": null,
    "created_at": "2024-11-25T10:00:00Z"
}
```

### Key Code Locations

| Layer | Directory | Example Files |
|-------|-----------|---------------|
| Model | `src/models/` | `users.rs`, `workspaces.rs`, `roles.rs` |
| Query | `src/queries/` | `users.rs`, `workspaces.rs`, `sessions.rs` |
| Service | `src/services/` | `users.rs`, `workspaces.rs`, `roles.rs` |

### Understanding Checklist

- [ ] I understand the purpose of each layer (Model, Query, Service)
- [ ] I understand data flows: Service → Query → Database → Model
- [ ] I understand Models define "what", Queries define "how to store", Services define "what to do"
- [ ] I understand the Query layer has NO business logic
- [ ] I understand different structs serve different purposes (input, storage, output)

### Q&A

**Q1: Why not put validation in the Model layer?**
> A: Models should be simple data containers. Validation is a business rule that belongs in the Service layer. This keeps Models reusable and testable.

**Q2: Can the Query layer call the Service layer?**
> A: No! Layers only call downward. Service → Query → Model. Never upward. This prevents circular dependencies and keeps responsibilities clear.

**Q3: Why have separate structs for RegisterUser and User?**
> A: Security and clarity. `RegisterUser` has plain-text password (from user input). `User` has `password_hash` (for storage). You never want to accidentally store plain-text passwords.

**Q4: What about transactions that span multiple queries?**
> A: The Service layer manages transactions. It starts a transaction, calls multiple Query functions, then commits or rolls back.

```rust
pub async fn create_workspace(conn: &mut DbConn, request: CreateWorkspaceRequest) -> Result<...> {
    let mut tx = conn.begin().await?;  // Start transaction

    let workspace = workspaces::create_workspace(&mut tx, ...).await?;
    let roles = roles::create_default_roles(&mut tx, workspace.id).await?;
    let member = workspace_members::create_member(&mut tx, ...).await?;

    tx.commit().await?;  // All succeed or all fail
    Ok(result)
}
```

**Q5: Where does error handling happen?**
> A: Each layer handles its own errors and converts them to the common `Error` type. Query layer converts `sqlx::Error`, Service layer adds `Validation` and `Authentication` errors.

---

## 3. Database Schema Design

### Core Concept

**Database Schema** defines how data is structured, related, and constrained in the database.

### First Principles

**Key Database Concepts**:

1. **Primary Key**: Unique identifier for each row
2. **Foreign Key**: Reference to another table's primary key
3. **Index**: Data structure for fast lookups
4. **Constraint**: Rule that data must follow

### UUID v7

**What is UUID v7?**

UUID v7 is a time-ordered unique identifier (128 bits):
```
018f1234-5678-7abc-def0-123456789abc
├───────────┤ │├──┤ │├──────────────┤
│            │ │    │ └── Random (62 bits) - uniqueness
│            │ │    └── Variant (2 bits) - RFC 4122
│            │ └── Random (12 bits)
│            └── Version 7 indicator (4 bits)
└── Timestamp (48 bits) - Unix milliseconds, makes UUIDs sortable
```

**UUID v7 Structure Breakdown:**
- **Timestamp (48 bits)**: Unix timestamp in milliseconds. This is why UUIDs generated later are lexicographically greater - they sort chronologically.
- **Version (4 bits)**: Always `7` (binary `0111`) to identify this as UUID v7.
- **Random A (12 bits)**: Additional randomness to prevent collisions within the same millisecond.
- **Variant (2 bits)**: Always `10` (binary) to indicate RFC 4122 compliance.
- **Random B (62 bits)**: Main source of uniqueness - provides ~4.6 quintillion unique values per millisecond.

**Why UUID v7 over auto-increment?**

| Feature | Auto-increment | UUID v7 |
|---------|---------------|---------|
| Uniqueness | Per-table only | Globally unique |
| Predictability | Easy to guess next ID | Unpredictable |
| Distributed Systems | Conflicts possible | No conflicts |
| Time-based Sorting | No | Yes (chronological) |
| Security | Reveals record count | No information leak |

**Implementation**:
```sql
-- PostgreSQL 18+ has uuidv7() built-in - no extension required
-- For PostgreSQL < 18, you would need: CREATE EXTENSION IF NOT EXISTS pg_uuidv7;

-- All tables use UUID v7 as primary key
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuidv7(),  -- Auto-generated UUID v7
    ...
);
```

### Core Tables

#### Users Table
```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    email TEXT UNIQUE NOT NULL,           -- Globally unique
    password_hash TEXT NOT NULL,          -- Argon2 hash
    full_name TEXT,                       -- Optional
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Fast email lookups (case-insensitive login)
CREATE INDEX idx_users_email ON users(email);
```

#### Workspaces Table
```sql
CREATE TABLE workspaces (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    name TEXT NOT NULL,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Fast owner lookups
CREATE INDEX idx_workspaces_owner_id ON workspaces(owner_id);
```

**Note**: `ON DELETE RESTRICT` prevents deleting a user who owns workspaces.

#### Roles Table
```sql
CREATE TABLE roles (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    UNIQUE(workspace_id, name)  -- Role names unique per workspace
);

CREATE INDEX idx_roles_workspace_id ON roles(workspace_id);
```

#### Workspace Members Table
```sql
CREATE TABLE workspace_members (
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (workspace_id, user_id)  -- Composite key
);

CREATE INDEX idx_workspace_members_workspace_id ON workspace_members(workspace_id);
CREATE INDEX idx_workspace_members_user_id ON workspace_members(user_id);
```

#### User Sessions Table
```sql
CREATE TABLE user_sessions (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token TEXT UNIQUE NOT NULL,           -- Session token
    expires_at TIMESTAMPTZ NOT NULL,      -- Expiration time
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_sessions_token ON user_sessions(token);
CREATE INDEX idx_user_sessions_user_id ON user_sessions(user_id);
CREATE INDEX idx_user_sessions_expires_at ON user_sessions(expires_at);
```

### Foreign Key Cascades

**What happens when parent records are deleted?**

| Cascade Type | Behavior |
|--------------|----------|
| `CASCADE` | Delete child records too |
| `RESTRICT` | Prevent deletion if children exist |
| `SET NULL` | Set foreign key to NULL |

**BuildScale's Strategy**:
```
User deleted:
  ├── Sessions → CASCADE (delete all user's sessions)
  ├── Workspace Members → CASCADE (remove from all workspaces)
  └── Owned Workspaces → RESTRICT (must transfer ownership first)

Workspace deleted:
  ├── Roles → CASCADE (delete all workspace roles)
  ├── Members → CASCADE (remove all memberships)
  └── Invitations → CASCADE (delete all invitations)
```

### Indexes for Performance

**When to add indexes**:
- Primary keys (automatic)
- Foreign keys (for JOINs)
- Columns in WHERE clauses
- Columns used for sorting

```sql
-- Common query patterns and their indexes:

-- "Find user by email" (login)
CREATE INDEX idx_users_email ON users(email);

-- "Find session by token" (authentication)
CREATE INDEX idx_user_sessions_token ON user_sessions(token);

-- "Find all members in workspace"
CREATE INDEX idx_workspace_members_workspace_id ON workspace_members(workspace_id);

-- "Find expired sessions" (cleanup)
CREATE INDEX idx_user_sessions_expires_at ON user_sessions(expires_at);
```

### Key Code Locations

| File | Purpose |
|------|---------|
| `migrations/20251009102916_extensions.up.sql` | pgvector extension |
| `migrations/20251009103739_users_and_workspaces.up.sql` | Core tables |
| `migrations/20251016221509_user_sessions.up.sql` | Sessions table |
| `migrations/20251016234349_workspace_invitations.up.sql` | Invitations table |

### Understanding Checklist

- [ ] I understand UUID v7 advantages over auto-increment IDs
- [ ] I understand foreign key relationships between tables
- [ ] I understand CASCADE vs RESTRICT delete behaviors
- [ ] I understand composite primary keys (workspace_id, user_id)
- [ ] I understand why indexes are added and which columns need them
- [ ] I understand the UNIQUE constraint on (workspace_id, name) for roles

### Q&A

**Q1: Why use TIMESTAMPTZ instead of TIMESTAMP?**
> A: TIMESTAMPTZ stores timezone information, ensuring consistent time handling across different server locations. Always use TIMESTAMPTZ for global applications.

**Q2: Why is owner_id ON DELETE RESTRICT?**
> A: Prevents accidental data loss. If you could delete a user who owns workspaces, those workspaces would be orphaned. Owner must transfer ownership before their account can be deleted.

**Q3: Why composite primary key for workspace_members?**
> A: The combination (workspace_id, user_id) naturally identifies a unique membership. A user can only have one membership per workspace. This also prevents duplicates at the database level.

**Q4: When should I add a new index?**
> A: Add indexes when:
> - Query performance is slow (check with EXPLAIN ANALYZE)
> - Column is frequently in WHERE clauses
> - Column is used in JOINs
> - Column is used for ORDER BY
>
> Don't add indexes for:
> - Columns rarely queried
> - Tables with few rows
> - Columns with low cardinality (few unique values)

**Q5: What's the difference between UNIQUE constraint and UNIQUE index?**
> A: Functionally identical. UNIQUE constraint is a logical rule; UNIQUE index is how it's enforced. PostgreSQL creates an index automatically for UNIQUE constraints.

---

## 4. Authentication System

### Core Concept

**Authentication** verifies "who you are" through credentials (email + password) and maintains that identity through sessions.

### First Principles

**The Authentication Problem**:
```
HTTP is stateless - each request is independent.

Request 1: "I'm user@example.com, here's my password"
           → Server: "OK, here's your data"

Request 2: "Give me my data"
           → Server: "Who are you?" (doesn't remember Request 1)
```

**The Solution: Sessions**
```
Login: "I'm user@example.com, here's my password"
       → Server: "OK, here's a token: abc123"

Request 2: "Give me my data, my token is abc123"
           → Server looks up abc123 → finds user@example.com → "Here's your data"
```

### Password Security with Argon2

**Why hash passwords?**
```
Plain text storage (BAD):
  Database: email="user@example.com", password="SecretPass123"
  If database is stolen → attacker has all passwords!

Hashed storage (GOOD):
  Database: email="user@example.com", password_hash="$argon2id$v=19$..."
  If database is stolen → attacker has useless hashes
```

**Argon2 Properties**:
- **Memory-hard**: Requires lots of RAM (expensive to crack in parallel)
- **Time-hard**: Takes measurable time to compute
- **Salt**: Each hash includes random data (same password → different hash)

**Implementation**:
```rust
// src/services/users.rs

pub fn generate_password_hash(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);  // Random salt
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| Error::Validation(format!("Failed to hash password: {}", e)))?
        .to_string();
    Ok(hash)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|_| Error::Validation("Invalid password hash format".to_string()))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
```

**Hash Format**:
```
$argon2id$v=19$m=65536,t=2,p=1$randomsalthere$hashoutputhere
│        │    │              │               │
│        │    │              │               └── Base64-encoded hash
│        │    │              └── Base64-encoded salt
│        │    └── Parameters: memory=65536KiB (64MiB), time=2, parallelism=1
│        └── Version 19
└── Algorithm identifier (argon2id)
```

### Session-Based Authentication

**Login Flow**:
```
1. User sends: { email, password }
2. Server finds user by email
3. Server verifies password against stored hash
4. Server generates random HMAC-signed session token
5. Server stores session in database with expiration
6. Server returns token to client
```

```rust
// src/services/users.rs

pub async fn login_user(conn: &mut DbConn, login: LoginUser) -> Result<LoginResult> {
    // 1. Validate input
    validate_email(&login.email)?;
    if login.password.is_empty() {
        return Err(Error::Validation("Password cannot be empty".to_string()));
    }

    // 2. Find user (case-insensitive email)
    let user = users::get_user_by_email(conn, &login.email)
        .await?
        .ok_or_else(|| Error::Authentication("Invalid email or password".to_string()))?;

    // 3. Verify password
    if !verify_password(&login.password, &user.password_hash)? {
        return Err(Error::Authentication("Invalid email or password".to_string()));
    }

    // 4. Generate session token
    let session_token = generate_session_token()?;
    let config = Config::load()?;
    let expires_at = Utc::now() + Duration::hours(config.sessions.expiration_hours);  // Default: 30 days

    // 5. Store session
    let session = sessions::create_session(conn, NewUserSession {
        user_id: user.id,
        token: session_token.clone(),
        expires_at,
    }).await?;

    // 6. Return result
    Ok(LoginResult {
        user,
        session_token: session.token,
        expires_at: session.expires_at,
    })
}
```

**Session Validation**:
```rust
pub async fn validate_session(conn: &mut DbConn, token: &str) -> Result<User> {
    // 1. Validate token format
    validate_session_token(token)?;

    // 2. Find valid (non-expired) session
    let session = sessions::get_valid_session_by_token(conn, token)
        .await?
        .ok_or_else(|| Error::InvalidToken("Invalid or expired session".to_string()))?;

    // 3. Get associated user
    let user = users::get_user_by_id(conn, session.user_id)
        .await?
        .ok_or_else(|| Error::NotFound("User not found".to_string()))?;

    Ok(user)
}
```

**Complete Authentication Flow Diagram**:
```
CLIENT
  │ POST /login { email, password }
  ▼
SERVICE LAYER
  │ 1. validate_email()
  │ 2. get_user_by_email() ───┐
  │ 3. verify_password()     │
  │ 4. generate UUID v7 token │
  │ 5. create_session() ─────┼─┐
  │ 6. return LoginResult    │ │
  ▼                        │ │
DATABASE LAYER              │ │
  │ users table             │ │
  │ ┌─────────────────┐     │ │
  │ │ id: uuid-123    │◄────┘ │
  │ │ email: user@e..│         │
  │ │ hash: $argon2id$│         │
  │ └─────────────────┘         │
  │                              │
  │ user_sessions table        │
  │ ┌─────────────────┐         │
  │ │ id: uuid-456    │         │
  │ │ user_id: uuid-123│◄────────┘
  │ │ token: abc...   │
  │ │ expires: 2024-12│
  │ └─────────────────┘
  ▼
CLIENT
  │ { session_token, expires_at, user }
```

### Subsequent Authenticated Requests

```
Client: GET /api/workspaces
        Authorization: Bearer abc-def-ghi-jkl

Server:
  1. Extract token from header
  2. validate_session(token) → User
  3. Process request with user context
  4. Return response
```

### Security Considerations

| Aspect | Implementation |
|--------|----------------|
| Password Storage | Argon2 with random salt per password |
| Token Generation | Random HMAC-signed (256-bit randomness, tamper-evident) |
| Token Storage | Database with expiration |
| Error Messages | Generic "Invalid email or password" (no user enumeration) |
| Case Sensitivity | Emails are case-insensitive |

### Key Code Locations

| File | Purpose |
|------|---------|
| `src/services/users.rs` | `register_user`, `login_user`, `validate_session` |
| `src/queries/sessions.rs` | Session CRUD operations |
| `src/models/users.rs` | `LoginUser`, `LoginResult`, `UserSession` |

### Understanding Checklist

- [ ] I understand why passwords are hashed (not encrypted or plain text)
- [ ] I understand Argon2's security properties (memory-hard, salted)
- [ ] I understand the login flow (validate → hash check → generate token → store session)
- [ ] I understand session validation (token lookup → expiration check → return user)
- [ ] I understand why error messages are generic (prevent user enumeration)
- [ ] I understand random HMAC-signed tokens provide 256-bit randomness and tamper detection

### Q&A

**Q1: Why sessions instead of JWT?**
> A: Sessions can be revoked immediately (delete from database). JWTs are valid until expiration. Sessions require database lookup; JWTs are self-contained. Trade-off: sessions are more secure but require more database reads.

**Q2: Why hash instead of encrypt passwords?**
> A: Encryption is reversible (decrypt with key). If attacker gets the key, all passwords exposed. Hashing is one-way - you can only verify by hashing again and comparing. Even if database is stolen, original passwords aren't recoverable.

**Q3: Why same error message for wrong email AND wrong password?**
> A: Prevents user enumeration attack. If "email not found" vs "wrong password" were different messages, attackers could discover which emails exist in the system.

**Q4: How long should sessions last?**
> A: Balance security vs convenience. BuildScale uses 30 days by default (720 hours), configurable via BUILDSCALE_SESSIONS_EXPIRATION_HOURS. Shorter = more secure but users must re-login often. Sessions can be refreshed before expiration.

**Q5: What happens to sessions when user changes password?**
> A: In this implementation, existing sessions remain valid. For higher security, you could revoke all sessions on password change using `revoke_all_user_sessions()`.

---

## 5. Role-Based Access Control (RBAC)

### Core Concept

**RBAC** controls what users can do based on their assigned role. Instead of assigning permissions directly to users, permissions are grouped into roles, and users are assigned roles.

### First Principles

**Why RBAC?**
```
Without RBAC (direct permissions):
  User A: can_read, can_write, can_delete, can_invite...
  User B: can_read, can_write, can_delete, can_invite...
  User C: can_read, can_write...
  (Managing 100 users = managing 100 × N permissions)

With RBAC:
  Admin role: all permissions
  Editor role: can_read, can_write, can_delete
  Viewer role: can_read

  User A → Admin
  User B → Admin
  User C → Editor
  (Managing 100 users = assigning 1 role each)
```

**The Problem It Solves**:
- Simplifies permission management
- Enforces consistent access patterns
- Makes auditing easier (who can do what?)
- Reduces human error in permission assignment

### BuildScale's Role Hierarchy

```
┌─────────────────────────────────────────────────────────────┐
│                         ADMIN                                │
│  Full control: manage members, settings, delete workspace   │
├─────────────────────────────────────────────────────────────┤
│                         EDITOR                               │
│  Content management: create, edit, delete any content       │
├─────────────────────────────────────────────────────────────┤
│                         MEMBER                               │
│  Participation: create, edit own content, comment           │
├─────────────────────────────────────────────────────────────┤
│                         VIEWER                               │
│  Read-only: view workspace and content                      │
└─────────────────────────────────────────────────────────────┘
```

### Permission Categories

**20 Permissions in 3 Categories**:

```rust
// src/models/permissions.rs

// Workspace Permissions (8)
pub mod workspace_permissions {
    pub const READ: &str = "workspace:read";
    pub const WRITE: &str = "workspace:write";
    pub const DELETE: &str = "workspace:delete";
    pub const MANAGE_MEMBERS: &str = "workspace:manage_members";
    pub const MANAGE_SETTINGS: &str = "workspace:manage_settings";
    pub const INVITE_MEMBERS: &str = "workspace:invite_members";
    pub const VIEW_ACTIVITY_LOG: &str = "workspace:view_activity_log";
    pub const EXPORT_DATA: &str = "workspace:export_data";
}

// Content Permissions (8)
pub mod content_permissions {
    pub const CREATE: &str = "content:create";
    pub const READ_OWN: &str = "content:read_own";
    pub const READ_ALL: &str = "content:read_all";
    pub const UPDATE_OWN: &str = "content:update_own";
    pub const UPDATE_ALL: &str = "content:update_all";
    pub const DELETE_OWN: &str = "content:delete_own";
    pub const DELETE_ALL: &str = "content:delete_all";
    pub const COMMENT: &str = "content:comment";
}

// Member Permissions (4)
pub mod member_permissions {
    pub const ADD_MEMBERS: &str = "members:add";
    pub const REMOVE_MEMBERS: &str = "members:remove";
    pub const UPDATE_ROLES: &str = "members:update_roles";
    pub const VIEW_MEMBERS: &str = "members:view";
}
```

### Role-Permission Matrix

| Permission | Admin | Editor | Member | Viewer |
|------------|:-----:|:------:|:------:|:------:|
| **Workspace** |
| `workspace:read` | ✓ | ✓ | ✓ | ✓ |
| `workspace:write` | ✓ | ✓ | ✗ | ✗ |
| `workspace:delete` | ✓ | ✗ | ✗ | ✗ |
| `workspace:manage_members` | ✓ | ✗ | ✗ | ✗ |
| `workspace:manage_settings` | ✓ | ✗ | ✗ | ✗ |
| `workspace:invite_members` | ✓ | ✗ | ✗ | ✗ |
| `workspace:view_activity_log` | ✓ | ✗ | ✗ | ✗ |
| `workspace:export_data` | ✓ | ✓ | ✗ | ✗ |
| **Content** |
| `content:create` | ✓ | ✓ | ✓ | ✗ |
| `content:read_own` | ✓ | ✓ | ✓ | ✓ |
| `content:read_all` | ✓ | ✓ | ✓ | ✓ |
| `content:update_own` | ✓ | ✓ | ✓ | ✗ |
| `content:update_all` | ✓ | ✓ | ✗ | ✗ |
| `content:delete_own` | ✓ | ✓ | ✓ | ✗ |
| `content:delete_all` | ✓ | ✓ | ✗ | ✗ |
| `content:comment` | ✓ | ✓ | ✓ | ✗ |
| **Members** |
| `members:add` | ✓ | ✗ | ✗ | ✗ |
| `members:remove` | ✓ | ✗ | ✗ | ✗ |
| `members:update_roles` | ✓ | ✗ | ✗ | ✗ |
| `members:view` | ✓ | ✓ | ✓ | ✓ |

### Permission Validation

```rust
// src/models/permissions.rs

pub struct PermissionValidator;

impl PermissionValidator {
    /// Check if role has a specific permission
    pub fn role_has_permission(role: &str, permission: &str) -> bool {
        ROLE_PERMISSIONS
            .get(role)
            .map(|perms| perms.contains(&permission))
            .unwrap_or(false)
    }

    /// Check if role has ANY of the permissions (OR logic)
    pub fn role_has_any_permission(role: &str, permissions: &[&str]) -> bool {
        permissions.iter().any(|p| Self::role_has_permission(role, p))
    }

    /// Check if role has ALL permissions (AND logic)
    pub fn role_has_all_permissions(role: &str, permissions: &[&str]) -> bool {
        permissions.iter().all(|p| Self::role_has_permission(role, p))
    }

    /// Get all permissions for a role
    pub fn get_role_permissions(role: &str) -> Vec<&'static str> {
        ROLE_PERMISSIONS
            .get(role)
            .map(|perms| perms.to_vec())
            .unwrap_or_default()
    }
}
```

### Workspace-Level Permission Check

```rust
// src/services/workspace_members.rs

pub async fn validate_workspace_permission(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
    required_permission: &str,
) -> Result<bool> {
    // 1. Owner always has all permissions
    if workspaces::is_workspace_owner(conn, workspace_id, user_id).await? {
        return Ok(true);
    }

    // 2. Get user's membership
    let member = workspace_members::get_workspace_member_optional(
        conn, workspace_id, user_id
    ).await?;

    // 3. Check role permission
    if let Some(membership) = member {
        let role = roles::get_role_by_id(conn, membership.role_id).await?;
        Ok(PermissionValidator::role_has_permission(
            &role.name.to_lowercase(),
            required_permission
        ))
    } else {
        Ok(false)  // Not a member = no permissions
    }
}
```

### Role Constants

```rust
// src/models/roles.rs

pub const ADMIN_ROLE: &str = "admin";
pub const EDITOR_ROLE: &str = "editor";
pub const MEMBER_ROLE: &str = "member";
pub const VIEWER_ROLE: &str = "viewer";

pub const DEFAULT_ROLES: [&str; 4] = [ADMIN_ROLE, EDITOR_ROLE, MEMBER_ROLE, VIEWER_ROLE];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceRole {
    Admin,
    Editor,
    Member,
    Viewer,
}
```

### Key Code Locations

| File | Purpose |
|------|---------|
| `src/models/roles.rs` | Role definitions and constants |
| `src/models/permissions.rs` | Permission categories and validator |
| `src/services/workspace_members.rs` | Permission validation |

### Understanding Checklist

- [ ] I understand RBAC simplifies permission management through roles
- [ ] I understand the 4-tier role hierarchy (Admin > Editor > Member > Viewer)
- [ ] I understand permissions are grouped into categories (workspace, content, member)
- [ ] I understand `PermissionValidator` checks role-permission mappings
- [ ] I understand workspace owner always has all permissions
- [ ] I understand roles are workspace-scoped (different role per workspace)

### Q&A

**Q1: Why 4 roles? Why not just Admin and User?**
> A: Granularity. Different organizations need different access levels. Editor can manage content but not members. Member can participate but not edit others' work. Viewer is for stakeholders who need read-only access.

**Q2: Can I create custom roles?**
> A: The system supports custom roles at the database level, but permissions are hardcoded. Custom roles would use the same 20 permissions. The 4 default roles cover most use cases.

**Q3: Why check owner separately from role?**
> A: Owner is a special status (owns the workspace), not a role. Owner always has all permissions, even if they somehow have a non-admin role. This prevents lockout scenarios.

**Q4: Why is `members:view` available to all roles?**
> A: Users need to know who else is in their workspace for collaboration. Hiding member list creates poor UX without meaningful security benefit.

**Q5: How do I add a new permission?**
> A:
> 1. Add constant to appropriate module in `permissions.rs`
> 2. Add to `ALL_PERMISSIONS` array
> 3. Update `ROLE_PERMISSIONS` map for each role
> 4. Update documentation and tests

---

## 6. Workspace Invitation System

### Core Concept

**Invitations** allow existing members to bring new users into a workspace through secure, token-based links with role assignments.

### First Principles

**Why Invitations?**
```
Without invitations:
  Admin: "Hey new person, give me your user ID"
  New person: "How do I find that?"
  Admin: "Create account first, then tell me"
  Admin: manually adds user by ID
  (Error-prone, bad UX)

With invitations:
  Admin: sends email to "newperson@example.com"
  New person: clicks link, creates account, automatically added
  (Seamless, secure)
```

**The Problem It Solves**:
- Onboarding new members without knowing their IDs
- Pre-assigning roles before user accepts
- Time-limited invitations (security)
- Audit trail of who invited whom

### Invitation State Machine

```
                 ┌─────────────┐
    create()     │   PENDING   │     expires_at < now
        ─────────►│             │─────────────────────┐
                 └──────┬──────┘                      │
                        │                             ▼
            accept()    │                      ┌─────────────┐
                        │                      │   EXPIRED   │
                        ▼                      └─────────────┘
                 ┌─────────────┐
                 │  ACCEPTED   │
                 └─────────────┘
                        ▲
                        │ (automatic on accept)
                        │
    ┌─────────────┐     │
    │   REVOKED   │◄────┘ revoke() from PENDING
    └─────────────┘
```

**State Transitions**:
- `PENDING` → `ACCEPTED`: User clicks link and accepts
- `PENDING` → `EXPIRED`: Time passes beyond `expires_at`
- `PENDING` → `REVOKED`: Admin cancels invitation

### Invitation Model

```rust
// src/models/invitations.rs

pub struct WorkspaceInvitation {
    pub id: Uuid,                          // Invitation ID
    pub workspace_id: Uuid,                // Target workspace
    pub invited_email: String,             // Email of invitee
    pub invited_by: Uuid,                  // User who sent invitation
    pub role_id: Uuid,                     // Role to assign on acceptance
    pub invitation_token: String,          // Secure token (UUID v7)
    pub status: String,                    // pending, accepted, expired, revoked
    pub expires_at: DateTime<Utc>,         // Expiration timestamp
    pub accepted_at: Option<DateTime<Utc>>,// When accepted (if accepted)
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Status constants
pub const INVITATION_STATUS_PENDING: &str = "pending";
pub const INVITATION_STATUS_ACCEPTED: &str = "accepted";
pub const INVITATION_STATUS_EXPIRED: &str = "expired";
pub const INVITATION_STATUS_REVOKED: &str = "revoked";

// Expiration constants
pub const DEFAULT_INVITATION_EXPIRATION_HOURS: i64 = 168;  // 7 days
pub const MAX_INVITATION_EXPIRATION_HOURS: i64 = 720;      // 30 days
```

### Invitation Flow

```
1. CREATION
   ┌──────────────┐
   │    Admin     │
   └──────┬───────┘
          │ create_invitation(email, role)
          ▼
   ┌──────────────────────────────────────────┐
   │ SERVICE LAYER                            │
   │                                          │
   │ 1. Validate admin has INVITE_MEMBERS     │
   │ 2. Validate email format                 │
   │ 3. Check not already member              │
   │ 4. Check no pending invitation exists    │
   │ 5. Generate UUID v7 token                │
   │ 6. Calculate expires_at                  │
   │ 7. Store invitation                      │
   └──────────────────────────────────────────┘
          │
          ▼
   ┌──────────────────────────────────────────┐
   │ DATABASE                                 │
   │                                          │
   │ workspace_invitations:                   │
   │   id: uuid-789                           │
   │   workspace_id: uuid-123                 │
   │   invited_email: "new@example.com"       │
   │   invited_by: uuid-456 (admin)           │
   │   role_id: uuid-member-role              │
   │   invitation_token: "secure-uuid-token"  │
   │   status: "pending"                      │
   │   expires_at: 2024-12-02T10:00:00Z       │
   └──────────────────────────────────────────┘

2. ACCEPTANCE
   ┌──────────────┐
   │  New User    │
   └──────┬───────┘
          │ Clicks: /invite/secure-uuid-token
          ▼
   ┌──────────────────────────────────────────┐
   │ SERVICE LAYER                            │
   │                                          │
   │ 1. Find invitation by token              │
   │ 2. Check status == "pending"             │
   │ 3. Check expires_at > now                │
   │ 4. Check user email matches invitation   │
   │ 5. Create workspace_member               │
   │ 6. Update status to "accepted"           │
   └──────────────────────────────────────────┘
          │
          ▼
   User is now a member with assigned role!
```

### API Implementation

```rust
// src/services/invitations.rs

pub async fn create_invitation(
    conn: &mut DbConn,
    request: CreateInvitationRequest,
    inviter_id: Uuid,
) -> Result<CreateInvitationResponse> {
    // 1. Validate inviter has permission
    require_workspace_permission(
        conn, request.workspace_id, inviter_id,
        workspace_permissions::INVITE_MEMBERS
    ).await?;

    // 2. Validate email
    validate_email(&request.invited_email)?;

    // 3. Check not already member
    if is_workspace_member(conn, request.workspace_id, /* by email */).await? {
        return Err(Error::Conflict("User is already a member".to_string()));
    }

    // 4. Check no pending invitation
    if has_pending_invitation(conn, request.workspace_id, &request.invited_email).await? {
        return Err(Error::Conflict("Pending invitation already exists".to_string()));
    }

    // 5. Generate token and expiration
    let token = Uuid::now_v7().to_string();
    let expires_at = Utc::now() + Duration::hours(
        request.expires_in_hours.unwrap_or(DEFAULT_INVITATION_EXPIRATION_HOURS)
    );

    // 6. Get role
    let role = roles::get_role_by_name(conn, request.workspace_id, &request.role_name).await?;

    // 7. Create invitation
    let invitation = invitations::create_invitation(conn, NewInvitation {
        workspace_id: request.workspace_id,
        invited_email: request.invited_email.to_lowercase(),
        invited_by: inviter_id,
        role_id: role.id,
        invitation_token: token,
        expires_at,
    }).await?;

    Ok(CreateInvitationResponse {
        invitation,
        invitation_url: format!("/invite/{}", invitation.invitation_token),
    })
}

pub async fn accept_invitation(
    conn: &mut DbConn,
    request: AcceptInvitationRequest,
    user_id: Uuid,
) -> Result<AcceptInvitationResponse> {
    // 1. Find invitation
    let invitation = invitations::get_invitation_by_token(conn, &request.invitation_token)
        .await?
        .ok_or_else(|| Error::NotFound("Invitation not found".to_string()))?;

    // 2. Validate status
    if invitation.status != INVITATION_STATUS_PENDING {
        return Err(Error::Validation(format!(
            "Invitation is {}", invitation.status
        )));
    }

    // 3. Check expiration
    if invitation.expires_at < Utc::now() {
        return Err(Error::Validation("Invitation has expired".to_string()));
    }

    // 4. Verify email matches (get user and check)
    let user = users::get_user_by_id(conn, user_id).await?
        .ok_or_else(|| Error::NotFound("User not found".to_string()))?;
    if user.email.to_lowercase() != invitation.invited_email.to_lowercase() {
        return Err(Error::Forbidden("Email does not match invitation".to_string()));
    }

    // 5. Create membership
    let member = workspace_members::create_workspace_member(conn, NewWorkspaceMember {
        workspace_id: invitation.workspace_id,
        user_id,
        role_id: invitation.role_id,
    }).await?;

    // 6. Update invitation status
    let updated_invitation = invitations::update_invitation_status(
        conn,
        invitation.id,
        INVITATION_STATUS_ACCEPTED,
        Some(Utc::now()),
    ).await?;

    Ok(AcceptInvitationResponse {
        invitation: updated_invitation,
        workspace_member: member,
    })
}
```

### Key Code Locations

| File | Purpose |
|------|---------|
| `src/models/invitations.rs` | Invitation data structures |
| `src/services/invitations.rs` | Invitation business logic |
| `src/queries/invitations.rs` | Invitation database operations |

### Understanding Checklist

- [ ] I understand invitations enable onboarding without user IDs
- [ ] I understand the state machine (pending → accepted/expired/revoked)
- [ ] I understand tokens are UUID v7 (secure, time-based)
- [ ] I understand invitations have expiration times
- [ ] I understand role is pre-assigned in the invitation
- [ ] I understand only users with `INVITE_MEMBERS` permission can send invitations

### Q&A

**Q1: Why store invitation in database instead of just emailing a token?**
> A: Database storage enables:
> - Tracking who invited whom (audit)
> - Preventing duplicate invitations
> - Revoking invitations
> - Role pre-assignment
> - Expiration enforcement

**Q2: Why prevent multiple pending invitations to same email?**
> A: Prevents spam and confusion. If admin sends multiple invitations, user would be confused about which to accept. Better to revoke old and create new.

**Q3: What if user doesn't have an account yet?**
> A: They create an account first, then accept. The invitation is tied to email, not user ID. When they accept, their user ID is linked.

**Q4: Why validate email matches invitation?**
> A: Security. Prevents user A from accepting invitation meant for user B. The person accepting must have the email that was invited.

**Q5: Can I bulk invite?**
> A: Yes! `bulk_create_invitations()` takes multiple emails and creates invitations for all. Useful for onboarding entire teams.

---

## 7. Input Validation & Error Handling

### Core Concept

**Validation** ensures data meets requirements before processing. **Error Handling** provides clear feedback when things go wrong.

### First Principles

**Why Validate?**
```
Without validation:
  User submits: email = "not-an-email"
  Database: ERROR - constraint violation
  User sees: "Internal server error"
  (Confusing, bad UX)

With validation:
  User submits: email = "not-an-email"
  Validation: "Invalid email format"
  User sees: "Invalid email format"
  (Clear, actionable)
```

**Defense in Depth**:
```
┌─────────────────────────────────────────────┐
│             CLIENT (Browser)                 │
│  HTML5 validation, JavaScript checks        │
├─────────────────────────────────────────────┤
│             SERVICE LAYER                    │
│  Business rule validation                   │
├─────────────────────────────────────────────┤
│             DATABASE                         │
│  Constraints (NOT NULL, UNIQUE, CHECK)      │
└─────────────────────────────────────────────┘

Each layer catches different issues.
All layers needed for security.
```

### Validation Functions

```rust
// src/validation.rs

/// Validate email format
pub fn validate_email(email: &str) -> Result<()> {
    if email.is_empty() {
        return Err(Error::Validation("Email cannot be empty".to_string()));
    }
    if email.len() > 254 {  // RFC 5321
        return Err(Error::Validation("Email too long".to_string()));
    }
    // Check format: local@domain
    if !email.contains('@') || email.starts_with('@') || email.ends_with('@') {
        return Err(Error::Validation("Invalid email format".to_string()));
    }
    Ok(())
}

/// Validate password strength
pub fn validate_password(password: &str) -> Result<()> {
    if password.len() < 8 {
        return Err(Error::Validation(
            "Password must be at least 8 characters long".to_string()
        ));
    }
    if password.len() > 128 {
        return Err(Error::Validation("Password too long".to_string()));
    }
    // Reject common weak passwords
    let weak_passwords = ["password", "12345678", "qwerty123", "admin123"];
    if weak_passwords.contains(&password.to_lowercase().as_str()) {
        return Err(Error::Validation("Password is too common".to_string()));
    }
    Ok(())
}

/// Validate workspace name
pub fn validate_workspace_name(name: &str) -> Result<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(Error::Validation("Workspace name cannot be empty".to_string()));
    }
    if trimmed.len() > 100 {
        return Err(Error::Validation(
            "Workspace name must be less than 100 characters".to_string()
        ));
    }
    Ok(())
}

/// Validate session token format
pub fn validate_session_token(token: &str) -> Result<()> {
    if token.is_empty() {
        return Err(Error::Validation("Session token cannot be empty".to_string()));
    }
    // Validate UUID format
    Uuid::parse_str(token)
        .map_err(|_| Error::Validation("Invalid session token format".to_string()))?;
    Ok(())
}

/// Sanitize string (trim, normalize whitespace)
pub fn sanitize_string(input: &str) -> String {
    input.trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
```

### Error Type Hierarchy

```rust
// src/error.rs

#[derive(Debug, Error)]
pub enum Error {
    // Database errors
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    // Input validation errors (400 Bad Request)
    #[error("Validation error: {0}")]
    Validation(String),

    // Resource not found (404 Not Found)
    #[error("Not found: {0}")]
    NotFound(String),

    // Permission denied (403 Forbidden)
    #[error("Forbidden: {0}")]
    Forbidden(String),

    // Resource conflicts (409 Conflict)
    #[error("Conflict: {0}")]
    Conflict(String),

    // Authentication failures (401 Unauthorized)
    #[error("Authentication failed: {0}")]
    Authentication(String),

    // Invalid/expired tokens (401 Unauthorized)
    #[error("Invalid token: {0}")]
    InvalidToken(String),

    // Session expired (401 Unauthorized)
    #[error("Session expired: {0}")]
    SessionExpired(String),

    // System errors (500 Internal Server Error)
    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, Error>;
```

### Error Handling Pattern

```rust
// Service layer error handling example

pub async fn register_user(conn: &mut DbConn, register: RegisterUser) -> Result<User> {
    // Validation errors (400)
    validate_email(&register.email)?;
    validate_password(&register.password)?;

    if register.password != register.confirm_password {
        return Err(Error::Validation("Passwords do not match".to_string()));
    }

    // Business logic
    let password_hash = generate_password_hash(&register.password)?;

    let new_user = NewUser {
        email: register.email.to_lowercase(),
        password_hash,
        full_name: register.full_name,
    };

    // Database error (might be 409 Conflict for duplicate email)
    users::create_user(conn, new_user).await
}

// API layer error conversion
fn handle_error(error: Error) -> HttpResponse {
    match error {
        Error::Validation(msg) => HttpResponse::BadRequest().json(ApiError {
            error: "validation_error",
            message: msg,
        }),
        Error::NotFound(msg) => HttpResponse::NotFound().json(ApiError {
            error: "not_found",
            message: msg,
        }),
        Error::Forbidden(msg) => HttpResponse::Forbidden().json(ApiError {
            error: "forbidden",
            message: msg,
        }),
        Error::Authentication(_) | Error::InvalidToken(_) | Error::SessionExpired(_) => {
            HttpResponse::Unauthorized().json(ApiError {
                error: "unauthorized",
                message: "Authentication required".to_string(),
            })
        },
        Error::Conflict(msg) => HttpResponse::Conflict().json(ApiError {
            error: "conflict",
            message: msg,
        }),
        Error::Sqlx(_) | Error::Internal(_) => {
            // Log actual error, return generic message
            HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error",
                message: "An unexpected error occurred".to_string(),
            })
        },
    }
}
```

### Error Messages Best Practices

| Principle | Bad | Good |
|-----------|-----|------|
| Be specific | "Invalid input" | "Email cannot be empty" |
| Be actionable | "Error" | "Password must be at least 8 characters" |
| Don't leak info | "User admin@x.com not found" | "Invalid email or password" |
| User-friendly | "UNIQUE constraint violation" | "Email already registered" |

### Key Code Locations

| File | Purpose |
|------|---------|
| `src/validation.rs` | Input validation functions |
| `src/error.rs` | Error type definitions |
| `src/services/*.rs` | Error handling in business logic |

### Understanding Checklist

- [ ] I understand validation happens at multiple layers (defense in depth)
- [ ] I understand the Error enum and its variants
- [ ] I understand validation errors are specific and actionable
- [ ] I understand security-sensitive errors are generic (no info leakage)
- [ ] I understand the `?` operator propagates errors
- [ ] I understand validation functions return `Result<()>`

### Q&A

**Q1: Why validate in service layer if database has constraints?**
> A: Better error messages. Database says "UNIQUE constraint violation". Service says "Email already registered". Also catches errors earlier (no wasted database round-trip).

**Q2: Why generic error for authentication failures?**
> A: Security. "Email not found" vs "Wrong password" tells attackers which emails exist. Generic "Invalid email or password" reveals nothing.

**Q3: How do I add a new validation rule?**
> A: Add function to `validation.rs`, call it in service layer before processing. Return `Error::Validation(message)` on failure.

**Q4: Why separate Validation from Authentication errors?**
> A: Different meanings. Validation = bad input format (user fixable). Authentication = wrong credentials (security-related). They might map to same HTTP status but have different handling.

**Q5: Should I validate in Query layer too?**
> A: No. Query layer is for database operations only. Validation belongs in Service layer. Let database constraints be the last line of defense, not the primary validation.

---

## 8. Entity Relationships

### Core Concept

**Entity Relationships** define how data objects connect to each other in the system.

### First Principles

**Relationship Types**:
```
One-to-One (1:1):
  User ←→ Profile
  Each user has exactly one profile

One-to-Many (1:N):
  User ←→ Sessions
  One user has many sessions

Many-to-Many (N:M):
  Users ←→ Workspaces
  Users can be in many workspaces
  Workspaces can have many users
```

### BuildScale Entity Relationships

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              USERS                                       │
│  Global accounts with email authentication                              │
└────────────┬─────────────────┬──────────────────┬──────────────────────┘
             │ 1:N             │ N:M              │ 1:N
             │                 │                  │
             ▼                 │                  ▼
┌─────────────────────┐       │       ┌─────────────────────┐
│   USER_SESSIONS     │       │       │    WORKSPACES       │
│   (login tokens)    │       │       │  (tenant containers)│
└─────────────────────┘       │       └──────────┬──────────┘
                              │                  │ 1:N
                              │                  │
                              ▼                  ▼
                    ┌─────────────────────────────────────┐
                    │       WORKSPACE_MEMBERS              │
                    │  (junction table with role)         │
                    └──────────────────┬──────────────────┘
                                       │ N:1
                                       ▼
                              ┌─────────────────┐
                              │     ROLES       │
                              │ (per workspace) │
                              └─────────────────┘
```

### Detailed Relationship Breakdown

#### Users ↔ Workspaces (Many-to-Many)

```
                    workspace_members
                    ┌──────────────────┐
┌───────────┐      │ workspace_id (PK) │      ┌──────────────┐
│  USERS    │      │ user_id (PK)      │      │  WORKSPACES  │
├───────────┤      │ role_id (FK)      │      ├──────────────┤
│ id (PK)   │◄────►│ created_at        │◄────►│ id (PK)      │
│ email     │      │ updated_at        │      │ name         │
│ ...       │      └──────────────────┘      │ owner_id(FK) │
└───────────┘                                 └──────────────┘

User can belong to multiple workspaces.
Workspace can have multiple users.
Each membership has exactly one role.
```

```rust
// Get all workspaces a user belongs to
pub async fn list_user_memberships(conn: &mut DbConn, user_id: Uuid)
    -> Result<Vec<WorkspaceMember>> {
    sqlx::query_as!(WorkspaceMember,
        "SELECT workspace_id, user_id, role_id, created_at, updated_at
         FROM workspace_members WHERE user_id = $1",
        user_id
    ).fetch_all(conn).await.map_err(Error::Sqlx)
}

// Get all members in a workspace
pub async fn list_workspace_members(conn: &mut DbConn, workspace_id: Uuid)
    -> Result<Vec<WorkspaceMember>> {
    sqlx::query_as!(WorkspaceMember,
        "SELECT workspace_id, user_id, role_id, created_at, updated_at
         FROM workspace_members WHERE workspace_id = $1",
        workspace_id
    ).fetch_all(conn).await.map_err(Error::Sqlx)
}
```

#### Users → Sessions (One-to-Many)

```
┌───────────┐              ┌─────────────────┐
│  USERS    │              │  USER_SESSIONS  │
├───────────┤              ├─────────────────┤
│ id (PK)   │◄────────────┤ user_id (FK)    │
│ email     │      1:N     │ token           │
│ ...       │              │ expires_at      │
└───────────┘              └─────────────────┘

One user can have many active sessions (multi-device).
Deleting user cascades to delete all sessions.
```

```rust
// Get all sessions for a user
pub async fn get_sessions_by_user(conn: &mut DbConn, user_id: Uuid)
    -> Result<Vec<UserSession>> {
    sqlx::query_as!(UserSession,
        "SELECT id, user_id, token, expires_at, created_at, updated_at
         FROM user_sessions WHERE user_id = $1
         ORDER BY created_at DESC",
        user_id
    ).fetch_all(conn).await.map_err(Error::Sqlx)
}
```

#### Workspaces → Roles (One-to-Many)

```
┌──────────────┐              ┌───────────────┐
│  WORKSPACES  │              │    ROLES      │
├──────────────┤              ├───────────────┤
│ id (PK)      │◄────────────┤ workspace_id  │
│ name         │      1:N     │ name          │
│ owner_id     │              │ description   │
└──────────────┘              └───────────────┘

Each workspace has its own set of roles.
Deleting workspace cascades to delete all roles.
Role names are unique within a workspace.
```

#### Workspaces ↔ Owner (Many-to-One, Special)

```
┌───────────┐              ┌──────────────┐
│  USERS    │              │  WORKSPACES  │
├───────────┤              ├──────────────┤
│ id (PK)   │◄────────────┤ owner_id(FK) │
│ email     │      1:N     │ name         │
│ ...       │   RESTRICT   │ ...          │
└───────────┘              └──────────────┘

One user can own multiple workspaces.
Cannot delete user who owns workspaces (RESTRICT).
Owner is ALSO a member (admin role) in workspace_members.
```

### Entity Models

```rust
// src/models/users.rs
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub full_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// src/models/workspaces.rs
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,  // FK to users
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// src/models/roles.rs
pub struct Role {
    pub id: Uuid,
    pub workspace_id: Uuid,  // FK to workspaces
    pub name: String,
    pub description: Option<String>,
}

// src/models/workspace_members.rs
pub struct WorkspaceMember {
    pub workspace_id: Uuid,  // Composite PK, FK to workspaces
    pub user_id: Uuid,       // Composite PK, FK to users
    pub role_id: Uuid,       // FK to roles
}

// src/models/users.rs
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,  // FK to users
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### Key Relationship Rules

| Relationship | Constraint | Reason |
|--------------|------------|--------|
| User → owned Workspaces | RESTRICT | Can't delete owner without transferring |
| Workspace → Members | CASCADE | Deleting workspace removes all memberships |
| Workspace → Roles | CASCADE | Deleting workspace removes all roles |
| User → Sessions | CASCADE | Deleting user removes all sessions |
| User → Memberships | CASCADE | Deleting user removes from all workspaces |
| Member → Role | CASCADE | If role deleted, membership deleted |

### Key Code Locations

| File | Purpose |
|------|---------|
| `src/models/users.rs` | User and Session entities |
| `src/models/workspaces.rs` | Workspace entity |
| `src/models/roles.rs` | Role entity |
| `src/models/workspace_members.rs` | Membership entity |

### Understanding Checklist

- [ ] I understand 1:1, 1:N, and N:M relationships
- [ ] I understand Users ↔ Workspaces is N:M via workspace_members
- [ ] I understand workspace_members has composite primary key
- [ ] I understand owner_id creates a special 1:N relationship with RESTRICT
- [ ] I understand CASCADE vs RESTRICT and when each is used
- [ ] I understand roles are workspace-scoped (not global)

### Q&A

**Q1: Why is owner_id separate from workspace_members?**
> A: Owner is a special status, not just a role. Owner has implicit full permissions and cannot be removed. It's a property of the workspace, not just a membership.

**Q2: Can a user be owner and also in workspace_members?**
> A: Yes! When workspace is created, owner is automatically added to workspace_members with admin role. owner_id defines ownership; workspace_members defines explicit permissions.

**Q3: What happens if I delete a role that members have?**
> A: CASCADE deletes those memberships. This is dangerous! In practice, prevent deleting roles that are in use, or reassign members first.

**Q4: Why not have a global roles table?**
> A: Workspace isolation. Each workspace might have different role names or custom roles. Global roles would break multi-tenancy and limit flexibility.

**Q5: How do I find all workspaces a user can access?**
> A: Query workspace_members by user_id. Also check workspaces where owner_id = user_id. User can access workspaces they own OR are members of.

---

## 9. Session Management

### Core Concept

**Session Management** handles the lifecycle of user authentication tokens - creation, validation, refresh, and cleanup.

### First Principles

**Why Sessions?**
```
HTTP is stateless. Each request is independent.
Sessions maintain state between requests.

Without sessions:
  Every request needs username + password
  (Insecure, bad UX)

With sessions:
  Login once → get token → use token for all requests
  (Secure, convenient)
```

### Session Lifecycle

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         SESSION LIFECYCLE                                │
└─────────────────────────────────────────────────────────────────────────┘

    LOGIN                    ACTIVE                    END
      │                         │                        │
      ▼                         ▼                        ▼
┌───────────┐  validate   ┌───────────┐  expires   ┌───────────┐
│  CREATE   │────────────►│  VALID    │───────────►│  EXPIRED  │
│  SESSION  │             │  SESSION  │            │           │
└───────────┘             └─────┬─────┘            └───────────┘
                                │
                         refresh │ logout
                                │    │
                                ▼    ▼
                          ┌───────────┐
                          │  REVOKED  │
                          └───────────┘

States:
- VALID: Token exists and expires_at > now
- EXPIRED: Token exists but expires_at < now
- REVOKED: Token deleted from database
```

### Session Creation

```rust
// src/services/users.rs

pub async fn login_user(conn: &mut DbConn, login: LoginUser) -> Result<LoginResult> {
    // ... validate credentials ...

    // Generate session
    let session_token = generate_session_token()?;  // Random HMAC-signed
    let config = Config::load()?;
    let expires_at = Utc::now() + Duration::hours(config.sessions.expiration_hours);  // Default: 30 days

    // Store session
    let session = sessions::create_session(conn, NewUserSession {
        user_id: user.id,
        token: session_token,
        expires_at,
    }).await?;

    Ok(LoginResult {
        user,
        session_token: session.token,
        expires_at: session.expires_at,
    })
}
```

### Session Validation

```rust
// src/services/users.rs

pub async fn validate_session(conn: &mut DbConn, token: &str) -> Result<User> {
    // 1. Validate token format
    validate_session_token(token)?;

    // 2. Query for valid session (token + not expired)
    let session = sessions::get_valid_session_by_token(conn, token)
        .await?
        .ok_or_else(|| Error::InvalidToken("Invalid or expired session".to_string()))?;

    // 3. Get user
    let user = users::get_user_by_id(conn, session.user_id)
        .await?
        .ok_or_else(|| Error::NotFound("User not found".to_string()))?;

    Ok(user)
}
```

```sql
-- Query in get_valid_session_by_token
SELECT id, user_id, token, expires_at, created_at, updated_at
FROM user_sessions
WHERE token = $1 AND expires_at > NOW()
```

### Session Refresh

```rust
// src/services/users.rs

pub async fn refresh_session(
    conn: &mut DbConn,
    session_token: &str,
    hours_to_extend: i64,
) -> Result<String> {
    // Load config to get max extension time
    let config = Config::load()?;
    // Limit extension to configured expiration (same value for both)
    if hours_to_extend > config.sessions.expiration_hours {
        return Err(Error::Validation(
            format!("Cannot extend session by more than {} hours", config.sessions.expiration_hours)
        ));
    }

    // Find valid session
    let session = sessions::get_valid_session_by_token(conn, session_token)
        .await?
        .ok_or_else(|| Error::InvalidToken("Session not found or expired".to_string()))?;

    // Calculate new expiration
    let new_expires_at = Utc::now() + Duration::hours(hours_to_extend);

    // Update session
    let updated = sessions::refresh_session(conn, session.id, new_expires_at).await?;

    Ok(updated.token)
}
```

### Multi-Device Support

```
User logs in from:
  - Laptop    → Session A (token-aaa)
  - Phone     → Session B (token-bbb)
  - Tablet    → Session C (token-ccc)

All sessions valid simultaneously.
Each device has its own token.
Logout from one doesn't affect others.
```

```rust
// Get all active sessions for a user
pub async fn get_user_active_sessions(conn: &mut DbConn, user_id: Uuid)
    -> Result<Vec<UserSession>> {
    sqlx::query_as!(UserSession,
        "SELECT id, user_id, token, expires_at, created_at, updated_at
         FROM user_sessions
         WHERE user_id = $1 AND expires_at > NOW()
         ORDER BY created_at DESC",
        user_id
    ).fetch_all(conn).await.map_err(Error::Sqlx)
}

// Logout from all devices
pub async fn revoke_all_user_sessions(conn: &mut DbConn, user_id: Uuid) -> Result<u64> {
    let result = sqlx::query("DELETE FROM user_sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(conn)
        .await
        .map_err(Error::Sqlx)?;
    Ok(result.rows_affected())
}
```

### Session Cleanup

```rust
// src/services/sessions.rs

/// Delete all expired sessions (run periodically)
pub async fn cleanup_expired_sessions(conn: &mut DbConn) -> Result<u64> {
    let result = sqlx::query("DELETE FROM user_sessions WHERE expires_at < NOW()")
        .execute(conn)
        .await
        .map_err(Error::Sqlx)?;
    Ok(result.rows_affected())
}
```

**Cleanup Strategy**:
- Run as scheduled job (e.g., daily cron)
- Removes sessions where `expires_at < NOW()`
- Reduces database size
- No user impact (sessions already invalid)

### Session Query Operations

```rust
// src/queries/sessions.rs

// Create
pub async fn create_session(conn: &mut DbConn, new: NewUserSession) -> Result<UserSession>

// Read
pub async fn get_session_by_token(conn: &mut DbConn, token: &str) -> Result<Option<UserSession>>
pub async fn get_valid_session_by_token(conn: &mut DbConn, token: &str) -> Result<Option<UserSession>>
pub async fn get_sessions_by_user(conn: &mut DbConn, user_id: Uuid) -> Result<Vec<UserSession>>
pub async fn is_session_valid(conn: &mut DbConn, token: &str) -> Result<bool>

// Update
pub async fn refresh_session(conn: &mut DbConn, id: Uuid, new_expires: DateTime<Utc>) -> Result<UserSession>

// Delete
pub async fn delete_session(conn: &mut DbConn, id: Uuid) -> Result<u64>
pub async fn delete_session_by_token(conn: &mut DbConn, token: &str) -> Result<u64>
pub async fn delete_sessions_by_user(conn: &mut DbConn, user_id: Uuid) -> Result<u64>
pub async fn delete_expired_sessions(conn: &mut DbConn) -> Result<u64>
```

### Key Code Locations

| File | Purpose |
|------|---------|
| `src/services/users.rs` | Login, validate_session, refresh_session |
| `src/services/sessions.rs` | Cleanup, bulk operations |
| `src/queries/sessions.rs` | Session CRUD operations |
| `src/models/users.rs` | UserSession struct |

### Understanding Checklist

- [ ] I understand sessions maintain state in stateless HTTP
- [ ] I understand the session lifecycle (create → validate → refresh/expire → delete)
- [ ] I understand tokens are validated by existence AND expiration check
- [ ] I understand multi-device support (multiple concurrent sessions)
- [ ] I understand cleanup removes expired sessions from database
- [ ] I understand "revoke all" logs user out from all devices

### Q&A

**Q1: Why store sessions in database instead of memory?**
> A: Database storage survives server restarts. Also enables load balancing (any server can validate any token). Memory is faster but loses sessions on restart.

**Q2: Why random HMAC-signed tokens instead of UUID?**
> A: Random tokens provide 256-bit randomness (vs 128-bit UUID) making them unpredictable. HMAC signature provides tamper detection - any modification is immediately detected. UUID v7 is still used for primary keys (time-ordered for database performance).

**Q3: Should I refresh sessions automatically?**
> A: Common patterns:
> - Sliding window: Refresh on every request (extends active users)
> - Manual refresh: Client calls refresh endpoint before expiration
> - Fixed expiration: No refresh, re-login required

**Q4: When should I revoke all user sessions?**
> A: Security events:
> - User changes password
> - Account compromise detected
> - User requests "logout everywhere"
> - Admin action

**Q5: How often should cleanup run?**
> A: Depends on traffic. Daily is usually sufficient. High-traffic sites might run hourly. Check session table size periodically.

---

## 10. Configuration & Environment

### Core Concept

**Configuration** externalizes settings that change between environments (dev, staging, production) without code changes.

### First Principles

**Why External Configuration?**
```
Hardcoded (BAD):
  let db_host = "localhost";      // Only works on dev machine
  let db_pass = "secret123";       // Exposed in source code

External config (GOOD):
  let db_host = env("DB_HOST");   // Different per environment
  let db_pass = env("DB_PASS");   // Secure, not in code
```

**12-Factor App Principle**: Store config in environment variables.

### BuildScale Configuration

**Environment Variables with Prefix**:
```bash
# Database configuration
BUILDSCALE_DATABASE_USER=buildscale
BUILDSCALE_DATABASE_PASSWORD=your_secure_password
BUILDSCALE_DATABASE_HOST=localhost
BUILDSCALE_DATABASE_PORT=5432
BUILDSCALE_DATABASE_DATABASE=buildscale

# For sqlx CLI
DATABASE_URL=postgresql://buildscale:password@localhost:5432/buildscale
```

**Prefix Pattern**: `BUILDSCALE_` prevents conflicts with other apps.

### Configuration Loading

```rust
// src/config.rs

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub user: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub database: String,
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        // Load from .env file if present
        dotenv::dotenv().ok();

        // Build configuration
        config::Config::builder()
            .add_source(config::Environment::with_prefix("BUILDSCALE")
                .separator("_"))
            .build()?
            .try_deserialize()
    }

    pub fn database_url(&self) -> String {
        format!(
            "postgresql://{}:{}@{}:{}/{}",
            self.database.user,
            self.database.password,
            self.database.host,
            self.database.port,
            self.database.database
        )
    }
}
```

### Environment Files

```bash
# .env (development - gitignored)
BUILDSCALE_DATABASE_USER=buildscale
BUILDSCALE_DATABASE_PASSWORD=dev_password
BUILDSCALE_DATABASE_HOST=localhost
BUILDSCALE_DATABASE_PORT=5432
BUILDSCALE_DATABASE_DATABASE=buildscale_dev

# .env.example (template - committed)
BUILDSCALE_DATABASE_USER=your_user
BUILDSCALE_DATABASE_PASSWORD=your_password
BUILDSCALE_DATABASE_HOST=localhost
BUILDSCALE_DATABASE_PORT=5432
BUILDSCALE_DATABASE_DATABASE=your_database
```

### Configuration Hierarchy

```
Priority (highest to lowest):
1. Environment variables (runtime)
2. .env file (development convenience)
3. Default values (fallbacks)

Example:
  .env: DB_HOST=localhost
  ENV:  DB_HOST=production.db.com

  Result: production.db.com (ENV wins)
```

### Hardcoded vs Configurable

| Setting | Type | Reason |
|---------|------|--------|
| Database connection | Configurable | Changes per environment |
| Password min length | Hardcoded | Security requirement, shouldn't change |
| Session duration | Hardcoded* | Can be made configurable |
| Default roles | Hardcoded | System requirement |
| Role permissions | Hardcoded | Core system behavior |

*Trade-off: More configuration = more flexibility but more complexity.

### Key Code Locations

| File | Purpose |
|------|---------|
| `src/config.rs` | Configuration loading |
| `.env.example` | Configuration template |
| `src/database.rs` | Database connection setup |

### Understanding Checklist

- [ ] I understand why configuration is externalized (security, flexibility)
- [ ] I understand the `BUILDSCALE_` prefix convention
- [ ] I understand `.env` is for development, environment variables for production
- [ ] I understand the configuration priority hierarchy
- [ ] I understand some settings are intentionally hardcoded

### Q&A

**Q1: Why double underscore in BUILDSCALE_DATABASE_HOST?**
> A: Convention for nested configuration. Maps to `{ buildscale: { database: { host: "..." } } }`. Double underscore distinguishes nesting from single words.

**Q2: Should I commit .env file?**
> A: NO! .env often contains secrets. Commit `.env.example` as a template. Add `.env` to `.gitignore`.

**Q3: How do I add a new configuration option?**
> A:
> 1. Add field to config struct in `config.rs`
> 2. Add environment variable with `BUILDSCALE_` prefix
> 3. Update `.env.example`
> 4. Update documentation

**Q4: Where should secrets go in production?**
> A: Options:
> - Environment variables (basic)
> - Secret management service (AWS Secrets Manager, HashiCorp Vault)
> - Kubernetes secrets
> Never in code or config files!

**Q5: Why use config crate instead of raw env vars?**
> A: Benefits:
> - Type safety (parse to u16, bool, etc.)
> - Nested configuration
> - Multiple sources (env, files)
> - Validation at startup

---

## 11. Testing Strategy

### Core Concept

**Testing** verifies code works correctly. Good tests catch bugs early, enable refactoring, and document behavior.

### First Principles

**Test Pyramid**:
```
        /\
       /  \        E2E Tests (few, slow, fragile)
      /----\
     /      \      Integration Tests (some, medium)
    /--------\
   /          \    Unit Tests (many, fast, stable)
  /------------\
```

**Why Test?**
- Catch bugs before users do
- Safe refactoring (tests tell you if you broke something)
- Documentation (tests show how code should be used)
- Design feedback (hard to test = probably bad design)

### Test Organization in BuildScale

```
tests/
├── common/
│   ├── mod.rs           # Test utilities exports
│   └── database.rs      # Database setup and cleanup
├── users/
│   ├── mod.rs
│   └── services/
│       └── test_registration.rs
├── workspaces/
│   ├── mod.rs
│   └── services/
│       └── test_workspace_creation.rs
└── ...
```

### Test Isolation

**Problem**: Tests share database, can interfere with each other.

**Solution**: Each test uses unique prefixes.

```rust
// tests/common/database.rs

pub struct TestDb {
    pub pool: PgPool,
    pub prefix: String,  // Unique per test
}

impl TestDb {
    pub async fn new(test_name: &str) -> Self {
        let prefix = format!("test_{}_{}", test_name, Uuid::new_v4().to_string()[..8]);
        let pool = create_test_pool().await;

        // Cleanup any leftover data from previous runs
        cleanup_test_data(&pool, &prefix).await;

        TestDb { pool, prefix }
    }

    pub fn test_email(&self, name: &str) -> String {
        format!("{}_{name}@test.com", self.prefix)
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        // Cleanup on test completion
        // (async cleanup via separate mechanism)
    }
}
```

### Test Patterns

**Unit Test (Service Layer)**:
```rust
#[tokio::test]
async fn test_user_registration_success() {
    let db = TestDb::new("registration_success").await;
    let mut conn = db.pool.acquire().await.unwrap();

    let register = RegisterUser {
        email: db.test_email("new_user"),
        password: "SecurePass123".to_string(),
        confirm_password: "SecurePass123".to_string(),
        full_name: Some("Test User".to_string()),
    };

    let result = register_user(&mut conn, register).await;

    assert!(result.is_ok());
    let user = result.unwrap();
    assert!(user.email.contains("new_user"));
    assert!(user.full_name.is_some());
}
```

**Error Case Test**:
```rust
#[tokio::test]
async fn test_user_registration_password_mismatch() {
    let db = TestDb::new("registration_mismatch").await;
    let mut conn = db.pool.acquire().await.unwrap();

    let register = RegisterUser {
        email: db.test_email("user"),
        password: "Password123".to_string(),
        confirm_password: "DifferentPass123".to_string(),  // Mismatch!
        full_name: None,
    };

    let result = register_user(&mut conn, register).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        Error::Validation(msg) => assert!(msg.contains("match")),
        _ => panic!("Expected Validation error"),
    }
}
```

**Integration Test (Multiple Services)**:
```rust
#[tokio::test]
async fn test_workspace_creation_flow() {
    let db = TestDb::new("workspace_flow").await;
    let mut conn = db.pool.acquire().await.unwrap();

    // 1. Create user
    let user = register_user(&mut conn, RegisterUser {
        email: db.test_email("owner"),
        password: "SecurePass123".to_string(),
        confirm_password: "SecurePass123".to_string(),
        full_name: Some("Owner".to_string()),
    }).await.unwrap();

    // 2. Create workspace
    let workspace_result = create_workspace(&mut conn, CreateWorkspaceRequest {
        name: format!("{}_workspace", db.prefix),
        owner_id: user.id,
    }).await.unwrap();

    // 3. Verify
    assert_eq!(workspace_result.roles.len(), 4);  // 4 default roles
    assert!(workspace_result.members.len() == 1);  // Owner is member
}
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_user_registration_success

# Run with output
cargo test -- --nocapture

# Run tests matching pattern
cargo test users::services

# Run single-threaded (if tests conflict)
cargo test -- --test-threads=1
```

### Key Code Locations

| File | Purpose |
|------|---------|
| `tests/common/database.rs` | Test database utilities |
| `tests/*/services/*.rs` | Service layer tests |
| `Cargo.toml` | Test dependencies |

### Understanding Checklist

- [ ] I understand the test pyramid (unit, integration, E2E)
- [ ] I understand test isolation via unique prefixes
- [ ] I understand TestDb helper for database testing
- [ ] I understand how to test success and error cases
- [ ] I understand how to run specific tests

### Q&A

**Q1: Why not use a separate test database?**
> A: You can! Trade-off:
> - Same database + prefixes: Simpler setup, tests real constraints
> - Separate database: Complete isolation, requires more setup

**Q2: How do I test without hitting the database?**
> A: Mocking:
> - Mock the Query layer for Service tests
> - Mock the Service layer for API tests
> - Trade-off: Faster but doesn't test real database behavior

**Q3: Should every function have a test?**
> A: No. Prioritize:
> - Business logic (high value)
> - Edge cases (error-prone)
> - Complex algorithms
> Skip trivial getters/setters.

**Q4: Why `#[tokio::test]` instead of `#[test]`?**
> A: Our code uses async/await (database operations). `#[tokio::test]` runs tests in async runtime. Regular `#[test]` doesn't support await.

**Q5: How do I debug a failing test?**
> A:
> 1. Run with `--nocapture` to see println output
> 2. Run single test with `-- --exact`
> 3. Add debug prints or use debugger
> 4. Check test isolation (is another test interfering?)

---

## 12. Security Best Practices

### Core Concept

**Security** protects the system from unauthorized access, data breaches, and attacks.

### First Principles

**Defense in Depth**:
```
┌─────────────────────────────────────────────────────────────┐
│                    NETWORK LAYER                             │
│  HTTPS, firewalls, rate limiting                            │
├─────────────────────────────────────────────────────────────┤
│                    APPLICATION LAYER                         │
│  Authentication, authorization, input validation            │
├─────────────────────────────────────────────────────────────┤
│                    DATA LAYER                                │
│  Encryption, access controls, constraints                   │
└─────────────────────────────────────────────────────────────┘

Multiple layers = attacker must break all to succeed
```

### Password Security

**Argon2 Hashing**:
```rust
// NEVER store plain text passwords
// ALWAYS hash with salt

pub fn generate_password_hash(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);  // Random per password
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| Error::Internal("Hash failed".to_string()))?
        .to_string();
    Ok(hash)
}
```

**Why Argon2?**
- Memory-hard (expensive to crack in parallel)
- Winner of Password Hashing Competition
- Resistant to GPU/ASIC attacks
- Includes salt in output

**Password Requirements**:
```rust
pub fn validate_password(password: &str) -> Result<()> {
    // Minimum length
    if password.len() < 8 {
        return Err(Error::Validation("Password too short".to_string()));
    }

    // Maximum length (prevent DoS via huge passwords)
    if password.len() > 128 {
        return Err(Error::Validation("Password too long".to_string()));
    }

    // Block common passwords
    let weak = ["password", "12345678", "qwerty123"];
    if weak.contains(&password.to_lowercase().as_str()) {
        return Err(Error::Validation("Password too common".to_string()));
    }

    Ok(())
}
```

### Session Security

**Secure Token Generation**:
```rust
// UUID v7: Unpredictable, time-ordered, unique
let token = Uuid::now_v7().to_string();

// NOT secure (predictable):
// let token = format!("session_{}", counter);
// let token = user_id.to_string();
```

**Session Properties**:
- Unique per session (no collisions)
- Unpredictable (can't guess other tokens)
- Time-limited (automatic expiration)
- Revocable (can be deleted)

### Input Validation

**Validate Everything**:
```rust
// Email
validate_email(&input.email)?;

// Password
validate_password(&input.password)?;

// IDs
let uuid = Uuid::parse_str(&input.id)
    .map_err(|_| Error::Validation("Invalid ID".to_string()))?;

// Strings
let clean = sanitize_string(&input.name);
```

**Prevent Injection**:
```rust
// SQL Injection: ALWAYS use parameterized queries
// GOOD:
sqlx::query!("SELECT * FROM users WHERE email = $1", email)

// BAD (vulnerable):
// format!("SELECT * FROM users WHERE email = '{}'", email)
```

### Authorization

**Check Permissions**:
```rust
// ALWAYS verify user has permission before action
pub async fn delete_workspace(conn: &mut DbConn, workspace_id: Uuid, user_id: Uuid)
    -> Result<()>
{
    // Check permission FIRST
    if !is_workspace_owner(conn, workspace_id, user_id).await? {
        return Err(Error::Forbidden("Only owner can delete workspace".to_string()));
    }

    // Then perform action
    workspaces::delete_workspace(conn, workspace_id).await
}
```

### Information Disclosure

**Don't Leak Information**:
```rust
// BAD: Reveals which emails exist
if user_not_found {
    return Err(Error::NotFound("User not found".to_string()));
}
if wrong_password {
    return Err(Error::Authentication("Wrong password".to_string()));
}

// GOOD: Generic message for both cases
if !valid_credentials {
    return Err(Error::Authentication("Invalid email or password".to_string()));
}
```

### Database Constraints

**Enforce at Database Level**:
```sql
-- Unique constraints prevent duplicates even if app fails
CREATE TABLE users (
    email TEXT UNIQUE NOT NULL
);

-- Foreign keys ensure referential integrity
CREATE TABLE workspace_members (
    workspace_id UUID REFERENCES workspaces(id) ON DELETE CASCADE
);

-- Check constraints enforce business rules
CREATE TABLE workspace_invitations (
    status TEXT CHECK (status IN ('pending', 'accepted', 'expired', 'revoked'))
);
```

### Security Checklist

| Area | Practice |
|------|----------|
| **Passwords** | Hash with Argon2, enforce minimum length |
| **Sessions** | Random HMAC-signed tokens, expiration, secure storage |
| **Input** | Validate all input, use parameterized queries |
| **Authorization** | Check permissions before every action |
| **Errors** | Generic messages for sensitive operations |
| **Database** | Constraints, foreign keys, indexes |
| **Secrets** | Environment variables, not in code |

### Key Code Locations

| File | Purpose |
|------|---------|
| `src/services/users.rs` | Password hashing, authentication |
| `src/validation.rs` | Input validation |
| `src/error.rs` | Error types |
| `migrations/*.sql` | Database constraints |

### Understanding Checklist

- [ ] I understand defense in depth (multiple security layers)
- [ ] I understand why Argon2 is used for password hashing
- [ ] I understand why error messages should be generic for auth
- [ ] I understand parameterized queries prevent SQL injection
- [ ] I understand authorization checks must happen before every sensitive action
- [ ] I understand database constraints are the last line of defense

### Q&A

**Q1: Is Argon2 better than bcrypt?**
> A: Argon2 is newer and won the Password Hashing Competition. Both are secure. Argon2 is memory-hard (better against GPU attacks). bcrypt is well-established. Either is fine; avoid MD5/SHA for passwords.

**Q2: Should I encrypt data at rest?**
> A: Depends on sensitivity. Passwords are hashed (not encrypted). Sensitive data (SSN, credit cards) should be encrypted. General data usually relies on disk encryption.

**Q3: How do I handle CSRF?**
> A: For API (JSON):
> - Use Bearer tokens in headers (not cookies)
> - Verify Content-Type header
> For web forms:
> - Include CSRF token in form
> - Validate on submit

**Q4: What about rate limiting?**
> A: Important for:
> - Login attempts (prevent brute force)
> - API requests (prevent abuse)
> - Registration (prevent spam)
> Implement at API gateway or application level.

**Q5: How do I handle security vulnerabilities?**
> A:
> 1. Keep dependencies updated (`cargo audit`)
> 2. Follow security advisories
> 3. Have a vulnerability disclosure process
> 4. Regular security reviews

---

## Final Checklist

Complete all section checklists before considering the material understood.

### Overall Understanding

- [ ] I can explain multi-tenancy and workspace isolation
- [ ] I can trace a request through Service → Query → Model
- [ ] I can design a database table with proper constraints
- [ ] I can implement secure authentication
- [ ] I can implement and validate RBAC
- [ ] I can implement an invitation system with state machine
- [ ] I can write proper input validation and error handling
- [ ] I can model entity relationships correctly
- [ ] I can implement session management
- [ ] I can manage configuration securely
- [ ] I can write effective tests
- [ ] I can apply security best practices

### Next Steps

1. Read the actual source code files mentioned in each section
2. Run the examples in `/examples/`
3. Write a small feature using these patterns
4. Review the test suite for more examples

---

**Congratulations!** If you've understood all sections, you have a solid foundation for working with the BuildScale backend system.
