# Backend System Documentation

Multi-tenant Rust backend with workspace-based RBAC, complete user management, and secure authentication.

## Quick Start

| Need | Guide |
|------|-------|
| **New to System?** | [Architecture Overview](./ARCHITECTURE.md#architecture-overview) |
| **Authentication & Security** | [Authentication & Security](./AUTHENTICATION.md#authentication-security) |
| **User & Workspace Management** | [User, Workspace & Member Management](./USER_WORKSPACE_MANAGEMENT.md#user-management) |
| **Roles & Permissions** | [Role-Based Access Control](./ROLE_MANAGEMENT.md#rbac-overview) |
| **Invitation System** | [Workspace Invitations](./WORKSPACE_INVITATIONS.md#workspace-invitations-overview) |
| **Developer API Reference** | [Complete API Guide](./API_GUIDE.md#service-layer-apis) |

## System Architecture

**Multi-tenant workspace-based RBAC** with three-layer architecture (Service → Query → Model).

### Core Components
- **Workspace Isolation**: Complete data separation between tenants
- **Four-Tier RBAC**: Admin > Editor > Member > Viewer hierarchy
- **Session-Based Auth**: UUID v7 tokens with Argon2 password hashing
- **Secure Invitations**: Token-based member onboarding with role assignments

### Module Structure
```
src/
├── models/           # Data structures & validation
│   ├── users.rs       # User, LoginUser, UserSession
│   ├── workspaces.rs  # Workspace entities
│   ├── roles.rs        # Role definitions & constants
│   ├── workspace_members.rs # Member assignments
│   ├── invitations.rs  # Invitation entities & validation
│   ├── permissions.rs  # 18 hardcoded permissions
│   └── requests.rs     # API request models
├── services/           # Business logic layer
│   ├── users.rs        # User auth & management
│   ├── workspaces.rs   # Workspace operations
│   ├── roles.rs        # Role management
│   ├── workspace_members.rs # Member operations
│   ├── invitations.rs  # Invitation system
│   └── sessions.rs     # Session management
├── queries/            # Database operations (SQLx)
├── config.rs           # Environment configuration
├── database.rs         # Connection pooling
├── error.rs            # Error handling
└── lib.rs              # Public exports
```

## Documentation Index (7 Files)

| Domain | Documentation | Focus |
|--------|---------------|--------|
| **Architecture** | [ARCHITECTURE.md](./ARCHITECTURE.md) | System design + database schema |
| **Authentication** | [AUTHENTICATION.md](./AUTHENTICATION.md) | Auth + security + validation |
| **User & Workspace** | [USER_WORKSPACE_MANAGEMENT.md](./USER_WORKSPACE_MANAGEMENT.md) | Users + workspaces + members |
| **Roles & Permissions** | [ROLE_MANAGEMENT.md](./ROLE_MANAGEMENT.md) | RBAC system + 18 permissions |
| **Invitations** | [WORKSPACE_INVITATIONS.md](./WORKSPACE_INVITATIONS.md) | Token-based invitation system |
| **Developer API** | [API_GUIDE.md](./API_GUIDE.md) | Complete APIs + examples + practices |

## Key Features

- **Secure Authentication**: UUID v7 session tokens, Argon2 password hashing
- **18 Hardcoded Permissions**: Across workspace, content, and member categories
- **Workspace Isolation**: Complete data separation with shared user accounts
- **Advanced Session Management**: Multi-device support, cleanup, monitoring
- **Secure Invitation System**: UUID v7 tokens with 7-day default expiration
- **Comprehensive Validation**: Multi-layer input validation and error handling
- **Database Design**: PostgreSQL with proper indexing and cascade constraints
- **Testing Infrastructure**: Parallel-safe isolated test environment

---

*Generated: 2025-10-17T00:36:38.534Z*