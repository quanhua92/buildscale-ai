← [Back to Index](./README.md) | **Developer API**: [Services API Guide](./SERVICES_API_GUIDE.md) | **Architecture**: [Everything is a File](./EVERYTHING_IS_A_FILE.md)

# Tools API Guide

HTTP REST API for the BuildScale extensible tool execution system.

## Table of Contents
- [Quick Reference](#quick-reference)
- [Overview](#overview)
- [API Endpoint](#api-endpoint)
- [Tool Specifications](#tool-specifications)
  - [ls - List Directory Contents](#ls---list-directory-contents)
  - [read - Read File Contents](#read---read-file-contents)
  - [write - Create or Update File](#write---create-or-update-file)
  - [rm - Delete File or Folder](#rm---delete-file-or-folder)
  - [mv - Move or Rename File](#mv---move-or-rename-file)
  - [touch - Update Timestamp or Create Empty File](#touch---update-timestamp-or-create-empty-file)
  - [mkdir - Create Directory](#mkdir---create-directory)
  - [edit - Edit File Content](#edit---edit-file-content)
  - [grep - Regex Search Files](#grep---regex-search-files)
- [Authentication & Authorization](#authentication--authorization)
- [Architecture & Extensibility](#architecture--extensibility)
- [Error Responses](#error-responses)
- [Code Examples](#code-examples)
- [Testing](#testing)
- [Related Documentation](#related-documentation)

---

## Quick Reference

| Tool | Description | Arguments | Returns |
|------|-------------|-----------|---------|
| `ls` | List directory contents | `path?`, `recursive?` | `path`, `entries[]` |
| `read` | Read file contents | `path` | `content` |
| `write` | Create or update file | `path`, `content`, `file_type?` | `file_id`, `version_id` |
| `rm` | Delete file or folder | `path` | `file_id` |
| `mv` | Move or rename file | `source`, `destination` | `from_path`, `to_path` |
| `touch` | Update time or create empty | `path` | `path`, `file_id` |
| `mkdir` | Create directory | `path` | `path`, `file_id` |
| `edit` | Edit file content | `path`, `old_string`, `new_string`, `last_read_hash?` | `path`, `file_id`, `version_id` |
| `grep` | Regex search files | `pattern`, `path_pattern?`, `case_sensitive?` | `matches[]` |

**Base URL**: `http://localhost:3000` (default)

**API Version**: `v1` (all endpoints are prefixed with `/api/v1`)

**Endpoint**: `POST /api/v1/workspaces/:id/tools`

---

## Overview

The Tools API provides a unified, extensible interface for executing filesystem operations within workspace contexts. Built on the "Everything is a File" philosophy, it enables AI agents, automation systems, and CLI tools to interact with workspace files through a simple JSON-based protocol.

### Key Features

- **Unified Endpoint**: All tools execute through a single POST endpoint
- **Extensible Architecture**: New tools can be added by implementing the `Tool` trait
- **Workspace Isolation**: All operations are scoped to a specific workspace
- **Database-Backed**: Tools operate on the database-backed file system
- **Version Control**: Write operations automatically create file versions
- **Soft Deletes**: File deletion preserves data for recovery

### Use Cases

- **AI Agents**: LLMs can read, write, and manage files autonomously
- **Automation Scripts**: Programmatic file operations via REST API
- **CLI Tools**: Command-line interfaces for workspace management
- **Web Applications**: Browser-based file editors and managers
- **Integration**: Third-party tools can interact with BuildScale workspaces

---

## API Endpoint

### POST /api/v1/workspaces/:id/tools

Executes a tool with given arguments within a workspace.

**Authentication**: Required (JWT access token)

**Authorization**: Required (User must be a workspace member)

#### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | UUID | Workspace identifier |

#### Request Headers

```
Content-Type: application/json
Authorization: Bearer <access_token>
```

#### Request Body

```json
{
  "tool": "string",
  "args": { ... }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `tool` | string | Yes | Tool name (`ls`, `read`, `write`, `rm`, `mv`, `touch`) |
| `args` | object | Yes | Tool-specific arguments (see [Tool Specifications](#tool-specifications)) |

#### Response (200 OK)

```json
{
  "success": true,
  "result": { ... },
  "error": null
}
```

| Field | Type | Description |
|-------|------|-------------|
| `success` | boolean | `true` if tool executed successfully |
| `result` | object | Tool-specific result data |
| `error` | string or null | Error message if `success` is `false` |

#### Error Responses

**401 Unauthorized** - Invalid or missing JWT
```json
{
  "error": "No valid token found in Authorization header or cookie",
  "code": "INVALID_TOKEN"
}
```

**403 Forbidden** - Not a workspace member
```json
{
  "error": "Access forbidden: User is not a member of this workspace",
  "code": "FORBIDDEN"
}
```

**404 Not Found** - Tool not found
```json
{
  "error": "Tool 'invalid_tool' not found",
  "code": "NOT_FOUND"
}
```

---

## File Type Content Handling

The Tools API handles content differently based on file types to optimize for AI/developer experience:

### Document Files (Auto-Wrap/Unwrap)
Document files use automatic wrapping and unwrapping for convenience:

**Write Behavior:**
- Raw strings are auto-wrapped: `"hello"` → `{"text": "hello"}`
- Explicit objects accepted: `{"text": "hello"}` → stored as-is
- **Why:** Simplifies AI input - no need to wrap every string in an object

**Read Behavior:**
- Simple documents auto-unwrapped: `{"text": "hello"}` → `"hello"`
- Complex documents preserved: `{"text": "hello", "metadata": {}}` → returned as-is
- **Why:** AI gets clean string content without JSON nesting

**Edit & Edit-Many Behavior:**
- Operates on the inner `text` field for `Document` types.
- Resilient: Automatically handles both wrapped (`{"text": "..."}`) and raw string content during the search phase.
- **Result:** Always commits a new version using the standard wrapped structure (`{"text": "..."}`).

**Grep Behavior:**
- Searches specifically within the `text` field of `Document` files.
- Resilient: Searches both wrapped and raw string content in the database.
- High performance: Executes regex directly in the database (PostgreSQL).
- Returns line-accurate results including line numbers and text chunks.

**Examples:**
```json
// Write - all three work:
{"content": "raw string"}           // Auto-wrapped
{"content": {"text": "explicit"}}   // Explicit object

// Read - returns:
{"result": {"content": "raw string"}}  // Unwrapped for convenience
```

### Other File Types (Raw JSONB)
Canvas, Whiteboard, Chat, Agent, Skill files preserve raw JSON:

**Write Behavior:**
- No auto-wrapping
- Content must match expected JSON structure for the type

**Read Behavior:**
- No auto-unwrapping
- Returns exact stored JSON structure

**Examples (Canvas):**
```json
// Write:
{
  "content": {
    "elements": [{"type": "rect", "x": 10}],
    "version": 1
  }
}

// Read - returns same structure:
{
  "result": {
    "content": {
      "elements": [{"type": "rect", "x": 10}],
      "version": 1
    }
  }
}
```

---

## Tool Specifications

### ls - List Directory Contents

Lists files and folders in a directory within a workspace. Supports both single-level and recursive listing.

#### Arguments

```json
{
  "path": "/folder",
  "recursive": false
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `path` | string | No | `/` | Directory path to list |
| `recursive` | boolean | No | `false` | Recursively list all descendants |

#### Request Example

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "ls",
    "args": {
      "path": "/documents",
      "recursive": false
    }
  }'
```

#### Response (200 OK)

```json
{
  "success": true,
  "result": {
    "path": "/documents",
    "entries": [
      {
        "name": "report.md",
        "path": "/documents/report.md",
        "file_type": "Document",
        "updated_at": "2026-01-24T10:30:00Z"
      },
      {
        "name": "archive",
        "path": "/documents/archive",
        "file_type": "Folder",
        "updated_at": "2026-01-23T15:45:00Z"
      }
    ]
  },
  "error": null
}
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `result.path` | string | The path that was listed |
| `result.entries[]` | array | Array of file/folder entries |
| `entries[].name` | string | Technical identifier (slug) used in paths |
| `entries[].display_name` | string | Human-readable name for UI display |
| `entries[].path` | string | Full path to the item |
| `entries[].file_type` | string | Type: `Document`, `Folder`, `Chat`, etc. |
| `entries[].is_virtual` | boolean | `true` if file is system-managed (e.g. Chat, Agent) |
| `entries[].updated_at` | string | ISO8601 timestamp of last update |

#### Behavior Notes

- **Non-recursive mode** (default): Returns immediate children only
- **Recursive mode**: Returns all descendants with paths matching the prefix
- **Directory Validation**: Returns `400 Bad Request` if the target path is a file
- **Path resolution**: Uses `get_file_by_path()` to resolve the directory
- **Sorting**: Entries sorted by path in ascending order

---

### read - Read File Contents

Reads the latest version of a file within a workspace.

#### Arguments

```json
{
  "path": "/file.txt"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | string | Yes | Full path to the file |

#### Request Example

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "read",
    "args": {
      "path": "/documents/report.md"
    }
  }'
```

#### Response (200 OK)

```json
{
  "success": true,
  "result": {
    "path": "/documents/report.md",
    "content": "# Annual Report\n\nThis is the content...",
    "hash": "a1b2c3d4..."
  },
  "error": null
}
```

**Note:** For Document types, the `content` field is auto-unwrapped to return just the text string (not `{"text": "..."}`) for convenience. For other file types, returns the full JSON structure.

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `result.path` | string | The path that was read |
| `result.content` | string or object | File content - unwrapped string for Documents, JSON object for other types |
| `result.hash` | string | SHA-256 hash of the content |

#### Behavior Notes

- **Latest version**: Always returns the most recent file version
- **Auto-unwrap for Documents**: Simple Document files (`{text: "..."}`) are automatically unwrapped to return just the string value
- **Complex Documents**: Documents with additional fields beyond `text` are returned as-is without unwrapping
- **Other types**: Canvas, Whiteboard, Chat, etc. return raw JSON without modification
- **Not found error**: Returns 404 if path does not exist

---

### write - Create or Update File

Creates a new file or updates an existing file with new content. Automatically creates nested folders if they don't exist.

#### Arguments

```json
{
  "path": "/file.txt",
  "content": { ... },
  "file_type": "document"
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `path` | string | Yes | - | Full path to the file |
| `content` | object | Yes | - | File content as JSON value |
| `file_type` | string | No | `document` | Type: `document`, `folder`, `canvas`, `chat`, `whiteboard` |

#### Request Example (Create New File)

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "write",
    "args": {
      "path": "/documents/notes.md",
      "content": "# My Notes\n\nCreated via Tools API",
      "file_type": "document"
    }
  }'
```

**Note:** For Document types, you can pass either a raw string (auto-wrapped to `{text: "..."}`) or an explicit `{text: "..."}` object. The example above uses the raw string format for simplicity.

#### Request Example (Create Document with Explicit Object)

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "write",
    "args": {
      "path": "/docs/report.md",
      "content": {
        "text": "# Report\n\nContent here..."
      },
      "file_type": "document"
    }
  }'
```

#### Request Example (Create Specialized Type)

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "write",
    "args": {
      "path": "/dashboards/main.canvas",
      "content": {
        "elements": [],
        "version": 1
      },
      "file_type": "canvas"
    }
  }'
```

#### Request Example (Update Existing File)

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "write",
    "args": {
      "path": "/documents/notes.md",
      "content": {
        "text": "# Updated Notes\n\nModified content"
      }
    }
  }'
```

#### Response (200 OK)

```json
{
  "success": true,
  "result": {
    "path": "/documents/notes.md",
    "file_id": "019b97ac-e5f5-735b-b0a6-f3a34fcd4ff1",
    "version_id": "019b97ac-e5f5-735b-b0a6-f3a34fcd4ff2",
    "hash": "a1b2c3d4..."
  },
  "error": null
}
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `result.path` | string | The path that was written |
| `result.file_id` | UUID | File identifier |
| `result.version_id` | UUID | New version identifier |
| `result.hash` | string | SHA-256 hash of the content |

#### Behavior Notes

- **Create mode**: If file doesn't exist, creates new file with content
- **Update mode**: If file exists, creates new version with updated content
- **Auto-wrap for Documents**: Raw strings are automatically wrapped to `{text: "..."}` for Document types
- **Document validation**: For Document types with explicit objects, must contain a `text` field with a string value
- **Other types**: Canvas, Whiteboard, Chat, etc. require their specific JSON structure without auto-wrapping
- **Auto-folder creation**: Uses `create_file_with_content()` with path to create nested folders
- **Versioning**: All writes create a new `FileVersion` on the `main` branch
- **File type**: Supported types are `document`, `folder`, `canvas`, `chat`, `whiteboard`, `agent`, `skill`. Defaults to `document`.
- **Folder Protection**: Returns `400 Bad Request` if attempting to write text content to an existing folder path.
- **Virtual File Protection**: Returns `400 Bad Request` if attempting to write to a system-managed file (where `is_virtual` is true, e.g., `.chat` files). Use specialized APIs (like the Chat API) to modify these resources.

---

### mkdir - Create Directory

Recursively creates folders to ensure the specified path exists.

#### Arguments

```json
{
  "path": "/src/components"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | string | Yes | Full path to the directory to create |

#### Request Example

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "mkdir",
    "args": {
      "path": "/docs/v1/api"
    }
  }'
```

#### Response (200 OK)

```json
{
  "success": true,
  "result": {
    "path": "/docs/v1/api",
    "file_id": "019b97ac-e5f5-735b-b0a6-f3a34fcd4ff1"
  },
  "error": null
}
```

#### Behavior Notes

- **Recursive**: Automatically creates all parent folders in the path if they don't exist.
- **Idempotent**: If the folder already exists, it returns success with the existing folder ID.
- **Conflict**: Returns `409 Conflict` if a file (not a folder) already exists at any point in the path.

---

### rm - Delete File or Folder


Soft deletes a file or empty folder within a workspace. Soft delete preserves data for recovery.

#### Arguments

```json
{
  "path": "/file.txt"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | string | Yes | Full path to the file or folder |

#### Request Example

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "rm",
    "args": {
      "path": "/documents/old-report.md"
    }
  }'
```

#### Response (200 OK)

```json
{
  "success": true,
  "result": {
    "path": "/documents/old-report.md",
    "file_id": "019b97ac-e5f5-735b-b0a6-f3a34fcd4ff1"
  },
  "error": null
}
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `result.path` | string | The path that was deleted |
| `result.file_id` | UUID | File identifier that was deleted |

#### Behavior Notes

- **Soft delete**: Sets `deleted_at` timestamp (file remains in database)
- **Folder Protection**: 
    - **Hierarchical Check**: Returns `409 Conflict` if the folder has active children (via `parent_id`).
    - **Logical Check**: Returns `409 Conflict` if any active file's path starts with the folder's path prefix (catches orphaned files).
    - **Requirement**: Folders must be completely empty (both hierarchically and logically) before deletion.
- **Not found error**: Returns 404 if path does not exist
- **Recovery**: Soft-deleted files can be restored via the files API

---

### mv - Move or Rename File

Moves or renames a file within the workspace.

#### Arguments

```json
{
  "source": "/old-path.txt",
  "destination": "/new-path.txt"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `source` | string | Yes | Current path of the file |
| `destination` | string | Yes | Target path or target directory (ends with `/`) |

#### Request Example (Rename)

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "mv",
    "args": {
      "source": "/documents/old-name.md",
      "destination": "/documents/new-name.md"
    }
  }'
```

#### Request Example (Move to Directory)

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "mv",
    "args": {
      "source": "/file.txt",
      "destination": "/archive/"
    }
  }'
```

#### Response (200 OK)

```json
{
  "success": true,
  "result": {
    "from_path": "/documents/old-name.md",
    "to_path": "/documents/new-name.md"
  },
  "error": null
}
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `result.from_path` | string | The original path of the file |
| `result.to_path` | string | The new path after move/rename |

#### Behavior Notes

- **Rename**: If `destination` is a path, the file is renamed/moved to that exact path.
- **Move to Folder**: If `destination` ends with `/` (e.g., `/folder/`), the file is moved into that directory keeping its original name.
- **Destination as Directory**: If `destination` is an existing directory (no trailing `/`), the file is moved into it.
- **Validation**: Returns `404 Not Found` if source does not exist.
- **Conflict**: Returns `409 Conflict` if destination path already exists as a file (prevents accidental overwrites).
- **Parent Validation**: Destination parent directory must exist.

---

### touch - Update Timestamp or Create Empty File

Updates the modification time of a file or creates a new empty file.

#### Arguments

```json
{
  "path": "/new-file.txt"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | string | Yes | Path to the file to touch |

#### Request Example (Create New File)

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "touch",
    "args": {
      "path": "/documents/placeholder.txt"
    }
  }'
```

#### Request Example (Update Timestamp)

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "touch",
    "args": {
      "path": "/documents/existing-file.txt"
    }
  }'
```

#### Response (200 OK)

```json
{
  "success": true,
  "result": {
    "path": "/documents/placeholder.txt",
    "file_id": "019b97ac-e5f5-735b-b0a6-f3a34fcd4ff1"
  },
  "error": null
}
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `result.path` | string | The path that was touched |
| `result.file_id` | UUID | File identifier |

#### Behavior Notes

- **Update**: If file exists, updates its `updated_at` timestamp.
- **Create**: If file does not exist, creates a new empty `document` file.
- **Recursive**: Automatically creates parent folders if missing during creation.
- **File Type**: New files are created with `document` type and empty text content.

---

### edit - Edit File Content

Edits a file by replacing a unique search string with a replacement string. This tool is designed for precision edits and requires the search string to be unique within the file to prevent accidental modifications.

#### Arguments

```json
{
  "path": "/src/main.rs",
  "old_string": "fn old_function() {",
  "new_string": "fn new_function() {",
  "last_read_hash": "a1b2c3d4..."
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | string | Yes | Full path to the file |
| `old_string` | string | Yes | Unique string to search for |
| `new_string` | string | Yes | Replacement string |
| `last_read_hash` | string | No | Hash of the content when last read. If provided, tool fails if content changed. |

#### Behavior Notes

- **Uniqueness Requirement**: The tool will fail with a `400 Bad Request` if `old_string` is not found OR if it is found multiple times.
- **Stale Protection**: If `last_read_hash` is provided, the tool will fail with a `409 Conflict` if the current file hash does not match.
- **Versatility**: Supports any file type that contains editable text (e.g., Markdown, JSON, plain text).
- **Versioning**: Each successful edit creates a new file version.
- **Virtual File Protection**: Returns `400 Bad Request` if attempting to edit a system-managed file (where `is_virtual` is true).

---

### grep - Regex Search Files

Searches for a regex pattern in all document files within the workspace. This tool uses database-level regex searching for high performance across the entire codebase.

#### Arguments

```json
{
  "pattern": "fn \\w+\\(",
  "path_pattern": "%.rs",
  "case_sensitive": false
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `pattern` | string | Yes | Regex pattern to search for (Postgres regex syntax) |
| `path_pattern` | string | No | Optional SQL LIKE pattern to filter file paths (e.g., `%.rs`, `/src/%`) |
| `case_sensitive` | boolean | No | Whether the search should be case-sensitive. Default: `false`. |

#### Request Example

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "grep",
    "args": {
      "pattern": "TODO:",
      "case_sensitive": true
    }
  }'
```

#### Response (200 OK)

```json
{
  "success": true,
  "result": {
    "matches": [
      {
        "path": "/src/main.rs",
        "line_number": 42,
        "line_text": "// TODO: Implement error handling"
      }
    ]
  },
  "error": null
}
```

#### Behavior Notes

- **Regex Engine**: Uses PostgreSQL POSIX regex operators (`~` and `~*`).
- **Fuzzy Path Matching**: The `path_pattern` is case-insensitive and automatically normalized. You can use `*` as a wildcard (e.g., `src/*` or `*.rs`). If no wildcards are provided, it assumes a fuzzy "contains" match on the path.
- **Performance**: Searches across the latest versions of all text-searchable files in the database.
- **Virtual Files**: Supports searching system-managed files (e.g., Chats) by automatically expanding their JSON content into a readable text format.
- **Results Limit**: Results are limited to the first 1000 matches to prevent large payloads.
- **Line Numbers**: Line numbers are 1-based and calculated dynamically from the stored content.

---

### Path Normalization

All tools automatically normalize provided paths to ensure consistency:
- **Whitespace**: Leading and trailing whitespace is trimmed.
- **Slashes**: Multiple consecutive slashes are collapsed (e.g., `//folder///file` → `/folder/file`).
- **Relative Segments**: Resolves `.` (current) and `..` (parent) segments.
- **Formatting**: Ensures path starts with `/` and has no trailing `/` (except for root).

---

## Authentication & Authorization

The Tools API uses a two-layer middleware system for security.

### Layer 1: JWT Authentication

**Middleware**: `jwt_auth_middleware`

Validates the JWT access token and extracts user identity:

```rust
pub struct AuthenticatedUser {
    pub id: Uuid,
    pub email: String,
    pub full_name: Option<String>,
}
```

**Token Sources** (priority: header > cookie):
- Authorization header: `Authorization: Bearer <access_token>`
- Cookie: `access_token=<token>` (browser clients)

**Configuration**:
- Expiration: 15 minutes (configurable via `BUILDSCALE__JWT__ACCESS_TOKEN_EXPIRATION_MINUTES`)
- Secret: `BUILDSCALE__JWT__SECRET` (minimum 32 characters recommended)

### Layer 2: Workspace Access Control

**Middleware**: `workspace_access_middleware`

Validates user is a member of the workspace and adds access context:

```rust
pub struct WorkspaceAccess {
    pub workspace_id: Uuid,
    pub user_id: Uuid,
    pub is_owner: bool,
    pub is_member: bool,
}
```

**Access Rules**:
- User must be a member (owner or regular member) of the workspace
- Owners automatically have member access
- All tools operate within this workspace context

### Middleware Stack

```
Request
  ↓
JWT Authentication Middleware
  ↓ (extracts AuthenticatedUser)
Workspace Access Middleware
  ↓ (extracts WorkspaceAccess)
Tool Handler
  ↓ (uses both contexts)
Tool Execution
  ↓
Response
```

---

## Architecture & Extensibility

The Tools API is built on an extensible trait-based architecture.

### Tool Trait

All tools implement the `Tool` trait:

```rust
use async_trait::async_trait;

#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the name of this tool
    fn name(&self) -> &'static str;

    /// Executes the tool with given arguments
    async fn execute(
        &self,
        conn: &mut DbConn,
        workspace_id: Uuid,
        user_id: Uuid,
        args: Value,
    ) -> Result<ToolResponse>;
}
```

**Trait Constraints**:
- `Send + Sync`: Required for async execution across threads
- `#[async_trait]`: Required macro for async functions in traits
- `execute()`: Receives database connection, context, and JSON arguments
- Returns: Standardized `ToolResponse` with success/result/error

### Tool Registry Pattern

Tools are registered in a central registry for dispatch:

```rust
pub fn get_tool_executor(tool_name: &str) -> Result<ToolExecutor> {
    match tool_name {
        "ls" => Ok(ToolExecutor::Ls),
        "read" => Ok(ToolExecutor::Read),
        "write" => Ok(ToolExecutor::Write),
        "rm" => Ok(ToolExecutor::Rm),
        _ => Err(Error::NotFound(format!("Tool '{}' not found", tool_name))),
    }
}
```

### ToolExecutor Enum

Dispatches to specific tool implementations:

```rust
pub enum ToolExecutor {
    Ls,
    Read,
    Write,
    Rm,
}

impl ToolExecutor {
    pub async fn execute(
        &self,
        conn: &mut DbConn,
        workspace_id: Uuid,
        user_id: Uuid,
        args: Value,
    ) -> Result<ToolResponse> {
        match self {
            ToolExecutor::Ls => ls::LsTool.execute(conn, workspace_id, user_id, args).await,
            ToolExecutor::Read => read::ReadTool.execute(conn, workspace_id, user_id, args).await,
            ToolExecutor::Write => write::WriteTool.execute(conn, workspace_id, user_id, args).await,
            ToolExecutor::Rm => rm::RmTool.execute(conn, workspace_id, user_id, args).await,
        }
    }
}
```

### How to Add a New Tool

#### Step 1: Create Tool Implementation

Create a new file in `backend/src/tools/your_tool.rs`:

```rust
use crate::{DbConn, error::Result, models::requests::{ToolResponse, YourToolArgs, YourToolResult}};
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::Tool;

pub struct YourTool;

#[async_trait]
impl Tool for YourTool {
    fn name(&self) -> &'static str {
        "your_tool"
    }

    async fn execute(
        &self,
        conn: &mut DbConn,
        workspace_id: Uuid,
        user_id: Uuid,
        args: Value,
    ) -> Result<ToolResponse> {
        // Parse arguments
        let tool_args: YourToolArgs = serde_json::from_value(args)?;

        // Implement your tool logic here
        // Use database connection, workspace_id, user_id as needed

        // Build result
        let result = YourToolResult {
            // ... result fields
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
```

#### Step 2: Add Request/Response Models

In `backend/src/models/requests.rs`:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct YourToolArgs {
    pub param1: String,
    pub param2: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct YourToolResult {
    pub result_data: String,
}
```

#### Step 3: Export Tool Module

In `backend/src/tools/mod.rs`:

```rust
pub mod your_tool;  // Add this
```

#### Step 4: Update Registry

In `backend/src/tools/mod.rs`, update `get_tool_executor()`:

```rust
pub fn get_tool_executor(tool_name: &str) -> Result<ToolExecutor> {
    match tool_name {
        "ls" => Ok(ToolExecutor::Ls),
        "read" => Ok(ToolExecutor::Read),
        "write" => Ok(ToolExecutor::Write),
        "rm" => Ok(ToolExecutor::Rm),
        "your_tool" => Ok(ToolExecutor::YourTool),  // Add this
        _ => Err(Error::NotFound(format!("Tool '{}' not found", tool_name))),
    }
}
```

#### Step 5: Add to ToolExecutor Enum

In `backend/src/tools/mod.rs`:

```rust
pub enum ToolExecutor {
    Ls,
    Read,
    Write,
    Rm,
    YourTool,  // Add this
}
```

#### Step 6: Add Execution Case

In `ToolExecutor::execute()`:

```rust
match self {
    ToolExecutor::Ls => ls::LsTool.execute(conn, workspace_id, user_id, args).await,
    ToolExecutor::Read => read::ReadTool.execute(conn, workspace_id, user_id, args).await,
    ToolExecutor::Write => write::WriteTool.execute(conn, workspace_id, user_id, args).await,
    ToolExecutor::Rm => rm::RmTool.execute(conn, workspace_id, user_id, args).await,
    ToolExecutor::YourTool => your_tool::YourTool.execute(conn, workspace_id, user_id, args).await,  // Add this
}
```

### Source Files Reference

| Component | File Path |
|-----------|-----------|
| Handler | `backend/src/handlers/tools.rs` |
| Tool trait | `backend/src/tools/mod.rs` |
| ls implementation | `backend/src/tools/ls.rs` |
| read implementation | `backend/src/tools/read.rs` |
| write implementation | `backend/src/tools/write.rs` |
| rm implementation | `backend/src/tools/rm.rs` |
| mv implementation | `backend/src/tools/mv.rs` |
| touch implementation | `backend/src/tools/touch.rs` |
| Request/Response models | `backend/src/models/requests.rs` |
| Route registration | `backend/src/lib.rs:328` |

---

## Error Responses

All error responses follow the standard format with an `error` message and `code` field.

### Error Response Structure

```json
{
  "error": "Descriptive error message",
  "code": "ERROR_CODE"
}
```

### HTTP Status Codes & Error Codes

| Status | Error Code | Meaning |
|--------|------------|---------|
| **400 Bad Request** | `VALIDATION_ERROR` | Invalid tool arguments |
| **401 Unauthorized** | `INVALID_TOKEN` | Invalid or expired JWT |
| **403 Forbidden** | `FORBIDDEN` | Not a workspace member |
| **404 Not Found** | `NOT_FOUND` | Tool or file not found |
| **500 Internal Server Error** | `INTERNAL_ERROR` | Database or server error |

### Common Error Scenarios

#### Tool Not Found (404)

```json
{
  "error": "Tool 'git_commit' not found",
  "code": "NOT_FOUND"
}
```

**Cause**: Tool name not registered in `get_tool_executor()`

#### File Not Found (404)

```json
{
  "error": "File not found: /nonexistent/file.txt",
  "code": "NOT_FOUND"
}
```

**Cause**: Path does not exist in workspace

#### Invalid Arguments (400)

```json
{
  "error": "missing field `path` at line 1 column 15",
  "code": "VALIDATION_ERROR"
}
```

**Cause**: Required argument missing or wrong type

#### Not a Workspace Member (403)

```json
{
  "error": "Access forbidden: User is not a member of this workspace",
  "code": "FORBIDDEN"
}
```

**Cause**: User not added to workspace (need to join or be invited)

---

## Code Examples

### cURL Examples

#### List Root Directory

```bash
WORKSPACE_ID="019b97ac-e5f5-735b-b0a6-f3a34fcd4ff1"
ACCESS_TOKEN="eyJ0eXAiOiJKV1QiLCJhbGc..."

curl -X POST http://localhost:3000/api/v1/workspaces/$WORKSPACE_ID/tools \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "ls",
    "args": {
      "path": "/",
      "recursive": false
    }
  }'
```

#### Create a New File

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/$WORKSPACE_ID/tools \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "write",
    "args": {
      "path": "/notes/thoughts.md",
      "content": {
        "text": "# My Thoughts\n\nThis is a new file."
      }
    }
  }'
```

#### Read File Contents

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/$WORKSPACE_ID/tools \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "read",
    "args": {
      "path": "/notes/thoughts.md"
    }
  }'
```

#### Update File (New Version)

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/$WORKSPACE_ID/tools \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "write",
    "args": {
      "path": "/notes/thoughts.md",
      "content": {
        "text": "# Updated Thoughts\n\nThis is the updated content."
      }
    }
  }'
```

#### Delete File

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/$WORKSPACE_ID/tools \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "rm",
    "args": {
      "path": "/notes/thoughts.md"
    }
  }'
```

#### Move/Rename File

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/$WORKSPACE_ID/tools \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "mv",
    "args": {
      "source": "/notes/thoughts.md",
      "destination": "/docs/thoughts.md"
    }
  }'
```

#### Touch File (Update Timestamp or Create)

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/$WORKSPACE_ID/tools \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "touch",
    "args": {
      "path": "/notes/placeholder.txt"
    }
  }'
```

### JavaScript/TypeScript Example

```typescript
// Tools API client
class ToolsClient {
  constructor(
    private baseUrl: string,
    private accessToken: string
  ) {}

  async executeTool(
    workspaceId: string,
    tool: string,
    args: Record<string, unknown>
  ): Promise<{ success: boolean; result: unknown; error: string | null }> {
    const response = await fetch(
      `${this.baseUrl}/api/v1/workspaces/${workspaceId}/tools`,
      {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${this.accessToken}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ tool, args }),
      }
    );

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || 'Tool execution failed');
    }

    return response.json();
  }

  // Convenience methods
  async ls(workspaceId: string, path: string, recursive = false) {
    return this.executeTool(workspaceId, 'ls', { path, recursive });
  }

  async read(workspaceId: string, path: string) {
    return this.executeTool(workspaceId, 'read', { path });
  }

  async write(workspaceId: string, path: string, content: unknown) {
    return this.executeTool(workspaceId, 'write', { path, content });
  }

  async rm(workspaceId: string, path: string) {
    return this.executeTool(workspaceId, 'rm', { path });
  }

  async mv(workspaceId: string, source: string, destination: string) {
    return this.executeTool(workspaceId, 'mv', { source, destination });
  }

  async touch(workspaceId: string, path: string) {
    return this.executeTool(workspaceId, 'touch', { path });
  }
}

// Usage
const client = new ToolsClient('http://localhost:3000', accessToken);

// List files
const files = await client.ls(workspaceId, '/documents');

// Read file
const content = await client.read(workspaceId, '/notes/thoughts.md');

// Write file
await client.write(workspaceId, '/notes/new-file.md', {
  text: 'Hello from Tools API!'
});

// Delete file
await client.rm(workspaceId, '/notes/old-file.md');

// Move/rename file
await client.mv(workspaceId, '/notes/thoughts.md', '/docs/thoughts.md');

// Touch file (create or update timestamp)
await client.touch(workspaceId, '/notes/placeholder.txt');
```

### Rust Integration Example

```rust
use buildscale::tools::get_tool_executor;
use serde_json::json;

async fn execute_ls_tool(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<()> {
    // Get tool executor
    let executor = get_tool_executor("ls")?;

    // Execute with arguments
    let args = json!({
        "path": "/",
        "recursive": false
    });

    let response = executor
        .execute(conn, workspace_id, user_id, args)
        .await?;

    if response.success {
        println!("Tool result: {}", serde_json::to_string_pretty(&response.result)?);
    } else {
        eprintln!("Tool error: {}", response.error.unwrap_or_default());
    }

    Ok(())
}
```

---

## Testing

The Tools API includes comprehensive test coverage across 17 test files.

### Running Tests

```bash
# Run all tools tests
cargo test tools

# Run specific tool tests
cargo test tools::ls_tests
cargo test tools::read_tests
cargo test tools::write_tests
cargo test tools::rm_tests
cargo test tools::mv_tests
cargo test tools::touch_tests

# Run integration tests
cargo test tools::integration_tests

# Run with output for debugging
cargo test tools -- --nocapture
```

### Test Files Location

```
backend/tests/tools/
├── mod.rs                  # Test module exports
├── common.rs               # Test utilities and fixtures
├── ls_tests.rs             # ls tool tests
├── read_tests.rs           # read tool tests
├── write_tests.rs          # write tool tests
├── rm_tests.rs             # rm tool tests
├── mv_tests.rs             # mv tool tests
├── touch_tests.rs          # touch tool tests
└── integration_tests.rs    # Cross-tool integration tests
```

### Test Coverage

The test suite covers:
- ✅ Successful tool execution
- ✅ Error handling (file not found, invalid arguments)
- ✅ Recursive vs non-recursive listing
- ✅ Create vs update behavior for write tool
- ✅ Move/rename behavior for mv tool
- ✅ Touch update vs create behavior for touch tool
- ✅ Workspace isolation
- ✅ Authentication and authorization
- ✅ Edge cases (empty directories, nested folders)

---

## Related Documentation

- **[REST API Guide](./REST_API_GUIDE.md)** - Complete REST API reference
- **[Services API Guide](./SERVICES_API_GUIDE.md)** - Service layer documentation
- **[Everything is a File](./EVERYTHING_IS_A_FILE.md)** - File system philosophy
- **[Architecture](../CLAUDE.md)** - Overall system architecture

---

## Database Queries Used

The Tools API leverages existing database query functions:

| Query | Purpose | Used By |
|-------|---------|---------|
| `get_file_by_path()` | Resolve path to file | ls, read, write, rm, mv, touch |
| `list_files_in_folder()` | List immediate children | ls (non-recursive) |
| `get_file_with_content()` | Read with latest version | read |
| `create_file_with_content()` | Create new file with content | write (create mode), touch (create mode) |
| `create_version()` | Create new file version | write (update mode) |
| `soft_delete_file()` | Soft delete file | rm |
| `touch_file()` | Update file timestamp | touch (update mode) |
| `update_file()` | Update file metadata | mv |

All queries are located in `backend/src/queries/files.rs`.

---

## Summary

The Tools API provides a clean, extensible interface for filesystem operations within BuildScale workspaces. Its trait-based architecture makes it easy to add new tools while maintaining consistency and security through workspace isolation and JWT authentication.

For questions or contributions, refer to the source code in `backend/src/tools/` and `backend/src/handlers/tools.rs`.
