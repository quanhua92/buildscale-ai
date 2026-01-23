← [Back to Index](./README.md)

# System Architecture

Multi-tenant workspace-based RBAC architecture with clear separation of concerns

## Overview

**Core Architecture**: Multi-tenant Rust backend implementing workspace-based isolation with role-based access control (RBAC).

**Key Characteristics**:
- **Workspace Isolation**: Complete data separation between workspaces
- **RBAC System**: Four-tier role hierarchy (Admin > Editor > Member > Viewer)
- **Single Owner Model**: Each workspace has exactly one owner
- **Flexible Membership**: Users can belong to multiple workspaces with different roles
- **Comprehensive Permission System**: Fine-grained permissions across workspace, content, and member management categories

## Module Structure

```
src/
├── lib.rs           # Public exports and configuration loading
├── main.rs          # Application entry point
├── config.rs        # Environment configuration with BUILDSCALE_ prefix
├── database.rs      # Database connection pooling
├── error.rs         # Comprehensive error handling
├── validation.rs    # Input validation utilities (email, password, workspace names, etc.)
├── models/          # Data structures and validation
│   ├── mod.rs       # Module exports
│   ├── users.rs     # User, LoginUser, UserSession, RegisterUser
│   ├── workspaces.rs # Workspace, NewWorkspace, UpdateWorkspace
│   ├── roles.rs     # Role, WorkspaceRole enum, role constants
│   ├── workspace_members.rs # WorkspaceMember assignments
│   ├── invitations.rs # Invitation entities and validation
│   ├── permissions.rs # Comprehensive permission system with role mappings
│   └── requests.rs  # Complex API request models
├── services/        # Business logic layer
│   ├── mod.rs       # Module exports
│   ├── users.rs     # User registration, login, session management
│   ├── workspaces.rs # Workspace creation, ownership transfer
│   ├── roles.rs     # Role creation, default role setup
│   ├── workspace_members.rs # Member assignment and validation
│   ├── invitations.rs # Invitation creation, acceptance, revocation
│   └── sessions.rs  # Session management, cleanup, monitoring
└── queries/         # Database operations layer (SQLx)
    ├── mod.rs       # Module exports
    ├── users.rs     # User CRUD operations
    ├── workspaces.rs # Workspace CRUD operations
    ├── roles.rs     # Role CRUD operations
    ├── workspace_members.rs # Member CRUD operations
    ├── invitations.rs # Invitation CRUD operations
    └── sessions.rs  # Session CRUD operations
```

## Entity Relationships

```
Users (1) ←→ (N) Workspaces
   ↓                   ↓
   └── Workspace Members ──→ Roles (per workspace)
   ↓
   └── User Sessions (authentication tokens)
   ↓
   └── Workspace Invitations (token-based)
```

**Key Relationships**:
- **Users → Workspaces**: Many-to-many via `workspace_members`
- **Workspaces → Roles**: One-to-many (workspace-scoped roles)
- **Users → Sessions**: One-to-many (multiple active sessions allowed)
- **Workspaces → Invitations**: One-to-many (workspace-specific invitations)

## Permission System

**20 Hardcoded Permissions** across 3 categories:

### Workspace Permissions (8)
- `workspace:read`, `workspace:write`, `workspace:delete`
- `workspace:manage_members`, `workspace:manage_settings`
- `workspace:invite_members`, `workspace:view_activity_log`, `workspace:export_data`

### Content Permissions (8)
- `content:create`, `content:read_own`, `content:read_all`
- `content:update_own`, `content:update_all`
- `content:delete_own`, `content:delete_all`, `content:comment`

### Member Permissions (4)
- `members:add`, `members:remove`, `members:update_roles`, `members:view`

**Role Hierarchy**:
- **Admin**: All permissions (full workspace control)
- **Editor**: Content creation and management permissions
- **Member**: Basic content participation permissions
- **Viewer**: Read-only access permissions

## Data Flow Architecture

### Three-Layer Architecture

1. **Service Layer** (`services/`)
   - Business logic and validation
   - Transaction coordination
   - Permission enforcement
   - Error handling and conversion

2. **Query Layer** (`queries/`)
   - Type-safe database operations (SQLx)
   - CRUD operations for all entities
   - Transaction management
   - Raw SQL queries with parameter binding

