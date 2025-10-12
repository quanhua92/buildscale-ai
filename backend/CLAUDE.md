# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Development Commands

### Building and Testing
```bash
# Build the project
cargo build

# Build with optimizations for production
cargo build --release

# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run a specific test module
cargo test users::services::user_registration

# Run a specific test
cargo test test_user_registration_success

# Run examples
cargo run --example 01_hello
cargo run --example 02_users_management
cargo run --example 03_workspaces_management
```

### Database Operations
```bash
# Run database migrations
sqlx migrate run

# Reset database (use with caution)
sqlx migrate revert

# Check migration status
sqlx migrate info

# Install sqlx CLI (if not installed)
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

### Development Setup
```bash
# Copy environment configuration
cp .env.example .env

# Edit .env with your database configuration
# Required: BUILDSCALE__DATABASE__USER, PASSWORD, HOST, PORT, DATABASE
```

## Architecture Overview

This is a Rust backend implementing a **multi-tenant workspace-based RBAC system** with the following core characteristics:

### System Architecture
- **Multi-tenant Architecture**: Complete workspace isolation with shared users
- **Role-Based Access Control (RBAC)**: Four-tier role hierarchy (Admin > Editor > Member > Viewer)
- **Single Owner Model**: Each workspace has exactly one owner with full control
- **Service-Query-Model Layering**: Clear separation between business logic, data access, and data models

### Core Entities and Relationships
```
Users (1) ←→ (N) Workspaces
   ↓                   ↓
   └── Workspace Members ──→ Roles (per workspace)
```

- **Users**: Global accounts that can belong to multiple workspaces
- **Workspaces**: Isolated containers with exactly one owner
- **Roles**: Workspace-scoped permission definitions (default + custom)
- **Workspace Members**: Many-to-many relationship with specific role assignments

### Module Structure

#### `/src/models/`
Data models and type definitions:
- `users.rs`: User entities (`User`, `RegisterUser`, `UpdateUser`)
- `workspaces.rs`: Workspace entities (`Workspace`, `NewWorkspace`, `UpdateWorkspace`)
- `roles.rs`: Role definitions and constants (`Role`, `WorkspaceRole` enum)
- `workspace_members.rs`: Member assignment entities
- `requests.rs`: API request models for complex operations

#### `/src/services/`
Business logic layer:
- `users.rs`: User registration, password hashing, validation
- `workspaces.rs`: Workspace creation, ownership transfer, access control
- `roles.rs`: Role creation, default role setup, role management
- `workspace_members.rs`: Member assignment and role validation

#### `/src/queries/`
Data access layer:
- Direct database operations using SQLx
- CRUD operations for all entities
- Transaction handling for complex operations

#### `/tests/`
Comprehensive test suite with isolated test data management:
- `common/database.rs`: Test database setup with automatic cleanup
- Individual test modules for each service layer
- Parallel-safe test execution with unique prefixes

### Key Design Patterns

#### Simplified Workspace Creation
The system provides simplified APIs that handle complex multi-step operations:
```rust
// Creates workspace + default roles + owner as admin in one transaction
let result = create_workspace(&mut conn, request).await?;
// Returns: CompleteWorkspaceResult with workspace, roles, owner_membership, members
```

#### Role System with Type Safety
- Uses `WorkspaceRole` enum for type-safe role handling
- Centralized role constants: `ADMIN_ROLE`, `EDITOR_ROLE`, `MEMBER_ROLE`, `VIEWER_ROLE`
- Automatic default role creation for all workspaces
- Support for custom workspace-specific roles

#### Test Isolation System
Tests use a sophisticated isolation system:
- Each test gets unique database namespace: `"test_{test_name}"`
- Automatic cleanup before/after each test
- Parallel-safe test execution
- Helper methods for creating test data with proper prefixes

#### Password Security
- Argon2 password hashing with unique salts
- Minimum 8-character password requirement
- Secure password verification with constant-time comparison
- Password confirmation required during registration

## Database Schema

### Core Tables
- `users`: Global user accounts with unique emails
- `workspaces`: Workspace containers with single owner
- `roles`: Workspace-scoped role definitions
- `workspace_members`: Many-to-many user-workspace relationships with roles

### Key Constraints
- `users.email`: Globally unique
- `roles(workspace_id, name)`: Unique role names per workspace
- `workspace_members(workspace_id, user_id)`: One membership per user per workspace
- Foreign key cascades: Deleting workspace deletes all roles and members

### Migration System
Uses SQLx migrations in `/migrations/` directory:
- `20251009102916_extensions.up.sql`: Database extensions setup
- `20251009103739_users_and_workspaces.up.sql`: Core tables and relationships

## Service Layer APIs

### User Management
```rust
// Basic user registration with password hashing
register_user(&mut conn, RegisterUser) -> Result<User>

// Combined user + workspace creation in single transaction
register_user_with_workspace(&mut conn, UserWorkspaceRegistrationRequest) -> Result<UserWorkspaceResult>
```

### Workspace Management
```rust
// Simplified workspace creation with automatic setup
create_workspace(&mut conn, CreateWorkspaceRequest) -> Result<CompleteWorkspaceResult>

