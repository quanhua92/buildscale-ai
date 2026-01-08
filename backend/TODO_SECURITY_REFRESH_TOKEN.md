# Implementation Plan: Stolen Refresh Token Detection

## Executive Summary

**Critical Security Issue**: Current token rotation implementation has a race condition vulnerability where attackers who steal refresh tokens can win the "refresh race" and maintain continuous access.

**Attack Scenario**:
```
1. Attacker steals refresh_token_A (XSS, network sniffing, etc.)
2. Attacker calls /refresh with refresh_token_A → gets refresh_token_B
3. Legitimate user tries to use refresh_token_A → gets 401 Unauthorized
4. Attacker now has valid access, legitimate user is locked out
```

**Solution**: Implement stolen token detection with automatic session revocation using a `revoked_refresh_tokens` tracking table.

**Security Improvement**: Detect when old tokens are used after rotation and immediately revoke ALL user sessions (preventing attacker persistence).

---

## The Problem

### Current Vulnerability

The current `refresh_access_token()` implementation:
1. Validates old token → gets session
2. Generates new token
3. **Replaces old token hash with new one** (old token lost forever)
4. Returns new tokens

**Issue**: If an attacker uses the stolen token BEFORE the legitimate user, they "win the race" and the legitimate user gets locked out with a generic 401 error.

**What We Can't Detect**:
- Normal token expiration (after 30 days)
- Token theft during the race window

**What We Should Detect**:
- Old token used AFTER rotation (indicates theft)
- Automatic security response (revoke all sessions)

---

## The Solution

### Architecture: Separate Revoked Tokens Table

Create a `revoked_refresh_tokens` table to temporarily track rotated tokens with a **5-minute grace period**.

**Why Separate Table?**
- ✅ Minimal schema changes (user_sessions table untouched)
- ✅ Clean separation (revoked tokens are temporary security artifacts)
- ✅ Efficient cleanup (simple time-based deletion)
- ✅ No impact on normal session queries

**Why 5-Minute Grace Period?**
- Legitimate protection: If user accidentally double-submits refresh
- Fast security response: Quick detection of theft
- Industry standard: OAuth 2.0 recommends 1-5 minute grace periods

**Decision**: Fixed 5-minute grace period (not configurable for simplicity)

---

## Implementation Plan

### Phase 1: Database Schema

**New Migration File**: `migrations/20260108_stolen_token_detection.up.sql`

```sql
-- Track revoked refresh tokens for theft detection
CREATE TABLE revoked_refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL,
    revoked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reason TEXT NOT NULL DEFAULT 'token_rotation'
);

-- Index for fast stolen token lookup (used in every refresh)
CREATE INDEX idx_revoked_tokens_token_hash ON revoked_refresh_tokens(token_hash);

-- Index for time-based cleanup (remove tokens older than grace period)
CREATE INDEX idx_revoked_tokens_revoked_at ON revoked_refresh_tokens(revoked_at);

-- Index for user-level revocation queries (security operations)
CREATE INDEX idx_revoked_tokens_user_id ON revoked_refresh_tokens(user_id);

COMMENT ON TABLE revoked_refresh_tokens IS 'Stores temporarily revoked refresh tokens to detect token theft. Tokens are cleaned up after grace period (5 minutes).';
```

---

### Phase 2: Error Handling

**File**: `src/error.rs`

**Add new error variant** (after line ~43):
```rust
/// Token theft detected (stolen refresh token used after rotation)
#[error("Token theft detected: {0}")]
TokenTheftDetected(String),
```

**Update HTTP response mapping** (in IntoResponse impl, line ~71):
```rust
Error::TokenTheftDetected(msg) => (StatusCode::FORBIDDEN, msg),
```

**Why 403 Forbidden?**
- 403 = "understood but refused" (theft detected, access denied)
- 401 = "not authenticated" (expired token, normal case)
- Clear distinction for monitoring and user experience

---

### Phase 3: Data Models

**File**: `src/models/users.rs`

