# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Development Commands

### Building and Testing
```bash
# Build the project
cargo build

# Build with optimizations for production
cargo build --release

# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run a specific test module
cargo test users::services::user_registration

# Run a specific test
cargo test test_user_registration_success

# Run examples
cargo run --example 01_hello
cargo run --example 02_users_management
cargo run --example 03_workspaces_management
```

### Database Operations
```bash
# Run database migrations
sqlx migrate run

# Reset database (use with caution)
sqlx migrate revert

# Check migration status
sqlx migrate info

# Install sqlx CLI (if not installed)
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

### Development Setup
```bash
# Copy environment configuration
cp .env.example .env

# Edit .env with your database configuration
# Required: BUILDSCALE__DATABASE__USER, PASSWORD, HOST, PORT, DATABASE
```

## Architecture Overview

This is a Rust backend implementing a **multi-tenant workspace-based RBAC system** with the following core characteristics:

### System Architecture
- **Multi-tenant Architecture**: Complete workspace isolation with shared users
- **Role-Based Access Control (RBAC)**: Four-tier role hierarchy (Admin > Editor > Member > Viewer)
- **Single Owner Model**: Each workspace has exactly one owner with full control
- **Three-Layer Architecture**: Clear separation of concerns across Service → Query → Model layers
  - **Service Layer**: Business logic, validation, authentication workflows
  - **Query Layer**: Type-safe database operations, CRUD functionality
  - **Model Layer**: Data structures, validation rules, type definitions

### Core Entities and Relationships
```
Users (1) ←→ (N) Workspaces
   ↓                   ↓
   └── Workspace Members ──→ Roles (per workspace)
   ↓
   └── User Sessions (authentication tokens)