3. **Model Layer** (`models/`)
   - Data structures with Serde serialization
   - Validation rules and constraints
   - Database entity mappings
   - Request/response models for APIs

### Request Processing Flow

```
HTTP Request → Service Layer → Query Layer → Database
                ↓                ↓             ↓
         Permission Check → SQLx Query → PostgreSQL
                ↓                ↓             ↓
         Response Format → Result Mapping → Entity Model
```

## Database Design

### Core Schema

#### `users` - Global Authentication
```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    full_name TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

#### `workspaces` - Tenant Isolation
```sql
CREATE TABLE workspaces (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    name TEXT NOT NULL,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

#### `roles` - Workspace-Specific RBAC
```sql
CREATE TABLE roles (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    UNIQUE(workspace_id, name)
);
```

#### `workspace_members` - User-Workplace Junction
```sql
CREATE TABLE workspace_members (
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    PRIMARY KEY (workspace_id, user_id)
);
```

#### `user_sessions` - Authentication Tokens
```sql
CREATE TABLE user_sessions (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token TEXT UNIQUE NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

#### `workspace_invitations` - Secure Member Onboarding
```sql
CREATE TABLE workspace_invitations (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    invited_email TEXT NOT NULL,
    invited_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    invitation_token TEXT UNIQUE NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'accepted', 'expired', 'revoked')),
    expires_at TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### Database Relationships

#### Foreign Key Constraints
- **workspaces.owner_id → users.id**: Single owner per workspace (RESTRICT)
- **roles.workspace_id → workspaces.id**: Roles scoped to workspace (CASCADE)
- **workspace_members**: Composite PK linking users to workspaces with roles (CASCADE)
- **user_sessions.user_id → users.id**: User authentication sessions (CASCADE)
- **workspace_invitations**: Multi-FK to workspace, user, and role (CASCADE)

#### Key Constraints
- **users.email**: Global uniqueness across all workspaces
- **roles(workspace_id, name)**: Unique role names per workspace
- **workspace_members(workspace_id, user_id)**: One membership per user per workspace
- **user_sessions.token**: Globally unique session tokens
- **workspace_invitations.invitation_token**: Globally unique invitation tokens

### Performance Indexes

```sql
-- User and authentication lookups
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_user_sessions_token ON user_sessions(token);
CREATE INDEX idx_user_sessions_user_id ON user_sessions(user_id);
CREATE INDEX idx_user_sessions_expires_at ON user_sessions(expires_at);

-- Workspace access patterns
CREATE INDEX idx_workspaces_owner_id ON workspaces(owner_id);
CREATE INDEX idx_roles_workspace_id ON roles(workspace_id);
CREATE INDEX idx_workspace_members_workspace_id ON workspace_members(workspace_id);
CREATE INDEX idx_workspace_members_user_id ON workspace_members(user_id);
CREATE INDEX idx_workspace_members_role_id ON workspace_members(role_id);

-- Invitation system performance
CREATE INDEX idx_workspace_invitations_workspace ON workspace_invitations(workspace_id);
CREATE INDEX idx_workspace_invitations_email ON workspace_invitations(invited_email);
CREATE INDEX idx_workspace_invitations_token ON workspace_invitations(invitation_token);
CREATE INDEX idx_workspace_invitations_status ON workspace_invitations(status);
CREATE INDEX idx_workspace_invitations_expires_at ON workspace_invitations(expires_at);
```

### Key Features
- **UUID v7**: Time-ordered unique identifiers for all entities
- **Cascade Deletes**: Data consistency across related tables
- **Performance Indexes**: Optimized for common query patterns
- **Foreign Key Constraints**: Referential integrity enforcement
- **RESTRICT on Owner**: Prevents accidental workspace deletion

## Security Architecture

**Authentication**:
- Session-based authentication with random HMAC-signed tokens (256-bit randomness)
- Argon2 password hashing with unique salts
- Configurable session expiration (default: 30 days) with refresh capability via BUILDSCALE__SESSIONS__EXPIRATION_HOURS

**Authorization**:
- Role-based access control with hardcoded permissions
- Workspace-level data isolation
- Invitation-based member onboarding

**Validation**:
- Input validation at model layer
- Email format verification
- Password strength requirements
- Token format validation (hex:hex format with HMAC signature)