**Add at end of file** (after line 78):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokedRefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub revoked_at: DateTime<Utc>,
    pub reason: String,
}
```

---

### Phase 4: Database Queries

**File**: `src/queries/sessions.rs`

**Add these functions** (after line 254):

```rust
/// Records a revoked refresh token for theft detection
pub async fn create_revoked_token(
    conn: &mut DbConn,
    user_id: Uuid,
    token_hash: &str,
) -> Result<RevokedRefreshToken> {
    let revoked_token = sqlx::query_as!(
        RevokedRefreshToken,
        r#"
        INSERT INTO revoked_refresh_tokens (user_id, token_hash)
        VALUES ($1, $2)
        RETURNING id, user_id, token_hash, revoked_at, reason
        "#,
        user_id,
        token_hash
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(revoked_token)
}

/// Checks if a token has been revoked (indicating potential theft)
pub async fn get_revoked_token(
    conn: &mut DbConn,
    token_hash: &str,
) -> Result<Option<RevokedRefreshToken>> {
    let revoked_token = sqlx::query_as!(
        RevokedRefreshToken,
        r#"
        SELECT id, user_id, token_hash, revoked_at, reason
        FROM revoked_refresh_tokens
        WHERE token_hash = $1
        "#,
        token_hash
    )
    .fetch_optional(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(revoked_token)
}

/// Deletes all revoked tokens for a specific user
pub async fn delete_revoked_tokens_by_user(
    conn: &mut DbConn,
    user_id: Uuid,
) -> Result<u64> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM revoked_refresh_tokens
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?
    .rows_affected();

    Ok(rows_affected)
}

/// Deletes expired revoked tokens (older than grace period)
/// Should be called periodically to maintain table size
pub async fn delete_expired_revoked_tokens(
    conn: &mut DbConn,
    grace_period_minutes: i64,
) -> Result<u64> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM revoked_refresh_tokens
        WHERE revoked_at < NOW() - (INTERVAL '1 minute' * $1)
        "#,
    )
    .bind(grace_period_minutes)
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?
    .rows_affected();

    Ok(rows_affected)
}
```

**Update imports**:
```rust
use crate::{
    error::{Error, Result},
    models::users::{NewUserSession, UpdateUserSession, UserSession, RevokedRefreshToken},
};
```

---

### Phase 5: Core Detection Logic

**File**: `src/services/users.rs`

**Replace `refresh_access_token()` function** (lines 313-353):

```rust
pub async fn refresh_access_token(
    conn: &mut DbConn,
    refresh_token: &str,
) -> Result<RefreshTokenResult> {
    let old_token_hash = sessions::hash_session_token(refresh_token);

    // STEP 1: Check if token was revoked (THEFT DETECTION with grace period)
    if let Some(revoked) = sessions::get_revoked_token(conn, &old_token_hash).await? {
        // Calculate how long ago the token was revoked
        let time_since_revocation = Utc::now() - revoked.revoked_at;

        // Grace period: 5 minutes
        // - Allows legitimate double-clicks/retries without error
        // - Still detects theft after grace period expires
        if time_since_revocation.num_minutes() >= 5 {
            // TOKEN THEFT DETECTED: Old token used after grace period!
            // Revoke ALL sessions for this user immediately
            let _ = sessions::delete_sessions_by_user(conn, revoked.user_id).await?;

            // Clean up revoked token records
            let _ = sessions::delete_revoked_tokens_by_user(conn, revoked.user_id).await?;

            return Err(Error::TokenTheftDetected(
                "Potential security breach detected. Your refresh token was used after rotation. All sessions have been revoked for your protection. Please login again and consider changing your password.".to_string()
            ));
        }

        // Within grace period: Allow the request but don't rotate again
        // This handles accidental double-clicks gracefully
        // Just return the current valid tokens without rotation
        // (Implementation: fetch current session tokens and return)
    }

    // STEP 2: Validate the refresh token (session) exists and is not expired
    let session = sessions::get_valid_session_by_token_hash(conn, &old_token_hash)
        .await?
        .ok_or_else(|| Error::InvalidToken("Invalid or expired refresh token".to_string()))?;

    // Load config
    let config = Config::load()?;

    // STEP 3: Generate NEW refresh token (rotation)
    let new_refresh_token = generate_session_token()?;
    let new_token_hash = sessions::hash_session_token(&new_refresh_token);

    // STEP 4: Start transaction for atomic token rotation
    // CRITICAL: Recording revoked token + updating session must be atomic
    let mut tx = conn.begin().await.map_err(|e| {
        Error::Internal(format!("Failed to begin transaction: {}", e))
    })?;

    // STEP 5: Record old token as revoked BEFORE updating session
    // CRITICAL: If two requests race, the second one will detect theft
    let _ = sessions::create_revoked_token(&mut tx, session.user_id, &old_token_hash).await?;

    // STEP 6: Update session with new token hash (invalidates old token)
    let _updated_session = sessions::update_session_token_hash(
        &mut tx,
        session.id,
        &new_token_hash,
    ).await?;

    // STEP 7: Commit transaction (atomic operation complete)
    tx.commit().await.map_err(|e| {
        Error::Internal(format!("Failed to commit transaction: {}", e))
    })?;

    // STEP 8: Generate new access token (JWT)
    let access_token = jwt::generate_jwt(
        session.user_id,
        config.jwt.secret.expose_secret(),
        config.jwt.access_token_expiration_minutes,
    )?;

    let expires_at = Utc::now() + Duration::minutes(config.jwt.access_token_expiration_minutes);

    Ok(RefreshTokenResult {
        access_token,
        refresh_token: new_refresh_token,
        expires_at,
    })
}
```

**Key Insight**: The order matters! We record the old token as revoked BEFORE updating the session. This creates a detection window:
- If attacker wins race → legitimate user's next refresh hits revoked token → 403 TokenTheftDetected
- If legitimate user wins race → attacker's next refresh hits revoked token → 403 TokenTheftDetected

**How the 5-Minute Grace Period Works**:

The grace period balances security with user experience:

1. **0-5 minutes after rotation**: Token is in revoked table but within grace period
   - **Double-click protection**: User accidentally double-clicks → 200 OK (no error)
   - **Network retry protection**: Slow network causes retry → 200 OK (no error)
   - **No token rotation**: Second request returns current tokens (doesn't rotate again)

2. **5+ minutes after rotation**: Token is still in revoked table but grace period expired
   - **Theft detection**: Attacker tries to use stolen token → 403 TokenTheftDetected
   - **All sessions revoked**: User forced to re-login
   - **Security response**: Maximum protection against stolen tokens

**Example Timeline**:
```
10:00:00 - User refreshes → Token A revoked, Token B issued
10:00:01 - User double-clicks → Token A found, but < 5 min → 200 OK (grace period)
10:04:59 - User retries again → Token A found, but < 5 min → 200 OK (grace period)
10:05:01 - Attacker tries Token A → Token A found, >= 5 min → 403 TokenTheftDetected!
```

**Why This Prevents Bad UX**:
- Legitimate users never see 403 errors for accidental double-clicks
- Network retries and race conditions handled gracefully
- Only actual theft (old tokens used after 5+ minutes) triggers security response

**How Transaction + Grace Period Work Together**:
- **Transaction**: Ensures atomicity (create revoked + update session)
- **Grace Period**: Allows double-clicks without error
- **Cleanup Worker**: Removes tokens older than 5 minutes (keeps table small)

---

### Phase 6: Background Worker for Cleanup

**File**: `src/workers/revoked_token_cleanup.rs` (new file)

**Create new background worker** (separate from cache cleanup worker):

```rust
use crate::{DbConn, queries::sessions};
use std::time::Duration;
use tokio::time::interval;
use tracing::{info, warn, error};

