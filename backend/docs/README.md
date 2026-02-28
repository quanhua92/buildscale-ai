# Backend System Documentation

Welcome to the BuildScale.ai Backend documentation. This directory contains comprehensive guides on the system's architecture, security, and APIs.

## Quick Links

| Document | Description |
|----------|-------------|
| [LINEAR_LEARNING_GUIDE.md](./LINEAR_LEARNING_GUIDE.md) | **START HERE** 📘 - Learn from first principles to implementation |
| [ARCHITECTURE.md](./ARCHITECTURE.md) | System architecture and layered module structure |
| [API_REFERENCE.md](./API_REFERENCE.md) | Complete API reference (REST, Tools, Services) |
| [AI_SYSTEM.md](./AI_SYSTEM.md) | AI agents, context engineering, providers, Rig integration |
| [AUTHENTICATION.md](./AUTHENTICATION.md) | Authentication, RBAC, and workspace invitations |
| [FILE_SYSTEM.md](./FILE_SYSTEM.md) | File system architecture and memory tools |
| [PLAN_SYSTEM.md](./PLAN_SYSTEM.md) | Plan mode workflow and plan tools |
| [CONFIGURATION.md](./CONFIGURATION.md) | Configuration, cache, events, and chat persistence |

## The Vision

BuildScale.ai transforms a standard file system into a **Distributed Operating System** for AI agents:

- **Everything is a File**: Every workspace is a self-contained "OS" with standardized folder taxonomy
- **Agentic Engine**: AI agents live, plan, and execute within workspace environments
- **Stateful Context**: Structured context management with automatic optimization
- **Plan-Then-Build**: Separate planning phase from execution for better results

## Core Concepts

### Authentication & Security

- **Dual-Token System**: JWT access tokens (15 min) + Session refresh tokens (30 days)
- **Argon2 Password Hashing**: Secure password storage with unique salts
- **RBAC**: Four-tier role hierarchy (Admin > Editor > Member > Viewer)
- **Workspace Invitations**: Secure token-based onboarding with role assignment

### File System

- **Database-Backed Registry**: All files tracked in PostgreSQL with versioning
- **Disk Storage**: Content stored on disk with SHA-256 content addressing
- **Soft Deletes**: Trash system with restore capability
- **Knowledge Graph**: Wikilinks and hashtags for file relationships

### AI System

- **State Machine**: FSM-driven agent lifecycle (Idle → Running → Completed)
- **Context Engineering**: Priority-based pruning and cache-optimized architecture
- **Multiple Providers**: OpenAI and OpenRouter support
- **Rig Integration**: AI execution through Rig.rs framework

### Plan Mode

- **Explore First**: AI explores project before modifying files
- **User Approval**: Plans require user confirmation before execution
- **Workflow Separation**: Clear division between intent (Plan) and execution (Build)

## Module Structure

```
src/
├── users/           # User management (layered)
├── workspaces/      # Workspace management (layered)
├── auth/            # Authentication (layered)
├── ai/              # AI providers and models (layered)
├── chat/            # Chat and agent services (layered)
├── tools/           # Core tools (layered)
│   ├── file/        # File system tools
│   ├── memory/      # Memory tools
│   ├── plan/        # Plan mode tools
│   └── web/         # Web tools
├── fs/              # File system core (layered)
├── workers/         # Background workers (layered)
└── middleware/      # HTTP middleware
```

## Getting Started

**New to BuildScale?** Start with the [Linear Learning Guide](./LINEAR_LEARNING_GUIDE.md) for a comprehensive tutorial from first principles to implementation details.

1. **Architecture Overview**: Continue with [ARCHITECTURE.md](./ARCHITECTURE.md)
2. **API Usage**: See [API_REFERENCE.md](./API_REFERENCE.md)
3. **Configuration**: Check [CONFIGURATION.md](./CONFIGURATION.md)
4. **Authentication**: Read [AUTHENTICATION.md](./AUTHENTICATION.md)

## Related Documentation

- [CLAUDE.md](../CLAUDE.md) - Development guidelines for Claude Code
- [.env.example](../.env.example) - Environment configuration template