```

- **Users**: Global accounts that can belong to multiple workspaces
- **Workspaces**: Isolated containers with exactly one owner
- **Roles**: Workspace-scoped permission definitions (default + custom)
- **Workspace Members**: Many-to-many relationship with specific role assignments
- **User Sessions**: Authentication tokens for user login sessions with expiration

### Module Structure

#### `/src/models/`
Data models and type definitions:
- `users.rs`: User entities (`User`, `NewUser`, `RegisterUser`, `UpdateUser`, `LoginUser`, `LoginResult`, `UserSession`, `NewUserSession`, `UpdateUserSession`)
- `workspaces.rs`: Workspace entities (`Workspace`, `NewWorkspace`, `UpdateWorkspace`)
- `roles.rs`: Role definitions and constants (`Role`, `WorkspaceRole` enum)
- `workspace_members.rs`: Member assignment entities
- `requests.rs`: API request models for complex operations

#### `/src/services/`
Business logic layer:
- `users.rs`: User registration, login, logout, session validation, password hashing, authentication, user management utilities
- `workspaces.rs`: Workspace creation, ownership transfer, access control
- `roles.rs`: Role creation, default role setup, role management
- `workspace_members.rs`: Member assignment and role validation
- `sessions.rs`: Session management, cleanup of expired sessions

#### `/src/queries/`
Data access layer:
- Direct database operations using SQLx
- CRUD operations for all entities
- `sessions.rs`: Session CRUD operations, validation, cleanup, user session queries
  - `create_session()`, `get_session_by_token_hash()`, `get_sessions_by_user()`
  - `delete_session()`, `delete_session_by_token_hash()`, `delete_sessions_by_user()`
  - `delete_expired_sessions()`, `is_session_valid()`, `get_valid_session_by_token_hash()`
  - `refresh_session()`, `hash_session_token()` - all session database operations
- Transaction handling for complex operations

#### `/tests/`
Comprehensive test suite with isolated test data management:
- `common/database.rs`: Test database setup with automatic cleanup
- Individual test modules for each service layer
- Parallel-safe test execution with unique prefixes

### Key Design Patterns

#### Simplified Workspace Creation
The system provides simplified APIs that handle complex multi-step operations:
```rust
// Creates workspace + default roles + owner as admin in one transaction
let result = create_workspace(&mut conn, request).await?;
// Returns: CompleteWorkspaceResult with workspace, roles, owner_membership, members
```

#### Role System with Type Safety
- Uses `WorkspaceRole` enum for type-safe role handling
- Centralized role constants: `ADMIN_ROLE`, `EDITOR_ROLE`, `MEMBER_ROLE`, `VIEWER_ROLE`
- Automatic default role creation for all workspaces
- Support for custom workspace-specific roles

#### Test Isolation System
Tests use a sophisticated isolation system:
- Each test gets unique database namespace: `"test_{test_name}"`
- Automatic cleanup before/after each test
- Parallel-safe test execution
- Helper methods for creating test data with proper prefixes

#### Password Security
- Argon2 password hashing with unique salts
- Minimum 8-character password requirement
- Secure password verification with constant-time comparison
- Password confirmation required during registration

#### Session Security (Current Implementation)
- Cryptographically secure random session tokens (256-bit randomness)
- **SHA-256 hashing** before database storage for security
- Configurable session expiration (default: 30 days, via BUILDSCALE__SESSIONS__EXPIRATION_HOURS)
- Automatic session cleanup for expired tokens
- Case-insensitive email lookup for user convenience
- Session invalidation on logout
- Session refresh functionality for extending sessions
- Constant-time comparison prevents timing attacks on token verification

#### Security Limitations and Considerations
- **Session Storage**: Sessions stored in database with SHA-256 hashed tokens
- **Token Hashing**: Tokens are one-way hashed (SHA-256) before storage, preventing token exposure in database backups
- **Token Security**: Plaintext tokens never stored in database, only transmitted securely to clients
- **Session Hijacking**: Tokens should be transmitted over HTTPS only
- **Concurrent Sessions**: Users can have multiple active sessions simultaneously
- **No Session Revocation on Password Change**: Manual revocation required for security operations
- **Database Dependency**: Session validation requires database connectivity

## Database Schema

### Core Tables
- `users`: Global user accounts with unique emails and hashed passwords
- `workspaces`: Workspace containers with single owner
- `roles`: Workspace-scoped role definitions
- `workspace_members`: Many-to-many user-workspace relationships with roles
- `user_sessions`: Authentication session tokens (SHA-256 hashed) with expiration tracking

### Key Constraints
- `users.email`: Globally unique
- `roles(workspace_id, name)`: Unique role names per workspace
- `workspace_members(workspace_id, user_id)`: One membership per user per workspace
- `user_sessions.token_hash`: Unique SHA-256 hashed session tokens
- Foreign key cascades: Deleting workspace deletes all roles and members; deleting user deletes all sessions

### Migration System
Uses SQLx migrations in `/migrations/` directory:
- `20251009102916_extensions.up.sql`: Database extensions setup
- `20251009103739_users_and_workspaces.up.sql`: Core tables and relationships
- `20251016221509_user_sessions.up.sql`: User authentication sessions table

## Service Layer APIs

### User Management
```rust
// Basic user registration with password hashing
register_user(&mut conn, RegisterUser) -> Result<User>

// User authentication and session creation
login_user(&mut conn, LoginUser) -> Result<LoginResult>

// Session validation and user retrieval
validate_session(&mut conn, session_token: &str) -> Result<User>

// Session termination
logout_user(&mut conn, session_token: &str) -> Result<()>

// Session expiration extension
refresh_session(&mut conn, session_token: &str, hours_to_extend: i64) -> Result<String>

// Advanced session management functions
cleanup_expired_sessions(&mut conn) -> Result<u64>
revoke_all_user_sessions(&mut conn, user_id: Uuid) -> Result<u64>
get_user_active_sessions(&mut conn, user_id: Uuid) -> Result<Vec<UserSession>>
user_has_active_sessions(&mut conn, user_id: Uuid) -> Result<bool>
revoke_session_by_token(&mut conn, session_token: &str) -> Result<()>
extend_all_user_sessions(&mut conn, user_id: Uuid, hours_to_extend: i64) -> Result<u64>

// Password utility functions
generate_password_hash(password: &str) -> Result<String>
verify_password(password: &str, hash: &str) -> Result<bool>
generate_session_token() -> Result<String>

