# Memory Tools Documentation

The Memory Management System provides persistent, long-term memory capabilities for AI agents. It allows agents to store, retrieve, and search information across sessions, enabling personalized and context-aware assistance.

## Overview

Memory tools operate on a file-based storage system using the "Everything is a File" philosophy. Memories are stored as Markdown files with YAML frontmatter, making them human-readable and version-controllable.

### Memory Scopes

| Scope | Path Pattern | Visibility |
|-------|-------------|------------|
| `user` | `/users/{user_id}/memories/{category}/{key}.md` | Private to the user |
| `global` | `/memories/{category}/{key}.md` | Shared across workspace |

### Memory File Format

```markdown
---
title: "Meeting Notes: Q4 Planning"
tags: ["meeting", "planning", "q4"]
category: "work"
created_at: "2025-01-15T10:30:00Z"
updated_at: "2025-01-15T10:30:00Z"
scope: "user"
---

# Meeting Notes: Q4 Planning

Content here...
```

---

## Tools

### memory_set

Creates or updates a memory with metadata.

**Arguments:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `scope` | string | Yes | `user` or `global` |
| `category` | string | Yes | Organization category (e.g., `preferences`, `project`) |
| `key` | string | Yes | Unique identifier within category |
| `title` | string | Yes | Human-readable title |
| `content` | string | Yes | Memory content in Markdown |
| `tags` | array | No | Tags for categorization and search |

**Example:**

```json
{
  "scope": "user",
  "category": "preferences",
  "key": "coding-style",
  "title": "Coding Style Preferences",
  "content": "User prefers TypeScript with strict mode, 2-space indentation.",
  "tags": ["coding", "typescript", "formatting"]
}
```

**Response:**

```json
{
  "success": true,
  "result": {
    "path": "/users/{uuid}/memories/preferences/coding-style.md",
    "file_id": "uuid",
    "version_id": "uuid",
    "hash": "sha256...",
    "scope": "user",
    "category": "preferences",
    "key": "coding-style",
    "title": "Coding Style Preferences",
    "tags": ["coding", "typescript", "formatting"]
  }
}
```

---

### memory_get

Retrieves a specific memory by scope, category, and key.

**Arguments:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `scope` | string | Yes | `user` or `global` |
| `category` | string | Yes | Category the memory belongs to |
| `key` | string | Yes | Unique key for the memory |

**Example:**

```json
{
  "scope": "user",
  "category": "preferences",
  "key": "coding-style"
}
```

**Response:**

```json
{
  "success": true,
  "result": {
    "path": "/users/{uuid}/memories/preferences/coding-style.md",
    "scope": "user",
    "category": "preferences",
    "key": "coding-style",
    "metadata": {
      "title": "Coding Style Preferences",
      "tags": ["coding", "typescript", "formatting"],
      "category": "preferences",
      "created_at": "2025-01-15T10:30:00Z",
      "updated_at": "2025-01-15T10:30:00Z",
      "scope": "user"
    },
    "content": "User prefers TypeScript with strict mode, 2-space indentation.",
    "hash": "sha256..."
  }
}
```

---

### memory_search

Searches across all memories with filtering capabilities. Uses grep (ripgrep or standard grep) for efficient pattern matching, then filters results by metadata (scope, category, tags) in memory.

**Implementation Notes:**
- Uses ripgrep (`rg`) if available, falls back to standard `grep`
- Grep performs pattern matching on filesystem for efficiency
- Results are filtered by scope, category, and tags after grep
- Only files matching the pattern are read and parsed for metadata
- Returns one match per unique memory file (deduplicated)
- Includes a truncated content preview (first ~100 words)

**Arguments:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `pattern` | string | Yes | Search pattern (regex supported) |
| `scope` | string | No | Filter by scope: `user` or `global` |
| `category` | string | No | Filter by category |
| `tags` | array | No | Filter by tags (AND logic - must have ALL tags) |
| `case_sensitive` | boolean | No | Case-sensitive search (default: `false`) |
| `limit` | integer | No | Max results (default: `50`, `0` for unlimited) |

**Example:**

```json
{
  "pattern": "typescript",
  "scope": "user",
  "tags": ["coding"],
  "limit": 10
}
```

**Response:**

```json
{
  "success": true,
  "result": {
    "total": 2,
    "matches": [
      {
        "path": "/users/{uuid}/memories/preferences/coding-style.md",
        "scope": "user",
        "category": "preferences",
        "key": "coding-style",
        "title": "Coding Style Preferences",
        "content_preview": "User prefers TypeScript with strict mode enabled. Always use 2-space indentation...",
        "tags": ["coding", "typescript", "formatting"],
        "updated_at": "2025-01-15T10:30:00Z"
      }
    ]
  }
}
```

---

### memory_delete

Deletes a specific memory by scope, category, and key. This performs a soft delete - the memory can be recovered from the deleted files view.

**Arguments:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `scope` | string | Yes | `user` or `global` |
| `category` | string | Yes | Category the memory belongs to |
| `key` | string | Yes | Unique key for the memory |

**Example:**

```json
{
  "scope": "user",
  "category": "preferences",
  "key": "coding-style"
}
```

**Response:**

```json
{
  "success": true,
  "result": {
    "path": "/users/{uuid}/memories/preferences/coding-style.md",
    "file_id": "uuid",
    "scope": "user",
    "category": "preferences",
    "key": "coding-style"
  }
}
```

---

## Usage Patterns

### Storing User Preferences