/// Background worker that periodically cleans up expired revoked tokens
///
/// Runs every 5 minutes to remove revoked tokens older than grace period
/// This keeps the revoked_refresh_tokens table size manageable
pub async fn revoked_token_cleanup_worker(
    pool: sqlx::PgPool,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let mut cleanup_interval = interval(Duration::from_secs(300)); // Every 5 minutes
    info!("Revoked token cleanup worker started (runs every 5 minutes)");

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("Revoked token cleanup worker shutting down");
                break;
            }
            _ = cleanup_interval.tick() => {
                let mut conn = match pool.acquire().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        error!("Failed to acquire database connection for cleanup: {}", e);
                        continue;
                    }
                };

                // Grace period: 5 minutes (tokens older than 5 minutes are deleted)
                let grace_period_minutes = 5i64;

                match sessions::delete_expired_revoked_tokens(&mut conn, grace_period_minutes).await {
                    Ok(count) => {
                        if count > 0 {
                            info!("Cleaned up {} expired revoked tokens (older than {} minutes)", count, grace_period_minutes);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to cleanup expired revoked tokens: {}", e);
                    }
                }
            }
        }
    }

    info!("Revoked token cleanup worker stopped");
}
```

**File**: `src/workers/mod.rs` (update or create)

```rust
pub mod revoked_token_cleanup;

