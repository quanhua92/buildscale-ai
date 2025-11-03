# Configuration Reference

This document serves as a central reference for all configurable values, constraints, and defaults in the system. When code changes affect these values, update this document first.

## Authentication Configuration

### Password Requirements
```rust
// Hardcoded in services/users.rs (line with password validation)
if new_password.len() < 8 {
    return Err(Error::Validation("Password must be at least 8 characters long".to_string()));
}
```

### Session Management
```rust
// Session extension limits (hardcoded in services/users.rs)
if hours_to_extend > 168 {
    return Err(Error::Validation("Cannot extend session by more than 168 hours (7 days)".to_string()));
}

// Session operations are handled in services/sessions.rs
// No hardcoded constants for session duration - passed as parameters
```

## Workspace Configuration

### Workspace Name Limits
```rust
// Hardcoded in validation.rs
pub fn validate_workspace_name(name: &str) -> Result<()> {
    if name.len() > 100 {
        return Err(Error::Validation("Workspace name must be less than 100 characters".to_string()));
    }
    // Additional validation rules apply
}
```

## Role System

### Default Roles
```rust
// Defined in models/roles.rs
pub const ADMIN_ROLE: &str = "admin";
pub const EDITOR_ROLE: &str = "editor";
pub const MEMBER_ROLE: &str = "member";
pub const VIEWER_ROLE: &str = "viewer";
pub const DEFAULT_ROLES: [&str; 4] = [ADMIN_ROLE, EDITOR_ROLE, MEMBER_ROLE, VIEWER_ROLE];
```

### Permission System
```rust
// Permission categories (from models/permissions.rs)
// - Workspace permissions: 8 permissions (workspace_permissions::READ, WRITE, DELETE, etc.)
// - Content permissions: 8 permissions (content_permissions::CREATE, READ_OWN, etc.)
// - Member permissions: 4 permissions (member_permissions::ADD_MEMBERS, etc.)
// Total: 20 permissions in ALL_PERMISSIONS array
```

## Invitation System Configuration

### Expiration Settings
```rust
// From models/invitations.rs
pub const DEFAULT_INVITATION_EXPIRATION_HOURS: i64 = 168; // 7 days
pub const MAX_INVITATION_EXPIRATION_HOURS: i64 = 720;   // 30 days
```

### Status Constants
```rust
// From models/invitations.rs
pub const INVITATION_STATUS_PENDING: &str = "pending";
pub const INVITATION_STATUS_ACCEPTED: &str = "accepted";
pub const INVITATION_STATUS_EXPIRED: &str = "expired";
pub const INVITATION_STATUS_REVOKED: &str = "revoked";
```

## Database Configuration

### Connection Settings
```rust
// Connection pool configuration
pub const MAX_CONNECTIONS: u32 = 10;
pub const MIN_CONNECTIONS: u32 = 1;
pub const CONNECTION_TIMEOUT: u64 = 30; // seconds
```

### Performance Indexes
```sql
-- Core performance indexes (see migrations/ for current schema)
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_user_sessions_token ON user_sessions(token);
CREATE INDEX idx_user_sessions_expires_at ON user_sessions(expires_at);
CREATE INDEX idx_workspaces_owner_id ON workspaces(owner_id);
CREATE INDEX idx_roles_workspace_id ON roles(workspace_id);
CREATE INDEX idx_workspace_members_workspace_id ON workspace_members(workspace_id);
CREATE INDEX idx_workspace_members_user_id ON workspace_members(user_id);
CREATE INDEX idx_workspace_invitations_token ON workspace_invitations(invitation_token);
```

## Development Configuration

### Environment Variables
```bash
# Database configuration (see .env.example)
BUILDSCALE__DATABASE__USER=buildscale
BUILDSCALE__DATABASE__PASSWORD=your_password
BUILDSCALE__DATABASE__HOST=localhost
BUILDSCALE__DATABASE__PORT=5432
BUILDSCALE__DATABASE__DATABASE=buildscale

# Optional: For sqlx CLI
DATABASE_URL=postgresql://buildscale:your_password@localhost:5432/buildscale
```

### Logging Configuration
```rust
// Log levels for development
pub const DEFAULT_LOG_LEVEL: &str = "info";
pub const DEBUG_LOG_LEVEL: &str = "debug";
```

## For Developers

### Finding Current Values

1. **Permission Counts**: Check `src/models/permissions.rs`
   ```bash
   # Count permissions programmatically
   grep -c "pub const" src/models/permissions.rs
   ```

2. **Database Constraints**: Check migration files
   ```bash
   # View current schema constraints
   sqlx migrate info
   psql -d buildscale -c "\d users" "\d workspaces" "\d roles"
   ```

3. **Configuration Constants**: Search source code
   ```bash
   # Find configuration constants
   grep -r "pub const" src/
   grep -r "const.*=.*[0-9]" src/
   ```

### Updating Documentation

When changing code that affects these values:

1. Update this CONFIGURATION.md first
2. Update cross-references in other documentation files
3. Update any examples that use specific values
4. Test that examples still work with new values

### Validation Scripts

Create validation scripts to check documentation against code:

```bash
#!/bin/bash
# docs/validate.sh

echo "Validating documentation against code..."

# Check permission counts
PERMISSION_COUNT=$(grep -c "pub const.*permissions" src/models/permissions.rs)
echo "Found $PERMISSION_COUNT permissions in code"

# Check role constants
ROLE_COUNT=$(grep -c "const.*ROLE.*=" src/models/roles.rs)
echo "Found $ROLE_COUNT role constants in code"

# Add more validation checks as needed
```

## Version Requirements

### Current Compatible Versions
- **Rust**: Current stable version (check Cargo.toml for exact requirement)
- **PostgreSQL**: Version 13+ (check .env.example for current recommendation)
- **sqlx CLI**: Latest version with rustls feature

### Updating Requirements
1. Test with newer versions
2. Update .env.example if needed
3. Update installation instructions in README.md
4. Update this configuration document

## Best Practices

1. **Centralize Configuration**: Keep configurable values in one place
2. **Use Descriptive Names**: Avoid magic numbers, use named constants
3. **Document Ranges**: Use ranges instead of exact numbers when possible
4. **Validate Constraints**: Ensure constraints are enforced at both database and application level
5. **Test Limits**: Include tests for boundary conditions and limits

---

*For the most current values, always check the source code files referenced in each section.*