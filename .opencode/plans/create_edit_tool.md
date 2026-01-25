# Plan: Create 'edit' Tool

## Goal
Implement a text-editor tool similar to Claude Code's edit capability. The tool will allow agents to modify files by specifying a unique search string (`old_string`) and a replacement string (`new_string`).

## Specification

### 1. Data Models (`backend/src/models/requests.rs`)

Add `EditArgs` to support the tool arguments.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EditArgs {
    pub path: String,
    pub old_string: String,
    pub new_string: String,
}
```

Reuse `WriteResult` for the response as it returns the necessary `file_id` and `version_id`.

### 2. Tool Implementation (`backend/src/tools/edit.rs`)

Create a new module implementing the `Tool` trait.

**Logic Flow:**
1.  **Arguments**: Parse `EditArgs`.
2.  **Validation**: `old_string` cannot be empty.
3.  **File Retrieval**:
    *   Normalize `path`.
    *   Fetch file from DB. Return `404 Not Found` if missing.
    *   Verify `FileType` is `Document`. Return `400 Bad Request` if `Folder` or other types.
4.  **Content Processing**:
    *   Read latest version content.
    *   Extract `text` field from JSON content.
    *   **Search**: Count occurrences of `old_string`.
        *   `count == 0`: Error "Search string not found in file content".
        *   `count > 1`: Error "Search string found {n} times. Please provide more context to ensure unique match.".
    *   **Replace**: Replace the single occurrence.
5.  **Commit**:
    *   Wrap new text in `{"text": ...}`.
    *   Call `files::create_version` to save.
    *   Return `WriteResult`.

### 3. Registration (`backend/src/tools/mod.rs`)

1.  Add `pub mod edit;`.
2.  Add `Edit` variant to `ToolExecutor`.
3.  Add case to `get_tool_executor` ("edit" -> `ToolExecutor::Edit`).
4.  Add dispatch logic in `ToolExecutor::execute`.

### 4. Agent Integration (`backend/src/services/chat/rig_tools.rs`)

Expose the tool to the Rig library.

```rust
define_rig_tool!(
    RigEditTool,
    tools::edit::EditTool,
    EditArgs,
    "edit",
    "Edits a file by replacing a unique search string with a replacement string. Fails if the search string is not found or found multiple times."
);
```

### 5. Documentation

*   **`backend/docs/TOOLS_API_GUIDE.md`**: Add a new section for the `edit` tool in the "Tool Specifications" section.
    *   **Description**: Explanation of the search/replace behavior.
    *   **Arguments**: `path`, `old_string`, `new_string`.
    *   **Behavior Notes**: Unique match requirement, Document-only support.

*   **`backend/docs/REST_API_GUIDE.md`**: Update the "Tools API" section.
    *   Add `edit` row to the "Available Tools" table.

### 6. Tests (`backend/tests/tools/edit_tests.rs`)

Implement integration tests:
*   `test_edit_success`: Create file -> Edit -> Verify content.
*   `test_edit_not_found`: Edit with missing string -> Verify error.
*   `test_edit_multiple_matches`: Edit with ambiguous string -> Verify error.
*   `test_edit_wrong_type`: Edit folder -> Verify error.

## Execution Steps

1.  Modify `backend/src/models/requests.rs`.
2.  Create `backend/src/tools/edit.rs`.
3.  Modify `backend/src/tools/mod.rs`.
4.  Modify `backend/src/services/chat/rig_tools.rs`.
5.  Create `backend/tests/tools/edit_tests.rs`.
6.  Modify `backend/docs/TOOLS_API_GUIDE.md`.
7.  Modify `backend/docs/REST_API_GUIDE.md`.
8.  Run tests: `cargo test tools::edit_tests`.