// Combined user + workspace creation in single transaction
register_user_with_workspace(&mut conn, UserWorkspaceRegistrationRequest) -> Result<UserWorkspaceResult>

// Enhanced user utility methods
get_user_by_id(&mut conn, user_id: Uuid) -> Result<Option<User>>
update_password(&mut conn, user_id: Uuid, new_password: &str) -> Result<()>
is_email_available(&mut conn, email: &str) -> Result<bool>

// Session information access
get_session_info(&mut conn, session_token: &str) -> Result<Option<UserSession>>

// User session management convenience methods
get_user_active_sessions(&mut conn, user_id: Uuid) -> Result<Vec<UserSession>>
revoke_all_user_sessions(&mut conn, user_id: Uuid) -> Result<u64>

// JWT access token management
refresh_access_token(&mut conn, refresh_token: &str) -> Result<RefreshTokenResult>
```

### Session Query Layer (Database Operations)
The query layer provides type-safe database operations for session management:

```rust
// Core session CRUD operations
create_session(&mut conn, NewUserSession) -> Result<UserSession>
get_session_by_token_hash(&mut conn, token_hash: &str) -> Result<Option<UserSession>>
get_sessions_by_user(&mut conn, user_id: Uuid) -> Result<Vec<UserSession>>
refresh_session(&mut conn, session_id: Uuid, new_expires_at: DateTime<Utc>) -> Result<UserSession>

// Session deletion operations
delete_session(&mut conn, session_id: Uuid) -> Result<u64>
delete_session_by_token_hash(&mut conn, token_hash: &str) -> Result<u64>
delete_sessions_by_user(&mut conn, user_id: Uuid) -> Result<u64>
delete_expired_sessions(&mut conn) -> Result<u64>

// Session validation operations
is_session_valid(&mut conn, token_hash: &str) -> Result<bool>
get_valid_session_by_token_hash(&mut conn, token_hash: &str) -> Result<Option<UserSession>>

// Token hashing utility
hash_session_token(token: &str) -> String  // SHA-256 hash
```

### Authentication and Session Management
The authentication system provides secure user login with dual-token authentication (JWT access tokens + session refresh tokens):

#### Authentication Flow
1. **User Registration**: Users register with email and password (hashed with Argon2)
2. **Login**: Users authenticate with email/password credentials
3. **Token Generation**: Successful login generates two tokens:
   - **Access Token (JWT)**: Short-lived token (default: 15 minutes) used for API requests
   - **Refresh Token (Session)**: Long-lived token (default: 30 days) used to get new access tokens
4. **Token Storage**: Refresh token is hashed with SHA-256 before database storage for security
5. **API Authentication**: Each API call uses the JWT access token (via `Authorization: Bearer <token>` header)
6. **Token Refresh**: When access token expires, use refresh token to get a new access token
7. **Logout**: Refresh token hash is invalidated on logout

#### Authentication Models
```rust
// Login request
pub struct LoginUser {
    pub email: String,
    pub password: String,
}

// Login response with dual-token authentication
pub struct LoginResult {
    pub user: User,
    pub access_token: String,              // JWT access token (15 minutes)
    pub refresh_token: String,             // Session token (30 days)
    pub access_token_expires_at: DateTime<Utc>,  // JWT expiration
    pub refresh_token_expires_at: DateTime<Utc>,  // Session expiration
}

// Refresh token response
pub struct RefreshTokenResult {
    pub access_token: String,                      // New JWT access token
    pub refresh_token: Option<String>,             // New refresh token (rotated), None if within grace period
    pub expires_at: DateTime<Utc>,                 // When the new access token expires
}

// User session entity
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,  // SHA-256 hashed session token
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Session creation entity
pub struct NewUserSession {
    pub user_id: Uuid,
    pub token_hash: String,  // SHA-256 hashed session token
    pub expires_at: DateTime<Utc>,
}

// Session update entity
pub struct UpdateUserSession {
    pub expires_at: Option<DateTime<Utc>>,
}

// User registration entity
pub struct RegisterUser {
    pub email: String,
    pub password: String,
    pub confirm_password: String,
    pub full_name: Option<String>,
}

