# Authentication & Security

Session-based authentication with UUID v7 tokens, Argon2 password hashing, and 7-day session expiration.

## Core API

### User Authentication
```rust
// User registration (8+ char password, email validation)
pub async fn register_user(conn: &mut DbConn, register_user: RegisterUser) -> Result<User>

// User authentication and session creation
pub async fn login_user(conn: &mut DbConn, login_user: LoginUser) -> Result<LoginResult>

// Session validation and user retrieval
pub async fn validate_session(conn: &mut DbConn, session_token: &str) -> Result<User>

// Session termination
pub async fn logout_user(conn: &mut DbConn, session_token: &str) -> Result<()>

// Session extension
pub async fn refresh_session(conn: &mut DbConn, session_token: &str, hours_to_extend: i64) -> Result<String>
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
    pub user: User,                   // Authenticated user
    pub session_token: String,          // UUID v7 token
    pub expires_at: DateTime<Utc>,    // Session expiration (7 days)
}

pub struct UserSession {
    pub id: Uuid,                    // Session primary key
    pub user_id: Uuid,               // Session owner
    pub token: String,               // Unique UUID v7 token
    pub expires_at: DateTime<Utc>,   // Expiration time
    pub created_at: DateTime<Utc>,   // Creation time
    pub updated_at: DateTime<Utc>,   // Last update
}
```

## Security Features

- **UUID v7 Tokens**: Time-based sortable unique session identifiers
- **Argon2 Hashing**: Industry-standard password hashing with unique salts
- **7-Day Expiration**: Default session duration with automatic cleanup
- **Case-Insensitive Email**: User-friendly login experience
- **Multi-Device Support**: Users can maintain concurrent sessions
- **Session Revocation**: Immediate token invalidation on logout

## Validation Rules

| Input | Requirement | Error Message |
|--------|-------------|---------------|
| Email | Required, valid format | "Email cannot be empty" / "Invalid email format" |
| Password | Min 8 characters | "Password must be at least 8 characters long" |
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

### Basic Authentication
```rust
let login_result = login_user(&mut conn, LoginUser {
    email: "user@example.com".to_string(),
    password: "securepassword".to_string(),
}).await?;

let user = validate_session(&mut conn, &login_result.session_token).await?;
logout_user(&mut conn, &login_result.session_token).await?;
```

### Session Management
```rust
// Get active sessions
let sessions = get_user_active_sessions(&mut conn, user.id).await?;

// Extend all sessions by 7 days (168 hours)
let extended = extend_all_user_sessions(&mut conn, user.id, 168).await?;

// Force logout from all devices
let revoked = revoke_all_user_sessions(&mut conn, user.id).await?;

// Cleanup expired sessions
let cleaned = cleanup_expired_sessions(&mut conn).await?;
```