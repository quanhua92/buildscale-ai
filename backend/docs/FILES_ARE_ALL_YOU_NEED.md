← [Back to Index](./README.md) | **Technical Implementation**: [Everything is a File](./EVERYTHING_IS_A_FILE.md)

# Files Are All You Need: The BuildScale.ai Platform Vision

This document outlines how BuildScale.ai transforms a standard file system into a **Distributed Operating System** for AI agents.

## 1. The Core Philosophy: "The Workspace is the OS"

In BuildScale, we don't build complex, custom integrations for every new capability. Instead, we treat **Everything as a File**.

Every workspace is a self-contained "Operating System." The AI interacts with the world through a standardized folder taxonomy and a unified toolset.

### The Standardized Taxonomy
Every workspace shares a consistent root structure:

*   **`/` (Root)**: The container for the entire logical volume.
*   **`/system/skills/<skill_name>/SKILL.md`**: The "Toolbox." Each subfolder represents a capability (e.g., `github`, `stripe`) with a markdown manifest defining how to use it.
*   **`/system/agents/<agent_name>/AGENT.md`**: The "Staff." Definitions for agent personas, system prompts, and constraints.
*   **`/chats/chat-{id}.chat`**: The "Memory." Active and archived conversation logs (files with YAML frontmatter).
*   **`/data/`**: The "Knowledge." Ingested raw documents (PDFs, Videos, CSVs).
*   **`/users/<user_id>/`**: The "Home Directory." User-specific workspace state (scratchpads, private drafts, personal agent configs).
*   **`/projects/<project_name>/`**: The "Project." The user's actual codebase and working files.

---

## 2. The Semantic Toolset (The Interface)

Agents don't just "open files." They use a standardized, semantic developer toolset to interact with this world.

### A. Discovery (`ls`, `glob`)
*   **Purpose**: Exploring the environment.
*   **Example**: `ls /system/skills` to see what tools are available.

### B. Ingestion (`read`)
*   **Purpose**: Absorbing context into the agent's window.
*   **Mechanism**: Reads the actual file from disk (supporting truncation and markdown conversion).

### C. Recall (`grep`)
*   **Purpose**: Finding specific information across the entire logical volume.
*   **Mechanism**: High-speed text search across the workspace.

### D. Action (`edit`, `write`)
*   **`edit`**: Atomic modifications. Instead of rewriting huge files, agents submit precise "search & replace" blocks.
*   **`write`**: Creating new permanent state (plans, artifacts, code).

---

## 3. Technical Implementation: Hybrid Storage

While users see "Everything as a File," the system employs a **Hybrid Disk/Database Architecture** to ensure performance, data integrity, and tool compatibility.

### The Dual-Layer Storage

Each workspace is **self-contained** within `/app/storage/workspaces/` directory structure:

**Directory Layout:**
```
/app/storage/workspaces/{workspace_id}/
├── latest/       # Current files (Source of Truth)
├── archive/      # Hash-based backup copies
└── trash/        # Soft-deleted files
```

### Storage Layers

1. **The Latest (`latest/`)**:
   - Hierarchical storage matching logical paths.
   - Example: `./latest/projects/backend/src/main.rs`
   - All reads hit this directory for O(1) access.

2. **The Archive (`archive/`)**:
   - Content-addressed storage for version history.
   - Files stored by SHA-256 hash with 2-level sharding: `./archive/e3/b0/e3b0...`
   - **Deduplication**: Same content = same hash = single blob.

3. **The Index (Database)**:
   - Single `files` table with inline version tracking.
   - `hash` column: SHA-256 of current content.
   - `versions TEXT[]` column: Array of historical hashes.
   - No separate version table needed.

### Write Flow

```
1. Calculate hash = SHA-256(content)
2. If file exists:
   a. Archive current content to archive/{hash}
   b. Append old hash to versions array
3. Write new content to latest/{path}
4. Update hash in database
```

### Implications for Tools

*   **`read`**: Reads from `latest/{full_path}` using the file's path from database.
*   **`grep`**: Uses ripgrep on the `latest/` directory - paths from disk match logical paths.
*   **`ls`**: Queries database for file hierarchy metadata.
*   **`write` / `edit`**: Must go through the API to ensure proper storage and database updates.