// User creation entity
pub struct NewUser {
    pub email: String,
    pub password_hash: String,
    pub full_name: Option<String>,
}

// User update entity
pub struct UpdateUser {
    pub password_hash: Option<String>,
    pub full_name: Option<String>,
}
```

#### JWT Authentication Features
- **JSON Web Tokens (JWT)**: Short-lived access tokens for API authentication (default: 15 minutes)
- **Bearer Token Authentication**: Standard `Authorization: Bearer <token>` header format
- **Automatic Token Expiration**: Access tokens expire quickly to enhance security
- **Token Refresh**: Use refresh token to get new access tokens without re-login
- **Configurable Expiration**: JWT expiration configurable via BUILDSCALE__JWT__ACCESS_TOKEN_EXPIRATION_MINUTES
- **Secure Token Storage**: JWT secret key stored in environment (BUILDSCALE__JWT__SECRET)

#### Session Management Features
- **Random HMAC-Signed Tokens**: 256-bit randomness with tamper-evident signature
- **Automatic Expiration**: Refresh tokens expire after configured duration (default: 30 days, via BUILDSCALE__SESSIONS__EXPIRATION_HOURS)
- **Session Refresh**: Extend session duration before expiration
- **Cleanup Service**: Automatic removal of expired sessions
- **Case-Insensitive Email**: Users can login with any email case variation
- **Secure Logout**: Immediate refresh token invalidation
- **Advanced Session Control**: Revoke all user sessions, extend multiple sessions, active session monitoring
- **Session Utilities**: Check for active sessions, retrieve user session list, token-based session revocation

### JWT Service
The JWT service module provides JSON Web Token generation and verification:

```rust
// Generate JWT access token
generate_jwt(user_id: Uuid, secret: &str, expiration_minutes: i64) -> Result<String>

// Verify JWT token and return claims
verify_jwt(token: &str, secret: &str) -> Result<Claims>

// Extract user_id from JWT token
get_user_id_from_token(token: &str, secret: &str) -> Result<Uuid>

// Authenticate JWT from Authorization header
authenticate_jwt_token(auth_header: Option<&str>, secret: &str) -> Result<Uuid>
```

**JWT Claims Structure**:
```rust
pub struct Claims {
    pub sub: String,  // user_id as string
    pub exp: i64,     // expiration time as Unix timestamp
    pub iat: i64,     // issued at time as Unix timestamp
}
```

#### Cookie-Based Authentication (Browser Support)

For web browser clients, the system supports cookie-based token storage and retrieval:

**Multi-Source Token Extraction**:
```rust
use backend::services::cookies::{
    extract_jwt_token,
    extract_refresh_token,
    CookieConfig,
};
use backend::services::jwt::authenticate_jwt_token_from_anywhere;

// Extract JWT from header OR cookie (priority: header > cookie)
let token = extract_jwt_token(
    Some("Bearer eyJhbGc..."),  // Authorization header
    Some("cookie_value"),        // Cookie fallback
)?;

// Authenticate from multiple sources
let user_id = authenticate_jwt_token_from_anywhere(
    auth_header,
    cookie_value,
    &secret,
)?;
```

**Cookie Building**:
```rust
use backend::services::cookies::{
    build_access_token_cookie,
    build_refresh_token_cookie,
    build_clear_token_cookie,
};

let config = CookieConfig::default();

// Build Set-Cookie headers
let access_cookie = build_access_token_cookie(&token, &config);
// Returns: "access_token=<token>; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=900"

let refresh_cookie = build_refresh_token_cookie(&refresh_token, &config);
// Returns: "refresh_token=<token>; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=2592000"

