← [Back to Index](./README.md) | **API Reference**: [REST API Guide](./REST_API_GUIDE.md#authentication)

# Authentication & Security

Dual-token authentication system with JWT access tokens (short-lived, 15 minutes) and session refresh tokens (long-lived, 30 days), Argon2 password hashing, and configurable expiration periods.

## Authentication Flow

### User Registration Flow
```mermaid
sequenceDiagram
    participant Client
    participant API
    participant Database

    Client->>API: POST /register (email, password, confirm_password)
    API->>API: Validate input (email format, password length >= 12)
    API->>API: Hash password with Argon2
    API->>Database: INSERT INTO users (email, password_hash, ...)
    Database-->>API: User created
    API-->>Client: 201 Created (User object)
```

### User Login Flow
```mermaid
sequenceDiagram
    participant Client
    participant API
    participant Database

    Client->>API: POST /login (email, password)
    API->>API: Validate input
    API->>Database: SELECT user WHERE email = ? (case-insensitive)
    Database-->>API: User record (if exists)
    API->>API: Verify password against hash
    API->>API: Generate JWT access token (15 min)
    API->>API: Generate random HMAC-signed refresh token (30 days)
    API->>Database: INSERT INTO user_sessions (user_id, token, expires_at)
    Database-->>API: Session created
    API-->>Client: 200 OK (access_token, refresh_token, expires_at, user_data)
    Client->>Client: Store tokens securely
```

### Session Validation Flow (JWT Access Token)
```mermaid
sequenceDiagram
    participant Client
    participant API
    participant Database

    Client->>API: Request with Authorization: Bearer <access_token>
    API->>API: Extract and validate JWT format
    API->>API: Verify JWT signature and expiration
    API->>API: Extract user_id from JWT claims
    API-->>API: User authenticated, proceed with request
    API-->>Client: Response with requested data
```

### JWT Access Token Refresh Flow
```mermaid
sequenceDiagram
    participant Client
    participant API
    participant Database

    Client->>API: POST /auth/refresh (refresh_token)
    API->>Database: SELECT session WHERE token = ? AND expires_at > NOW()
    Database-->>API: Valid session
    API->>API: Generate new JWT access token
    API-->>Client: 200 OK (new_access_token, expires_at)
```

### Session Refresh Flow
```mermaid
sequenceDiagram
    participant Client
    participant API
    participant Database

    Client->>API: POST /session/refresh (session_token, hours_to_extend)
    API->>Database: SELECT session WHERE token = ? AND expires_at > NOW()
    Database-->>API: Valid session
    API->>API: Calculate new expiration time
    API->>Database: UPDATE session SET expires_at = ? WHERE id = ?
    Database-->>API: Session updated
    API-->>Client: 200 OK (new_expires_at)
```

### Logout Flow
```mermaid
sequenceDiagram
    participant Client
    participant API
    participant Database

    Client->>API: POST /logout (session_token)
    API->>Database: DELETE FROM user_sessions WHERE token = ?
    Database-->>API: Session deleted
    API-->>Client: 200 OK
    Client->>Client: Clear stored session token
```

### Session Cleanup Flow (Automated)
```mermaid
sequenceDiagram
    participant System
    participant Database

    loop Every 24 hours
        System->>Database: DELETE FROM user_sessions WHERE expires_at < NOW()
        Database-->>System: Count of expired sessions deleted
        System->>System: Log cleanup metrics
    end
```

## Core API

### User Authentication
```rust
// User registration (minimum 12 character password, email validation)
pub async fn register_user(conn: &mut DbConn, register_user: RegisterUser) -> Result<User>

// User authentication and session creation
pub async fn login_user(conn: &mut DbConn, login_user: LoginUser) -> Result<LoginResult>

// Session validation and user retrieval
pub async fn validate_session(conn: &mut DbConn, session_token: &str) -> Result<User>

// Session termination
pub async fn logout_user(conn: &mut DbConn, session_token: &str) -> Result<()>

// Session extension
pub async fn refresh_session(conn: &mut DbConn, session_token: &str, hours_to_extend: i64) -> Result<String>

// JWT access token refresh
pub async fn refresh_access_token(conn: &mut DbConn, refresh_token: &str) -> Result<RefreshTokenResult>
```

### Session Management
```rust
// Advanced session operations
pub async fn cleanup_expired_sessions(conn: &mut DbConn) -> Result<u64>
pub async fn revoke_all_user_sessions(conn: &mut DbConn, user_id: Uuid) -> Result<u64>
pub async fn get_user_active_sessions(conn: &mut DbConn, user_id: Uuid) -> Result<Vec<UserSession>>
pub async fn revoke_session_by_token(conn: &mut DbConn, session_token: &str) -> Result<()>
pub async fn extend_all_user_sessions(conn: &mut DbConn, user_id: Uuid, hours_to_extend: i64) -> Result<u64>
```

### Password Utilities
```rust
pub fn generate_password_hash(password: &str) -> Result<String>
pub fn verify_password(password: &str, hash: &str) -> Result<bool>
pub fn generate_session_token() -> Result<String>
```

## Data Models

### Core Authentication Models
```rust
pub struct LoginUser {
    pub email: String,     // Case-insensitive lookup
    pub password: String,  // Plain text verification
}

pub struct LoginResult {
    pub user: User,                        // Authenticated user
    pub access_token: String,              // JWT access token (15 minutes)
    pub refresh_token: String,             // Session token (30 days)
    pub access_token_expires_at: DateTime<Utc>,  // JWT expiration
    pub refresh_token_expires_at: DateTime<Utc>,  // Session expiration
}

pub struct RefreshTokenResult {
    pub access_token: String,                      // New JWT access token
    pub refresh_token: Option<String>,             // New refresh token (rotated), None if within grace period
    pub expires_at: DateTime<Utc>,                 // When the new access token expires
}

pub struct UserSession {
    pub id: Uuid,                    // Session primary key
    pub user_id: Uuid,               // Session owner
    pub token: String,               // Random HMAC-signed token (refresh token)
    pub expires_at: DateTime<Utc>,   // Expiration time
    pub created_at: DateTime<Utc>,   // Creation time
    pub updated_at: DateTime<Utc>,   // Last update
}
```

## Security Features

### JWT Access Token Security
- **Short-Lived Tokens**: 15-minute expiration reduces window for token misuse
- **Bearer Token Authentication**: Standard RFC 6750 compliant format
- **Signature Verification**: HMAC-SHA256 signing prevents token tampering
- **No Database Lookup**: JWT validation is stateless and fast
- **Automatic Expiration**: Tokens expire quickly, forcing regular refresh

### Session Refresh Token Security
- **Random Tokens**: 256-bit cryptographic randomness
- **Long-Lived Tokens**: 30-day expiration for user convenience
- **SHA-256 Hashing**: Tokens hashed before database storage for security
- **Database Storage**: Revocable token hashes stored in database (plaintext tokens never stored)
- **Integrity Verification**: SHA-256 one-way hashing prevents token exposure in database backups
- **Constant-Time Comparison**: Prevents timing attacks on token verification
- **Configurable Expiration**: Default session duration with automatic cleanup
- **Case-Insensitive Email**: User-friendly login experience
- **Multi-Device Support**: Users can maintain concurrent sessions
- **Session Revocation**: Immediate token invalidation on logout

## Validation Rules

| Input | Requirement | Error Message |
|--------|-------------|---------------|
| Email | Required, valid format | "Email cannot be empty" / "Invalid email format" |
| Password | Minimum 12 characters, patterns | "Password must be at least 12 characters long" |
| Session Token | Required, non-empty | "Session token cannot be empty" |
| Login | Valid credentials | "Invalid email or password" |
| Session | Non-expired token | "Invalid or expired session token" |

## Error Types

```rust
Authentication(String)     // Invalid credentials
InvalidToken(String)      // Invalid/expired tokens
SessionExpired(String)     // Session expiration
Validation(String)        // Input validation errors
```

## Database Schema

```sql
CREATE TABLE user_sessions (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token TEXT UNIQUE NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Performance indexes
CREATE INDEX idx_user_sessions_user_id ON user_sessions(user_id);
CREATE INDEX idx_user_sessions_token ON user_sessions(token);
CREATE INDEX idx_user_sessions_expires_at ON user_sessions(expires_at);
```

## Usage Examples

### Basic Authentication with JWT
```rust
// Login and get both tokens
let login_result = login_user(&mut conn, LoginUser {
    email: "user@example.com".to_string(),
    password: "SecurePass123!".to_string(),
}).await?;

// Use JWT access token for API requests (in Authorization header)
// Authorization: Bearer <login_result.access_token>

// When access token expires, refresh it using refresh token
let new_token_result = refresh_access_token(&mut conn, &login_result.refresh_token).await?;

// Logout (invalidates refresh token)
logout_user(&mut conn, &login_result.refresh_token).await?;
```

### Session Management
```rust
// Get active sessions
let sessions = get_user_active_sessions(&mut conn, user.id).await?;

// Extend all sessions by default duration
let extended = extend_all_user_sessions(&mut conn, user.id, 168).await?;

// Force logout from all devices
let revoked = revoke_all_user_sessions(&mut conn, user.id).await?;

// Cleanup expired sessions
let cleaned = cleanup_expired_sessions(&mut conn).await?;
```

## Cookie-Based Authentication

For web browser clients, tokens can be stored in cookies for seamless authentication without manual header management.

### Token Storage Options

**Access Token (JWT, 15 minutes)**:
- Authorization header: `Authorization: Bearer <token>` (mobile apps, SPAs)
- Cookie: `access_token=<token>` (traditional web apps)

**Refresh Token (HMAC-signed, 30 days)**:
- Request body: POST /auth/refresh with token (API clients)
- Cookie: `refresh_token=<token>` (automatic browser sending)

### Multi-Source Token Extraction

The authentication system supports both headers and cookies with priority:

```rust
use backend::services::jwt::authenticate_jwt_token_from_anywhere;

// Works for both API and browser clients
let user_id = authenticate_jwt_token_from_anywhere(
    auth_header,           // Priority 1: Authorization header
    cookie_value,          // Priority 2: Cookie fallback
    &secret,
)?;
```

**Priority Order**:
1. Authorization header (API/mobile clients take precedence)
2. Cookie (browser clients fallback)

### Cookie Security

All cookies include security flags:

```rust
pub struct CookieConfig {
    pub http_only: bool,        // true - Prevents JavaScript access (XSS protection)
    pub secure: bool,           // true - HTTPS only (production)
    pub same_site: SameSite,    // Lax (default) - CSRF protection while allowing external links
    pub path: String,           // "/" - Apply to all paths
    pub domain: Option<String>,  // Optional - e.g., ".example.com"
}
```

**SameSite=Lax** (default): Allows users to click links from emails, Slack, or OAuth redirects while blocking CSRF attacks from embedded content (forms, AJAX, images).

**Example Cookie Header**:
```http
Set-Cookie: access_token=eyJhbGc...; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=900
Set-Cookie: refresh_token=a1b2c3...; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=2592000
```

### Cookie Utilities

**Extract tokens**:
```rust
use backend::services::cookies::{
    extract_jwt_token,
    extract_refresh_token,
};

// Extract access token from header or cookie
let token = extract_jwt_token(
    Some("Bearer eyJhbGc..."),
    Some("cookie_value"),
)?;

// Extract refresh token from cookie
let refresh_token = extract_refresh_token(
    Some("cookie_value")
)?;
```

**Build cookies**:
```rust
use backend::services::cookies::{
    build_access_token_cookie,
    build_refresh_token_cookie,
    build_clear_token_cookie,
    CookieConfig,
};

let config = CookieConfig::default();

// Build Set-Cookie header for access token
let access_cookie = build_access_token_cookie(&token, &config);

// Build Set-Cookie header for refresh token
let refresh_cookie = build_refresh_token_cookie(&refresh_token, &config);

// Build cookie to clear token (logout)
let clear_cookie = build_clear_token_cookie("access_token");
```

### Login Flow with Cookies

For browser clients, the login endpoint sets cookies automatically:

```http
POST /login HTTP/1.1
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "SecurePass123!"
}

---

HTTP/1.1 200 OK
Set-Cookie: access_token=eyJhbGc...; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=900
Set-Cookie: refresh_token=a1b2c3...; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=2592000
Content-Type: application/json

{
  "user": {...},
  "access_token": "eyJhbGc...",
  "refresh_token": "a1b2c3...",
  "access_token_expires_at": "2025-01-06T10:30:00Z",
  "refresh_token_expires_at": "2025-02-05T10:00:00Z"
}
```

**Browser automatically includes cookies** in subsequent requests:
```http
GET /api/workspaces HTTP/1.1
Cookie: access_token=eyJhbGc...; refresh_token=a1b2c3...
```

### Logout Flow with Cookies

Logout clears cookies by setting Max-Age=0:

```http
POST /logout HTTP/1.1

---

HTTP/1.1 200 OK
Set-Cookie: access_token=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0
Set-Cookie: refresh_token=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0
```

### Configuration

Cookie settings are configured in `src/config.rs`:

```rust
pub struct Config {
    pub database: DatabaseConfig,
    pub sessions: SessionsConfig,
    pub jwt: JwtConfig,
    pub cookies: CookieConfig,  // NEW
}
```

**Default CookieConfig**:
```rust
CookieConfig {
    access_token_name: "access_token",
    refresh_token_name: "refresh_token",
    http_only: true,           // XSS protection
    secure: false,              // Set to true in production
    same_site: SameSite::Lax,   // CSRF protection + allows email/OAuth links
    path: "/",
    domain: None,
}
```

**Environment Configuration** (future):
```bash
# Cookie settings (when env support is added)
BUILDSCALE__COOKIES__SECURE=true
BUILDSCALE__COOKIES__SAME_SITE=Strict
BUILDSCALE__COOKIES__DOMAIN=.example.com
```

## Role-Based Access Control (RBAC)

Multi-tier role hierarchy with comprehensive permissions across workspace, content, and member management categories.

### Role Hierarchy

| Role | Description | Scope |
|------|-------------|-------|
| **Admin** | Full workspace control | All permissions |
| **Editor** | Content creation and editing | Content management + workspace access |
| **Member** | Basic content participation | Own content + workspace access |
| **Viewer** | Read-only access | View-only permissions |

### Permission Categories

#### Workspace Permissions
- `workspace:read` - View workspace
- `workspace:write` - Modify workspace
- `workspace:delete` - Delete workspace
- `workspace:manage_members` - Manage members
- `workspace:manage_settings` - Manage settings
- `workspace:invite_members` - Invite members
- `workspace:view_activity_log` - View activity log
- `workspace:export_data` - Export data

#### Content Permissions
- `content:create` - Create content
- `content:read_own` / `content:read_all` - Read content
- `content:update_own` / `content:update_all` - Update content
- `content:delete_own` / `content:delete_all` - Delete content
- `content:comment` - Comment on content

#### Member Permissions
- `members:add` - Add members
- `members:remove` - Remove members
- `members:update_roles` - Update roles
- `members:view` - View members

### Role Permission Matrix

| Permission | Admin | Editor | Member | Viewer |
|------------|--------|--------|--------|--------|
| `workspace:read` | ✓ | ✓ | ✓ | ✓ |
| `workspace:write` | ✓ | ✓ | ✗ | ✗ |
| `workspace:delete` | ✓ | ✗ | ✗ | ✗ |
| `workspace:manage_members` | ✓ | ✗ | ✗ | ✗ |
| `content:create` | ✓ | ✓ | ✓ | ✗ |
| `content:update_all` | ✓ | ✓ | ✗ | ✗ |
| `members:add` | ✓ | ✗ | ✗ | ✗ |

### Core APIs

```rust
// Create default 4-tier role system for workspace
pub async fn create_default_roles(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<Role>>

// Permission validation
pub fn role_has_permission(role: &str, permission: &str) -> bool
pub async fn validate_workspace_permission(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
    permission: &str,
) -> Result<()>

// Role lookup
pub async fn get_role_by_name(conn: &mut DbConn, workspace_id: Uuid, role_name: &str) -> Result<Role>
pub async fn list_workspace_roles(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<Role>>
```

---

## Workspace Invitation System

Secure token-based invitation system with role assignments for workspace member onboarding.

### Key Features

- **UUID v7 Tokens**: Secure invitation tokens with configurable expiration
- **Role Assignment**: Direct role assignment on invitation acceptance
- **Permission Validation**: Requires `INVITE_MEMBERS` permission
- **State Management**: pending → accepted/expired/revoked lifecycle
- **Bulk Operations**: Support for inviting multiple users

### Invitation Status

| Status | Description |
|--------|-------------|
| `pending` | Awaiting user response |
| `accepted` | User joined workspace |
| `expired` | Past expiration date |
| `revoked` | Cancelled by sender/admin |

### Core APIs

```rust
// Create invitation with role assignment
pub async fn create_invitation(
    conn: &mut DbConn,
    request: CreateInvitationRequest,
    inviter_id: Uuid,
) -> Result<CreateInvitationResponse>

// Accept invitation and create membership
pub async fn accept_invitation(
    conn: &mut DbConn,
    request: AcceptInvitationRequest,
    user_id: Uuid,
) -> Result<AcceptInvitationResponse>

// Revoke pending invitation
pub async fn revoke_invitation(
    conn: &mut DbConn,
    request: RevokeInvitationRequest,
    revoker_id: Uuid,
) -> Result<WorkspaceInvitation>

// Bulk create invitations (up to 100 users)
pub async fn bulk_create_invitations(
    conn: &mut DbConn,
    workspace_id: Uuid,
    emails: Vec<String>,
    role_name: String,
    inviter_id: Uuid,
    expires_in_hours: Option<i64>,
) -> Result<Vec<CreateInvitationResponse>>

// Cleanup expired invitations
pub async fn cleanup_expired_invitations(conn: &mut DbConn) -> Result<u64>
```

### Security Features

- **UUID v7 Tokens**: Time-based sortable unique tokens, single-use
- **Expiration Handling**: Configurable duration with maximum limits
- **Case-Insensitive Email**: Stored in lowercase
- **Duplicate Prevention**: One pending invitation per email per workspace
- **Access Control**: Requires `INVITE_MEMBERS` permission

---

## Related Documentation

- [ARCHITECTURE.md](./ARCHITECTURE.md) - System architecture overview
- [API_REFERENCE.md](./API_REFERENCE.md) - Complete API reference
- [FILE_SYSTEM.md](./FILE_SYSTEM.md) - File system architecture

## For Developers

### Finding Current Configuration Values
```bash
# Check password length requirements (central utility in src/validation.rs)
grep -n "password.len() < 12" src/validation.rs

# Check session extension limits
grep -n "Cannot extend session by more than" src/users/services/users.rs

# Check session management functions
grep -n "pub async fn.*session" src/services/sessions.rs

# View session table structure
psql -d buildscale -c "\d user_sessions"

# Check workspace name validation
grep -n "validate_workspace_name" src/validation.rs
```

### Session Management Configuration
Session management settings are typically found in:
- `src/validation.rs`: Password validation requirements (12+ characters)
- `src/auth/services/sessions.rs`: Session cleanup and management
- `src/users/services/users.rs`: Authentication logic

### Security Configuration
- **Token Generation**: Random HMAC-signed tokens generated in `src/services/refresh_tokens.rs`
- **Password Hashing**: Argon2 configuration in password utility functions
- **Session Validation**: Token format and HMAC signature verification

### Changing Security Settings
1. Update constants in appropriate source files
2. Update database schema if needed (migrations/)
3. Add tests for new security settings
4. Update CONFIGURATION.md with new values
5. Update API documentation if interfaces change

### Current Security Defaults
See [CONFIGURATION.md](./CONFIGURATION.md) for current security-related configuration values, including session durations, password requirements, and token settings.
