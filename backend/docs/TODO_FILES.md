# Future Development: AI Engine & File System

This document tracks planned improvements and future work for the BuildScale.ai AI Engine and the "Everything is a File" system.

## 🧠 The Brain (Memory & Context)

### 1. Agentic Memory
- [ ] **Chat Compactor**: Logic to automatically serialize long chat histories into versioned `.chat` files when context limits are reached.
- [ ] **Thought Search**: A specialized tool for the AI to query its own past sessions ("What did we decide about X?").
- [ ] **Context Window Optimization**: Dynamic chunk sizing based on document structure (Headers, Paragraphs).

### 2. Advanced Retrieval (RAG)
- [ ] **Hybrid Search**: Combine BM25 keyword search (Postgres FTS) with Vector search (`pgvector`) for maximum accuracy.
- [ ] **Re-ranking**: Implement a cross-encoder stage to re-rank top-K results from semantic search.
- [ ] **Multi-modal Parsing**: Integration with LlamaParse to convert PDFs and Spreadsheets into searchable Markdown versions.

## 🛠️ The Toolbox (Skills & Action)

### 1. Dynamic Skills
- [ ] **Skills Registry**: Create a protected `/system/skills` directory logic.
- [ ] **Auto-Loader**: Logic to inject relevant "Skill Manuals" (Markdown files) into the system prompt based on user intent.
- [ ] **Skill Validation**: A mechanism to verify that a user-uploaded skill manual is safe to run.

### 2. The Hydrated Sandbox (Docker)
- [ ] **Docker Orchestrator**: Service to spawn safe, transient Linux containers.
- [ ] **S3-to-Sandbox Sync**: High-speed utility to "hydrate" a container volume with files from the registry.
- [ ] **Command Bridge**: A secure `bash` tool that allows the AI to execute shell commands (`grep`, `sed`, `awk`) inside the sandbox.

### 3. Distributed Platform Features
- [ ] **Parallel Coordination**: Logic to use file `status` (`Pending`, `Processing`) as a distributed lock for multi-agent work.
- [ ] **Real-time Presence**: WebSocket integration to see which agent is currently "writing" to a file identity.
- [ ] **Notifications**: Event bus to alert agents when a dependency file has been updated.

## 📂 File System Core

### 1. Storage Evolution
- [ ] **S3 Storage Driver**: Move `FileType::Binary` and large content blobs from Postgres to S3.
- [ ] **Pre-signed URLs**: Support for direct client uploads to S3.
- [ ] **Streaming Downloads**: Efficient handling of large file retrieval.

### 2. Advanced Networking
- [ ] **Graph Visualization**: API to export the link/backlink structure for frontend rendering.
- [ ] **Automated Tagging**: Use the AI Engine to suggest tags based on file content.