// Clear cookies (logout)
let clear_access = build_clear_token_cookie("access_token");
let clear_refresh = build_clear_token_cookie("refresh_token");
```

**Cookie Security Configuration**:
```rust
pub struct CookieConfig {
    pub access_token_name: String,      // "access_token"
    pub refresh_token_name: String,     // "refresh_token"
    pub http_only: bool,                // true (XSS protection)
    pub secure: bool,                   // true in production (HTTPS only)
    pub same_site: SameSite,            // Lax (default, allows links from emails/OAuth)
    pub path: String,                   // "/"
    pub domain: Option<String>,         // Optional (e.g., ".example.com")
}
```

**Default**: `SameSite::Lax` - Allows top-level navigations from external sites (emails, Slack, OAuth) while blocking CSRF attacks from embedded content (forms, AJAX, images).

**Cookie Service Module** (`src/services/cookies.rs`):
- `extract_jwt_token()`: Extract from header or cookie with priority
- `extract_refresh_token()`: Extract refresh token from cookie
- `authenticate_jwt_token_multi_source()`: Validate JWT from header or cookie
- `build_access_token_cookie()`: Create Set-Cookie header for JWT
- `build_refresh_token_cookie()`: Create Set-Cookie header for refresh token
- `build_clear_token_cookie()`: Create Set-Cookie header to clear token
- `CookieConfig`: Cookie security configuration

**Token Storage Options**:
- **Mobile/API clients**: Use `Authorization: Bearer <token>` header
- **Browser clients**: Use cookies with `HttpOnly`, `Secure`, `SameSite=Lax` flags
- **Priority**: Header takes precedence over cookie for backward compatibility


### Workspace Management
```rust
// Simplified workspace creation with automatic setup
create_workspace(&mut conn, CreateWorkspaceRequest) -> Result<CompleteWorkspaceResult>

// Workspace creation with initial team members
create_workspace_with_members(&mut conn, CreateWorkspaceWithMembersRequest) -> Result<CompleteWorkspaceResult>

// Ownership transfer with role management
update_workspace_owner(&mut conn, workspace_id, current_owner_id, new_owner_id) -> Result<Workspace>
```

### Role Management
```rust
// Create default roles (admin, editor, member, viewer) for workspace
create_default_roles(&mut conn, workspace_id) -> Result<Vec<Role>>

