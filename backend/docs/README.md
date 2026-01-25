# Backend System Documentation

Welcome to the BuildScale.ai Backend documentation. This directory contains comprehensive guides on the system's architecture, security, and APIs.

## 🌟 The Vision
- **[The Agentic Engine](./AGENTIC_ENGINE.md)**: The specification for the Agentic Engine backend and workflows.
- **[Files Are All You Need](./FILES_ARE_ALL_YOU_NEED.md)**: Our core philosophy on why folders and tools are the future of AI.
- **[Everything Is A File](./EVERYTHING_IS_A_FILE.md)**: The technical implementation of our unified file-based architecture.

## 🏗️ System Architecture
- **[Architecture Overview](./ARCHITECTURE.md)**: High-level design and database schema.
- **[User & Workspace Management](./USER_WORKSPACE_MANAGEMENT.md)**: How multi-tenancy and memberships work.
- **[Role-Based Access Control (RBAC)](./ROLE_MANAGEMENT.md)**: Permission system and role hierarchy.
- **[Workspace Invitations](./WORKSPACE_INVITATIONS.md)**: Secure onboarding flow for new members.
- **[Cache Implementation](./CACHE.md)**: Async caching with TTL and Redis compatibility.

## 🔐 Security & Operations
- **[Authentication & Security](./AUTHENTICATION.md)**: Dual-token system, Argon2 hashing, and validation rules.
- **[Configuration Reference](./CONFIGURATION.md)**: Environment variables and system constraints.
- **[Static File Serving](./STATIC_FILE_SERVING.md)**: Documentation on how files are served to clients.

## 🔌 API References
- **[REST API Guide](./REST_API_GUIDE.md)**: HTTP endpoints, request formats, and dual-token usage.
- **[Tools API Guide](./TOOLS_API_GUIDE.md)**: Extensible tool execution system (ls, read, write, rm).
- **[Services API Guide](./SERVICES_API_GUIDE.md)**: Internal Rust service layer functions and usage examples.

---

## 🛠️ Ongoing Development
- **[API Implementation Plan](./API_IMPLEMENTATION_PLAN.md)**: Roadmap for new features.
- **[Files System TODO](./TODO_FILES.md)**: Pending tasks for the file management engine.