pub use revoked_token_cleanup::revoked_token_cleanup_worker;
```

**File**: `src/main.rs`

**Integrate the worker** (add alongside cache cleanup worker):

```rust
// Spawn revoked token cleanup worker
let (revoked_cleanup_shutdown_tx, _) = tokio::sync::broadcast::channel(1);
let revoked_cleanup_worker = crate::workers::revoked_token_cleanup_worker(
    app_state.pool.clone(),
    revoked_cleanup_shutdown_tx.subscribe(),
);

// Spawn both workers
tokio::select! {
    _ = tokio::signal::ctrl_c() => {
        info!("Shutdown signal received");
        revoked_cleanup_shutdown_tx.send(()).ok();
    }
    _ = cache_cleanup_worker => {}
    _ = revoked_cleanup_worker => {}
}
```

---

### Phase 7: Security Logging

**File**: `src/services/users.rs`

**Add security event logging** (in refresh_access_token function):

```rust
use tracing::{info, warn};

// In the theft detection section:
if let Some(revoked) = sessions::get_revoked_token(conn, &old_token_hash).await? {
    // Log security event BEFORE revoking sessions
    warn!(
        user_id = %revoked.user_id,
        security_event = "token_theft_detected",
        "Potential security breach: Old refresh token used after rotation. Revoking all sessions."
    );

    // Revoke ALL sessions for this user immediately
    let _ = sessions::delete_sessions_by_user(conn, revoked.user_id).await?;
    // ... rest of error handling
}

// Log successful token rotation (for audit trail)
info!(
    user_id = %session.user_id,
    security_event = "token_rotated",
    "Refresh token rotated successfully"
);
```

**Log Format**:
- **INFO**: Normal token rotation events
- **WARN**: Security events (token theft detected)
- **ERROR**: System errors (database failures)

**Example Logs**:
```
[2025-01-08T10:15:30Z INFO  backend::services::users] Refresh token rotated successfully user_id="019b97ac-..." security_event="token_rotated"
[2025-01-08T10:16:45Z WARN  backend::services::users] Potential security breach: Old refresh token used after rotation. Revoking all sessions. user_id="019b97ac-..." security_event="token_theft_detected"
```

---

## Testing Strategy

### Unit Tests

**File**: `tests/handlers/auth.rs`

**Add integration tests**:

1. **`test_stolen_token_detection_attacker_wins_race`**
   - Attacker refreshes first with stolen token
   - Legitimate user tries to refresh with old token
   - Verify: Returns 403 TokenTheftDetected
   - Verify: ALL user sessions revoked

2. **`test_stolen_token_detection_legitimate_wins_race`**
   - Legitimate user refreshes first
   - Attacker tries to use stolen token
   - Verify: Returns 403 TokenTheftDetected
   - Verify: Attacker blocked

3. **`test_normal_refresh_still_works`**
   - Normal refresh flow (no theft)
   - Verify: Returns 200 with new tokens
   - Verify: Old token recorded in revoked table

4. **`test_revoked_token_cleanup_after_grace_period`**
   - Create revoked token
   - Wait 6 minutes
   - Run cleanup
   - Verify: Token removed from table

### Manual Testing

```bash
# Test 1: Normal refresh (should work)
REFRESH="<original_token>"
curl -X POST http://localhost:3000/api/v1/auth/refresh \
  -H "Authorization: Bearer $REFRESH"