// Create custom workspace-specific role
create_single_role(&mut conn, NewRole) -> Result<Role>
```

## Configuration

### Environment Variables
Uses `BUILDSCALE_` prefix with double underscore separators:
- `BUILDSCALE__DATABASE__USER`: Database username
- `BUILDSCALE__DATABASE__PASSWORD`: Database password
- `BUILDSCALE__DATABASE__HOST`: Database host
- `BUILDSCALE__DATABASE__PORT`: Database port
- `BUILDSCALE__DATABASE__DATABASE`: Database name
- `BUILDSCALE__SESSIONS__EXPIRATION_HOURS`: Session expiration time in hours (default: 720 = 30 days)
  - Used for both initial session expiration AND maximum extension time
- `BUILDSCALE__JWT__SECRET`: Secret key for signing JWT tokens (minimum 32 characters recommended)
- `BUILDSCALE__JWT__ACCESS_TOKEN_EXPIRATION_MINUTES`: JWT access token expiration in minutes (default: 15)

### Configuration Loading
- Loads from `.env` file if present
- Overrides with environment variables
- Provides sensible defaults for development
- Supports `DATABASE_URL` for sqlx CLI

## Error Handling

### Error Hierarchy
```rust
pub enum Error {
    Sqlx(#[from] sqlx::Error),           // Database errors
    Validation(String),                   // Input validation errors
    NotFound(String),                      // Resource not found
    Forbidden(String),                    // Permission denied
    Conflict(String),                      // Resource conflicts
    Authentication(String),               // Authentication failures (invalid credentials)
    InvalidToken(String),                 // Invalid or expired session tokens
    SessionExpired(String),               // Session expiration errors
    Internal(String),                      // System errors
}
```

### Validation Rules
- **Users**: Email uniqueness, 8+ character passwords, password confirmation, case-insensitive email lookup
- **Authentication**: Email and password required, session tokens must be valid and non-expired, JWT tokens must be valid and non-expired
- **Workspaces**: 1-100 character names, owner must exist
- **Roles**: Unique names per workspace, 100 char name limit, 500 char description limit
- **Sessions**: Unique tokens, required expiration time, automatic cleanup of expired sessions
- **JWT**: Tokens must have valid signature, non-expired expiration time, valid UUID in sub field

## Testing Strategy

### Test Organization
- Unit tests for individual service functions
- Integration tests for complete workflows
- Database constraint testing
- Error scenario coverage
- Authentication flow testing (login, logout, session validation, refresh)

### Test Data Management
Uses `TestApp` and `TestDb` utilities in `/tests/common/database.rs`:
- Automatic test database initialization
- Unique test prefixes for isolation
- Helper methods for creating test entities
- Automatic cleanup on test completion

### Running Tests
```bash
# Run all tests
cargo test

# Run specific test file
cargo test tests/users/services/

# Run with output for debugging
cargo test -- --nocapture

# Run single test
cargo test test_user_registration_success
```

## Examples

### Available Examples
- `01_hello.rs`: Basic configuration loading
- `02_users_management.rs`: User registration, authentication, login, logout, and session management
- `03_workspaces_management.rs`: Complete workspace creation with roles and members

### Running Examples
```bash
cargo run --example 01_hello
cargo run --example 02_users_management
cargo run --example 03_workspaces_management
```

## Development Workflow: Code → Tests → Examples → Documentation

This codebase follows a strict 4-step development workflow for all features:

### 1. Code Implementation
- **Models**: Define data structures and validation in `/src/models/`
- **Services**: Implement business logic in `/src/services/`
- **Queries**: Add data access layer in `/src/queries/`
- **Error Handling**: Add comprehensive error types and validation

### 2. Test Coverage
- **Unit Tests**: Test individual service functions
- **Integration Tests**: Test complete workflows across services
- **Edge Cases**: Test validation rules and error scenarios
- **Test Isolation**: Use unique prefixes for parallel-safe testing
- **Location**: `/tests/` mirrors the `/src/` structure

### 3. Example Implementation
- **Demonstration**: Create practical examples showing feature usage
- **Real-world Scenarios**: Show common patterns and workflows
- **Verification**: Examples should run successfully and validate functionality
- **Location**: `/examples/` with clear naming (01_hello, 02_users_management, etc.)

### 4. Documentation Updates
- **API Documentation**: Update docstrings for all public functions
- **System Documentation**: Update `/docs/USERS_ROLES_WORKSPACES.md` with architectural changes
- **Usage Examples**: Add code examples to documentation
- **Role Constant Updates**: Include new roles in all relevant documentation sections

### 5. Final Comprehensive Test Workflow
After completing the 4-step development workflow, run the final validation:

```bash
# 1. Final Test Suite - Ensure all tests pass
cargo test

# 2. All Examples - Verify examples work correctly
cargo run --example 01_hello
cargo run --example 02_users_management
cargo run --example 03_workspaces_management

# 3. Project Build - Ensure no compilation errors
cargo build --release

# 4. Commit Changes - Save completed work
git add .
git commit -m "Commit message describing the completed feature"
```

### Quality Gates
- **All tests must pass** before proceeding to next step
- **Examples must run successfully** before documentation
- **Documentation must be comprehensive** before considering feature complete
- **No step should be skipped** - each builds on the previous

### Workflow Example: Member Role Implementation
```bash
# 1. Code: Added MEMBER_ROLE constant and WorkspaceRole::Member variant
# 2. Tests: Updated all tests to expect 4 default roles instead of 3
# 3. Examples: Updated workspace_management example to demonstrate Member role
# 4. Documentation: Updated comprehensive system documentation
```

### Quality Gates
- **All tests must pass** before proceeding to next step
- **Examples must run successfully** before documentation
- **Documentation must be comprehensive** before considering feature complete
- **No step should be skipped** - each builds on the previous

## Development Guidelines

### Code Organization
- Separate concerns: models (data), services (business logic), queries (data access)
- Use type-safe enums for role management
- Centralized constants for role names
- Comprehensive error handling with specific error types

### Database Patterns
- Use transactions for multi-step operations
- Parameterized queries to prevent SQL injection
- Database constraints for data integrity
- Cascade operations for data consistency

### Testing Patterns
- Use test prefixes for data isolation
- Clean up test data automatically
- Test both success and failure scenarios
- Use helper methods for common test setup

### Security Considerations
- Argon2 password hashing with unique salts
- Workspace data isolation
- Role-based access control
- Input validation and sanitization