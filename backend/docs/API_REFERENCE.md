# API Reference

Complete API reference for BuildScale multi-tenant workspace-based RBAC system.

## Table of Contents

- [Quick Reference](#quick-reference)
- [Authentication](#authentication)
- [REST API Endpoints](#rest-api-endpoints)
- [Tools API](#tools-api)
- [Services API](#services-api)
- [Error Handling](#error-handling)

---

## Quick Reference

### Base URL
- **Development**: `http://localhost:3000`
- **API Version**: `v1` (all endpoints prefixed with `/api/v1`)

### Authentication
- **Type**: JWT Bearer Token + Session Refresh Token
- **Header**: `Authorization: Bearer <access_token>`
- **Cookie**: `access_token` and `refresh_token` cookies for browser clients

### Response Format
```json
{
  "success": true,
  "result": { ... },
  "error": null
}
```

---

## Authentication

### Endpoints

| Endpoint | Method | Description | Auth |
|----------|--------|-------------|------|
| `/api/v1/auth/register` | POST | Register new user | No |
| `/api/v1/auth/login` | POST | Login and get tokens | No |
| `/api/v1/auth/refresh` | POST | Refresh access token | No (uses refresh token) |
| `/api/v1/auth/logout` | POST | Logout and invalidate session | No (uses refresh token) |
| `/api/v1/auth/me` | GET | Get current user profile | Yes (JWT) |

### Login Request
```json
{
  "email": "user@example.com",
  "password": "securepassword123"
}
```

### Login Response
```json
{
  "user": { "id": "uuid", "email": "...", "full_name": "..." },
  "access_token": "eyJhbGc...",
  "refresh_token": "session-token",
  "access_token_expires_at": "2026-02-15T11:00:00Z",
  "refresh_token_expires_at": "2026-03-15T10:30:00Z"
}
```

### Token Expiration
- **Access Token (JWT)**: 15 minutes (configurable via `BUILDSCALE__JWT__ACCESS_TOKEN_EXPIRATION_MINUTES`)
- **Refresh Token**: 30 days (configurable via `BUILDSCALE__SESSIONS__EXPIRATION_HOURS`)

---

## REST API Endpoints

### Health Check

| Endpoint | Method | Description | Auth |
|----------|--------|-------------|------|
| `/api/v1/health` | GET | Simple health check | No |
| `/api/v1/health/cache` | GET | Cache metrics | Yes (JWT) |

### Workspaces

| Endpoint | Method | Description | Auth |
|----------|--------|-------------|------|
| `/api/v1/workspaces` | POST | Create new workspace | Yes (JWT) |
| `/api/v1/workspaces` | GET | List my workspaces | Yes (JWT) |
| `/api/v1/workspaces/:id` | GET | Get workspace details | Yes (JWT + Member) |
| `/api/v1/workspaces/:id` | PATCH | Update workspace | Yes (JWT + Owner) |
| `/api/v1/workspaces/:id` | DELETE | Delete workspace | Yes (JWT + Owner) |

### Workspace Members

| Endpoint | Method | Description | Auth |
|----------|--------|-------------|------|
| `/api/v1/workspaces/:id/members` | GET | List workspace members | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/members` | POST | Add member by email | Yes (JWT + Admin) |
| `/api/v1/workspaces/:id/members/me` | GET | Get my membership | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/members/:uid` | PATCH | Update member role | Yes (JWT + Admin) |
| `/api/v1/workspaces/:id/members/:uid` | DELETE | Remove member / Leave | Yes (JWT + Member) |

### Files

| Endpoint | Method | Description | Auth |
|----------|--------|-------------|------|
| `/api/v1/workspaces/:id/files` | POST | Create file/folder | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid` | GET | Get file & latest version | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid` | PATCH | Move or rename file | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid` | DELETE | Soft delete file | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid/restore` | POST | Restore from trash | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid/purge` | DELETE | Permanently delete file | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/trash` | GET | List trash items | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid/versions` | POST | Create new version | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/search` | POST | Full-text search files | Yes (JWT + Member) |

### Tags and Links

| Endpoint | Method | Description | Auth |
|----------|--------|-------------|------|
| `/api/v1/workspaces/:id/files/tags/:tag` | GET | List files by tag | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid/tags` | POST | Add tag to file | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid/tags/:tag` | DELETE | Remove tag | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid/links` | POST | Link two files | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid/links/:tid` | DELETE | Remove file link | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/files/:fid/network` | GET | Get file network graph | Yes (JWT + Member) |

### AI Providers

| Endpoint | Method | Description | Auth |
|----------|--------|-------------|------|
| `/api/v1/providers` | GET | Get all configured providers | Yes (JWT) |
| `/api/v1/workspaces/:id/providers` | GET | Get workspace providers | Yes (JWT + Member) |

### Chat

| Endpoint | Method | Description | Auth |
|----------|--------|-------------|------|
| `/api/v1/workspaces/:id/chats` | GET | List recent chats | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/chats` | POST | Start new agentic chat | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/chats/:cid` | GET | Get chat history | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/chats/:cid` | POST | Send message | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/chats/:cid` | PATCH | Update chat metadata | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/chats/:cid/stop` | POST | Stop AI generation | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/chats/:cid/events` | GET | SSE event stream | Yes (JWT + Member) |
| `/api/v1/workspaces/:id/chats/:cid/context` | GET | Get chat context | Yes (JWT + Member) |

### Agent Sessions

| Endpoint | Method | Description | Auth |
|----------|--------|-------------|------|
| `/api/v1/workspaces/:id/agent-sessions` | GET | List workspace sessions | Yes (JWT + Member) |
| `/api/v1/agent-sessions/:sid` | GET | Get session details | Yes (JWT + Owner) |
| `/api/v1/agent-sessions/:sid/pause` | POST | Pause session | Yes (JWT + Owner) |
| `/api/v1/agent-sessions/:sid/resume` | POST | Resume session | Yes (JWT + Owner) |
| `/api/v1/agent-sessions/:sid` | DELETE | Cancel session | Yes (JWT + Owner) |

---

## Tools API

Execute workspace tools through a unified endpoint.

### Endpoint

`POST /api/v1/workspaces/:id/tools`

### Request Format

```json
{
  "tool": "string",
  "args": { ... }
}
```

### Available Tools

#### File Operations

| Tool | Description | Key Arguments |
|------|-------------|---------------|
| `ls` | List directory contents | `path?`, `recursive?`, `limit?` |
| `read` | Read file contents | `path`, `offset?`, `limit?` |
| `write` | Create or update file | `path`, `content`, `file_type?` |
| `edit` | Edit file content | `path`, `old_string`, `new_string` |
| `rm` | Delete file or folder | `path` |
| `mv` | Move or rename file | `source`, `destination` |
| `touch` | Update timestamp or create | `path` |
| `mkdir` | Create directory | `path` |
| `grep` | Regex search files | `pattern`, `path_pattern?`, `case_sensitive?` |
| `glob` | Pattern-based discovery | `patterns[]`, `path?` |
| `find` | Search by metadata | `name?`, `path?`, `file_type?` |
| `cat` | Concatenate files | `paths[]`, `offset?`, `limit?` |
| `file_info` | Query file metadata | `path` |
| `read_multiple_files` | Batch file reading | `paths[]`, `limit?` |

#### Plan Tools

| Tool | Description | Key Arguments |
|------|-------------|---------------|
| `plan_write` | Create plan file | `title`, `content`, `path?`, `status?` |
| `plan_read` | Read plan file | `path?`, `name?`, `offset?`, `limit?` |
| `plan_edit` | Edit plan file | `path`, `old_string?`, `new_string?` |
| `plan_list` | List plan files | `status?`, `limit?` |

#### Memory Tools

| Tool | Description | Key Arguments |
|------|-------------|---------------|
| `memory_set` | Store a memory | `scope`, `category`, `key`, `title`, `content`, `tags?` |
| `memory_get` | Retrieve a memory | `scope`, `category`, `key` |
| `memory_search` | Search memories | `pattern`, `scope?`, `category?`, `tags?` |
| `memory_delete` | Delete a memory | `scope`, `category`, `key` |
| `memory_list` | List categories/tags | `list_type`, `scope?`, `category?` |

#### System Tools

| Tool | Description | Key Arguments |
|------|-------------|---------------|
| `ask_user` | Request user input | `questions[]` |
| `exit_plan_mode` | Transition to Build Mode | `allowedPrompts?`, `pushToRemote?` |

#### Web Tools

| Tool | Description | Key Arguments |
|------|-------------|---------------|
| `web_fetch` | Fetch web content | `url`, `format?`, `timeout?` |
| `web_search` | Search the web | `query`, `max_results?` |

### File Sync Status

Tool responses include a `synced` boolean:
- `true`: File is in the database (full metadata)
- `false`: File exists on disk only (basic metadata)

---

## Services API

### User Authentication

```rust
// Registration
register_user(conn, RegisterUser) -> Result<User>
register_user_with_workspace(conn, UserWorkspaceRegistrationRequest) -> Result<UserWorkspaceResult>

// Authentication
login_user(conn, LoginUser) -> Result<LoginResult>
validate_session(conn, session_token) -> Result<User>
logout_user(conn, session_token) -> Result<()>
refresh_access_token(conn, refresh_token) -> Result<RefreshTokenResult>

// Password utilities
verify_password(password, hash) -> Result<bool>
generate_session_token() -> Result<String>
update_password(conn, user_id, new_password) -> Result<()>
```

### Workspace Management

```rust
create_workspace(conn, CreateWorkspaceRequest) -> Result<CompleteWorkspaceResult>
create_workspace_with_members(conn, CreateWorkspaceWithMembersRequest) -> Result<CompleteWorkspaceResult>
get_workspace(conn, id) -> Result<Workspace>
list_user_workspaces(conn, owner_id) -> Result<Vec<Workspace>>
update_workspace_owner(conn, workspace_id, current_owner_id, new_owner_id) -> Result<Workspace>
can_access_workspace(conn, workspace_id, user_id) -> Result<bool>
delete_workspace(conn, id) -> Result<u64>
```

### Member Management

```rust
list_workspace_members(conn, workspace_id) -> Result<Vec<WorkspaceMemberDetailed>>
get_my_membership(conn, workspace_id, user_id) -> Result<WorkspaceMemberDetailed>
add_member_by_email(conn, workspace_id, requester_user_id, AddMemberRequest) -> Result<WorkspaceMemberDetailed>
update_member_role(conn, workspace_id, target_user_id, requester_user_id, UpdateMemberRoleRequest) -> Result<WorkspaceMemberDetailed>
remove_member(conn, workspace_id, target_user_id, requester_user_id) -> Result<()>
```

### File Management

```rust
create_file_with_content(conn, CreateFileRequest) -> Result<FileWithContent>
get_file_with_content(conn, workspace_id, file_id) -> Result<FileWithContent>
move_or_rename_file(conn, workspace_id, file_id, request) -> Result<File>
soft_delete_file(conn, workspace_id, file_id) -> Result<File>
restore_file(conn, workspace_id, file_id) -> Result<File>
purge_file(conn, workspace_id, file_id) -> Result<u64>
list_trash(conn, workspace_id) -> Result<Vec<File>>
```

### Role Management

```rust
create_default_roles(conn, workspace_id) -> Result<Vec<Role>>
get_role_by_name(conn, workspace_id, name) -> Result<Option<Role>>
list_workspace_roles(conn, workspace_id) -> Result<Vec<Role>>
```

### Invitation Management

```rust
create_invitation(conn, workspace_id, inviter_id, CreateInvitationRequest) -> Result<Invitation>
accept_invitation(conn, token, user_id) -> Result<WorkspaceMember>
revoke_invitation(conn, invitation_id, workspace_id, revoker_id) -> Result<()>
bulk_create_invitations(conn, workspace_id, inviter_id, requests) -> Result<Vec<InvitationResult>>
cleanup_expired_invitations(conn) -> Result<u64>
```

---

## Error Handling

### Error Response Format

```json
{
  "error": "Descriptive error message",
  "code": "ERROR_CODE"
}
```

### Validation Errors

```json
{
  "error": "Validation failed",
  "code": "VALIDATION_ERROR",
  "fields": {
    "email": "Invalid email format",
    "password": "Password must be at least 12 characters"
  }
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `VALIDATION_ERROR` | 400 | Field validation failed |
| `NOT_FOUND` | 404 | Resource not found |
| `FORBIDDEN` | 403 | Access denied |
| `CONFLICT` | 409 | Resource conflict |
| `AUTHENTICATION_FAILED` | 401 | Invalid credentials |
| `INVALID_TOKEN` | 401 | Invalid or malformed token |
| `SESSION_EXPIRED` | 401 | Session token expired |
| `TOKEN_THEFT` | 403 | Stolen refresh token detected |
| `INTERNAL_ERROR` | 500 | Internal server error |

---

## Related Documentation

- [ARCHITECTURE.md](./ARCHITECTURE.md) - System architecture overview
- [AUTHENTICATION.md](./AUTHENTICATION.md) - Authentication and RBAC details
- [AI_SYSTEM.md](./AI_SYSTEM.md) - AI agent system architecture
- [FILE_SYSTEM.md](./FILE_SYSTEM.md) - File system architecture