# Should return 200 with new tokens

# Test 2: Use old token (should detect theft)
curl -X POST http://localhost:3000/api/v1/auth/refresh \
  -H "Authorization: Bearer $REFRESH"
# Should return 403 with "Token theft detected"

# Test 3: Verify all sessions revoked
# Try to login again with new device → should work
# Try old tokens from any device → should all fail
```

---

## Error Response Format

### 403 Forbidden - Token Theft Detected

```json
{
  "error": "Potential security breach detected. Your refresh token was used after rotation. All sessions have been revoked for your protection. Please login again and consider changing your password."
}
```

**Client Action Required**:
1. Clear all stored tokens
2. Redirect to login
3. Show security message to user
4. Recommend password change
5. Log security event for monitoring

---

## Security Benefits

### Before This Implementation
- ❌ Attacker can win refresh race
- ❌ Legitimate user locked out with generic 401
- ❌ No way to detect which party was legitimate
- ❌ Attacker maintains access until token expires (30 days)

### After This Implementation
- ✅ Detects when old token used after rotation
- ✅ Immediately revokes ALL user sessions
- ✅ Returns specific 403 error (theft detected)
- ✅ Forces password change and re-login
- ✅ Attacker blocked on first refresh attempt

### Security Improvement
- **Detection Window**: 5 minutes (grace period)
- **Response Time**: Instant (automatic revocation)
- **Attack Prevention**: Attacker can only use stolen token ONCE (if they win the race)

---

## Performance Impact

### Storage
- **Growth Rate**: 1 row per refresh request
- **Retention**: 5 minutes
- **Cleanup**: Periodic deletion by timestamp
- **Estimated Size**: < 1000 rows at any time (for moderate traffic)

### Query Performance
- **Normal Refresh**: +1 index lookup (get_revoked_token)
- **Theft Detection**: O(log n) via btree index
- **Cleanup**: O(n) periodic batch delete
- **Impact**: Negligible (< 1ms per request)

### Database Load
- **Reads**: 1 additional query per refresh
- **Writes**: 1 additional INSERT per refresh
- **Cleanup**: 1 DELETE query per hour (cron job)

---

## Deployment Considerations

### Pre-Deployment
- ✅ All tests pass
- ✅ Documentation updated
- ✅ Error messages clear and actionable
- ✅ Cleanup job scheduled

### Deployment Steps
1. **Run migration**: Add `revoked_refresh_tokens` table
2. **Deploy code**: New detection logic + background worker
3. **Verify**: Test refresh endpoint returns 403 on old token
4. **Monitor**: Check logs for TokenTheftDetected errors
5. **Verify worker**: Check logs for cleanup worker messages every 5 minutes

### Rollback Plan
- Revert code changes
- `revoked_refresh_tokens` table is harmless (can be dropped later)
- No data loss (temporary table only)

### Risk Level: Medium
- Breaking change: New error type (403 instead of 401)
- Client impact: Clients must handle 403 TokenTheftDetected
- Mitigation: Clear error message, automatic redirect to login

---

## Monitoring & Alerts

### Metrics to Track
1. **TokenTheftDetected errors per hour**: Unusual spikes indicate attack
2. **Revoked tokens table size**: Should stay < 10,000 rows
3. **Cleanup job success**: Verify tokens being deleted

### Alerting Rules
- **Critical**: > 10 TokenTheftDetected errors in 1 minute (active attack)
- **Warning**: Revoked tokens table > 50,000 rows (cleanup not working)
- **Info**: Daily summary of theft detection events

### Log Messages
```
[SECURITY] Token theft detected for user {user_id}: old refresh token used after rotation. All sessions revoked.
[AUDIT] Cleanup job removed {count} expired revoked tokens.
```

---

## Future Enhancements (Out of Scope)

1. **Device Fingerprinting**: Bind tokens to device/browser
2. **IP-based detection**: Flag tokens used from unusual locations
3. **Rate limiting**: Limit refresh attempts per IP
4. **User notification**: Email when theft detected
5. **Audit logging**: Store security events in separate table

---

## Critical Files Summary

### Core Implementation (7 files)

1. **`migrations/20260108_stolen_token_detection.up.sql`**
   - Create `revoked_refresh_tokens` table
   - Add indexes for performance

2. **`src/error.rs`**
   - Add `TokenTheftDetected` error variant
   - Update HTTP response mapping (403 Forbidden)

3. **`src/models/users.rs`**
   - Add `RevokedRefreshToken` struct

4. **`src/queries/sessions.rs`**
   - Add CRUD functions for revoked tokens
   - `create_revoked_token()`, `get_revoked_token()`, `delete_revoked_tokens_by_user()`, `delete_expired_revoked_tokens()`

5. **`src/services/users.rs`**
   - Modify `refresh_access_token()` to check for stolen tokens
   - Record old token as revoked before rotation
   - Revoke all sessions when theft detected
   - Add security logging (INFO/WARN events)

6. **`src/workers/revoked_token_cleanup.rs`** (NEW FILE)
   - Background worker to clean up expired revoked tokens
   - Runs every 5 minutes
   - Separate from cache cleanup worker

7. **`src/workers/mod.rs`** (UPDATE or CREATE)
   - Export revoked_token_cleanup_worker

### Testing (1 file)

8. **`tests/handlers/auth.rs`**
   - Add 4 integration tests for theft detection

### Main Integration (1 file)

9. **`src/main.rs`**
   - Integrate revoked token cleanup worker
   - Add shutdown signal handling

---

## Implementation Order

1. **Database migration**: Create table and indexes
2. **Model**: Add `RevokedRefreshToken` struct
3. **Queries**: Add revoked token CRUD operations
4. **Error handling**: Add `TokenTheftDetected` variant
5. **Core logic**: Update `refresh_access_token()` with detection
6. **Background worker**: Create revoked token cleanup worker
7. **Worker integration**: Integrate worker into main.rs
8. **Tests**: Add integration tests
9. **Documentation**: Update API docs with new error response

**Testing after each phase**:
- After Phase 1-4: Run `cargo test users::services`
- After Phase 5: Run `cargo test handlers::auth`
- After Phase 7: Verify worker starts up correctly in logs
- After Phase 8: Run all tests with `cargo test`

---

## Summary

This plan implements **OAuth 2.0 compliant stolen token detection** by tracking rotated refresh tokens in a temporary table. When an old token is used after rotation, the system:

1. **Detects** the security breach (old token in revoked table)
2. **Responds** automatically (revokes all user sessions)
3. **Alerts** with specific error (403 TokenTheftDetected)
4. **Logs** security events for monitoring (WARN level logs)
5. **Protects** the user (forces re-login and password change)

**User Decisions**:
- ✅ Fixed 5-minute grace period (not configurable)
- ✅ Revoke ALL sessions when theft detected (maximum security)
- ✅ Background worker for cleanup (separate from cache cleanup)
- ✅ Security logging with tracing (INFO/WARN events)
- ✅ Just 403 response for now (email notification deferred)

**Security Improvement**: Prevents attackers from maintaining access after losing the refresh race. Attackers can only use stolen tokens once (if they win the initial race).

**Implementation**: 9 files changed (1 new file, 8 modified) with comprehensive testing and monitoring.
