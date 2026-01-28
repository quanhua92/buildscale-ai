← [Back to Index](./README.md) | **Developer API**: [Services API Guide](./SERVICES_API_GUIDE.md)

# REST API Guide

HTTP REST API endpoints for the BuildScale multi-tenant workspace-based RBAC system.

## Table of Contents
- [Quick Reference](#quick-reference)
- [Getting Started](#getting-started)
- [Authentication](#authentication)
- [JWT Authentication Middleware](#jwt-authentication-middleware)
- [API Endpoints](#api-endpoints)
  - [Health Check](#health-check)
  - [User Registration](#user-registration)
  - [User Login](#user-login)
  - [User Profile](#user-profile)
  - [Refresh Access Token](#refresh-access-token)
  - [User Logout](#user-logout)
- [Workspaces API](#workspaces-api)
- [Workspace Members API](#workspace-members-api)
- [Files & AI](#files-and-ai)
- [Tools API](#tools-api)
- [Agentic Chat API](#agentic-chat-api)
- [Error Responses](#error-responses)
- [Testing the API](#testing-the-api)
- [Production Considerations](#production-considerations)

---

## Quick Reference

| Endpoint | Method | Description | Auth Required |
|----------|--------|-------------|---------------|
| `/api/v1/health` | GET | Health check - simple status | No |
| `/api/v1/health/cache` | GET | Cache metrics | Yes (JWT) |
| `/api/v1/auth/register` | POST | Register new user | No |
| `/api/v1/auth/login` | POST | Login and get tokens | No |
| `/api/v1/auth/refresh` | POST | Refresh access token | No (uses refresh token) |
| `/api/v1/auth/logout` | POST | Logout and invalidate session | No (uses refresh token) |
| `/api/v1/auth/me` | GET | Get current user profile | Yes (JWT) |
| `/api/v1/workspaces` | POST | Create new workspace | Yes (JWT) |
| `/api/v1/workspaces` | GET | List my workspaces | Yes (JWT) |
| `/api/v1/workspaces/:id` | GET | Get workspace details | Yes (JWT + Member) |
| `/api/v1/workspaces/:id` | PATCH | Update workspace | Yes (JWT + Owner) |
| `/api/v1/workspaces/:id` | DELETE | Delete workspace | Yes (JWT + Owner) |
| `/api/v1/workspaces/:id/members` | GET | List workspace members | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/members` | POST | Add member by email | Yes (JWT + Admin) |
| `/api/v1/workspaces/:id/members/me` | GET | Get my membership details | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/members/:uid` | PATCH | Update member role | Yes (JWT + Admin) |
| `/api/v1/workspaces/:id/members/:uid` | DELETE | Remove member / Leave | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files` | POST | Create file/folder | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid` | GET | Get file & latest version | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid` | PATCH | Move or rename file | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid` | DELETE | Soft delete file | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid/restore` | POST | Restore file from trash | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/trash` | GET | List trash items | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid/versions` | POST | Create new version | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/search` | POST | Semantic search | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/tags/:tag` | GET | List files by tag | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid/tags` | POST | Add tag to file | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid/tags/:tag` | DELETE | Remove tag from file | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid/links` | POST | Link two files | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid/links/:tid` | DELETE | Remove file link | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid/network` | GET | Get file network graph | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/tools` | POST | Execute tool (ls, read, write, rm, mv, touch) | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/chats` | POST | Start new agentic chat | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/chats/:cid` | GET | Get chat history and config | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/chats/:cid` | POST | Send message to existing chat | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/chats/:cid/stop` | POST | Stop AI generation | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/chats/:cid/events` | GET | Connect to SSE event stream | Yes (JWT + Member) |

**Base URL**: `http://localhost:3000` (default)

**API Version**: `v1` (all endpoints are prefixed with `/api/v1`)

---

## Files and AI

Manage the "Everything is a File" system and use the AI Engine.

### Create File
Create a new file or folder.

**Endpoint**: `POST /api/v1/workspaces/:id/files`

**Authentication**: Required (JWT access token)

#### Request
```json
{
  "parent_id": "optional-folder-uuid",
  "name": "My Document.md",
  "slug": "optional-custom-slug.md",
  "path": "optional/path/to/create/folders/recursively.md",
  "file_type": "document",
  "content": { "text": "Hello world" },
  "app_data": { "cursor": 0 }
}
```

**Note on `path`:** If provided, `path` overrides `parent_id` and `slug`. The system will recursively create any missing folders in the path.

---
### Get File
Retrieve file metadata and its latest content version.

**Endpoint**: `GET /api/v1/workspaces/:id/files/:file_id`

**Authentication**: Required (JWT access token)

#### Response
```json
{
  "file": {
    "id": "...",
    "name": "My Document.md",
    "slug": "my-document.md",
    "path": "/my-document.md",
    "file_type": "document",
    "status": "ready"
  },
  "latest_version": {
    "id": "...",
    "hash": "..."
  },
  "content": { "text": "Hello world" }
}
```

---

### Update File
Move or rename a file or folder.

**Endpoint**: `PATCH /api/v1/workspaces/:id/files/:file_id`

**Authentication**: Required (JWT access token)

#### Request
```json
{
  "parent_id": "new-folder-uuid-or-null-for-root",
  "name": "New Name.md",
  "slug": "new-slug.md"
}
```

---

### Delete File
Soft delete a file or folder. Folders must be empty before deletion.

**Endpoint**: `DELETE /api/v1/workspaces/:id/files/:file_id`

**Authentication**: Required (JWT access token)

---

### Restore File
Restore a soft-deleted file from the trash.

**Endpoint**: `POST /api/v1/workspaces/:id/files/:file_id/restore`

**Authentication**: Required (JWT access token)

---

### List Trash
List all soft-deleted files in the workspace.

**Endpoint**: `GET /api/v1/workspaces/:id/files/trash`

**Authentication**: Required (JWT access token)

---

### Create Version
Append a new content version to an existing file. Content is automatically deduplicated.

**Endpoint**: `POST /api/v1/workspaces/:id/files/:file_id/versions`

**Authentication**: Required (JWT access token)

#### Request
```json
{
  "content": { "text": "Updated content" },
  "app_data": { "cursor": 42 },
  "branch": "main"
}
```

---

### Knowledge Graph

Build a networked knowledge base using tags and bidirectional links.

#### Add Tag
`POST /api/v1/workspaces/:id/files/:file_id/tags`
```json
{ "tag": "research" }
```

#### Remove Tag
`DELETE /api/v1/workspaces/:id/files/:file_id/tags/:tag`

#### List Files by Tag
`GET /api/v1/workspaces/:id/files/tags/:tag`

#### Link Files
Create a bidirectional link between two files.
`POST /api/v1/workspaces/:id/files/:file_id/links`
```json
{ "target_file_id": "target-uuid" }
```

#### Remove Link
`DELETE /api/v1/workspaces/:id/files/:file_id/links/:target_id`

#### Get File Network
Retrieve all tags, outbound links, and backlinks for a file.
`GET /api/v1/workspaces/:id/files/:file_id/network`

---

### Semantic Search
Search for content across all files in the workspace using vector similarity.

**Endpoint**: `POST /api/v1/workspaces/:id/search`

**Authentication**: Required (JWT access token)

#### Request
```json
{
  "query_vector": [0.1, 0.2, ...], // 1536-dim vector
  "limit": 5
}
```

---

## Tools API

Execute filesystem tools (ls, read, write, rm, mv, touch) within a workspace through a unified endpoint. This API provides an extensible interface for AI agents, automation scripts, and CLI tools.

### Execute Tool

**Endpoint**: `POST /api/v1/workspaces/:id/tools`

**Authentication**: Required (JWT + Workspace Member)

#### Request
```json
{
  "tool": "read",
  "args": { "path": "/file.txt" }
}
```

#### Available Tools
| Tool | Description | Arguments |
|------|-------------|-----------|
| `ls` | List directory contents | `path` (optional), `recursive` (optional, default: false) |
| `read` | Read file contents | `path` (required) |
| `write` | Create or update file | `path` (required), `content` (required), `file_type` (optional, defaults to `document`) |
| `rm` | Delete file or folder | `path` (required) |
| `mv` | Move or rename file | `source` (required), `destination` (required) |
| `touch` | Update timestamp or create empty file | `path` (required) |
| `mkdir` | Create folder structure recursively | `path` (required) |
| `edit` | Edit file content by unique replace | `path`, `old_string`, `new_string`, `last_read_hash?` |
| `grep` | Workspace-wide regex search | `pattern`, `path_pattern?`, `case_sensitive?` |

**Content Handling by File Type**:
- **Documents**: Raw strings are auto-wrapped to `{text: "..."}`. On read, simple documents are auto-unwrapped to return just the string.
- **Other types** (canvas, whiteboard, etc.): Require and return raw JSON structures without transformation.

For complete tool specifications, examples, and behavior details, see **[Tools API Guide](./TOOLS_API_GUIDE.md)**.

#### Response
```json
{
  "success": true,
  "result": { ... },
  "error": null
}
```

**See**: [Tools API Guide](./TOOLS_API_GUIDE.md) for complete documentation.

---

## Agentic Chat API

Interact with AI agents that have direct access to your workspace tools and files.

### Start New Chat
Initialize a stateful agentic session.

**Endpoint**: `POST /api/v1/workspaces/:id/chats`

**Authentication**: Required (JWT access token)

##### Request
```json
{
  "goal": "I want to start a new blog post about Rust.",
    "files": ["019bf537-f228-7cd3-aa1c-3da8af302e12"],

  "role": "assistant",
  "model": "gpt-4o-mini"
}
```

- `goal`: The initial prompt or objective for the agent.
- `files`: Optional array of UUIDs for files to include in the initial context.
- `role`: Optional agent role (e.g., `assistant`). Defaults to `assistant` (Coworker).
- `model`: Optional LLM model override.

##### Response (201 Created)
```json
{
  "chat_id": "uuid-chat-session-id",
  "plan_id": null
}
```

---

### Send Message
Send a subsequent message to an active chat session.

**Endpoint**: `POST /api/v1/workspaces/:id/chats/:chat_id`

**Authentication**: Required (JWT access token)

##### Request
```json
{
  "content": "Please read the outline and suggest a title."
}
```

##### Response (202 Accepted)
```json
{
  "status": "accepted"
}
```

---

### Get Chat History
Retrieve full message history and configuration for a chat session.

**Endpoint**: `GET /api/v1/workspaces/:id/chats/:chat_id`

**Authentication**: Required (JWT access token)

##### Response (200 OK)
```json
{
  "file_id": "019bfa7f-7b41-7d31-9368-aed217a36c7e",
  "agent_config": {
    "model": "gpt-4o-mini",
    "temperature": 0.7,
    "persona_override": null
  },
  "messages": [
    {
      "id": "019bfa7f-...",
      "role": "user",
      "content": "Hello",
      "created_at": "2024-01-26T12:00:00Z"
    }
  ]
}
```

---

### Chat Events (SSE)
Connect to the real-time event stream for an agentic session.

**Endpoint**: `GET /api/v1/workspaces/:id/chats/:chat_id/events`

**Authentication**: Required (JWT access token or Cookie)

**Format**: `text/event-stream`

**Events**:
- `thought`: Internal reasoning from the agent.
- `call`: Tool invocation details.
- `observation`: Tool execution results (includes `success` boolean).
- `chunk`: Incremental text chunks for the response.
- `done`: Finalization of the execution turn.
- `stopped`: Graceful cancellation signal (includes `reason` and optional `partial_response`).

---

### Stop Chat Generation
Gracefully stop an ongoing AI generation. Allows current tool execution to complete before stopping.

**Endpoint**: `POST /api/v1/workspaces/:id/chats/:chat_id/stop`

**Authentication**: Required (JWT access token)

##### Request
No request body required.

##### Response (200 OK)
```json
{
  "status": "cancelled",
  "chat_id": "uuid-chat-session-id"
}
```

##### Behavior
- **Graceful**: If AI is executing a tool, it completes before stopping
- **Partial Save**: Any text generated before cancellation is saved to `chat_messages` table
- **System Marker**: A system message is added for AI context: `[System: Response was interrupted by user (user_cancelled)]`
- **Actor Continues**: The chat actor remains alive for future interactions

##### SSE Event
After stopping, the SSE stream sends:
```json
{
  "type": "stopped",
  "data": {
    "reason": "user_cancelled",
    "partial_response": "Text generated before stop..."
  }
}
```

##### Error Responses
**404 Not Found** - Chat actor doesn't exist or timed out:
```json
{
  "error": "Chat actor not found for chat {chat_id}",
  "code": "NOT_FOUND"
}
```

##### Example
```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/chats/{chat_id}/stop \
  -H "Authorization: Bearer <access_token>"
```

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
   → Returns NEW access_token + NEW refresh_token (rotation)
   → Old refresh_token is immediately invalidated
   → IMPORTANT: Always use the NEW refresh_token from response
   → Sets both access_token and refresh_token cookies (browser clients)

5. Repeat step 3-4 until refresh_token expires (30 days)
   → Then login again (step 2)

6. POST /api/v1/auth/logout (optional - logout before expiration)
   → Invalidates refresh_token server-side
   → Clears both access_token and refresh_token cookies
   → User must login again to access protected resources
```

**Token Rotation Security Benefit**: Each refresh generates a new refresh token and invalidates the old one. This limits token theft replay attacks with automatic stolen token detection (5-minute grace period for legitimate double-clicks, 403 error for token theft after grace period).

---

## JWT Authentication Middleware

The API provides a reusable JWT authentication middleware that can be applied to any protected endpoint.

### Overview

The middleware provides:
- **JWT validation**: Verifies JWT signature and expiration
- **User caching**: Reduces database queries by caching authenticated users
- **Multi-source authentication**: Supports Authorization header (API clients) and Cookie (browser clients)
- **Automatic user context**: Adds `AuthenticatedUser` to request extensions for handler access

### How It Works

1. **Request arrives** at protected endpoint
2. **Middleware extracts JWT** from Authorization header OR Cookie (header takes priority)
3. **Validates JWT** signature and expiration using secret
4. **Extracts user_id** from JWT claims
5. **Checks cache** for user data (key: `user:{user_id}`)
6. **On cache hit**: Returns cached user data (no database query)
7. **On cache miss**: Queries database, caches user with 15-minute TTL
8. **Adds user** to request extensions as `AuthenticatedUser`
9. **Calls handler** with user context available

### Performance Benefits

- **First request**: Validate JWT + Query DB + Cache user
- **Subsequent requests**: Validate JWT + Cache hit (no DB query)
- **Result**: Significantly reduced database load for authenticated requests

### Configuration

```bash
# User cache TTL in seconds (default: 900 = 15 minutes)
# Matches JWT access token expiration for consistency
BUILDSCALE__CACHE__USER_CACHE_TTL_SECONDS=900
```

### Using Protected Endpoints

#### API/Mobile Clients (Authorization Header)

```bash
# Get access token from login response
ACCESS_TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

# Access protected endpoint with Authorization header
curl http://localhost:3000/api/v1/health/cache \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

#### Browser Clients (Cookie)

```javascript
// Cookies are set automatically by login endpoint
// Access protected endpoint - cookie sent automatically
fetch('/api/v1/health/cache')
  .then(response => response.json())
  .then(data => console.log(data));
```

### Creating Protected Endpoints

#### Step 1: Add Route to Protected Router

In `src/lib.rs`, add your route to the protected router:

```rust
Router::new()
    .route("/health/cache", get(health_cache))
    .route("/your-protected-endpoint", get(your_protected_handler))  // Add here
    .route_layer(middleware::from_fn_with_state(
        state.clone(),
        jwt_auth_middleware,
    ))
```

#### Step 2: Extract AuthenticatedUser in Handler

```rust
use axum::{Extension, State};
use serde_json::Json;
use crate::middleware::auth::AuthenticatedUser;
use crate::state::AppState;

pub async fn your_protected_handler(
    Extension(user): Extension<AuthenticatedUser>,  // Extract from middleware
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, Error> {
    // Access user data
    let user_id = user.id;
    let email = &user.email;
    let full_name = &user.full_name;

    // Your handler logic here
    Ok(Json(json!({
        "user_id": user_id,
        "email": email,
    })))
}
```

### Available User Fields

The `AuthenticatedUser` struct provides:

| Field | Type | Description |
|-------|------|-------------|
| `id` | `Uuid` | User's unique identifier |
| `email` | `String` | User's email address |
| `full_name` | `Option<String>` | User's full name (optional) |

### Error Handling

#### 401 Unauthorized (Invalid JWT)

```json
{
  "error": "No valid token found in Authorization header or cookie"
}
```

**Causes**:
- Missing Authorization header and Cookie
- Invalid JWT signature
- Expired JWT token
- Malformed JWT token

#### Solution

1. **Check token is present**: Include Authorization header or Cookie
2. **Verify token validity**: Ensure token is from a valid login
3. **Refresh if expired**: Use `/api/v1/auth/refresh` to get new access token
4. **Login if needed**: Use `/api/v1/auth/login` to get fresh tokens

### Token Priority

When both Authorization header and Cookie are present:
1. **Authorization header takes priority**
2. Cookie is used as fallback
3. Prevents conflicts in multi-client scenarios

### Security Features

- **HMAC-signed tokens**: Prevents token tampering
- **Automatic expiration**: Tokens expire after 15 minutes
- **Secure caching**: User data cached separately from authentication tokens
- **No sensitive data in public endpoints**: Commit hashes, build info not exposed
- **Configurable TTL**: User cache expiration matches JWT expiration

### Example: Complete Protected Endpoint

```rust
// In src/lib.rs - Add to protected router
Router::new()
    .route("/api/user/profile", get(get_user_profile))
    .route_layer(middleware::from_fn_with_state(
        state.clone(),
        jwt_auth_middleware,
    ))

// In src/handlers/users.rs - Handler implementation
use axum::{Extension, Json};
use serde_json::json;

pub async fn get_user_profile(
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Json<serde_json::Value>, Error> {
    Ok(Json(json!({
        "id": user.id,
        "email": user.email,
        "full_name": user.full_name,
    })))
}
```

---

## API Endpoints

### Health Check

Monitor server health and status. Two endpoints are available:

- **Public Health Check** (`GET /api/v1/health`) - Simple status without authentication
- **Cache Health Metrics** (`GET /api/v1/health/cache`) - Detailed cache metrics requiring JWT authentication

---

#### Public Health Check

Simple health status for load balancers and health monitoring. No authentication required.

**Endpoint**: `GET /api/v1/health`

**Authentication**: Not required

**Security**: No sensitive information exposed (no commit hashes, build timestamps, or cache metrics)

##### Request

```bash
curl http://localhost:3000/api/v1/health
```

##### Response (200 OK)

```json
{
  "status": "ok"
}
```

##### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `status` | string | Status indicator (always "ok") |

##### Use Cases

- **Load balancer health checks**: Verify server is running
- **Infrastructure monitoring**: Simple uptime monitoring
- **Docker HEALTHCHECK**: Container health monitoring
- **Kubernetes readiness probes**: Check if server is ready

##### Security Note

This endpoint does NOT expose:
- Git commit hashes
- Build timestamps
- Version information
- Cache metrics
- Any other sensitive information

Use the protected `/api/v1/health/cache` endpoint for detailed monitoring.

---

#### Cache Health Metrics

Detailed cache performance metrics. Requires JWT authentication.

**Endpoint**: `GET /api/v1/health/cache`

**Authentication**: Required (JWT access token)

**Token Sources**:
- **Authorization header** (API/Mobile clients): `Authorization: Bearer <access_token>`
- **Cookie** (Browser clients): `access_token=<token>` (automatically sent)

##### Request

**With Authorization header** (API clients):
```bash
curl http://localhost:3000/api/v1/health/cache \
  -H "Authorization: Bearer <access_token>"
```

**With Cookie** (browser clients):
```bash
curl http://localhost:3000/api/v1/health/cache \
  -H "Cookie: access_token=<token>"
```

##### Response (200 OK)

```json
{
  "num_keys": 42,
  "last_worker_time": "2026-01-08T10:00:00Z",
  "cleaned_count": 5,
  "size_bytes": 18432
}
```

##### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `num_keys` | integer | Current number of entries in cache |
| `last_worker_time` | string or null | ISO8601 timestamp of last cleanup (null if never run) |
| `cleaned_count` | integer | Number of entries removed by last cleanup |
| `size_bytes` | integer | Estimated memory usage in bytes |

##### Error Responses

**401 Unauthorized** (Invalid or missing JWT):
```json
{
  "error": "No valid token found in Authorization header or cookie"
}
```

##### Use Cases

- **Cache monitoring**: Track cache performance and size
- **Debugging**: Verify cleanup worker is functioning correctly
- **Load testing**: Monitor cache growth during performance tests
- **Performance analysis**: Identify cache bottlenecks

##### Authentication & Caching

This endpoint uses JWT authentication middleware with user caching:

1. **JWT Validation**: Middleware validates JWT access token
2. **User Cache Check**: Checks cache for user data (key: `user:{user_id}`)
3. **Cache Miss**: Queries database and caches user with 15-minute TTL
4. **Cache Hit**: Uses cached user data (no database query)
5. **Handler Access**: User data available via `Extension<AuthenticatedUser>`

**Configuration**:
- User cache TTL: `BUILDSCALE__CACHE__USER_CACHE_TTL_SECONDS` (default: 900 seconds = 15 minutes)
- Matches JWT access token expiration for consistency

**Benefits**:
- Reduces database queries for authenticated requests
- Improves response time for subsequent requests
- Scales better under high load

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
  "password": "SecurePass123!",
  "confirm_password": "SecurePass123!",
  "full_name": "John Doe"
}
```

#### Request Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `email` | string | Yes | User's email address (must be unique, valid email format) |
| `password` | string | Yes | User's password (minimum 12 characters) |
| `confirm_password` | string | Yes | Password confirmation (must match `password`) |
| `full_name` | string | No | User's full name (letters, spaces, hyphens, apostrophes, periods) |

#### Validation Rules

- **Email**: Must be valid email format, converted to lowercase
- **Password**: Minimum 12 characters, rejects weak patterns (including "password")
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
  "error": "Password must be at least 12 characters long"
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
    "password": "SecurePass123!",
    "confirm_password": "SecurePass123!",
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
    password: 'SecurePass123!',
    confirm_password: 'SecurePass123!',
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
  "password": "SecurePass123!"
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
    "password": "SecurePass123!"
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
    password: 'SecurePass123!'
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

### User Profile

Retrieve the currently authenticated user's profile.

**Endpoint**: `GET /api/v1/auth/me`

**Authentication**: Required (JWT access token)

#### Response (200 OK)
```json
{
  "user": {
    "id": "019b97ac-e5f5-735b-b0a6-f3a34fcd4ff1",
    "email": "user@example.com",
    "full_name": "John Doe"
  }
}
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

**Token Rotation** (OAuth 2.0 Security Best Practice):
Each refresh request generates a **NEW refresh token** and **invalidates the old one**. This limits token theft replay attacks with automatic stolen token detection (5-minute grace period for legitimate double-clicks).

**Behavior differences by client type**:
- **API/Mobile clients**: Returns JSON only (access_token + refresh_token), does NOT set cookies
- **Browser clients**: Returns JSON AND sets both `access_token` and `refresh_token` cookies

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
  "refresh_token": "a296d8b58edbc757f07670aa8055e9727...",
  "expires_at": "2026-01-07T09:15:00Z"
}
```

**Cookies Set** (browser clients only):
```
Set-Cookie: access_token=eyJ0eXAiOiJKV1QiLCJhbGc...; HttpOnly; SameSite=Lax; Path=/; Max-Age=900
Set-Cookie: refresh_token=a296d8b58edbc757f07670aa80...; HttpOnly; SameSite=Lax; Path=/; Max-Age=2592000
```

**No cookies** are set for API clients using Authorization header.

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `access_token` | string (JWT) | New JWT access token (15 minute expiration) |
| `refresh_token` | string or null | **NEW refresh token** (rotated, old token invalidated), or `null` if within 5-minute grace period after token rotation |
| `expires_at` | string (ISO8601) | When the new access token expires |

#### Token Expiration

- **Access Token**: 15 minutes (configurable via `BUILDSCALE__JWT__ACCESS_TOKEN_EXPIRATION_MINUTES`)
- **Refresh Token**: 30 days (configurable via `BUILDSCALE__SESSIONS__EXPIRATION_HOURS`)
  - **Extended on each refresh**: Session expiration is extended to 30 days from each successful refresh

#### Migration Guide for API Clients

**⚠️ Breaking Change**: The refresh endpoint now returns `refresh_token` in the response.

**Old behavior** (before rotation):
```json
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "expires_at": "2026-01-07T09:15:00Z"
}
```

**New behavior** (with rotation):
```json
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "refresh_token": "a296d8b58edbc757f07670aa8055e9727...",
  "expires_at": "2026-01-07T09:15:00Z"
}
```

**Grace period behavior** (token reused within 5 minutes):
```json
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "refresh_token": null,
  "expires_at": "2026-01-07T09:15:00Z"
}
```

**Action Required**:
1. Extract the `refresh_token` from the response (may be `null`)
2. If `refresh_token` is not `null`, replace the old refresh_token in storage with the new one
3. If `refresh_token` is `null` (grace period), keep using your current token
4. Use the refresh_token for subsequent refresh requests

**Example migration**:
```javascript
// NEW CODE (handles token rotation + grace period)
const data = await response.json();
accessToken = data.access_token;

if (data.refresh_token) {
  // Normal rotation: store new token
  refreshToken = data.refresh_token;
} else {
  // Grace period: keep current token (transparent retry)
  console.log('Token reused within grace period');
}
localStorage.setItem('access_token', accessToken);

if (data.refresh_token) {
  // Normal rotation: store new token
  refreshToken = data.refresh_token;
  localStorage.setItem('refresh_token', refreshToken);
}
// If refresh_token is null (grace period), keep current token
```

**Why this is critical**:
- Old refresh_token is **invalidated** after rotation
- Reusing old refresh_token within 5 minutes: Returns 200 with `refresh_token: null` (grace period)
- Reusing old refresh_token after 5 minutes: Returns `403 Forbidden` (token theft detected)
- Only the latest refresh_token from the most recent refresh is valid

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

**403 Forbidden** - Token theft detected
```json
{
  "error": "Potential security breach detected. Your refresh token was used after rotation. All sessions have been revoked for your protection. Please login again and consider changing your password."
}
```

**When this occurs**:
- A refresh token that was previously rotated is used after the 5-minute grace period
- This indicates potential token theft (stolen token used after rotation)
- ALL user sessions are immediately revoked for security

**Client action required**:
1. Clear all stored tokens (access token and refresh token)
2. Redirect user to login page
3. Show security message explaining the situation
4. Recommend user change their password
5. Log security event for monitoring

**Example client handling**:
```javascript
try {
  const response = await fetch('/api/v1/auth/refresh', {
    method: 'POST',
    credentials: 'include' // Include cookies
  });

  if (response.status === 403) {
    // Token theft detected - clear all tokens and redirect to login
    localStorage.clear();
    sessionStorage.clear();
    window.location.href = '/login?security=token-theft-detected';
    return;
  }

  const data = await response.json();
  // Store new tokens...
} catch (error) {
  console.error('Token refresh failed:', error);
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
  "refresh_token": "a296d8b58edbc757f07670aa8055e9727...",
  "expires_at": "2026-01-07T10:30:00Z"
}

# IMPORTANT: Store the new refresh_token for next refresh
# Old refresh_token is now invalid
```

#### Example Usage (Browser Client)

```bash
# Refresh using Cookie
curl -X POST http://localhost:3000/api/v1/auth/refresh \
  -H "Cookie: refresh_token=<token>" \
  -c cookies.txt

# Response: JSON + both cookies are set
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "refresh_token": "a296d8b58edbc757f07670aa8055e9727...",
  "expires_at": "2026-01-07T10:30:00Z"
}

# Both access_token and refresh_token cookies are automatically set
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
  refreshToken = data.refresh_token; // CRITICAL: Store the new refresh token
  localStorage.setItem('access_token', accessToken);
  localStorage.setItem('refresh_token', refreshToken); // CRITICAL: Update stored token
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
  // Both access_token and refresh_token cookies are automatically set by the server
  // No need to manually update localStorage
  return data.access_token;
};

// Subsequent requests automatically include both cookies
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

#### Security Benefits of Token Rotation

**Token rotation with stolen token detection** significantly improves security by preventing token theft replay attacks:

**Before rotation** (old behavior):
- Stolen refresh token usable for 30 days
- Attacker can access API until token expires
- Security window: 30 days

**After rotation** (current behavior):
- ✅ **MITIGATED**: Stolen token detection with automatic session revocation
- Stolen refresh token only usable ONCE (if attacker wins the initial race)
- After 5-minute grace period: Using old token triggers 403 error + revokes ALL sessions
- Attacker blocked immediately on first use after grace period
- Legitimate user protected by automatic session revocation

**Attack Scenario**:
```
1. Attacker steals refresh_token via XSS/network sniffing
2. Attacker uses refresh_token → gets NEW refresh_token (attacker wins initial race)
3. Old refresh_token is recorded as revoked in database
4. Legitimate user tries to refresh within 5 minutes → gets 200 with access_token only (grace period)
5. Legitimate user tries to refresh after 5 minutes → gets 403 Forbidden + all sessions revoked
6. Attacker tries to use old token after 5 minutes → gets 403 Forbidden (theft detected)
7. Both attacker and legitimate user must login again
```

**Security Benefits**:
- Attacker can only use stolen token ONCE (if they win the initial race)
- Grace period (5 min) prevents false positives from double-clicks
- After grace period, automatic theft detection blocks attacker
- ALL user sessions revoked for security (prevents lateral movement)

**Compliance**:
- OAuth 2.0 Security Best Current Practice (RFC 6819 Section 5.2.2.1)
- Recommended for all public-facing applications
- Industry standard for mobile/web applications

---

### User Logout

Logout the current user by invalidating their refresh token session. Supports both Authorization header (API clients) and Cookie (browser clients).

**Endpoint**: `POST /api/v1/auth/logout`

**Authentication**: No (uses refresh token instead)

#### How It Works

The logout endpoint accepts refresh tokens from two sources with **priority handling**:

1. **Authorization header** (API/Mobile clients): `Authorization: Bearer <refresh_token>`
2. **Cookie** (Browser clients): `refresh_token=<token>`

**Priority**: Authorization header takes precedence if both are present.

**What happens on logout**:
- **Session invalidated**: Refresh token is deleted from database (cannot be used again)
- **Cookies cleared**: Both `access_token` and `refresh_token` cookies are cleared with `Max-Age=0`
- **Tokens revoked**: Both access and refresh tokens become invalid immediately

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
  "message": "Logout successful"
}
```

**Cookies Cleared** (both client types):
```
Set-Cookie: access_token=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0
Set-Cookie: refresh_token=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0
```

Both cookies are set with `Max-Age=0` to instruct the browser to immediately delete them.

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `message` | string | Confirmation message |

#### Security Behavior

After logout:
- ✅ **Refresh token** is deleted from database (cannot be reused)
- ✅ **Access token** remains valid until expiration (15 minutes)
- ✅ **Both cookies** are cleared immediately (browser clients)
- ✅ **API clients** should delete stored tokens from secure storage
- ⚠️ **Access tokens** cannot be immediately revoked (JWT limitation)
  - Best practice: Implement token blacklist for immediate revocation
  - Alternative: Use short expiration times (15 minutes)

#### Error Responses

**400 Bad Request** - Invalid token format
```json
{
  "error": "No valid refresh token found in Authorization header or cookie"
}
```

**401 Unauthorized** - Token not found or already logged out
```json
{
  "error": "Invalid refresh token"
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
# Logout using Authorization header
curl -X POST http://localhost:3000/api/v1/auth/logout \
  -H "Authorization: Bearer <refresh_token>"

# Response: JSON + clear cookie headers
{
  "message": "Logout successful"
}

# Set-Cookie headers clear both tokens
# (API clients should delete tokens from local storage)
```

#### Example Usage (Browser Client)

```bash
# Logout using Cookie
curl -X POST http://localhost:3000/api/v1/auth/logout \
  -H "Cookie: refresh_token=<token>" \
  -c cookies.txt

# Response: JSON + clear cookie headers
{
  "message": "Logout successful"
}

# Both cookies are automatically cleared by browser
# Set-Cookie headers with Max-Age=0 instruct browser to delete cookies
```

#### JavaScript/TypeScript Example

**API Client Logout**:
```javascript
const logout = async () => {
  const response = await fetch('http://localhost:3000/api/v1/auth/logout', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${refreshToken}`
    }
  });

  if (!response.ok) {
    console.error('Logout failed');
    return;
  }

  // Clear tokens from local storage
  localStorage.removeItem('access_token');
  localStorage.removeItem('refresh_token');

  // Redirect to login
  window.location.href = '/login';
};
```

**Browser Client Logout**:
```javascript
const logout = async () => {
  const response = await fetch('http://localhost:3000/api/v1/auth/logout', {
    method: 'POST',
    credentials: 'include' // Send cookies automatically
  });

  if (!response.ok) {
    console.error('Logout failed');
    return;
  }

  // Cookies are automatically cleared by server
  // No need to manually clear localStorage

  // Redirect to login
  window.location.href = '/login';
};
```

#### Best Practices

1. **Always call logout on user logout action**
   - Don't just clear local storage
   - Invalidate server-side session for security

2. **Handle logout errors gracefully**
   - Show user-friendly error message
   - Allow retry or continue with local cleanup

3. **Clear local token storage**
   - API clients: Delete tokens from localStorage/keychain
   - Browser clients: Cookies cleared automatically by server

4. **Redirect to login after logout**
   - Prevent access to protected resources
   - Clear any application state

5. **Handle concurrent sessions**
   - Logout invalidates only the specific refresh token used
   - User may have other active sessions (different devices)
   - Consider "logout all devices" functionality for security

#### When to Logout

- **User initiates logout**: Explicit logout button/action
- **Security event**: Suspicious activity detected
- **Password change**: Optional - revoke all sessions on password reset
- **Account deletion**: Invalidate all user sessions
- **Admin action**: Force logout specific user (admin functionality)

---


---

## Workspaces API

### Create Workspace

Create a new workspace with the authenticated user as the owner.

**Endpoint**: `POST /api/v1/workspaces`

**Authentication**: Required (JWT access token)

#### Request

**Headers**:
```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**Body**:
```json
{
  "name": "My New Startup"
}
```

#### Request Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Workspace name (1-100 characters) |

#### Response (200 OK)

```json
{
  "workspace": {
    "id": "019b97ac-e5f5-735b-b0a6-f3a34fcd4ff1",
    "name": "My New Startup",
    "owner_id": "019b97ac-e5f5-735b-b0a6-f3a34fcd4ff1",
    "created_at": "2026-01-07T09:00:00Z",
    "updated_at": "2026-01-07T09:00:00Z"
  },
  "roles": [
    {
      "id": "...",
      "name": "Owner",
      "workspace_id": "...",
      "is_system": true,
      ...
    },
    ...
  ],
  "owner_membership": {
    "user_id": "...",
    "workspace_id": "...",
    "role_id": "...",
    ...
  }
}
```

---

### List User Workspaces

List all workspaces where the authenticated user is a member (including owned workspaces).

**Endpoint**: `GET /api/v1/workspaces`

**Authentication**: Required (JWT access token)

#### Request

**Headers**:
```
Authorization: Bearer <access_token>
```

#### Response (200 OK)

```json
{
  "workspaces": [
    {
      "id": "019b97ac-e5f5-735b-b0a6-f3a34fcd4ff1",
      "name": "My New Startup",
      "owner_id": "019b97ac-e5f5-735b-b0a6-f3a34fcd4ff1",
      "created_at": "2026-01-07T09:00:00Z",
      "updated_at": "2026-01-07T09:00:00Z"
    }
  ],
  "count": 1
}
```

---

### Get Single Workspace

Get details of a specific workspace.

**Endpoint**: `GET /api/v1/workspaces/:id`

**Authentication**: Required (JWT access token)
**Permission**: User must be a member of the workspace.

#### Request

**Path Parameters**:
- `id`: Workspace UUID

**Headers**:
```
Authorization: Bearer <access_token>
```

#### Response (200 OK)

```json
{
  "workspace": {
    "id": "019b97ac-e5f5-735b-b0a6-f3a34fcd4ff1",
    "name": "My New Startup",
    "owner_id": "019b97ac-e5f5-735b-b0a6-f3a34fcd4ff1",
    "created_at": "2026-01-07T09:00:00Z",
    "updated_at": "2026-01-07T09:00:00Z"
  }
}
```

#### Error Responses

**403 Forbidden** - Not a Member
```json
{
  "error": "Access forbidden: User is not a member of this workspace",
  "code": "FORBIDDEN"
}
```

---

### Update Workspace

Update workspace details (e.g., name).

**Endpoint**: `PATCH /api/v1/workspaces/:id`

**Authentication**: Required (JWT access token)
**Permission**: User must be the **Owner** of the workspace.

#### Request

**Path Parameters**:
- `id`: Workspace UUID

**Headers**:
```
Content-Type: application/json
Authorization: Bearer <access_token>
```

**Body**:
```json
{
  "name": "Rebranded Startup"
}
```

#### Response (200 OK)

```json
{
  "workspace": {
    "id": "019b97ac-e5f5-735b-b0a6-f3a34fcd4ff1",
    "name": "Rebranded Startup",
    "owner_id": "...",
    "created_at": "...",
    "updated_at": "..."
  }
}
```

#### Error Responses

**403 Forbidden** - Not Owner
```json
{
  "error": "Only the workspace owner can update workspace details",
  "code": "FORBIDDEN"
}
```

---

### Delete Workspace

Delete a workspace and all associated data (roles, members).

**Endpoint**: `DELETE /api/v1/workspaces/:id`

**Authentication**: Required (JWT access token)
**Permission**: User must be the **Owner** of the workspace.

#### Request

**Path Parameters**:
- `id`: Workspace UUID

**Headers**:
```
Authorization: Bearer <access_token>
```

#### Response (200 OK)

```json
{
  "message": "Workspace deleted successfully"
}
```

#### Error Responses

**403 Forbidden** - Not Owner
```json
{
  "error": "Only the workspace owner can delete the workspace",
  "code": "FORBIDDEN"
}
```

---

## Workspace Members API

Manage workspace members and access control.

### List Members

Lists all members in a workspace with detailed user and role information.

**Endpoint**: `GET /api/v1/workspaces/:id/members`

**Authentication**: Required (JWT access token)
**Permission**: User must be a member of workspace with `members:read` permission.

#### Response (200 OK)
```json
{
  "members": [
    {
      "workspace_id": "...",
      "user_id": "...",
      "email": "user@example.com",
      "full_name": "John Doe",
      "role_id": "...",
      "role_name": "Editor"
    }
  ],
  "count": 1
}
```

---

### Add Member

Add a new member to the workspace by email. The user must already exist in the system.

**Endpoint**: `POST /api/v1/workspaces/:id/members`

**Authentication**: Required (JWT access token)
**Permission**: User must have `members:write` permission.

#### Request
```json
{
  "email": "teammate@example.com",
  "role_name": "member"
}
```

---

### Get My Membership

Get the authenticated user's membership details for a specific workspace.

**Endpoint**: `GET /api/v1/workspaces/:id/members/me`

**Authentication**: Required (JWT access token)

#### Response (200 OK)
```json
{
  "member": {
    "workspace_id": "...",
    "user_id": "...",
    "email": "...",
    "full_name": "...",
    "role_id": "...",
    "role_name": "admin"
  }
}
```

---

### Update Member Role

Update a member's role in the workspace.

**Endpoint**: `PATCH /api/v1/workspaces/:id/members/:user_id`

**Authentication**: Required (JWT access token)
**Permission**: User must have `members:write` permission.

#### Request
```json
{
  "role_name": "editor"
}
```

---

### Remove Member

Remove a member from the workspace or leave the workspace (if removing yourself).

**Endpoint**: `DELETE /api/v1/workspaces/:id/members/:user_id`

**Authentication**: Required (JWT access token)
**Permission**: User must have `members:write` permission OR be removing themselves.
**Note**: The workspace owner cannot be removed.

---

## Error Responses

All error responses follow a consistent format with error codes and optional field-level details.

### Error Response Structure

**Generic Error**:
```json
{
  "error": "Human-readable error message",
  "code": "ERROR_CODE"
}
```

**Validation Error (Single Field)**:
```json
{
  "error": "Validation failed",
  "code": "VALIDATION_ERROR",
  "fields": {
    "email": "Invalid email format"
  }
}
```

**Validation Error (Multiple Fields)**:
```json
{
  "error": "Validation failed",
  "code": "VALIDATION_ERROR",
  "fields": {
    "email": "Invalid email format",
    "password": "Password must be at least 12 characters long"
  }
}
```

### HTTP Status Codes & Error Codes

| Status | Error Code | Meaning |
|--------|------------|---------|
| **200 OK** | - | Request completed successfully |
| **400 Bad Request** | `VALIDATION_ERROR` | Invalid input data (email format, password length) |
| **401 Unauthorized** | `AUTHENTICATION_FAILED` | Wrong email/password |
| **401 Unauthorized** | `INVALID_TOKEN` | Token is invalid or malformed |
| **401 Unauthorized** | `SESSION_EXPIRED` | Token has expired |
| **403 Forbidden** | `FORBIDDEN` | Access denied (not member/owner) |
| **403 Forbidden** | `TOKEN_THEFT` | Token theft detected (security breach) |
| **404 Not Found** | `NOT_FOUND` | Resource not found |
| **409 Conflict** | `CONFLICT` | Resource already exists (duplicate email) |
| **500 Internal Server Error** | `INTERNAL_ERROR` | Database or server error |
| **500 Internal Server Error** | `CONFIG_ERROR` | Configuration error |
| **500 Internal Server Error** | `CACHE_ERROR` | Cache operation failed |

### Common Error Messages

| Error Message | Code | HTTP Status | Cause |
|---------------|------|-------------|-------|
| `"Email cannot be empty"` | `VALIDATION_ERROR` | 400 | Email field missing or empty |
| `"Password must be at least 12 characters long"` | `VALIDATION_ERROR` | 400 | Password too short |
| `"Passwords do not match"` | `VALIDATION_ERROR` | 400 | Password and confirmation don't match |
| `"Email 'user@example.com' already exists"` | `CONFLICT` | 409 | Duplicate email registration |
| `"Invalid email or password"` | `AUTHENTICATION_FAILED` | 401 | Wrong login credentials |
| `"Access forbidden: User is not a member of this workspace"` | `FORBIDDEN` | 403 | Accessing workspace without membership |
| `"Only the workspace owner can update workspace details"` | `FORBIDDEN` | 403 | Non-owner trying to update workspace |
| `"Potential security breach detected..."` | `TOKEN_THEFT` | 403 | Stolen refresh token used after rotation |

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
    "password": "SecurePass123!",
    "confirm_password": "SecurePass123!",
    "full_name": "Test User"
  }'

# 3. Login
curl -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "SecurePass123!"
  }'

# 4. Test duplicate email (should return 409)
curl -X POST http://localhost:3000/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "NewSecurePass123!",
    "confirm_password": "NewSecurePass123!"
  }'

# 5. Logout with Authorization header (API client)
# Note: Replace <refresh_token> with actual token from login response
curl -X POST http://localhost:3000/api/v1/auth/logout \
  -H "Authorization: Bearer <refresh_token>"

# 6. Logout with Cookie (browser client)
curl -X POST http://localhost:3000/api/v1/auth/logout \
  -H "Cookie: refresh_token=<token>"
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
- ✅ Token refresh with both header and cookie modes
- ✅ User logout with cookie clearing
- [ ] Verification that logged-out tokens cannot be reused
- [ ] Error handling (wrong password, duplicate email, weak password)
- [ ] Full request/response logging with headers

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
     "password": "SecurePass123!"
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

See `docs/SERVICES_API_GUIDE.md` for complete service layer API reference.
