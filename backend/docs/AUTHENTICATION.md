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
    API->>API: Validate input (email format, password length >= 8)
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
// User registration (minimum password length requirements, email validation)
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
    pub access_token: String,              // New JWT access token
    pub expires_at: DateTime<Utc>,         // When the new access token expires
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
- **Random HMAC-Signed Tokens**: 256-bit randomness with tamper-evident signature
- **Long-Lived Tokens**: 30-day expiration for user convenience
- **Database Storage**: Revocable tokens stored in database
- **Integrity Verification**: HMAC-SHA256 signature prevents token tampering
- **Constant-Time Comparison**: Prevents timing attacks on token verification
- **Configurable Expiration**: Default session duration with automatic cleanup
- **Case-Insensitive Email**: User-friendly login experience
- **Multi-Device Support**: Users can maintain concurrent sessions
- **Session Revocation**: Immediate token invalidation on logout

## Validation Rules

| Input | Requirement | Error Message |
|--------|-------------|---------------|
| Email | Required, valid format | "Email cannot be empty" / "Invalid email format" |
| Password | Minimum length | "Password must meet minimum length requirements" |
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
    password: "securepassword".to_string(),
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

## Related Documentation

- **[Architecture Overview](./ARCHITECTURE.md)** - System design and database schema
- **[User & Workspace Management](./USER_WORKSPACE_MANAGEMENT.md)** - User registration and management APIs
- **[Role Management](./ROLE_MANAGEMENT.md)** - RBAC system and permissions
- **[API Guide](./API_GUIDE.md)** - Complete API reference with error handling
- **[Installation Guide](./README.md#installation--setup)** - Development setup and troubleshooting

## For Developers

### Finding Current Configuration Values
```bash
# Check password length requirements (hardcoded in services/users.rs)
grep -n "password.len() < 8" src/services/users.rs

# Check session extension limits
grep -n "Cannot extend session by more than" src/services/users.rs

# Check session management functions
grep -n "pub async fn.*session" src/services/sessions.rs

# View session table structure
psql -d buildscale -c "\d user_sessions"

# Check workspace name validation
grep -n "validate_workspace_name" src/validation.rs
```

### Session Management Configuration
Session management settings are typically found in:
- `src/models/users.rs`: Password validation requirements
- `src/services/sessions.rs`: Session cleanup and management
- `src/services/users.rs`: Authentication logic

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