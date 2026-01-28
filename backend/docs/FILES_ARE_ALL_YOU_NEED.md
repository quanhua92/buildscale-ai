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
*   **`/chats/<session_id>.md`**: The "Memory." Active and archived conversation logs.
*   **`/data/`**: The "Knowledge." Ingested raw documents (PDFs, Videos, CSVs).
*   **`/users/<user_id>/`**: The "Home Directory." User-specific workspace state (scratchpads, private drafts, personal agent configs).
*   **`/projects/<project_name>/`**: The "Project." The user's actual codebase and working files.

---

## 2. The Semantic Toolset (The Interface)

Agents don't just "open files." They use a standardized, semantic developer toolset to interact with this world.

### A. Discovery (`ls`, `glob`)
*   **Purpose**: exploring the environment.
*   **Example**: `ls /system/skills` to see what tools are available.

### B. Ingestion (`read`)
*   **Purpose**: Absorbing context into the agent's window.
*   **Mechanism**: Reads the actual file from disk (supporting truncation and markdown conversion).

### C. Recall (`grep`)
*   **Purpose**: Finding specific information across the entire logical volume.
*   **Mechanism**: High-speed semantic or regex search across the workspace index.

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
├── archive/      # All file versions (Content-Addressable Store)
└── trash/        # Soft-deleted files
```

**Components:**
1.  **The Latest (`latest/`)**:
    *   This is **Source of Truth** for current state.
    *   Files are stored hierarchically using their full `path` (e.g., `/projects/backend/main.rs`, `/notes/personal/note1.md`).
    *   Folders are created as actual directories on disk to preserve the folder structure.
    *   **Benefit**: Fast access, natural folder structure, easy navigation and browsing.

2.  **The Archive (`archive/`)**:
    *   A **Content-Addressable Store** (CAS) containing every file version ever written for this workspace.
    *   Files are stored by their SHA-256 hash with 2-level sharding (e.g., `./archive/e3/b0/e3b0...`).
    *   **Benefit**: Infinite version history with automatic deduplication per workspace.

3.  **The Trash (`trash/`)**:
    *   Soft-deleted files for this workspace.
    *   Supports recovery before permanent deletion.

4.  **The Index (PostgreSQL)**:
    *   Stores metadata (Permissions, Authorship, Relationships, Vector Embeddings).
    *   Maps logical paths (e.g., `/projects/backend/main.rs`) to physical storage (`/main.rs`).
    *   Stores only hash references to archive, not content.

### The "Double Write" Protocol

To maintain consistency and provide both version history and fast access, every write operation performs **two disk writes**:

1.  **Hash**: The content is hashed (SHA-256).
2.  **Archive** (First Write): The content is written to CAS (`./storage/workspaces/{workspace_id}/archive/`).
    *   **Purpose**: Version history, deduplication, and restoration.
    *   **Benefit**: Multiple versions with same content share storage (O(1) space).
3.  **Commit** (Second Write): The content is written to the Latest directory at `{full_path}` (hierarchical storage).
    *   **Purpose**: Fast O(1) access for reads, grep, and AI tools.
    *   **Benefit**: No database query needed to read file content.
4.  **Index**: The database is updated with the new metadata and hash reference.
    *   **Purpose**: Stores file metadata and version hash references.
    *   Stores only hash, not content.

### Implications for Tools

*   **`read`**: Reads from `latest/{full_path}` using the file's path from database.
*   **`grep`**: Uses ripgrep on the `latest/` directory - paths from disk match logical paths.
*   **`ls`**: Queries database for file hierarchy metadata.
*   **`write` / `edit`**: Must go through the API to ensure proper storage and database updates.

---

## 4. Use Cases (Applied Vision)

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
3.  **Result**: It finds the relevant conversation log (preserved by the auto-archiving system) without filling its context window with irrelevant history.

---

## 5. The Platform Layer (Infrastructure & Scale)

While the agent sees a simple file system, the **Platform** powers it with a massive distributed architecture.

### The Global Shared Surface
A workspace is not a folder on a disk; it is a **Globally Synchronized State Layer**.
1.  **PostgreSQL**: Acts as the **High-Speed Index** (Permissions, Metadata, Vector Search, Relationships).
2.  **S3 / Object Store**: Acts as the **Massive Memory** (Content Blobs, Archives).

### Multi-User Collaboration
Since a workspace is shared by a team, the file system handles multi-user concurrency naturally:
*   **Shared Project**: `/projects/<project_name>/` is the collaborative codebase.
*   **User Isolation**: `/users/<user_id>/` allows agents to work on "Personal Context" (e.g., a "Draft Plan") without cluttering the main project until it's ready to merge.
*   **Permissions**: RBAC controls which agents/users can write to `/system/` vs `/projects/`.

### The Sandbox Hydration Pattern (Solving Data Gravity)
Agents often need to run native tools (`bash`, `python`, `npm`) that expect a local filesystem.
1.  **Spin Up**: A Docker Sandbox starts in a region near the data.
2.  **Hydrate**: The platform **actively syncs** the relevant slice of the workspace from S3/Postgres into the container's local volume.
3.  **Execute**: The AI runs `ls -la` or `python script.py` at native NVMe speeds.
4.  **Security & State**: The synced workspace files are **read-only** within the container. Temporary work can be done in `/tmp`, but any changes intended for the global workspace must be committed using the **`write`** or **`edit`** tools.

**Result**: The AI feels like it's on a local laptop with high-speed access to massive datasets, while maintaining a strict, tool-gated audit trail for all global state changes.

---

## 6. Distributed Agentic Workflows

The File System acts as the **State of Record** for complex, multi-agent pipelines, decoupled from the execution.

### The "Handoff" Workflow
1.  **Agent A (Planner)**: A fast CPU model writes a spec to `/projects/<project_name>/plan.md`. Status: `Ready`.
2.  **The Task System**: Detects the file change and triggers the next job.
3.  **Agent B (Coder)**: A heavy GPU coding model spins up, reads the plan, writes code to `/projects/<project_name>/src/`, and updates status to `Review`.
4.  **Agent C (Reviewer)**: A specialized security model reads the code and writes comments to `/projects/<project_name>/review.md`.

**The Benefit**: These agents are physically decoupled. They don't need to know about each other; they only need to know the **File IDs** and the **Standardized Folder Structure**. The File System provides the persistent shared memory that binds them together.
