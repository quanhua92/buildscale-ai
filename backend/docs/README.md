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

## Installation & Setup

### Prerequisites

- **Rust**: Current stable version
- **PostgreSQL**: Current stable version (13+)
- **sqlx CLI**: For database migrations
- **Git**: For cloning the repository

### 1. Clone Repository

```bash
git clone <repository-url>
cd buildscale-ai/backend
```

### 2. Install Dependencies

```bash
# Install Rust dependencies
cargo build

# Install sqlx CLI (if not already installed)
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

### 3. Database Setup

#### Start PostgreSQL
```bash
# Using Docker (recommended)
docker run --name postgres-buildscale \
  -e POSTGRES_DB=buildscale \
  -e POSTGRES_USER=buildscale \
  -e POSTGRES_PASSWORD=your_secure_password \
  -p 5432:5432 \
  -d postgres:latest

# Or use your local PostgreSQL installation
# Ensure you have a database named 'buildscale' and user 'buildscale'
```

#### Configure Environment
```bash
# Copy environment configuration template
cp .env.example .env

# Edit .env with your database configuration
# Replace PASSWORD with your actual PostgreSQL password
nano .env
```

**Required .env Configuration:**
```env
# Database Configuration
BUILDSCALE__DATABASE__USER=buildscale
BUILDSCALE__DATABASE__PASSWORD=your_secure_password
BUILDSCALE__DATABASE__HOST=localhost
BUILDSCALE__DATABASE__PORT=5432
BUILDSCALE__DATABASE__DATABASE=buildscale

# For sqlx CLI operations
DATABASE_URL=postgresql://buildscale:your_secure_password@localhost:5432/buildscale
```

### 4. Run Database Migrations

```bash
# Check migration status (optional)
sqlx migrate info

# Run all migrations to set up database schema
sqlx migrate run

# Verify tables were created
psql -h localhost -U buildscale -d buildscale -c "\dt"
```

### 5. Verify Installation

```bash
# Build the project (should complete without errors)
cargo build

# Run tests to verify everything works
cargo test

# Run example to see the system in action
cargo run --example 01_hello
```

### 6. Development Workflow

```bash
# Start development
cargo run  # Start the application

# Run with logging
RUST_LOG=debug cargo run

# Run specific tests
cargo test users::services
cargo test test_user_registration_success

# Run with test output
cargo test -- --nocapture
```

## Troubleshooting

### Common Issues

**Build Errors:**
```bash
# Ensure Rust is up to date
rustup update stable

# Clean and rebuild
cargo clean && cargo build
```

**Database Connection Errors:**
```bash
# Check PostgreSQL is running
docker ps | grep postgres

# Test database connection
psql -h localhost -U buildscale -d buildscale

# Verify .env configuration
cat .env
```

**Migration Issues:**
```bash
# Check migration status
sqlx migrate info

# Reset and rerun migrations (use with caution)
sqlx migrate revert  # Revert last migration
sqlx migrate run     # Run migrations again
```

**Test Failures:**
```bash
# Run tests with verbose output
cargo test -- --nocapture

# Run specific failing test
cargo test test_name -- --exact --nocapture
```

### Getting Help

- Check logs: `RUST_LOG=debug cargo run`
- Review test output: `cargo test -- --nocapture`
- Verify database schema: `\dt` in psql
- Check environment variables: `env | grep BUILDSCALE`

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
│   ├── permissions.rs  # Comprehensive permission system
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

## Documentation Index (8 Files)

| Domain | Documentation | Focus |
|--------|---------------|--------|
| **Architecture** | [ARCHITECTURE.md](./ARCHITECTURE.md) | System design + database schema |
| **Authentication** | [AUTHENTICATION.md](./AUTHENTICATION.md) | Auth + security + validation |
| **User & Workspace** | [USER_WORKSPACE_MANAGEMENT.md](./USER_WORKSPACE_MANAGEMENT.md) | Users + workspaces + members |
| **Roles & Permissions** | [ROLE_MANAGEMENT.md](./ROLE_MANAGEMENT.md) | RBAC system with comprehensive permissions |
| **Invitations** | [WORKSPACE_INVITATIONS.md](./WORKSPACE_INVITATIONS.md) | Token-based invitation system |
| **Configuration** | [CONFIGURATION.md](./CONFIGURATION.md) | System settings and constraints |
| **Developer API** | [API_GUIDE.md](./API_GUIDE.md) | Complete APIs + examples + practices |

## Key Features

- **Secure Authentication**: UUID v7 session tokens, Argon2 password hashing
- **Comprehensive Permission System**: Fine-grained permissions across workspace, content, and member management categories
- **Workspace Isolation**: Complete data separation with shared user accounts
- **Advanced Session Management**: Multi-device support, cleanup, monitoring
- **Secure Invitation System**: UUID v7 tokens with configurable default expiration
- **Comprehensive Validation**: Multi-layer input validation and error handling
- **Database Design**: PostgreSQL with proper indexing and cascade constraints
- **Testing Infrastructure**: Parallel-safe isolated test environment

---

*Last updated: See git history for latest changes*