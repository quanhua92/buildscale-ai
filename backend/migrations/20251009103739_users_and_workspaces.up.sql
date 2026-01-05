-- Global user accounts for authentication across all workspaces.
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT,
    full_name TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- The top-level entity for multi-tenancy. Each workspace is an isolated environment.
CREATE TABLE workspaces (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    name TEXT NOT NULL,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT, -- An owner is required.
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Defines roles within a workspace for Role-Based Access Control (RBAC).
CREATE TABLE roles (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    UNIQUE(workspace_id, name)
);

-- Junction table linking users to workspaces and assigning them a specific role.
CREATE TABLE workspace_members (
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    PRIMARY KEY (workspace_id, user_id)
);

-- Indexes for performance optimization

-- Index for finding workspaces by owner (frequently queried in ownership operations)
CREATE INDEX idx_workspaces_owner_id ON workspaces(owner_id);

-- Index for finding roles within a workspace (constantly filtered in role-based queries)
CREATE INDEX idx_roles_workspace_id ON roles(workspace_id);

-- Index for finding workspaces by user (needed for user workspace listing)
CREATE INDEX idx_workspace_members_user_id ON workspace_members(user_id);

-- Index for finding members by workspace (needed for workspace member listing)
CREATE INDEX idx_workspace_members_workspace_id ON workspace_members(workspace_id);

-- Index for finding users by role (needed for membership and permission checks)
CREATE INDEX idx_workspace_members_role_id ON workspace_members(role_id);

-- Index for users email lookup (authentication - case-insensitive searches)
CREATE INDEX idx_users_email ON users(email);