---

## 4. Knowledge Graph (Obsidian-Style)

BuildScale uses Obsidian-style content parsing for links and tags instead of separate database tables.

### Wikilinks
```markdown
See [[Project Alpha]] for details.
See [[meetings/standup|Daily Standup]] for the schedule.
```

### Hashtags
```markdown
This is #important for #project/alpha.
```

### Benefits
- **Simplicity**: No separate tables for links and tags
- **Flexibility**: Links and tags are part of the content itself
- **Portability**: Files can be exported and links/tags remain valid
- **Backlinks**: Computed on-demand from content

---

## 5. Use Cases (Applied Vision)

### Just-in-Time Learning
1.  Agent is asked to "Open a PR."
2.  It runs `ls /system/skills` and sees `github-integration`.
3.  It reads `/system/skills/github-integration/SKILL.md`.
4.  **Result**: It instantly learns the API schema and workflow to open a PR.

### Persona Loading
1.  User assigns a task to the "Security Auditor."
2.  The platform reads `/system/agents/security-auditor/AGENT.md`.
3.  **Result**: The agent's system prompt is hydrated with specific security rules and checklists.

### Infinite Chat
1.  Agent needs to know "What did we decide about the database schema last month?"
2.  It runs `grep "database schema" /chats`.
3.  **Result**: It finds the relevant conversation log without filling its context window with irrelevant history.

### Knowledge Navigation
1.  User asks "What's connected to the Q4 Planning document?"
2.  System extracts `[[wikilinks]]` from all files.
3.  **Result**: Shows backlinks - which documents reference the Q4 Planning doc.

---

## 6. The Platform Layer (Infrastructure & Scale)

While the agent sees a simple file system, the **Platform** powers it with a distributed architecture.

### The Global Shared Surface
A workspace is not a folder on a disk; it is a **Globally Synchronized State Layer**.
1.  **PostgreSQL**: Acts as the **High-Speed Index** (Permissions, Metadata, Relationships).
2.  **Disk / Object Store**: Acts as the **Massive Memory** (Content Blobs, Archives).

### Multi-User Collaboration
Since a workspace is shared by a team, the file system handles multi-user concurrency naturally:
*   **Shared Project**: `/projects/<project_name>/` is the collaborative codebase.
*   **User Isolation**: `/users/<user_id>/` allows agents to work on "Personal Context" (e.g., a "Draft Plan") without cluttering the main project until it's ready to merge.
*   **Permissions**: RBAC controls which agents/users can write to `/system/` vs `/projects/`.

### The Sandbox Hydration Pattern (Solving Data Gravity)
Agents often need to run native tools (`bash`, `python`, `npm`) that expect a local filesystem.
1.  **Spin Up**: A Docker Sandbox starts in a region near the data.
2.  **Hydrate**: The platform **actively syncs** the relevant slice of the workspace from storage into the container's local volume.
3.  **Execute**: The AI runs `ls -la` or `python script.py` at native NVMe speeds.
4.  **Security & State**: The synced workspace files are **read-only** within the container. Temporary work can be done in `/tmp`, but any changes intended for the global workspace must be committed using the **`write`** or **`edit`** tools.

**Result**: The AI feels like it's on a local laptop with high-speed access to massive datasets, while maintaining a strict, tool-gated audit trail for all global state changes.

---

## 7. Distributed Agentic Workflows

The File System acts as the **State of Record** for complex, multi-agent pipelines, decoupled from the execution.

### The "Handoff" Workflow
1.  **Agent A (Planner)**: A fast CPU model writes a spec to `/projects/<project_name>/plan.md`. Status: `Ready`.
2.  **The Task System**: Detects the file change and triggers the next job.
3.  **Agent B (Coder)**: A heavy GPU coding model spins up, reads the plan, writes code to `/projects/<project_name>/src/`, and updates status to `Review`.
4.  **Agent C (Reviewer)**: A specialized security model reads the code and writes comments to `/projects/<project_name>/review.md`.

**The Benefit**: These agents are physically decoupled. They don't need to know about each other; they only need to know the **File IDs** and the **Standardized Folder Structure**. The File System provides the persistent shared memory that binds them together.
