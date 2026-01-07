# REST API Guide

HTTP REST API endpoints for the BuildScale multi-tenant workspace-based RBAC system.

## Table of Contents
- [Quick Reference](#quick-reference)
- [Getting Started](#getting-started)
- [Authentication](#authentication)
- [API Endpoints](#api-endpoints)
  - [Health Check](#health-check)
  - [User Registration](#user-registration)
  - [User Login](#user-login)
  - [Refresh Access Token](#refresh-access-token)
- [Error Responses](#error-responses)
- [Testing the API](#testing-the-api)

---

## Quick Reference

| Endpoint | Method | Description | Auth Required |
|----------|--------|-------------|---------------|
| `/api/v1/health` | GET | Health check with cache metrics | No |
| `/api/v1/auth/register` | POST | Register new user | No |
| `/api/v1/auth/login` | POST | Login and get tokens | No |
| `/api/v1/auth/refresh` | POST | Refresh access token | No (uses refresh token) |

**Base URL**: `http://localhost:3000` (default)

**API Version**: `v1` (all endpoints are prefixed with `/api/v1`)

---

## Getting Started

### Prerequisites

1. **Start the server**:
   ```bash
   cargo run --bin main
   ```

2. **Verify server is running**:
   ```bash
   curl http://localhost:3000/api/v1/health
   ```

### Response Format

All successful responses return JSON:
```json
{
  "field": "value"
}
```

All error responses return JSON:
```json
{
  "error": "Error message describing what went wrong"
}
```

---

## Authentication

The API uses **dual-token authentication** for secure access:

### Token Types

1. **JWT Access Token** (short-lived)
   - Lifetime: 15 minutes (configurable via `BUILDSCALE__JWT__ACCESS_TOKEN_EXPIRATION_MINUTES`)
   - Used in: `Authorization: Bearer <access_token>` header
   - Purpose: API requests requiring authentication

2. **Session Refresh Token** (long-lived)
   - Lifetime: 30 days (configurable via `BUILDSCALE__SESSIONS__EXPIRATION_HOURS`)
   - Stored in: `refresh_token` cookie (or used via refresh endpoint)
   - Purpose: Get new access tokens without re-login

### Client Types

#### Browser Clients (Web Applications)
- **Tokens stored automatically as cookies**
- Cookies are sent with every request automatically
- No manual header management needed
- Cookie security flags:
  - `HttpOnly`: Prevents JavaScript access (XSS protection)
  - `SameSite=Lax`: CSRF protection
  - `Secure`: HTTPS-only (set to `true` in production)

#### API/Mobile Clients
- **Extract tokens from login response JSON**
- Include access token in `Authorization` header:
  ```bash
  Authorization: Bearer <access_token>
  ```
- Store refresh token securely (keychain/encrypted storage)
- Use refresh token to get new access tokens when they expire

### Authentication Flow

```
1. POST /api/v1/auth/register
   → Creates user account

2. POST /api/v1/auth/login
   → Returns access_token + refresh_token
   → Sets cookies (browser clients)

3. API Request with access_token
   → Authorization: Bearer <access_token>

4. When access_token expires (15 min)
   → POST /api/v1/auth/refresh
   → API clients: Authorization header with refresh_token
   → Browser clients: Cookie with refresh_token
   → Returns new access_token
   → Sets access_token cookie (browser clients only)

5. Repeat step 3-4 until refresh_token expires (30 days)
   → Then login again (step 2)
```

---

## API Endpoints

### Health Check

Monitor server health and cache performance.

**Endpoint**: `GET /api/v1/health`

**Authentication**: Not required

#### Request

```bash
curl http://localhost:3000/api/v1/health
```

#### Response (200 OK)

```json
{
  "num_keys": 42,
  "last_worker_time": "2026-01-07T09:00:00Z",
  "cleaned_count": 5,
  "size_bytes": 18432
}
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `num_keys` | integer | Current number of entries in cache |
| `last_worker_time` | string or null | ISO8601 timestamp of last cleanup (null if never run) |
| `cleaned_count` | integer | Number of entries removed by last cleanup |
| `size_bytes` | integer | Estimated memory usage in bytes |

#### Use Cases

- **Health monitoring**: Check if server is running
- **Cache metrics**: Monitor cache performance
- **Load testing**: Track cache growth during testing
- **Debugging**: Verify cleanup worker is running

---

### User Registration

Register a new user account with email and password.

**Endpoint**: `POST /api/v1/auth/register`

**Authentication**: Not required

#### Request

**Headers**:
```
Content-Type: application/json
```

**Body**:
```json
{
  "email": "user@example.com",
  "password": "securepassword123",
  "confirm_password": "securepassword123",
  "full_name": "John Doe"
}
```

#### Request Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `email` | string | Yes | User's email address (must be unique, valid email format) |
| `password` | string | Yes | User's password (minimum 8 characters) |
| `confirm_password` | string | Yes | Password confirmation (must match `password`) |
| `full_name` | string | No | User's full name (letters, spaces, hyphens, apostrophes, periods) |

#### Validation Rules

- **Email**: Must be valid email format, converted to lowercase
- **Password**: Minimum 8 characters
- **Password confirmation**: Must exactly match password
- **Full name**: Letters, spaces, hyphens, apostrophes, and periods only (if provided)

#### Response (200 OK)

```json
{
  "user": {
    "id": "019b97ac-e5f5-735b-b0a6-f3a34fcd4ff1",
    "email": "user@example.com",
    "password_hash": "$argon2id$v=19$m=19456,t=2,p=1$...",
    "full_name": "John Doe",
    "created_at": "2026-01-07T09:00:00Z",
    "updated_at": "2026-01-07T09:00:00Z"
  }
}
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `user.id` | string (UUID) | Unique user identifier |
| `user.email` | string | User's email (lowercase) |
| `user.password_hash` | string | Argon2 password hash (never return actual password) |
| `user.full_name` | string or null | User's full name |
| `user.created_at` | string (ISO8601) | Account creation timestamp |
| `user.updated_at` | string (ISO8601) | Last update timestamp |

#### Error Responses

**400 Bad Request** - Validation Error
```json
{
  "error": "Password must be at least 8 characters long"
}
```

**400 Bad Request** - Passwords Don't Match
```json
{
  "error": "Passwords do not match"
}
```

**409 Conflict** - Email Already Exists
```json
{
  "error": "Email 'user@example.com' already exists"
}
```

#### Example Usage

```bash
# Register new user
curl -X POST http://localhost:3000/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "john@example.com",
    "password": "securepass123",
    "confirm_password": "securepass123",
    "full_name": "John Doe"
  }'
```

```javascript
// JavaScript/TypeScript example
const response = await fetch('http://localhost:3000/api/v1/auth/register', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    email: 'john@example.com',
    password: 'securepass123',
    confirm_password: 'securepass123',
    full_name: 'John Doe'
  })
});

const data = await response.json();
console.log(data.user.id); // User UUID
```

---

### User Login

Authenticate with email and password to receive access and refresh tokens.

**Endpoint**: `POST /api/v1/auth/login`

**Authentication**: Not required (use this endpoint to get tokens)

#### Request

**Headers**:
```
Content-Type: application/json
```

**Body**:
```json
{
  "email": "user@example.com",
  "password": "securepassword123"
}
```

#### Request Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `email` | string | Yes | User's email address |
| `password` | string | Yes | User's password |

#### Response (200 OK)

**JSON Body**:
```json
{
  "user": {
    "id": "019b97ac-e5f5-735b-b0a6-f3a34fcd4ff1",
    "email": "user@example.com",
    "password_hash": "$argon2id$v=19$m=19456,t=2,p=1$...",
    "full_name": "John Doe",
    "created_at": "2026-01-07T09:00:00Z",
    "updated_at": "2026-01-07T09:00:00Z"
  },
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "refresh_token": "a296d8b58edbc757f07670aa8055e9727...",
  "access_token_expires_at": "2026-01-07T09:15:00Z",
  "refresh_token_expires_at": "2026-02-06T09:00:00Z"
}
```

**Cookies Set** (Browser clients):
```
Set-Cookie: access_token=eyJ0eXAiOiJKV1QiLCJhbGc...; HttpOnly; SameSite=Lax; Path=/; Max-Age=900
Set-Cookie: refresh_token=a296d8b58edbc757f07670aa80...; HttpOnly; SameSite=Lax; Path=/; Max-Age=2592000
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `user` | object | Complete user object |
| `access_token` | string (JWT) | JWT access token for API requests |
| `refresh_token` | string | Session refresh token |
| `access_token_expires_at` | string (ISO8601) | When access token expires (15 minutes) |
| `refresh_token_expires_at` | string (ISO8601) | When refresh token expires (30 days) |

#### Token Usage

**Access Token** (for API requests):
```bash
# Include in Authorization header
curl http://localhost:3000/api/v1/protected-endpoint \
  -H "Authorization: Bearer <access_token>"
```

**Refresh Token** (to get new access token):
- Stored in cookie for browser clients (automatic)
- Store securely for mobile/API clients
- Use when access token expires

#### Error Responses

**400 Bad Request** - Validation Error
```json
{
  "error": "Email cannot be empty"
}
```

**401 Unauthorized** - Invalid Credentials
```json
{
  "error": "Invalid email or password"
}
```

#### Example Usage

```bash
# Login and get tokens
curl -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "john@example.com",
    "password": "securepass123"
  }' \
  -c cookies.txt  # Save cookies for browser clients

# Use access token for API requests
curl http://localhost:3000/api/v1/protected-endpoint \
  -H "Authorization: Bearer <access_token>"

# Use cookies for browser clients
curl http://localhost:3000/api/v1/protected-endpoint \
  -b cookies.txt  # Send cookies
```

```javascript
// JavaScript/TypeScript example
const loginResponse = await fetch('http://localhost:3000/api/v1/auth/login', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
  },
  credentials: 'include', // Include cookies for browser clients
  body: JSON.stringify({
    email: 'john@example.com',
    password: 'securepass123'
  })
});

const loginData = await loginResponse.json();

// Store tokens for API clients
const { access_token, refresh_token, user } = loginData;
localStorage.setItem('access_token', access_token);
localStorage.setItem('refresh_token', refresh_token);

// Use access token for subsequent requests
const apiResponse = await fetch('http://localhost:3000/api/v1/protected', {
  headers: {
    'Authorization': `Bearer ${access_token}`
  }
});
```

---

### Refresh Access Token

Refresh an expired access token using a valid refresh token. Supports both Authorization header (API clients) and Cookie (browser clients).

**Endpoint**: `POST /api/v1/auth/refresh`

**Authentication**: No (uses refresh token instead)

#### How It Works

The refresh endpoint accepts refresh tokens from two sources with **priority handling**:

1. **Authorization header** (API/Mobile clients): `Authorization: Bearer <refresh_token>`
2. **Cookie** (Browser clients): `refresh_token=<token>`

**Priority**: Authorization header takes precedence if both are present.

**Behavior differences by client type**:
- **API/Mobile clients**: Returns JSON only, does NOT set cookie
- **Browser clients**: Returns JSON AND sets `access_token` cookie

#### Request (API/Mobile Client)

**Headers**:
```
Content-Type: application/json
Authorization: Bearer <refresh_token>
```

**Body**: None (token in Authorization header)

#### Request (Browser Client)

**Headers**:
```
Content-Type: application/json
Cookie: refresh_token=<token>
```

**Body**: None (token in Cookie)

#### Response (200 OK)

**JSON Body** (both client types):
```json
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "expires_at": "2026-01-07T09:15:00Z"
}
```

**Cookie Set** (browser clients only):
```
Set-Cookie: access_token=eyJ0eXAiOiJKV1QiLCJhbGc...; HttpOnly; SameSite=Lax; Path=/; Max-Age=900
```

**No cookie** is set for API clients using Authorization header.

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `access_token` | string (JWT) | New JWT access token (15 minute expiration) |
| `expires_at` | string (ISO8601) | When the new access token expires |

#### Token Expiration

- **Access Token**: 15 minutes (configurable via `BUILDSCALE__JWT__ACCESS_TOKEN_EXPIRATION_MINUTES`)
- **Refresh Token**: 30 days (configurable via `BUILDSCALE__SESSIONS__EXPIRATION_HOURS`)

#### Error Responses

**401 Unauthorized** - Invalid or expired refresh token
```json
{
  "error": "No valid refresh token found in Authorization header or cookie"
}
```

**401 Unauthorized** - Session expired
```json
{
  "error": "Session expired"
}
```

#### Example Usage (API Client)

```bash
# Refresh using Authorization header
curl -X POST http://localhost:3000/api/v1/auth/refresh \
  -H "Authorization: Bearer <refresh_token>"

# Response: JSON only, no cookie set
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "expires_at": "2026-01-07T10:30:00Z"
}
```

#### Example Usage (Browser Client)

```bash
# Refresh using Cookie
curl -X POST http://localhost:3000/api/v1/auth/refresh \
  -H "Cookie: refresh_token=<token>" \
  -c cookies.txt

# Response: JSON + access_token cookie is set
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "expires_at": "2026-01-07T10:30:00Z"
}

# access_token cookie is automatically set
# Can be used for subsequent requests
curl http://localhost:3000/api/v1/protected-endpoint \
  -b cookies.txt
```

#### JavaScript/TypeScript Example

```javascript
// Automatic token refresh for API clients
let accessToken = localStorage.getItem('access_token');
let refreshToken = localStorage.getItem('refresh_token');

const refreshAccessToken = async () => {
  const response = await fetch('http://localhost:3000/api/v1/auth/refresh', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${refreshToken}`
    }
  });

  if (!response.ok) {
    // Refresh token expired, need to login again
    window.location.href = '/login';
    return;
  }

  const data = await response.json();
  accessToken = data.access_token;
  localStorage.setItem('access_token', accessToken);
  return accessToken;
};

// Use in API calls with automatic refresh
const apiCall = async () => {
  try {
    let response = await fetch('http://localhost:3000/api/v1/protected', {
      headers: {
        'Authorization': `Bearer ${accessToken}`
      }
    });

    // If access token expired, refresh and retry
    if (response.status === 401) {
      accessToken = await refreshAccessToken();
      response = await fetch('http://localhost:3000/api/v1/protected', {
        headers: {
          'Authorization': `Bearer ${accessToken}`
        }
      });
    }

    return await response.json();
  } catch (error) {
    console.error('API call failed:', error);
  }
};
```

#### Browser Client with Automatic Cookie Management

```javascript
// Browser clients - cookies are handled automatically
const refreshAccessToken = async () => {
  const response = await fetch('http://localhost:3000/api/v1/auth/refresh', {
    method: 'POST',
    credentials: 'include' // Send and receive cookies automatically
  });

  if (!response.ok) {
    // Refresh token expired, redirect to login
    window.location.href = '/login';
    return;
  }

  const data = await response.json();
  // New access_token is automatically set in cookie by the server
  return data.access_token;
};

// Subsequent requests automatically include the access_token cookie
const apiCall = async () => {
  const response = await fetch('http://localhost:3000/api/v1/protected', {
    credentials: 'include' // Cookies sent automatically
  });
  return await response.json();
};
```

#### When to Refresh

Refresh the access token when:
- API calls return `401 Unauthorized`
- Access token expiration time is reached (15 minutes)
- User resumes activity after extended period

**Do NOT** refresh:
- On every request (refresh only when needed)
- If refresh token is expired (30 days) - user must login again
- More frequently than necessary (reduces security)

---

## Error Responses

All error responses follow a consistent format:

### Error Response Structure

```json
{
  "error": "Human-readable error message"
}
```

### HTTP Status Codes

| Status | Meaning | Example Scenarios |
|--------|---------|-------------------|
| **200 OK** | Success | Request completed successfully |
| **400 Bad Request** | Validation Error | Invalid email, weak password, missing fields |
| **401 Unauthorized** | Authentication Failed | Wrong email/password, expired token |
| **409 Conflict** | Resource Conflict | Email already exists |
| **500 Internal Server Error** | Server Error | Database connection failed |

### Common Error Messages

| Error Message | Status | Cause |
|---------------|--------|-------|
| `"Email cannot be empty"` | 400 | Email field missing or empty |
| `"Password must be at least 8 characters long"` | 400 | Password too short |
| `"Passwords do not match"` | 400 | Password and confirmation don't match |
| `"Email 'user@example.com' already exists"` | 409 | Duplicate email registration |
| `"Invalid email or password"` | 401 | Wrong login credentials |
| `"Database error"` | 500 | Server-side database issue |

---

## Testing the API

### Using cURL

```bash
# 1. Check server health
curl http://localhost:3000/api/v1/health

# 2. Register new user
curl -X POST http://localhost:3000/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "testpass123",
    "confirm_password": "testpass123",
    "full_name": "Test User"
  }'

# 3. Login
curl -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "testpass123"
  }'

# 4. Test duplicate email (should return 409)
curl -X POST http://localhost:3000/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "newpass123",
    "confirm_password": "newpass123"
  }'
```

### Using the Provided Example

The project includes a comprehensive example demonstrating all authentication features:

```bash
# Run the authentication API example
cargo run --example 05_auth_api

# Use custom API base URL
API_BASE_URL=http://localhost:3001/api/v1 cargo run --example 05_auth_api
```

The example demonstrates:
- ✅ User registration
- ✅ User login with dual-token support
- ✅ Cookie handling for browser clients
- ✅ Authorization header for API clients
- ✅ Error handling (wrong password, duplicate email, weak password)
- ✅ Full request/response logging with headers

### Using Postman or Insomnia

**Import as cURL**:
1. Copy any cURL command from above
2. Import into Postman/Insomnia
3. Run and inspect response

**Manual Setup**:
1. Create new request
2. Set URL: `http://localhost:3000/api/v1/auth/login`
3. Set method: `POST`
4. Add header: `Content-Type: application/json`
5. Add body (raw JSON):
   ```json
   {
     "email": "test@example.com",
     "password": "testpass123"
   }
   ```

---

## Production Considerations

### Environment Variables

```bash
# Database Configuration
BUILDSCALE__DATABASE__USER=your_db_user
BUILDSCALE__DATABASE__PASSWORD=your_db_password
BUILDSCALE__DATABASE__HOST=localhost
BUILDSCALE__DATABASE__PORT=5432
BUILDSCALE__DATABASE__DATABASE=your_db_name

# JWT Configuration
BUILDSCALE__JWT__SECRET=your-jwt-secret-min-32-chars
BUILDSCALE__JWT__ACCESS_TOKEN_EXPIRATION_MINUTES=15

# Session Configuration
BUILDSCALE__SESSIONS__EXPIRATION_HOURS=720

# Cookie Configuration
BUILDSCALE__COOKIE__SECURE=true  # Enable for HTTPS (production)
```

### Security Best Practices

1. **Always use HTTPS in production**
   - Set `BUILDSCALE__COOKIE__SECURE=true`
   - Cookies will only be sent over HTTPS

2. **Protect your JWT secret**
   - Use strong, random secret (minimum 32 characters)
   - Never commit to version control
   - Rotate periodically

3. **Implement rate limiting**
   - Prevent brute force attacks on login
   - Limit registration attempts per IP

4. **Monitor and log**
   - Track failed login attempts
   - Monitor for suspicious activity
   - Implement account lockout after N failed attempts

5. **Token storage**
   - Browser: HttpOnly cookies (automatic)
   - Mobile: Encrypted keychain/keystore
   - Never store tokens in localStorage (XSS vulnerable)

---

## Next Steps

- **Workspace Management API**: Create and manage workspaces
- **Member Management API**: Add/remove members with role assignments
- **Permission System**: Role-based access control (RBAC)
- **Invitation System**: Invite users to workspaces via email

See `docs/SERVICES_API_GUIDE.md` for complete service layer API reference.