// Workspace creation with initial team members
create_workspace_with_members(&mut conn, CreateWorkspaceWithMembersRequest) -> Result<CompleteWorkspaceResult>

// Ownership transfer with role management
update_workspace_owner(&mut conn, workspace_id, current_owner_id, new_owner_id) -> Result<Workspace>
```

### Role Management
```rust
// Create default roles (admin, editor, member, viewer) for workspace
create_default_roles(&mut conn, workspace_id) -> Result<Vec<Role>>

// Create custom workspace-specific role
create_single_role(&mut conn, NewRole) -> Result<Role>
```

## Configuration

### Environment Variables
Uses `BUILDSCALE__` prefix with double underscore separators:
- `BUILDSCALE__DATABASE__USER`: Database username
- `BUILDSCALE__DATABASE__PASSWORD`: Database password
- `BUILDSCALE__DATABASE__HOST`: Database host
- `BUILDSCALE__DATABASE__PORT`: Database port
- `BUILDSCALE__DATABASE__DATABASE`: Database name

### Configuration Loading
- Loads from `.env` file if present
- Overrides with environment variables
- Provides sensible defaults for development
- Supports `DATABASE_URL` for sqlx CLI

## Error Handling

### Error Hierarchy
```rust
pub enum Error {
    Sqlx(#[from] sqlx::Error),           // Database errors
    Validation(String),                   // Input validation errors
    NotFound(String),                      // Resource not found
    Forbidden(String),                    // Permission denied
    Conflict(String),                      // Resource conflicts
    Internal(String),                      // System errors
}
```

### Validation Rules
- **Users**: Email uniqueness, 8+ character passwords, password confirmation
- **Workspaces**: 1-100 character names, owner must exist
- **Roles**: Unique names per workspace, 100 char name limit, 500 char description limit

## Testing Strategy

### Test Organization
- Unit tests for individual service functions
- Integration tests for complete workflows
- Database constraint testing
- Error scenario coverage

### Test Data Management
Uses `TestApp` and `TestDb` utilities in `/tests/common/database.rs`:
- Automatic test database initialization
- Unique test prefixes for isolation
- Helper methods for creating test entities
- Automatic cleanup on test completion

### Running Tests
```bash
# Run all tests
cargo test

# Run specific test file
cargo test tests/users/services/

# Run with output for debugging
cargo test -- --nocapture

# Run single test
cargo test test_user_registration_success
```

## Examples

### Available Examples
- `01_hello.rs`: Basic configuration loading
- `02_users_management.rs`: User registration and management
- `03_workspaces_management.rs`: Complete workspace creation with roles and members

### Running Examples
```bash
cargo run --example 01_hello
cargo run --example 02_users_management
cargo run --example 03_workspaces_management
```

## Development Workflow: Code → Tests → Examples → Documentation

This codebase follows a strict 4-step development workflow for all features:

### 1. Code Implementation
- **Models**: Define data structures and validation in `/src/models/`
- **Services**: Implement business logic in `/src/services/`
- **Queries**: Add data access layer in `/src/queries/`
- **Error Handling**: Add comprehensive error types and validation

### 2. Test Coverage
- **Unit Tests**: Test individual service functions
- **Integration Tests**: Test complete workflows across services
- **Edge Cases**: Test validation rules and error scenarios
- **Test Isolation**: Use unique prefixes for parallel-safe testing
- **Location**: `/tests/` mirrors the `/src/` structure

### 3. Example Implementation
- **Demonstration**: Create practical examples showing feature usage
- **Real-world Scenarios**: Show common patterns and workflows
- **Verification**: Examples should run successfully and validate functionality
- **Location**: `/examples/` with clear naming (01_hello, 02_users_management, etc.)

### 4. Documentation Updates
- **API Documentation**: Update docstrings for all public functions
- **System Documentation**: Update `/docs/USERS_ROLES_WORKSPACES.md` with architectural changes
- **Usage Examples**: Add code examples to documentation
- **Role Constant Updates**: Include new roles in all relevant documentation sections

### Workflow Example: Member Role Implementation
```bash
# 1. Code: Added MEMBER_ROLE constant and WorkspaceRole::Member variant
# 2. Tests: Updated all tests to expect 4 default roles instead of 3
# 3. Examples: Updated workspace_management example to demonstrate Member role
# 4. Documentation: Updated comprehensive system documentation
```

### Quality Gates
- **All tests must pass** before proceeding to next step
- **Examples must run successfully** before documentation
- **Documentation must be comprehensive** before considering feature complete
- **No step should be skipped** - each builds on the previous

## Development Guidelines

### Code Organization
- Separate concerns: models (data), services (business logic), queries (data access)
- Use type-safe enums for role management
- Centralized constants for role names
- Comprehensive error handling with specific error types

### Database Patterns
- Use transactions for multi-step operations
- Parameterized queries to prevent SQL injection
- Database constraints for data integrity
- Cascade operations for data consistency

### Testing Patterns
- Use test prefixes for data isolation
- Clean up test data automatically
- Test both success and failure scenarios
- Use helper methods for common test setup

### Security Considerations
- Argon2 password hashing with unique salts
- Workspace data isolation
- Role-based access control
- Input validation and sanitization