← [Back to Index](./README.md) | **Developer API**: [Services API Guide](./SERVICES_API_GUIDE.md) | **Tools API**: [Tools API Guide](./TOOLS_API_GUIDE.md)

# Tools Module Layered Architecture

This document describes the layered architecture for the tools module, introduced to improve code organization and discoverability.

## Overview

The tools module was reorganized from a flat structure to a layered structure, grouping tools by functionality. This change improves:

- **Discoverability**: Related tools are now grouped together
- **Maintainability**: Clear boundaries between tool categories
- **Scalability**: Easy to add new tool categories
- **Code Review**: Smaller, focused changes per category

## Module Structure

```
src/tools/
├── mod.rs              # Tool registry, ToolExecutor, Tool trait
├── helpers.rs          # Shared helper functions (stays at root)
│
├── file/               # File system tools (14 tools)
│   ├── mod.rs
│   ├── cat.rs
│   ├── edit.rs
│   ├── file_info.rs
│   ├── find.rs
│   ├── glob.rs
│   ├── grep.rs
│   ├── ls.rs
│   ├── mkdir.rs
│   ├── mv.rs
│   ├── read.rs
│   ├── read_multiple_files.rs
│   ├── rm.rs
│   ├── touch.rs
│   └── write.rs
│
├── memory/             # Memory tools (5 tools)
│   ├── mod.rs
│   ├── delete.rs
│   ├── get.rs
│   ├── list.rs
│   ├── search.rs
│   └── set.rs
│
├── plan/               # Plan mode tools (6 tools)
│   ├── mod.rs
│   ├── ask_user.rs
│   ├── edit.rs
│   ├── exit_plan_mode.rs
│   ├── list.rs
│   ├── read.rs
│   └── write.rs
│
└── web/                # Web tools (2 tools)
    ├── mod.rs
    ├── fetch.rs
    └── search.rs
```

## Design Principles

### 1. Categorization

Tools are grouped by functionality:
- **file/**: All file system operations (ls, read, write, edit, rm, mv, touch, mkdir, grep, glob, find, cat, file_info, read_multiple_files)
- **memory/**: Persistent AI agent storage (memory_set, memory_get, memory_search, memory_delete, memory_list)
- **plan/**: Structured planning workflow (ask_user, exit_plan_mode, plan_write, plan_read, plan_edit, plan_list)
- **web/**: External content access (web_fetch, web_search)

### 2. Discoverability

Related tools are now easy to find:
- Need file operations? Look in `src/tools/file/`
- Need memory tools? Look in `src/tools/memory/`
- Need plan mode tools? Look in `src/tools/plan/`
- Need web tools? Look in `src/tools/web/`

### 3. Consistency

This structure follows the same pattern as `src/fs/`, which has a similar layered organization.

### 4. Tool Names Unchanged

**IMPORTANT**: Tool names remain unchanged for AI compatibility.

| Old Path | New Path | Tool Name (Unchanged) |
|----------|----------|----------------------|
| `tools::ls::LsTool` | `tools::file::LsTool` | `ls` |
| `tools::read::ReadTool` | `tools::file::ReadTool` | `read` |
| `tools::memory_set::MemorySetTool` | `tools::memory::MemorySetTool` | `memory_set` |
| `tools::plan_write::PlanWriteTool` | `tools::plan::PlanWriteTool` | `plan_write` |
| `tools::web_fetch::WebFetchTool` | `tools::web::WebFetchTool` | `web_fetch` |

## Adding a New Tool

### Step-by-Step Guide

1. **Choose the appropriate category** (file, memory, plan, web)
2. **Create the tool file** in that subdirectory
3. **Add to the subdirectory's mod.rs** (both `mod` and `pub use`)
4. **Register in main mod.rs**:
   - Add to `ToolExecutor` enum
   - Add to `get_tool_executor()` function
   - Add to `execute()` match arms
   - Add to `get_all_tool_definitions()` function
5. **Tool name stays at root level** (e.g., "ls" not "file::ls")

### Example: Adding a File Tool

1. Create `src/tools/file/compress.rs`

2. Update `src/tools/file/mod.rs`:
```rust
mod compress;
pub use compress::CompressTool;
```

3. Update `src/tools/mod.rs`:
```rust
// In ToolExecutor enum (in the File tools section)
Compress,

// In get_tool_executor()
"compress" => Ok(ToolExecutor::Compress),

// In execute()
ToolExecutor::Compress => file::CompressTool.execute(...).await,

// In get_all_tool_definitions()
ToolDefinition {
    name: "compress".into(),
    description: file::CompressTool.description().into(),
    parameters: file::CompressTool.definition(),
},
```

## Import Patterns

### From Within Tool Files

Tools import the trait and config from the main module:

```rust
use crate::tools::{Tool, ToolConfig};
use crate::tools::normalize_path;  // If needed
use crate::tools::PLAN_MODE_ERROR;  // If needed
```

### From External Code

External code uses the re-exported paths:

```rust
// Direct tool access
use crate::tools::file::LsTool;
use crate::tools::memory::MemorySetTool;
use crate::tools::plan::PlanWriteTool;
use crate::tools::web::WebFetchTool;

// Trait and types
use crate::tools::{Tool, ToolConfig, ToolExecutor};
```

## Benefits

| Benefit | Before (Flat) | After (Layered) |
|---------|---------------|-----------------|
| File count in root | 29 files | 3 files + 4 dirs |
| Find related tools | Search through all | Look in category |
| Add new category | Not structured | Add new directory |
| Code review scope | Entire tools/ | Single category |

## Summary

| Category | Files | Location |
|----------|-------|----------|
| File tools | 14 | `src/tools/file/` |
| Memory tools | 5 | `src/tools/memory/` |
| Plan tools | 6 | `src/tools/plan/` |
| Web tools | 2 | `src/tools/web/` |
| Shared | 2 | `src/tools/` (mod.rs, helpers.rs) |
| **Total** | **29** | |

## Related Documentation

- [Tools API Guide](./TOOLS_API_GUIDE.md) - HTTP REST API for tools
- [Everything is a File](./EVERYTHING_IS_A_FILE.md) - Architecture philosophy
- [Services API Guide](./SERVICES_API_GUIDE.md) - Backend services documentation
