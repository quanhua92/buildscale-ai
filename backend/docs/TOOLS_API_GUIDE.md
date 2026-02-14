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
  - [glob - Pattern-Based File Discovery](#glob---pattern-based-file-discovery)
  - [file_info - Query File Metadata](#file_info---query-file-metadata)
  - [find - Search Files by Metadata](#find---search-files-by-metadata)
  - [read_multiple_files - Batch File Reading](#read_multiple_files---batch-file-reading)
  - [cat - Concatenate Files with Formatting](#cat---concatenate-files-with-formatting)
  - [ask_user - Request User Input](#ask_user---request-user-input)
  - [exit_plan_mode - Transition to Build Mode](#exit_plan_mode---transition-to-build-mode)
  - [plan_write - Create Plan File](#plan_write---create-plan-file)
  - [plan_read - Read Plan File](#plan_read---read-plan-file)
  - [plan_edit - Edit Plan File](#plan_edit---edit-plan-file)
  - [plan_list - List Plan Files](#plan_list---list-plan-files)
  - [Path Normalization](#path-normalization)
- [Authentication & Authorization](#authentication--authorization)
- [Architecture & Extensibility](#architecture--extensibility)
- [Error Responses](#error-responses)
- [Code Examples](#code-examples)
- [Testing](#testing)
- [Related Documentation](#related-documentation)
- [Database Queries Used](#database-queries-used)

---

## Quick Reference

| Tool | Description | Arguments | Returns |
|------|-------------|-----------|---------|
| `ls` | List directory contents | `path?`, `recursive?` | `path`, `entries[]` with `synced` status |
| `read` | Read file contents with line range control | `path`, `offset?`, `limit?` | `content`, `synced`, `total_lines`, `truncated`, `offset`, `limit`, `hash` |
| `write` | Create or update file | `path`, `content`, `file_type?` | `file_id`, `version_id` |
| `rm` | Delete file or folder | `path` | `file_id` (or null for filesystem-only) |
| `mv` | Move or rename file | `source`, `destination` | `from_path`, `to_path` |
| `touch` | Update time or create empty | `path` | `path`, `file_id` |
| `mkdir` | Create directory | `path` | `path`, `file_id` |
| `edit` | Edit file content | `path`, `old_string`, `new_string`, `insert_line?`, `insert_content?`, `last_read_hash?` | `path`, `file_id`, `version_id` |
| `grep` | Regex search files with context | `pattern`, `path_pattern?`, `case_sensitive?`, `before_context?`, `after_context?`, `context?` | `matches[]` with context lines |
| `cat` | Concatenate files with formatting | `paths[]`, `offset?`, `limit?`, `show_ends?`, `show_tabs?`, `squeeze_blank?`, `number_lines?`, `show_headers?` | `content`, `files[]` with `synced`, `offset`, `limit`, `total_lines` |
| `glob` | Pattern-based file discovery | `patterns[]`, `path?` | `pattern`, `base_path`, `matches[]` with `synced` status |
| `file_info` | Query file metadata | `path` | `path`, `synced`, `file_type`, `size`, `line_count`, `timestamps`, `hash` |
| `find` | Search files by metadata | `name?`, `path?`, `file_type?`, `min_size?`, `max_size?`, `recursive?` | `matches[]` with `synced` status |
| `read_multiple_files` | Read multiple files in single call | `paths[]`, `limit?` | `files[]` with per-file `synced` status |
| `ask_user` | Request input or confirmation from user | `questions[]` | `question_id`, `questions[]` |
| `exit_plan_mode` | Transition from Plan to Build Mode | `allowedPrompts?`, `pushToRemote?`, `remoteSessionId?`, `remoteSessionUrl?`, `remoteSessionTitle?` | `mode`, `plan_file` |
| `plan_write` | Create plan file with auto-naming and frontmatter | `title`, `content`, `path?`, `status?` | `path`, `file_id`, `version_id`, `hash`, `metadata` |
| `plan_read` | Read plan file with parsed frontmatter | `path?`, `name?`, `offset?`, `limit?` | `path`, `metadata`, `content`, `hash` |
| `plan_edit` | Edit plan file preserving frontmatter | `path`, `old_string?`, `new_string?`, `insert_line?`, `insert_content?` | `path`, `file_id`, `version_id`, `hash` |
| `plan_list` | List plan files with metadata | `status?`, `limit?` | `plans[]`, `total` |

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

## File Sync Status

The Tools API supports **hybrid file discovery** that works with both database-synced and filesystem-only files. This provides flexibility for files created through various means:

### Sync Status Field

All tool responses that return file information include a `synced` boolean field:

| Value | Meaning | Metadata Available |
|-------|---------|-------------------|
| `true` | File is in the database | Full metadata (id, created_at, versions, etc.) |
| `false` | File exists on disk only | Basic metadata (name, path, size, updated_at) |

### How Files Become Unsynced

Files may be `synced: false` when:

1. **Created via SSH**: Direct file system access
2. **Migration scripts**: Batch imports
3. **External tools**: Files from other systems
4. **Manual operations**: Direct file system manipulation

### Tool Behavior with Sync Status

#### Discovery Tools (ls, find, glob)
- Return **all files** (synced + unsynced)
- `synced: true` for database entries
- `synced: false` for filesystem-only entries
- Database entries take precedence in merging

#### Access Tools (read, cat, file_info, read_multiple_files)
- **Try database first** → return with `synced: true`
- **Fallback to disk** → return with `synced: false`
- **404 only if** not found in either location

#### Mutation Tools (edit, mv)
- **Try database first** → operate normally
- **Fallback to disk** → **auto-import to database**, then operate
- Result: file becomes `synced: true` after operation

#### Deletion Tool (rm)
- **If synced**: Delete from database + disk
- **If unsynced**: Delete from disk only
- Returns `file_id: null` for filesystem-only deletions

### Visual Indicators (UI Integration)

Frontend applications can use the `synced` field to show visual status:

```typescript
// Example: File list item rendering
{file.synced ? (
  <Tooltip title="Synced to database">
    <CheckCircleIcon className="text-green-500" />
  </Tooltip>
) : (
  <Tooltip title="Exists on disk only">
    <CloudOffIcon className="text-yellow-500" />
  </Tooltip>
)}
```

### Benefits

1. **No broken workflows**: Discovery matches access capabilities
2. **Transparent auto-import**: Editing external files seamlessly imports them
3. **Clear status indication**: Users know which files are fully managed
4. **Backward compatible**: Existing clients work without changes

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

**Note:** The backend stores all file content uniformly as JSON. Content is returned as-is without modification - the AI is responsible for formatting and structure.

---

## Tool Specifications

### ls - List Directory Contents

Lists files and folders in a directory within a workspace. Supports both single-level and recursive listing with optional result limiting.

#### Arguments

```json
{
  "path": "/folder",
  "recursive": false,
  "limit": 50
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `path` | string | No | `/` | Directory path to list |
| `recursive` | boolean | No | `false` | Recursively list all descendants |
| `limit` | integer | No | `50` | Maximum entries to return. Use `0` for unlimited |

#### Request Example

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "ls",
    "args": {
      "path": "/documents",
      "recursive": false,
      "limit": 100
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
| `entries[].id` | UUID or null | Database ID (null for filesystem-only files) |
| `entries[].synced` | boolean | `true` if file is in database, `false` if filesystem-only |
| `entries[].name` | string | Technical identifier (slug) used in paths |
| `entries[].display_name` | string | Human-readable name for UI display |
| `entries[].path` | string | Full path to the item |
| `entries[].file_type` | string | Type: `Document`, `Folder`, `Chat`, etc. |
| `entries[].is_virtual` | boolean | `true` if file is system-managed (e.g. Chat, Agent) |
| `entries[].updated_at` | string | ISO8601 timestamp of last update |

#### Behavior Notes

- **Non-recursive mode** (default): Returns immediate children only
- **Recursive mode**: Returns all descendants with paths matching the prefix
- **Limit behavior**:
  - Default limit is 50 entries when not specified
  - `limit: 0` returns unlimited entries (useful for UI components that need all entries)
  - Limit is applied after merging database and filesystem entries, and after sorting (folders first)
- **Directory Validation**: Returns `400 Bad Request` if the target path is a file
- **Path resolution**: Uses `get_file_by_path()` to resolve the directory
- **Sorting**: Folders first, then sorted by path in ascending order

**CRITICAL USAGE NOTES:**
- Use `recursive: true` for discovering all files in a directory tree
- Use `limit: 0` when you need all entries (e.g., file explorer dialogs)
- Returns `path` as "/" when listing root directory
- Folders are returned first in the entries list for better readability
- Check `is_virtual` field to identify system-managed files that cannot be edited directly

---

### read - Read File Contents

Reads the latest version of a file within a workspace. Supports line range control for efficient token usage.

#### Arguments

```json
{
  "path": "/file.txt",
  "offset": 100,
  "limit": 50
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `path` | string | Yes | - | Full path to the file |
| `offset` | integer | No | 0 | Starting line number (0-indexed). `0` is the first line. Positive values count from beginning (e.g., `100` starts at line 100). Negative values count from end (e.g., `-100` reads the last 100 lines). |
| `limit` | integer | No | 500 | Maximum number of lines to read. Content is truncated at this limit. |

#### Request Examples

**Default read (first 500 lines):**
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

**Read middle section of file:**
```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "read",
    "args": {
      "path": "/src/main.rs",
      "offset": 100,
      "limit": 50
    }
  }'
```

**Read last 100 lines (like `tail -n 100`):**
```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "read",
    "args": {
      "path": "/logs/error.log",
      "offset": -100
    }
  }'
```

**Read last 1000 lines, return only first 50:**
```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "read",
    "args": {
      "path": "/large.log",
      "offset": -1000,
      "limit": 50
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
    "hash": "a1b2c3d4...",
    "total_lines": 5000,
    "truncated": true,
    "offset": 0,
    "limit": 500
  },
  "error": null
}
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `result.path` | string | The path that was read |
| `result.content` | string or object | File content as stored (truncated if applicable) |
| `result.hash` | string | SHA-256 hash of the full content (not affected by truncation) |
| `result.synced` | boolean | `true` if file is in database (full metadata), `false` if filesystem-only |
| `result.total_lines` | integer or null | Total number of lines in the full file (null for non-text content) |
| `result.truncated` | boolean or null | `true` if content was truncated due to limit, `false` otherwise (null for non-text) |
| `result.offset` | integer or null | The offset used for this read |
| `result.limit` | integer or null | The limit used for this read |

#### Behavior Notes

- **Latest version**: Always returns the most recent file version
- **Line-based truncation**: Only applies to text content (strings). JSON objects returned as-is.
- **Positive offset**: Reads from beginning (e.g., offset=100 starts at line 100)
- **Negative offset**: Reads from end (e.g., offset=-100 reads last 100 lines)
- **Default limit**: Without `limit`, reads first 500 lines (configurable via `DEFAULT_READ_LIMIT` constant)
- **Hash integrity**: The `hash` field represents the FULL file content, not the truncated portion
- **0-indexed offset**: Line 0 is the first line in the file
- **No content modification**: Content is returned as-is without any transformation (except truncation)

**CRITICAL USAGE NOTES:**
- Returns `hash` field that MUST be used with `edit` tool's `last_read_hash` parameter
- `hash` is computed from the FULL file content, even when reading a truncated portion
- Cannot read folders - will return validation error if path is a folder
- Always read a file before editing to get the latest content hash
- For large files, use `offset`/`limit` to read specific sections efficiently
- `total_lines` helps AI understand file structure when reading truncated content

---

### write - Create or Replace File

Creates a new file or completely replaces an existing file with new content. Automatically creates nested folders if they don't exist.

**IMPORTANT**: This tool performs **complete file replacement**, not partial edits. For modifying existing files, use the `edit` tool instead.

#### Overwrite Protection

By default (`overwrite=false`), the tool returns an error if the file already exists to prevent accidental overwrites. To explicitly overwrite an existing file, set `overwrite=true`.

**Recommendation**: Use the `edit` tool for modifying existing files instead of overwriting.

#### Arguments

```json
{
  "path": "/file.txt",
  "content": { ... },
  "file_type": "document",
  "overwrite": false
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `path` | string | Yes | - | Full path to the file |
| `content` | object | Yes | - | File content as JSON value |
| `file_type` | string | No | `document` | Type: `document`, `folder`, `canvas`, `chat`, `whiteboard` |
| `overwrite` | boolean | No | `false` | Set to `true` to overwrite existing files. Default prevents accidental overwrites. |

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

#### Request Example (Create File with JSON Object)

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

#### Request Example (Overwrite Existing File)

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "write",
    "args": {
      "path": "/documents/notes.md",
      "content": "# Completely Replaced Content\n\nThis replaces the entire file",
      "overwrite": true
    }
  }'
```

**Note**: For partial file modifications, use the `edit` tool instead.

#### Error Example (File Exists Without Overwrite)

```json
{
  "success": false,
  "error": "Validation failed",
  "code": "VALIDATION_ERROR",
  "fields": {
    "path": "File already exists: /documents/notes.md. To overwrite, set overwrite=true. However, for modifying existing files, the 'edit' tool is recommended instead of overwriting."
  }
}
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
- **No content modification**: Content is stored as-is without transformation
- **Auto-folder creation**: Uses `create_file_with_content()` with path to create nested folders
- **Versioning**: All writes create a new `FileVersion` on the `main` branch
- **File type**: Supported types are `document`, `folder`, `canvas`, `chat`, `whiteboard`, `agent`, `skill`. Defaults to `document`.
- **Folder Protection**: Returns `400 Bad Request` if attempting to write text content to an existing folder path.
- **Virtual File Protection**: Returns `400 Bad Request` if attempting to write to a system-managed file (where `is_virtual` is true, e.g., `.chat` files). Use specialized APIs (like the Chat API) to modify these resources.

**CRITICAL USAGE NOTES:**
- **NOT for partial edits** - this replaces the ENTIRE file content. Use `edit` tool for partial modifications
- For new files: content can be any JSON value (string, number, object, array)
- For existing files: complete content replacement occurs - original content is lost
- Use `edit` tool instead when modifying specific sections of existing files

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

**CRITICAL USAGE NOTES:**
- Creates ALL parent directories automatically - no need to create each level separately
- Example: `mkdir /a/b/c` creates /a, then /a/b, then /a/b/c as needed
- Succeeds silently if directory already exists (no error for existing folders)
- Use `touch` to create files, `mkdir` to create directories

---

### rm - Delete File or Folder


Soft deletes a file or empty folder within a workspace. Works with both database-synced files and filesystem-only files.

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
| `result.file_id` | UUID or null | File identifier that was deleted (null for filesystem-only files) |

#### Behavior Notes

- **Hybrid deletion**: Works with both database-synced and filesystem-only files
- **Database files**: Sets `deleted_at` timestamp (file remains in database) and deletes from disk
- **Filesystem-only files**: Deletes from disk only (no database record)
- **Folder Protection**:
    - **Hierarchical Check**: Returns `409 Conflict` if the folder has active children (via `parent_id`).
    - **Logical Check**: Returns `409 Conflict` if any active file's path starts with the folder's path prefix (catches orphaned files).
    - **Requirement**: Folders must be completely empty (both hierarchically and logically) before deletion.
- **Not found error**: Returns 404 if path does not exist
- **Recovery**: Soft-deleted files can be restored via the files API

**CRITICAL USAGE NOTES:**
- Soft delete means data is recoverable but not through the tools interface
- Cannot undo deletion via tool API - use Files API for recovery
- **Non-empty folders will fail** - delete all children first before deleting folder
- Use with caution - this operation cannot be easily undone

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

**CRITICAL USAGE NOTES:**
- **RENAME syntax**: `/old/path.txt` → `/new/path.txt` (full path with new filename)
- **MOVE syntax**: `/file.txt` → `/folder/` (trailing `/` moves into directory)
- Destination file already exists → returns conflict error (prevents overwrites)
- Destination parent directory must exist or operation fails

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
- **File Type**: New files are created with `document` type and empty content.

**CRITICAL USAGE NOTES:**
- Creates **empty Document files** with content `""`
- Use this to create placeholder files or refresh timestamps
- Does **not create directories** - use `mkdir` instead
- Existing files only get timestamp updated, content is unchanged

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

**CRITICAL USAGE NOTES:**
- `old_string` **MUST be non-empty** - the tool will fail with a validation error if empty
- This is a **REPLACE operation**, not an insert. The `old_string` is completely removed and replaced by `new_string`
- If you want to **preserve the original line**, you MUST include it in `new_string`
- Example: To change "let x = 1" to "let x = 2", use:
  - `old_string`: "let x = 1"
  - `new_string`: "let x = 2" (the original line is NOT preserved automatically)
- **Always provide `last_read_hash`** from a prior `read` call to prevent conflicting edits

---

### grep - Regex Search Files

Searches for a regex pattern in all document files within the workspace. This tool uses database-level regex searching for high performance across the entire codebase.

#### Arguments

```json
{
  "pattern": "fn \\w+\\(",
  "path_pattern": "%.rs",
  "case_sensitive": false,
  "limit": 50
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `pattern` | string | Yes | Regex pattern to search for (Postgres regex syntax) |
| `path_pattern` | string | No | Optional SQL LIKE pattern to filter file paths (e.g., `%.rs`, `/src/%`) |
| `case_sensitive` | boolean | No | Whether the search should be case-sensitive. Default: `false`. |
| `limit` | integer | No | Maximum matches to return. Default: `50`. Use `0` for unlimited. |

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
- **Results Limit**: Results are limited to the first 50 matches by default (configurable via `limit` parameter). Use `limit: 0` for unlimited.
- **Line Numbers**: Line numbers are 1-based and calculated dynamically from the stored content.

**CRITICAL USAGE NOTES:**
- `pattern` uses **PostgreSQL POSIX regex syntax** (not PCRE or JavaScript regex)
- `path_pattern` supports `*` wildcards which convert to SQL LIKE (`%`)
- `case_sensitive: false` (default) makes pattern case-insensitive
- Use for **pattern discovery** across all files - faster than reading each file
- Returns matching file paths with line numbers and line text context

---

### glob - Pattern-Based File Discovery

Finds files matching glob patterns (e.g., `*.rs`, `**/*.md`, `/src/**/*.rs`). Uses ripgrep for efficient file discovery without searching file contents. Returns matches with metadata including sync status (synced: true for database files, synced: false for filesystem-only).

#### Arguments

```json
{
  "pattern": "*.rs",
  "path": "/src"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `pattern` | string | Yes | Glob pattern to match files (e.g., `*.rs`, `**/*.md`, `test_*`) |
| `path` | string | No | Base directory for search (default: `/` for workspace root) |

#### Request Example

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "glob",
    "args": {
      "pattern": "*.rs"
    }
  }'
```

#### Response (200 OK)

```json
{
  "success": true,
  "result": {
    "pattern": "*.rs",
    "base_path": "/",
    "matches": [
      {
        "path": "/main.rs",
        "name": "main.rs",
        "file_type": "document",
        "is_virtual": false,
        "size": null,
        "updated_at": "2025-01-15T10:30:00Z"
      },
      {
        "path": "/src/lib.rs",
        "name": "lib.rs",
        "file_type": "document",
        "is_virtual": false,
        "size": null,
        "updated_at": "2025-01-15T10:30:00Z"
      }
    ]
  },
  "error": null
}
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `result.pattern` | string | The glob pattern that was used |
| `result.base_path` | string | The base directory for the search |
| `result.matches[]` | array | Array of matching files |
| `matches[].path` | string | Full path to the file |
| `matches[].name` | string | File name |
| `matches[].synced` | boolean | `true` if file is in database, `false` if filesystem-only |
| `matches[].file_type` | string | Type: `document`, `folder`, etc. |
| `matches[].is_virtual` | boolean | `true` if file is system-managed |
| `matches[].size` | integer or null | File size in bytes (null if not calculated) |
| `matches[].updated_at` | string | ISO8601 timestamp |

#### Behavior Notes

- **Implementation**: Uses ripgrep (`rg --files`) for efficient file discovery.
- **Glob Syntax**: Supports standard glob patterns including `*` (matches any characters), `**` (matches across directories), and `?` (matches single character).
- **Workspace Isolation**: The search is restricted to the workspace directory to prevent accessing files outside the workspace.
- **Path Normalization**: All returned paths are workspace-relative and start with `/`.
- **Virtual Files**: Returns metadata for both regular files and virtual files (e.g., Chats, system-managed files).
- **Performance**: Much faster than recursive `ls` for pattern matching, especially in large codebases.
- **Pattern Validation**: Rejects patterns containing `..` to prevent path traversal attacks.

**SUPPORTED PATTERNS:**
- `*.rs` - matches all `.rs` files in the current directory
- `**/*.md` - matches all `.md` files recursively
- `/src/**/*.rs` - matches all `.rs` files under `/src/`
- `test_*` - matches files/folders starting with `test_`
- `*/file.txt` - matches `file.txt` in any immediate subdirectory

**CRITICAL USAGE NOTES:**
- **Requires ripgrep**: The tool requires `ripgrep (rg)` to be installed on the system
- **Use for file discovery**: Use glob when you need to find files by pattern without reading contents
- **Use ls for browsing**: Use the `ls` tool when browsing directory contents or exploring folder structure
- **Use grep for content**: Use the `grep` tool when searching for patterns within file contents
- **No content reading**: Glob only returns file metadata, not file contents

---

### cat - Concatenate Files with Unix-Style Formatting

Concatenates and displays multiple files with Unix-style formatting options for debugging. Use cat to reveal hidden characters (tabs, trailing whitespace) or combine multiple files. Use read for single-file navigation with pagination.

#### Arguments

```json
{
  "paths": ["/config.json", "/.env"],
  "show_ends": true,
  "show_tabs": true,
  "squeeze_blank": true,
  "number_lines": false,
  "show_headers": false,
  "offset": 100,
  "limit": 50
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `paths` | array | Yes | List of file paths to concatenate (max 20 files) |
| `show_ends` | boolean | No | Display `$` at end of each line to show trailing whitespace (default: `false`) |
| `show_tabs` | boolean | No | Display tab characters as `^I` (default: `false`) |
| `squeeze_blank` | boolean | No | Suppress repeated empty lines (default: `false`) |
| `number_lines` | boolean | No | Add line numbers to all lines (default: `false`). Line numbers reflect actual file position when using offset. |
| `show_headers` | boolean | No | Add filename headers before each file (default: `false`) |
| `offset` | integer | No | Starting line position (default: `0`). Positive values start from beginning (e.g., `100` = line 100+). Negative values read from end (e.g., `-50` = last 50 lines). |
| `limit` | integer | No | Maximum number of lines to read per file (default: unlimited). Use with offset to read specific ranges. |

#### Request Examples

**Basic concatenation with special characters:**
```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "cat",
    "args": {
      "paths": ["/src/main.rs", "/src/lib.rs"],
      "show_ends": true,
      "show_tabs": true
    }
  }'
```

**Read specific line range with smart numbering:**
```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "cat",
    "args": {
      "paths": ["/src/main.rs"],
      "offset": 100,
      "limit": 50,
      "number_lines": true,
      "show_tabs": true
    }
  }'
```

**Read last 100 lines:**
```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "cat",
    "args": {
      "paths": ["/logs/error.log"],
      "offset": -100,
      "show_ends": true
    }
  }'
```

#### Response (200 OK)

```json
{
  "success": true,
  "result": {
    "content": "fn main() {\n    println!(\"Hello\");$}\n...",
    "files": [
      {
        "path": "/src/main.rs",
        "content": "fn main() {\n    println!(\"Hello\");$}",
        "line_count": 2,
        "offset": 0,
        "limit": 50,
        "total_lines": 150
      },
      {
        "path": "/src/lib.rs",
        "content": "pub fn hello() {}\n$",
        "line_count": 1,
        "offset": 0,
        "limit": 50,
        "total_lines": 50
      }
    ]
  },
  "error": null
}
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `result.content` | string | Concatenated and formatted content of all files |
| `result.files[]` | array | Array of per-file entries |
| `files[].path` | string | File path |
| `files[].content` | string | Formatted content for this file |
| `files[].line_count` | integer | Number of lines in this file |
| `files[].synced` | boolean | `true` if file is in database, `false` if filesystem-only |
| `files[].offset` | integer or null | Starting line position for this file |
| `files[].limit` | integer or null | Maximum lines read from this file |
| `files[].total_lines` | integer or null | Total lines in the full file |

#### Behavior Notes

- **Concatenation**: Joins multiple files sequentially with optional separators.
- **Special Characters**: Uses Unix-style formatting (`show_ends` adds `$`, `show_tabs` shows `^I`).
- **Smart Line Numbers**: When `number_lines=true` with `offset`, line numbers reflect actual file position (e.g., `offset=100` starts numbering at 101).
- **Line Range Filtering**: `offset` and `limit` enable reading specific portions of files for targeted debugging.
  - **Positive offset**: Reads from specified line number (0-indexed, so `offset=100` starts at line 100).
  - **Negative offset**: Reads from end (e.g., `offset=-50` reads last 50 lines).
  - **Per-file application**: Offset/limit applies to each file individually when concatenating multiple files.
- **Blank Line Squeezing**: `squeeze_blank` replaces multiple consecutive newlines with a single newline.
- **Headers**: When `show_headers` is `true`, adds `==> filename <==` before each file.
- **File Limit**: Maximum 20 files per request to prevent excessive output.
- **Partial Success**: If some files fail to read, errors are shown in their entries but other files are returned.

**SPECIAL CHARACTER OPTIONS:**
- `show_ends`: Displays `$` at line endings to reveal trailing whitespace (like `cat -E`)
- `show_tabs`: Displays tabs as `^I` to distinguish from spaces (like `cat -T`)
- `squeeze_blank`: Suppresses repeated empty lines (like `cat -s`)

**LINE RANGE FILTERING:**
- `offset`: Controls starting position (positive from start, negative from end)
- `limit`: Controls maximum lines to read per file
- Smart numbering helps identify which lines have issues when debugging specific ranges

**CRITICAL USAGE NOTES:**
- **Use cat for debugging**: Special character display helps identify tabs vs spaces, trailing whitespace in specific line ranges
- **Use read for navigation**: The `read` tool supports pagination and scrolling for large files
- **Token efficiency**: Combine offset/limit with special characters to debug specific sections without reading entire files
- **Line numbers**: Smart numbering with offset helps pinpoint exact line locations for debugging
- **Multiple files**: Cat is more efficient than multiple `read` calls for concatenation with line range filtering

---

### file_info - Query File Metadata

Gets file metadata without reading full content. Returns path, file_type, size, line_count (for text files), timestamps, and content hash. Use file_info to check file size or verify existence before reading.

#### Arguments

```json
{
  "path": "/large-file.log"
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
    "tool": "file_info",
    "args": {
      "path": "/config.json"
    }
  }'
```

#### Response (200 OK)

```json
{
  "success": true,
  "result": {
    "path": "/config.json",
    "file_type": "document",
    "size": 1024,
    "line_count": 25,
    "created_at": "2025-01-15T10:30:00Z",
    "updated_at": "2025-01-15T10:30:00Z",
    "hash": "a1b2c3d4e5f6..."
  },
  "error": null
}
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `result.path` | string | File path |
| `result.file_type` | string | Type: `document`, `folder`, etc. |
| `result.size` | integer or null | File size in bytes |
| `result.line_count` | integer or null | Number of lines (for text files) |
| `result.synced` | boolean | `true` if file is in database, `false` if filesystem-only |
| `result.created_at` | string | ISO8601 timestamp when file was created |
| `result.updated_at` | string | ISO8601 timestamp of last update |
| `result.hash` | string | SHA-256 hash of the file content |

#### Behavior Notes

- **Metadata Query**: Only reads file metadata, not full content (except for line_count).
- **Line Count**: For text files (`file_type: "document"`), content is read to count lines.
- **Size Field**: Returns actual file size in bytes by querying the filesystem metadata.
- **Content Hash**: Returns SHA-256 hash of the full file content from the latest version.
- **Timestamps**: Returns both `created_at` and `updated_at` for file tracking.
- **Virtual Files**: Works with both regular files and virtual files (e.g., Chats).
- **Token Efficiency**: Use this tool to check file properties before reading full content.

**CRITICAL USAGE NOTES:**
- **Check before reading**: Use `file_info` to verify file existence and size before using `read`
- **Line count accuracy**: For text files, the content is read to count lines (not metadata-only)
- **Hash for editing**: The `hash` field is required for `edit` tool's `last_read_hash` parameter
- **Size accuracy**: The `size` field returns the actual file size from disk (not database storage)
- **Use case decisions**: Use `file_info` for quick checks, `read` when you need the actual content

---

### find - Search Files by Metadata

Finds files by metadata criteria using Unix find command for filesystem discovery, then enriches results with database metadata. Complements `grep` which searches by content. Use find to locate files without reading their contents.

#### Arguments

```json
{
  "name": "*.txt",
  "path": "/src",
  "file_type": "folder",
  "min_size": 1048576,
  "max_size": 10485760,
  "recursive": true,
  "limit": 50
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | No | Filename pattern with wildcards (e.g., `*.txt`, `test_*`) |
| `path` | string | No | Base directory for search (default: `/` for workspace root) |
| `file_type` | string | No | File type filter (`document`, `folder`, `canvas`, etc.) |
| `min_size` | integer | No | Minimum file size in bytes |
| `max_size` | integer | No | Maximum file size in bytes |
| `recursive` | boolean | No | Search subdirectories (default: `true`) |
| `limit` | integer | No | Maximum matches to return. Default: `50`. Use `0` for unlimited. |

#### Request Example

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "find",
    "args": {
      "name": "*.rs",
      "path": "/src"
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
        "name": "main.rs",
        "file_type": "document",
        "size": null,
        "updated_at": "2025-01-15T10:30:00Z"
      },
      {
        "path": "/src/lib.rs",
        "name": "lib.rs",
        "file_type": "document",
        "size": null,
        "updated_at": "2025-01-15T10:30:00Z"
      }
    ]
  },
  "error": null
}
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `result.matches[]` | array | Array of matching files |
| `matches[].path` | string | Full path to the file |
| `matches[].name` | string | File name |
| `matches[].synced` | boolean | `true` if file is in database, `false` if filesystem-only |
| `matches[].file_type` | string | Type: `document`, `folder`, etc. |
| `matches[].size` | integer or null | File size in bytes (from filesystem stat) |
| `matches[].updated_at` | string | ISO8601 timestamp |

#### Behavior Notes

- **Implementation**: Uses Unix `find` command for filesystem discovery (like `find . -name "*.txt" -type f`).
- **Database Enrichment**: Results are enriched with database metadata (file_type enum, is_virtual flag, etc.).
- **Name Patterns**: Supports find-style wildcards (`*`, `?`, `[]`) for filename matching.
- **Path Filtering**: Normalizes path patterns and restricts search to workspace directory.
- **File Type Filter**: Maps file_type to find's `-type` option (`folder` → `-type d`, others → `-type f`).
- **Recursive Search**: When `true` (default), searches all subdirectories. When `false`, uses `-maxdepth 1`.
- **Size Filtering**: Uses find's `-size` flag with byte units (`+1048576c` = >1MB, `-10485760c` = <10MB).
- **Workspace Isolation**: The search is restricted to the workspace directory to prevent accessing files outside the workspace.

**DIFFERENCES FROM OTHER TOOLS:**
- **find**: Searches by metadata (name, type, size) using Unix find command
- **grep**: Searches by content (text within files)
- **glob**: Pattern matching for filenames using ripgrep (faster, simpler)
- **ls**: Lists directory contents (browsing, not searching)

**CRITICAL USAGE NOTES:**
- **Requires find**: The tool requires Unix `find` command to be installed on the system
- **Use with grep**: Combine `find` (metadata) and `grep` (content) for comprehensive searches
- **Pattern syntax**: Name patterns use find-style wildcards (more powerful than simple `*`)
- **Size filtering**: Now fully implemented using find's `-size` flag
- **Real files only**: Only finds files/folders that exist on disk (database-only entities won't be found)
- **Performance**: Unix find is highly optimized for filesystem traversal

---

### read_multiple_files - Read Multiple Files in Single Call

Reads multiple files in a single tool call, reducing network round-trips. Returns per-file success/error status with content, hash, and metadata. Use for batch file analysis or cross-referencing.

#### Arguments

```json
{
  "paths": ["/config.json", "/README.md", "/src/main.rs"],
  "limit": 100
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `paths` | array | Yes | List of file paths to read (max 50 files) |
| `limit` | integer | No | Maximum lines per file (default: `500`) |

#### Request Example

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "read_multiple_files",
    "args": {
      "paths": ["/config.json", "/README.md"],
      "limit": 100
    }
  }'
```

#### Response (200 OK)

```json
{
  "success": true,
  "result": {
    "files": [
      {
        "path": "/config.json",
        "success": true,
        "content": "{\n  \"key\": \"value\"\n}",
        "hash": "abc123...",
        "error": null,
        "total_lines": 3,
        "truncated": false
      },
      {
        "path": "/README.md",
        "success": true,
        "content": "# Project\n\n...",
        "hash": "def456...",
        "error": null,
        "total_lines": 150,
        "truncated": true
      }
    ]
  },
  "error": null
}
```

#### Response (Partial Success - Some Files Failed)

```json
{
  "success": true,
  "result": {
    "files": [
      {
        "path": "/config.json",
        "success": true,
        "content": "{...}",
        "hash": "abc123...",
        "error": null,
        "total_lines": 3,
        "truncated": false
      },
      {
        "path": "/nonexistent.txt",
        "success": false,
        "content": null,
        "hash": null,
        "error": "File not found: /nonexistent.txt",
        "total_lines": null,
        "truncated": null
      }
    ]
  },
  "error": null
}
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `result.files[]` | array | Array of per-file results |
| `files[].path` | string | File path |
| `files[].success` | boolean | `true` if file was read successfully |
| `files[].content` | object or null | File content (null if failed) |
| `files[].hash` | string or null | SHA-256 hash of content (null if failed) |
| `files[].synced` | boolean | `true` if file is in database, `false` if filesystem-only |
| `files[].error` | string or null | Error message (null if success) |
| `files[].total_lines` | integer or null | Total lines in file (null if failed or non-text) |
| `files[].truncated` | boolean or null | `true` if content was truncated (null if failed) |

#### Behavior Notes

- **Batch Reading**: Reads multiple files in a single API call (reduces network latency).
- **Sequential Processing**: Files are read sequentially (not parallel) due to database connection constraints.
- **Partial Success**: If some files fail, errors are returned in their entries but other files succeed.
- **Line Limit**: The `limit` parameter applies to each file individually (default: 500 lines per file).
- **Content Truncation**: When content exceeds the limit, `truncated: true` and `total_lines` shows the full count.
- **Hash Integrity**: Each file's SHA-256 hash is returned for content verification.
- **File Limit**: Maximum 50 files per request to prevent excessive memory usage.

**CRITICAL USAGE NOTES:**
- **Reduced round-trips**: More efficient than multiple individual `read` calls
- **Error handling**: Check `success` field for each file result to handle failures
- **Token efficiency**: Content is truncated per-file to stay within token limits
- **Use for batches**: Ideal for config files, documentation, or log analysis across multiple files
- **Hash for editing**: Use returned `hash` values with `edit` tool's `last_read_hash` parameter

---

### ask_user - Request User Input

Request structured input or confirmation from the user during AI execution. Supports multiple questions in a single call with flexible JSON Schema validation.

#### When to Use

1. **Clarification Needed**: User's request is ambiguous or incomplete
2. **Multiple Valid Approaches**: Several options exist and user preference matters
3. **Confirmation Required**: Action is significant or irreversible (deletion, major changes)
4. **Missing Information**: You need specific details to proceed
5. **Design Decisions**: User's input affects the outcome

#### Arguments

```json
{
  "questions": [
    {
      "name": "choice",
      "question": "Which approach do you prefer?",
      "schema": "{\"type\":\"string\",\"enum\":[\"Option A\",\"Option B\",\"Option C\"]}",
      "buttons": [
        {"label": "Option A", "value": "A"},
        {"label": "Option B", "value": "B"},
        {"label": "Option C", "value": "C"}
      ]
    }
  ]
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `questions` | array | Yes | Array of question objects (1-4 questions) |
| `questions[].name` | string | Yes | Variable name for the answer |
| `questions[].question` | string | Yes | Question text to display |
| `questions[].schema` | string | Yes | JSON Schema string for validation |
| `questions[].buttons` | array | No | Optional button labels for single-select (string enum) questions |

#### Common JSON Schema Patterns

**String with enum (radio buttons):**
```json
{"type":"string","enum":["Yes","No","Cancel"]}
```

**String with pattern (text input):**
```json
{"type":"string","pattern":"^[a-z0-9]+$","minLength":3}
```

**Number with range:**
```json
{"type":"number","minimum":1,"maximum":100}
```

**Array/checkbox (multi-select):**
```json
{"type":"array","items":{"type":"string","enum":["A","B","C"]},"minItems":1}
```

#### Critical Usage Rules

**Single-Select Questions** (Use buttons):
- Schema: `{"type": "string", "enum": ["A", "B", "C"]}`
- Add `buttons` field for better UX
- DO NOT use buttons for array-type questions

**Multi-Select Questions** (NO buttons!):
- Schema: `{"type": "array", "items": {"type": "string", "enum": ["A", "B", "C"]}}`
- DO NOT add `buttons` field - frontend will render checkboxes automatically

**Always:**
- Make questions specific and concise
- Provide context in the question text
- Use "Select one" or "choose all that apply" suffix to indicate selection type

#### Request Example

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "ask_user",
    "args": {
      "questions": [
        {
          "name": "framework",
          "question": "Which testing framework should I use?",
          "schema": "{\"type\":\"string\",\"enum\":[\"Jest\",\"Vitest\",\"Mocha\"]}",
          "buttons": [
            {"label": "Jest", "value": "Jest"},
            {"label": "Vitest", "value": "Vitest"},
            {"label": "Mocha", "value": "Mocha"}
          ]
        }
      ]
    }
  }'
```

#### Response Example

```json
{
  "success": true,
  "result": {
    "status": "question_pending",
    "question_id": "0193abcd-1234-5678-9abc-123456789abc",
    "questions": [
      {
        "name": "framework",
        "question": "Which testing framework should I use?",
        "schema": {"type":"string","enum":["Jest","Vitest","Mocha"]},
        "buttons": [
          {"label": "Jest", "value": "Jest"},
          {"label": "Vitest", "value": "Vitest"},
          {"label": "Mocha", "value": "Mocha"}
        ]
      }
    ]
  },
  "error": null
}
```

#### Behavior Notes

- **Ephemeral Questions**: Questions exist only in SSE stream and frontend memory (not persisted to database)
- **Time-Ordered IDs**: Uses UUID v7 for better debugging and logging capabilities
- **Validation**: Questions array cannot be empty
- **Multi-Question Support**: Can ask 1-4 questions in a single call
- **Flexible Schema**: Supports any valid JSON Schema for validation
- **Button Detection**: Frontend renders buttons automatically for single-select string enum questions

**CRITICAL USAGE NOTES:**
- **Human-in-the-Loop**: Use when AI needs user guidance to proceed
- **Structured Input**: Provides type-safe input validation via JSON Schema
- **Better UX**: Use buttons for single-select questions (not multi-select)
- **Confirmation**: Ideal for significant or irreversible operations
- **Clarification**: Use when requirements are ambiguous or incomplete

---

### exit_plan_mode - Transition to Build Mode

Transitions the workspace from Plan Mode (strategy/planning) to Build Mode (implementation). This tool should only be called after the user has explicitly approved an implementation plan.

#### Safety Warning

⚠️ **IMMEDIATE TRANSITION**: This tool exits Plan Mode immediately. Only call after EXPLICIT user approval via button click.

#### Safety Checklist (Must Verify All)

Before calling this tool, you MUST verify ALL of the following:
1. ✅ You just received a response from `ask_user` tool
2. ✅ The response value is exactly `"Accept"` (not `"accept"`, not similar, EXACTLY `"Accept"`)
3. ✅ This response came from a BUTTON CLICK, not a chat message
4. ✅ You previously showed an Accept/Reject question to the user
5. ✅ The plan file exists and has `file_type="plan"`

If ANY of the above is FALSE, DO NOT CALL THIS TOOL.

#### What is a Valid Plan File

A valid plan file MUST have:
1. **File extension**: `.plan` (e.g., `/plans/mighty-willow-symphony.plan`)
2. **File name**: 3 random words joined by hyphens (NOT `implementation.plan`)
3. **File type**: `"plan"` (set via `file_type` parameter when creating)
4. **Content**: Implementation plan with tasks and execution strategy

**Good plan file names** (3 random words):
- `/plans/gleeful-tangerine-expedition.plan`
- `/plans/jubilant-river-transformation.plan`
- `/plans/bold-meadow-revelation.plan`

**Bad plan file names**:
- `/plans/implementation.plan` (too generic)
- `/plans/my-plan.plan` (not unique enough)

#### Arguments

```json
{
  "plan_file_path": "/plans/fearless-ember-invention.plan"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `plan_file_path` | string | Yes | Path to the plan file (.plan extension) |

#### How to Create a Plan File

Use the `write` tool with BOTH the `.plan` extension AND `file_type` parameter:

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "write",
    "args": {
      "path": "/plans/fearless-ember-invention.plan",
      "content": "# Implementation Plan\n\n## Task 1\n...",
      "file_type": "plan"
    }
  }'
```

**CRITICAL**: If you don't specify `file_type="plan"`, the file will be created as type `"document"` and `exit_plan_mode` will fail with "File is not a plan file".

#### Required Workflow

**Step 1**: Create plan file with `write` tool (with `.plan` extension and `file_type="plan"`)
**Step 2**: IMMEDIATELY call `ask_user` with:
  - question: "Review the implementation plan. Ready to proceed to Build Mode?"
  - schema: `type="string", enum=["Accept", "Reject"]`
  - buttons: Accept → "Accept", Reject → "Reject"
**Step 3**: Wait for user response
**Step 4**: IF response is `"Accept"` (from button click):
  - THEN call `exit_plan_mode`
  - ELSE IF `"Reject"`: Ask for feedback, revise, go to Step 2
**Step 5**: System transitions to Build Mode

#### Request Example

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "exit_plan_mode",
    "args": {
      "plan_file_path": "/plans/fearless-ember-invention.plan"
    }
  }'
```

#### Response Example

```json
{
  "success": true,
  "result": {
    "mode": "build",
    "plan_file": "/plans/fearless-ember-invention.plan"
  },
  "error": null
}
```

#### When to Call (Only This Scenario)

✅ **CORRECT** - Call immediately when:
- User clicked "Accept" button on your `ask_user` question
- You just received: `{"approval": "Accept"}` from `ask_user` response
- You previously showed the Accept/Reject question

This is the ONLY valid scenario. No other situation justifies calling this tool.

#### When NOT to Call (All These Are Wrong)

❌ **WRONG** - User said in chat (NOT button click):
- "do it", "work on it", "proceed", "let's start"
- "looks good", "that's fine", "go ahead"
- "start implementing", "I approve", "yes, do it"

**Instead**: Show Accept/Reject question via `ask_user` to confirm

❌ **WRONG** - User seemed positive but unclear:
- "I think that works"
- "seems good to me"
- "ok let's try it"
- "sounds like a plan"

**Instead**: Show Accept/Reject question via `ask_user` to confirm

❌ **WRONG** - You just finished creating the plan:
- No user input yet
- User hasn't seen the plan
- You haven't asked for approval

**Instead**: Show Accept/Reject question via `ask_user` first

#### How to Detect Button Click

A response from `ask_user` is a **button click** when:
- You just called `ask_user` tool
- The response contains the question name and answer
- The value matches one of your button values exactly

A chat message is **NOT a button click** when:
- User typed in the chat input
- You did NOT just call `ask_user`
- The message is natural language, not a structured answer

**Remember**: BUTTON CLICK = `ask_user` response, CHAT MESSAGE = user typing

#### Behavior Notes

- **Validation**: Verifies plan file exists and has `file_type="plan"`
- **Immediate Transition**: Exits Plan Mode immediately upon successful execution
- **Database Update**: Updates chat metadata to switch workspace context to Build Mode
- **File Verification**: Checks plan file has content before transitioning
- **Error Handling**: Returns detailed error if plan file is invalid or missing

**CRITICAL USAGE NOTES:**
- **Explicit Approval Only**: Only call after user clicks "Accept" button
- **Plan File Validation**: Plan must have `.plan` extension and `file_type="plan"`
- **No Chat Approval**: Natural language approval is NOT sufficient (must use button)
- **Create → Ask → Exit**: Always follow the 3-step workflow (create plan, ask user, exit plan mode)
- **Unique Names**: Use random 3-word hyphenated names for plan files

---

### plan_write - Create Plan File

Creates or updates a plan file with auto-generated name and YAML frontmatter.

**Features:**
- Auto-generates unique 3-word hyphenated names if path not provided
- Automatically adds YAML frontmatter with title, status, and created_at
- Sets file_type to "plan" automatically
- Works in both Build Mode and Plan Mode

#### Arguments

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `title` | string | Yes | Title of the plan (shown in frontmatter) |
| `content` | string | Yes | Plan content in markdown |
| `path` | string | No | Custom path. If omitted, auto-generates name like `/plans/word-word-word.plan` |
| `status` | string | No | Plan status: `draft` (default), `approved`, `implemented`, `archived` |

#### Request Example

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "plan_write",
    "args": {
      "title": "Feature Implementation Plan",
      "content": "# Overview\n\n## Goals\n- Goal 1\n- Goal 2",
      "status": "draft"
    }
  }'
```

#### Response Example

```json
{
  "success": true,
  "result": {
    "path": "/plans/gleeful-tangerine-expedition.plan",
    "file_id": "550e8400-e29b-41d4-a716-446655440000",
    "version_id": "660e8400-e29b-41d4-a716-446655440000",
    "hash": "abc123def456",
    "metadata": {
      "title": "Feature Implementation Plan",
      "status": "draft",
      "created_at": "2025-01-15T10:30:00Z"
    }
  },
  "error": null
}
```

---

### plan_read - Read Plan File

Reads a plan file with parsed frontmatter.

**Features:**
- Parses YAML frontmatter and returns metadata separately
- Supports lookup by path or name (searches /plans/ directory)
- Returns content without frontmatter for cleaner display

#### Arguments

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | string | No* | Full path to plan file |
| `name` | string | No* | Plan name (without `.plan`), searches `/plans/` directory |
| `offset` | integer | No | Starting line position |
| `limit` | integer | No | Maximum lines to read (default: 500) |
| `cursor` | integer | No | Cursor position for scroll mode |

*Either `path` or `name` is required.

#### Request Example

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "plan_read",
    "args": {
      "name": "gleeful-tangerine-expedition"
    }
  }'
```

#### Response Example

```json
{
  "success": true,
  "result": {
    "path": "/plans/gleeful-tangerine-expedition.plan",
    "metadata": {
      "title": "Feature Implementation Plan",
      "status": "draft",
      "created_at": "2025-01-15T10:30:00Z"
    },
    "content": "# Overview\n\n## Goals\n- Goal 1\n- Goal 2",
    "hash": "abc123def456",
    "total_lines": 5
  },
  "error": null
}
```

**Note:** Legacy plans without frontmatter return `metadata: null` with content as-is.

---

### plan_edit - Edit Plan File

Edits a plan file while preserving YAML frontmatter.

**Features:**
- Preserves frontmatter during edits (status, title, created_at)
- Supports both replace and insert operations
- Only works on `.plan` files

#### Arguments

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | string | Yes | Path to plan file (must end with `.plan`) |
| `old_string` | string | No* | For REPLACE: text to find (must be unique) |
| `new_string` | string | No* | For REPLACE: replacement text |
| `insert_line` | integer | No* | For INSERT: line number (0-indexed) |
| `insert_content` | string | No* | For INSERT: content to insert |
| `last_read_hash` | string | No | Hash from latest read (prevents conflicts) |

*Either (`old_string` and `new_string`) OR (`insert_line` and `insert_content`) is required.

#### Request Example

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "plan_edit",
    "args": {
      "path": "/plans/gleeful-tangerine-expedition.plan",
      "old_string": "- Goal 1",
      "new_string": "- Goal 1 (completed)"
    }
  }'
```

#### Response Example

```json
{
  "success": true,
  "result": {
    "path": "/plans/gleeful-tangerine-expedition.plan",
    "file_id": "550e8400-e29b-41d4-a716-446655440000",
    "version_id": "770e8400-e29b-41d4-a716-446655440000",
    "hash": "def789ghi012"
  },
  "error": null
}
```

---

### plan_list - List Plan Files

Lists all plan files with metadata and optional status filtering.

**Features:**
- Returns title, status, and created_at for each plan
- Optionally filter by status
- Sorted by created_at descending (newest first)

#### Arguments

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `status` | string | No | Filter by status: `draft`, `approved`, `implemented`, `archived` |
| `limit` | integer | No | Maximum number of plans to return (default: 50) |

#### Request Example

```bash
curl -X POST http://localhost:3000/api/v1/workspaces/{workspace_id}/tools \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "plan_list",
    "args": {
      "status": "approved",
      "limit": 10
    }
  }'
```

#### Response Example

```json
{
  "success": true,
  "result": {
    "plans": [
      {
        "path": "/plans/gleeful-tangerine-expedition.plan",
        "name": "gleeful-tangerine-expedition",
        "metadata": {
          "title": "Feature Implementation Plan",
          "status": "approved",
          "created_at": "2025-01-15T10:30:00Z"
        }
      }
    ],
    "total": 1
  },
  "error": null
}
```

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
      "content": "# My Thoughts\n\nThis is a new file."
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
      "content": "# Updated Thoughts\n\nThis is the updated content."
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
await client.write(workspaceId, '/notes/new-file.md', 'Hello from Tools API!');

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
