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
// Session extension limits (from config)
let config = Config::load()?;
if hours_to_extend > config.sessions.expiration_hours {
    return Err(Error::Validation(format!(
        "Cannot extend session by more than {} hours",
        config.sessions.expiration_hours
    )));
}

// Session operations are handled in services/sessions.rs
// Session duration is configurable via BUILDSCALE__SESSIONS__EXPIRATION_HOURS
```

## Input Validation System

The system includes comprehensive input validation utilities in `src/validation.rs` to ensure data integrity and security across all operations.

### Core Validation Functions

#### Email Validation (`validate_email`)
```rust
// Validation rules:
// - Required field (cannot be empty)
// - Maximum 254 characters (RFC 5321)
// - Valid email format with local part and domain
// - Must contain exactly one @ symbol
// - Cannot start or end with @
// - Local part: 1-64 characters, alphanumerics + .!#$%&'*+/=?^_`{|}~-
// - Domain part: valid domain format
// Returns: Result<()> with descriptive error messages
```

#### Password Validation (`validate_password`)
```rust
// Validation rules:
// - Minimum 8 characters (configurable minimum length)
// - Cannot be empty
// - No additional complexity requirements (only length-based)
// - Password hashing is handled separately with Argon2
// Returns: Result<()> with "Password must be at least X characters long" error
```

#### Workspace Name Validation (`validate_workspace_name`)
```rust
// Validation rules:
// - Required field (cannot be empty after trimming)
// - Maximum 100 characters
// - Leading/trailing whitespace trimmed
// - No other format restrictions (allows Unicode characters)
// Returns: Result<()> with descriptive error messages
```

#### Full Name Validation (`validate_full_name`)
```rust
// Validation rules:
// - Optional field (can be None or empty string)
// - If provided, maximum 100 characters after trimming
// - Leading/trailing whitespace trimmed
// - Allows Unicode characters for international names
// Returns: Result<()> with descriptive error messages
```

#### Session Token Validation (`validate_session_token`)
```rust
// Validation rules:
// - Required field (cannot be empty)
// - Must be a valid UUID v7 format
// - Uses UUID parsing for format validation
// - Used for session token format checking before database lookup
// Returns: Result<()> with "Session token cannot be empty" or "Invalid session token format"
```

#### UUID Validation (`validate_uuid`)
```rust
// Validation rules:
// - Required field (cannot be empty)
// - Must be valid UUID format (any version)
// - Returns parsed Uuid on success
// - Used for validating UUID parameters in API endpoints
// Returns: Result<Uuid> with "Invalid UUID format" error
```

### Utility Functions

#### String Sanitization (`sanitize_string`)
```rust
// Functionality:
// - Trims leading and trailing whitespace
// - Removes internal multiple consecutive spaces
// - Normalizes whitespace for consistent storage
// - Preserves Unicode characters
// Returns: String with cleaned whitespace
```

#### Required String Validation (`validate_required_string`)
```rust
// Functionality:
// - Checks for empty or whitespace-only strings
// - Applies trim() to remove whitespace
// - Returns cleaned string on success
// - Used for general string field validation
// Parameters: (input: &str, field_name: &str)
// Returns: Result<String> with "{field_name} cannot be empty" error
```

### Integration with Service Layer

The validation functions are used throughout the service layer:

```rust
// Usage examples in services:
use crate::validation::*;

// User registration
validate_email(&register_user.email)?;
validate_password(&register_user.password)?;
validate_full_name(&register_user.full_name)?;

// Workspace creation
validate_workspace_name(&request.name)?;

// Session validation
validate_session_token(session_token)?;
validate_uuid(workspace_id_str)?;
```

### Error Handling

All validation functions return `Result<()>` or `Result<T>` with descriptive error messages:

```rust
// Example error messages:
- "Email cannot be empty"
- "Invalid email format"
- "Password must be at least 8 characters long"
- "Workspace name must be less than 100 characters"
- "Session token cannot be empty"
- "Invalid session token format"
- "Invalid UUID format"
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

# Session configuration
BUILDSCALE__SESSIONS__EXPIRATION_HOURS=720  # Default: 30 days

# Optional: For sqlx CLI
DATABASE_URL=postgresql://buildscale:your_password@localhost:5432/buildscale
```

### Session Configuration

Session behavior is controlled by a single environment variable:

- `BUILDSCALE__SESSIONS__EXPIRATION_HOURS`: How long sessions remain valid (default: 720 = 30 days)
  - This value is used for both initial session creation AND maximum extension time

Example:
```bash
# Set session expiration to 7 days for testing
BUILDSCALE__SESSIONS__EXPIRATION_HOURS=168

# Set session expiration to 60 days for production
BUILDSCALE__SESSIONS__EXPIRATION_HOURS=1440
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