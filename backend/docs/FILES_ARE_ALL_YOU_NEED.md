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
*   **Mechanism**: Streams content from the global store (supporting truncation and markdown conversion).

### C. Recall (`grep`)
*   **Purpose**: Finding specific information across the entire logical volume.
*   **Mechanism**: High-speed semantic or regex search across the workspace index.

### D. Action (`edit`, `write`)
*   **`edit`**: Atomic modifications. Instead of rewriting huge files, agents submit precise "search & replace" blocks.
*   **`write`**: Creating new permanent state (plans, artifacts, code).

---

## 3. Use Cases (Applied Vision)

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

## 4. The Platform Layer (Infrastructure & Scale)

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

## 5. Distributed Agentic Workflows

The File System acts as the **State of Record** for complex, multi-agent pipelines, decoupled from the execution.

### The "Handoff" Workflow
1.  **Agent A (Planner)**: A fast CPU model writes a spec to `/projects/<project_name>/plan.md`. Status: `Ready`.
2.  **The Task System**: Detects the file change and triggers the next job.
3.  **Agent B (Coder)**: A heavy GPU coding model spins up, reads the plan, writes code to `/projects/<project_name>/src/`, and updates status to `Review`.
4.  **Agent C (Reviewer)**: A specialized security model reads the code and writes comments to `/projects/<project_name>/review.md`.

**The Benefit**: These agents are physically decoupled. They don't need to know about each other; they only need to know the **File IDs** and the **Standardized Folder Structure**. The File System provides the persistent shared memory that binds them together.