```json
memory_set({
  "scope": "user",
  "category": "preferences",
  "key": "editor-config",
  "title": "Editor Configuration",
  "content": "indent_size=2\nindent_style=space\ncharset=utf-8",
  "tags": ["editor", "formatting"]
})
```

### Storing Project Decisions

```json
memory_set({
  "scope": "global",
  "category": "decisions",
  "key": "database-choice",
  "title": "Database Selection Decision",
  "content": "Decided to use PostgreSQL for:\n- ACID compliance\n- JSON support\n- Full-text search",
  "tags": ["architecture", "database", "decision"]
})
```

### Storing API Endpoints

```json
memory_set({
  "scope": "global",
  "category": "project",
  "key": "api-endpoints",
  "title": "API Endpoint Reference",
  "content": "## Authentication\nPOST /auth/login\nPOST /auth/logout\n\n## Files\nGET /files\nPOST /files",
  "tags": ["api", "reference"]
})
```

### Searching for Context

```json
// Find all coding-related memories
memory_search({
  "pattern": "typescript|javascript",
  "scope": "user",
  "tags": ["coding"]
})
```

---

## Recommended Categories

| Category | Purpose | Example Keys |
|----------|---------|--------------|
| `preferences` | User preferences | `coding-style`, `editor-config`, `language` |
| `project` | Project-specific context | `api-endpoints`, `tech-stack`, `folder-structure` |
| `decisions` | Architecture/design decisions | `auth-strategy`, `database-choice`, `api-design` |
| `context` | General work context | `current-focus`, `team-info`, `deadlines` |
| `corrections` | User corrections to remember | `no-console-log`, `prefer-async` |
| `patterns` | Discovered patterns | `error-handling`, `naming-convention` |
| `references` | Quick reference guides | `git-commands`, `docker-cheatsheet` |

---

## Agent Integration

Agents are configured to automatically use memory tools. See [AGENTIC_ENGINE.md](AGENTIC_ENGINE.md) for details on agent personas.

### When Agents SET Memories

1. **User Preferences**: After asking about preferences
2. **Important Decisions**: When decisions are made with user
3. **Project Context**: Key project details discovered
4. **Recurring Patterns**: Patterns noticed across interactions
5. **User Corrections**: When user corrects agent behavior

### When Agents GET/SEARCH Memories

1. **Session Start**: Recall user context and preferences
2. **Before Decisions**: Check stored preferences
3. **When Referenced**: User mentions past work
4. **For Consistency**: Retrieve stored conventions

---

## Technical Details

### File Type

Memory files use `FileType::Memory` in the database, allowing efficient querying by type.

### Path Generation

Paths are automatically generated from scope, category, and key:

```
User:   /users/{user_id}/memories/{category}/{key}.md
Global: /memories/{category}/{key}.md
```

### Frontmatter Schema

```yaml
title: string          # Required
tags: string[]         # Optional, defaults to []
category: string       # Required
created_at: datetime   # Auto-set on creation
updated_at: datetime   # Auto-updated on modification
scope: "user" | "global"  # Required
```

### Security

- User-scoped memories are isolated by user_id in the path
- Users cannot access other users' private memories
- Global memories are visible to all workspace members
- Path validation prevents directory traversal

---

## REST API

### Execute Memory Tools

```http
POST /api/v1/workspaces/{workspace_id}/tools
Authorization: Bearer {token}
Content-Type: application/json

{
  "tool": "memory_set",
  "args": {
    "scope": "user",
    "category": "preferences",
    "key": "coding-style",
    "title": "Coding Style",
    "content": "TypeScript with strict mode"
  }
}
```

---

## Error Handling

| Error | Description |
|-------|-------------|
| 404 Not Found | Memory does not exist (memory_get, memory_delete) |
| 403 Forbidden | Access denied to another user's memory |
| 400 Validation Error | Invalid arguments (empty category, key, etc.) |
| 500 Internal Error | Database or filesystem error |

---

## Examples

### Complete Workflow

```javascript
// 1. Store a preference
await memory_set({
  scope: "user",
  category: "preferences",
  key: "react-style",
  title: "React Coding Style",
  content: "- Use functional components with hooks\n- Prefer arrow functions\n- Use TypeScript",
  tags: ["react", "typescript", "frontend"]
});

// 2. Retrieve it later
const memory = await memory_get({
  scope: "user",
  category: "preferences",
  key: "react-style"
});

// 3. Search for all frontend preferences
const results = await memory_search({
  pattern: "react|typescript",
  scope: "user",
  category: "preferences"
});

// 4. Update the memory (same key = update)
await memory_set({
  scope: "user",
  category: "preferences",
  key: "react-style",
  title: "React Coding Style (Updated)",
  content: "- Use functional components\n- Server Components by default\n- Use Tailwind for styling",
  tags: ["react", "typescript", "frontend", "tailwind"]
});

// 5. Delete the memory when no longer needed
await memory_delete({
  scope: "user",
  category: "preferences",
  key: "react-style"
});
```

---

## Related Documentation

- [PLAN_TOOLS.md](PLAN_TOOLS.md) - Plan management tools
- [TOOLS_API_GUIDE.md](TOOLS_API_GUIDE.md) - Complete tools API reference
- [EVERYTHING_IS_A_FILE.md](EVERYTHING_IS_A_FILE.md) - File philosophy
- [AGENTIC_ENGINE.md](AGENTIC_ENGINE.md) - Agent system architecture
