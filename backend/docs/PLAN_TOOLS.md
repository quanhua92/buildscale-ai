# Plan Management Tools

## Overview

BuildScale-AI provides specialized tools for managing plan files with automatic naming, YAML frontmatter, and metadata parsing. These tools wrap the core file operations with plan-specific conveniences.

## Tools

### plan_write

Creates or updates a plan file with auto-generated name and YAML frontmatter.

**Arguments:**
- `title` (required): Title of the plan (shown in frontmatter)
- `content` (required): Plan content in markdown
- `path` (optional): Custom path. If omitted, auto-generates a name like `/plans/word-word-word.plan`
- `status` (optional): Plan status - `draft` (default), `approved`, `implemented`, `archived`

**Example:**
```json
{
  "title": "Feature Implementation Plan",
  "content": "# Overview\n\n## Goals\n- Goal 1\n- Goal 2",
  "status": "draft"
}
```

**Response:**
```json
{
  "success": true,
  "result": {
    "path": "/plans/gleeful-tangerine-expedition.plan",
    "file_id": "uuid",
    "version_id": "uuid",
    "hash": "abc123",
    "metadata": {
      "title": "Feature Implementation Plan",
      "status": "draft",
      "created_at": "2025-01-15T10:30:00Z"
    }
  }
}
```

### plan_read

Reads a plan file with parsed frontmatter.

**Arguments:**
- `path` (optional): Full path to plan file
- `name` (optional): Plan name (without `.plan`), searches `/plans/` directory
- `offset`, `limit`, `cursor` (optional): Read parameters

**Example:**
```json
{"name": "gleeful-tangerine-expedition"}
```

**Response:**
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
    "hash": "abc123",
    "total_lines": 5
  }
}
```

### plan_edit

Edits a plan file while preserving YAML frontmatter.

**Arguments:**
- `path` (required): Path to plan file
- `old_string` and `new_string`: For replace operation
- `insert_line` and `insert_content`: For insert operation
- `last_read_hash` (optional): Hash from latest read (prevents conflicts)

**Example:**
```json
{
  "path": "/plans/gleeful-tangerine-expedition.plan",
  "old_string": "- Goal 1",
  "new_string": "- Goal 1 (completed)"
}
```

### plan_list

Lists all plan files with metadata.

**Arguments:**
- `status` (optional): Filter by status
- `limit` (optional): Maximum results (default: 50)

**Example:**
```json
{"status": "approved", "limit": 10}
```

**Response:**
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
  }
}
```

## Plan File Format

Plans use YAML frontmatter:

```markdown
---
title: Feature Implementation Plan
status: draft
created_at: 2025-01-15T10:30:00Z
---

# Overview

Plan content goes here...
```

## Status Values

| Status | Description |
|--------|-------------|
| `draft` | Plan is being written/edited (default) |
| `approved` | Plan has been approved for implementation |
| `implemented` | Plan has been fully implemented |
| `archived` | Plan is no longer active |

## Legacy Plans

Plans created before this feature (without frontmatter) will have `metadata: null` when read with `plan_read`. The content is returned as-is.

## Auto-generated Names

When `path` is not provided to `plan_write`, a unique 3-word hyphenated name is automatically generated, e.g.:
- `gleeful-tangerine-expedition`
- `calm-forest-symphony`
- `brave-mountain-voyage`

This ensures unique, memorable identifiers without requiring the user to specify a path.

## Plan Mode Integration

All plan tools work in both Build Mode and Plan Mode. In Plan Mode:
- Plan files can be read, written, and edited
- The tools respect the same security model as core file operations

## Implementation Details

### Utility Functions

The plan tools use shared utilities in `src/utils/`:

- `plan_namer.rs`: Generates random 3-word hyphenated names
- `frontmatter.rs`: Parses and prepends YAML frontmatter

### Wrapping Pattern

Each plan tool wraps an existing core tool:
- `plan_write` → wraps `write` (adds frontmatter, auto-naming)
- `plan_read` → wraps `read` (parses frontmatter, name lookup)
- `plan_edit` → wraps `edit` (preserves frontmatter)
- `plan_list` → wraps `ls` (filters .plan files, returns metadata)

This design ensures consistency with core file operations while adding plan-specific functionality.
