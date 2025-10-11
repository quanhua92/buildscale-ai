# Examples and Testing Guide

This guide provides comprehensive documentation for the examples and testing infrastructure in the backend project. It covers how to run examples, understand the testing framework, and develop new tests following the established patterns.

## Table of Contents

1. [Overview](#overview)
2. [Examples](#examples)
   - [01_hello.rs - Basic Configuration](#01_hello_rs---basic-configuration)
   - [02_users_management.rs - Complete User Management](#02_users_management_rs---complete-user-management)
3. [Testing Framework](#testing-framework)
   - [Test Infrastructure](#test-infrastructure)
   - [Test Isolation Strategy](#test-isolation-strategy)
   - [Running Tests](#running-tests)
4. [Test Categories](#test-categories)
   - [Service Layer Tests](#service-layer-tests)
   - [Database Query Tests](#database-query-tests)
5. [Writing New Tests](#writing-new-tests)
   - [Service Test Example](#service-test-example)
   - [Database Test Example](#database-test-example)
6. [Best Practices](#best-practices)
7. [Troubleshooting](#troubleshooting)

## Overview

This backend project demonstrates a complete multi-tenant system with:
- **Robust testing** with comprehensive test coverage across all modules
- **Working examples** showcasing all features
- **Database integration** with PostgreSQL
- **Proper error handling** and validation
- **Password security** with Argon2 hashing
- **Transaction management** and isolation
- **Modular test organization** for maintainable test suites

## Examples

### 01_hello.rs - Basic Configuration

**Purpose**: Demonstrates basic configuration loading and display.

**Features**:
- Loads configuration from environment/`.env` file
- Displays configuration in pretty-printed format
- Shows basic error handling patterns

**Running**:
```bash
cargo run --example 01_hello
```

**Expected Output**:
```
Loaded configuration:
{
  "database": {
    "user": "buildscale",
    "host": "localhost",
    "port": 5432,
    "database": "buildscale"
  }
}
```

**Key Takeaways**:
- Configuration is loaded using `backend::load_config()`
- The configuration implements `Display` for pretty printing
- Error handling uses `Result<(), Box<dyn std::error::Error>>`

### 02_users_management.rs - Complete User Management

**Purpose**: Comprehensive demonstration of all user management features.

**Features**:
- ✅ User registration with validation
- ✅ Password hashing and verification (Argon2)
- ✅ Multiple user scenarios (different emails, passwords)
- ✅ User updates (full and partial)
- ✅ Database lookup methods
- ✅ Transaction isolation demonstration
- ✅ Database constraint testing
- ✅ Direct database operations
- ✅ Error handling and validation
- ✅ Re-run safety with automatic cleanup

**Running**:
```bash
cargo run --example 02_users_management
```

**Safety Features**:
- **Idempotent**: Safe to run multiple times
- **Auto-cleanup**: Automatically cleans up previous runs
- **Prefix Isolation**: Uses `example_02_users_management` prefix
- **Error Resilience**: Handles missing tables gracefully

## Testing Framework

### Test Infrastructure

The testing framework is built around a robust isolation system with modular organization:

```
tests/
├── common/
│   ├── database.rs    # Test utilities (TestDb, TestApp)
│   └── mod.rs         # Module exports
├── users/             # User-related tests
│   ├── services/      # Service layer tests
│   └── queries/       # Database query tests
├── workspaces/        # Workspace-related tests
│   ├── services/      # Service layer tests
│   └── queries/       # Database query tests
├── roles/             # Role-related tests
│   ├── services/      # Service layer tests
│   └── queries/       # Database query tests
└── workspace_members/ # Workspace member tests
    ├── services/      # Service layer tests
    └── queries/       # Database query tests
```

### Test Isolation Strategy

**Prefix-Based Isolation**: Each test uses a unique database namespace to prevent conflicts:

- **Service Tests**: `test_{function_name}` prefix (e.g., `test_user_registration_success`)
- **Query Tests**: `test_{function_name}` prefix (e.g., `test_create_user_query`)
- **Examples**: `example_{module_name}` prefix

**Data Isolation**:
- Test emails: `test_{function_name}_{<uuid>}@example.com`
- Test entities: All database entities use the test prefix for names
- Automatic cleanup before and after each test
- Parallel test execution safety

**TestApp Helper Methods**:
The `TestApp` struct provides comprehensive helpers for all entity types:

```rust
// User helpers
test_app.generate_test_user()
test_app.generate_test_user_with_password()
test_app.count_test_users()
test_app.user_exists()

// Workspace helpers
test_app.create_test_workspace_with_user()
test_app.generate_test_workspace_with_owner_id()
test_app.count_test_workspaces()
test_app.workspace_exists()

// Role helpers
test_app.generate_test_role()
test_app.generate_test_role_with_name()
test_app.count_test_roles()
test_app.role_exists()

// Complete scenario helpers
test_app.create_complete_test_scenario()
test_app.is_workspace_member()
test_app.count_workspace_members()
```

### Running Tests

**All Tests**:
```bash
cargo test
```

**Specific Module Tests**:
```bash
cargo test --test users
cargo test --test workspaces
cargo test --test roles
cargo test --test workspace_members
```

**Specific Test Categories**:
```bash
# Service layer tests only
cargo test services

# Query layer tests only
cargo test queries
```

**Specific Test Functions**:
```bash
cargo test test_user_registration_success
cargo test test_create_user_query
```

**With Output**:
```bash
cargo test -- --nocapture
```

## Test Categories

### Service Layer Tests

**Purpose**: Test business logic and service layer functionality.

**Coverage**:
- ✅ User registration workflow with validation
- ✅ Password security (hashing, verification, validation)
- ✅ Email validation and edge cases
- ✅ Database constraints and error handling
- ✅ Transaction isolation
- ✅ Workspace creation and management
- ✅ Role-based access control (RBAC)
- ✅ Workspace member management
- ✅ Multi-tenant business logic
- ✅ Complex relationship handling

**Key Service Test Patterns**:

1. **Entity Creation with Validation**:
   ```rust
   let test_app = TestApp::new("test_entity_creation").await;
   let mut conn = test_app.get_connection().await;
   let entity_data = test_app.generate_test_entity();
   let result = create_entity_service(&mut conn, entity_data).await;
   assert!(result.is_ok());
   ```

2. **Business Logic Validation**:
   ```rust
   // Test invalid scenarios
   let invalid_data = test_app.generate_test_entity();
   // Modify to create invalid state
   let result = create_entity_service(&mut conn, invalid_data).await;
   assert!(result.is_err());
   assert!(result.unwrap_err().to_string().contains("specific error"));
   ```

3. **Multi-Entity Scenarios**:
   ```rust
   // Create complete test scenarios
   let (user, workspace, role, member) = test_app.create_complete_test_scenario().await.unwrap();
   // Test complex business logic involving multiple entities
   ```

### Database Query Tests

**Purpose**: Test direct database operations and query layer functionality.

**Coverage**:
- ✅ Direct CRUD operations for all entities
- ✅ Database constraint enforcement
- ✅ Query result validation
- ✅ Error handling for non-existent records
- ✅ Relationship integrity (foreign keys)
- ✅ Unique constraint validation
- ✅ Query parameter validation
- ✅ Data type handling
- ✅ Null/optional field handling

**Key Query Test Patterns**:

1. **Direct Database Operations**:
   ```rust
   let test_app = TestApp::new("test_query_operations").await;
   let mut conn = test_app.get_connection().await;
   let entity_data = test_app.generate_test_entity();
   let entity = create_entity_query(&mut conn, entity_data).await.unwrap();
   ```

2. **Constraint Testing**:
   ```rust
   // First creation succeeds
   create_entity_query(&mut conn, valid_data).await.unwrap();
   // Duplicate/invalid creation fails
   let result = create_entity_query(&mut conn, duplicate_data).await;
   assert!(result.is_err());
   ```

3. **Relationship Testing**:
   ```rust
   // Test foreign key constraints
   let valid_entity = create_entity_with_valid_relationship(&mut conn, data).await;
   let invalid_entity = create_entity_with_invalid_relationship(&mut conn, data).await;
   assert!(invalid_entity.is_err()); // Should fail due to FK constraint
   ```

## Writing New Tests

### Service Test Example

When writing new service layer tests, follow this pattern:

```rust
#[tokio::test]
async fn test_new_service_feature() {
    // 1. Create test app with unique name matching test function
    let test_app = TestApp::new("test_new_service_feature").await;
    let mut conn = test_app.get_connection().await;

    // 2. Generate test data using TestApp helpers
    let entity_data = test_app.generate_test_entity();

    // For complex scenarios, use complete test scenario
    let (user, workspace, role, _) = test_app.create_complete_test_scenario().await.unwrap();

    // 3. Execute service logic
    let result = your_service_function(&mut conn, entity_data).await;

    // 4. Assert results with specific error checking
    assert!(result.is_ok(), "Service should work correctly");

    // 5. Verify database state if needed
    let final_count = test_app.count_test_entities().await.unwrap();
    assert_eq!(final_count, expected_count, "Should have created expected number");

    // 6. Test error scenarios
    let invalid_data = test_app.generate_test_entity();
    // Modify to create invalid state
    let error_result = your_service_function(&mut conn, invalid_data).await;
    assert!(error_result.is_err());
    assert!(error_result.unwrap_err().to_string().contains("expected error message"));
}
```

### Database Test Example

For database layer tests, use TestDb directly:

```rust
#[tokio::test]
async fn test_new_query_function() {
    // 1. Create test database with unique name
    let test_app = TestApp::new("test_new_query_function").await;
    let mut conn = test_app.get_connection().await;

    // 2. Create prerequisite test data
    let (user, workspace) = test_app.create_test_workspace_with_user().await.unwrap();
    let entity_data = test_app.generate_test_entity(workspace.id);
    let entity = create_entity_query(&mut conn, entity_data).await.unwrap();

    // 3. Test your query function
    let found = your_query_function(&mut conn, entity.id).await.unwrap();

    // 4. Assert results
    assert_eq!(found.id, entity.id, "Should find correct entity");
    assert_eq!(found.name, entity.name, "Name should match");

    // 5. Test non-existent scenarios
    let fake_id = uuid::Uuid::now_v7();
    let not_found = your_query_function(&mut conn, fake_id).await;
    assert!(not_found.is_err(), "Should return error for non-existent entity");
}
```

### Constraint Testing Example

```rust
#[tokio::test]
async fn test_entity_constraints() {
    let test_app = TestApp::new("test_entity_constraints").await;
    let mut conn = test_app.get_connection().await;

    // Create valid entity
    let valid_data = test_app.generate_test_entity();
    let entity = create_entity_query(&mut conn, valid_data).await.unwrap();

    // Test unique constraint violation
    let duplicate_data = test_app.generate_test_entity();
    // Set to same unique field value
    duplicate_data.unique_field = entity.unique_field.clone();

    let result = create_entity_query(&mut conn, duplicate_data).await;
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("duplicate") || error_msg.contains("unique"));

    // Test foreign key constraint
    let invalid_fk_data = test_app.generate_test_entity();
    invalid_fk_data.workspace_id = uuid::Uuid::now_v7(); // Non-existent workspace

    let result = create_entity_query(&mut conn, invalid_fk_data).await;
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("foreign") || error_msg.contains("constraint"));
}
```

## Best Practices

### Test Organization

- **Module Structure**: Organize tests by feature module (users, workspaces, roles, etc.)
- **Layer Separation**: Separate service layer tests from query layer tests
- **Descriptive Names**: Use descriptive test names that explain what's being tested
- **Consistent Prefixes**: Always match TestApp name with test function name

```rust
// ✅ Good - organized by module and layer
tests/
├── users/services/registration_tests.rs
├── users/queries/create_user_tests.rs
├── workspaces/services/workspace_management_tests.rs
└── roles/queries/role_constraints_tests.rs
```

### Test Naming

- **Function Names**: Use descriptive names that explain the scenario
- **Test Prefix**: Always use `test_` prefix for TestApp/TestDb construction
- **Scenario Description**: Include what's being tested and expected outcome

```rust
#[tokio::test]
async fn test_user_registration_success() {              // ✅ Good - clear and descriptive
    let test_app = TestApp::new("test_user_registration_success").await; // ✅ Matches
}

#[tokio::test]
async fn test_workspace_creation_duplicate_name_validation() { // ✅ Good - specific scenario
    let test_app = TestApp::new("test_workspace_creation_duplicate_name_validation").await;
}
```

### Data Management

- **Use TestApp Helpers**: Always use appropriate TestApp helpers for data creation
- **Prefix Consistency**: Never hardcode entity names without test prefixes
- **Complete Scenarios**: Use `create_complete_test_scenario()` for complex multi-entity tests
- **Relationship Integrity**: Ensure proper foreign key relationships in test data

```rust
// ✅ Good - uses TestApp helpers
let user_data = test_app.generate_test_user();
let (user, workspace) = test_app.create_test_workspace_with_user().await.unwrap();
let complete = test_app.create_complete_test_scenario().await.unwrap();

// ❌ Bad - hardcoded data without prefix
let user_data = RegisterUser {
    email: "test@example.com".to_string(), // No prefix!
    password: "password".to_string(),
};
```

### Error Testing

- **Comprehensive Coverage**: Test both success and failure scenarios
- **Specific Error Messages**: Assert on specific error messages when relevant
- **Constraint Validation**: Test all database constraints
- **Business Logic Validation**: Test service layer validation rules

```rust
// ✅ Good - tests both success and failure with specific error checking
let valid_result = register_user(&mut conn, valid_data).await;
assert!(valid_result.is_ok());

let invalid_result = register_user(&mut conn, invalid_data).await;
assert!(invalid_result.is_err());
let error_msg = invalid_result.unwrap_err().to_string();
assert!(error_msg.contains("Passwords do not match") || error_msg.contains("Password validation"));
```

### Multi-Entity Testing

- **Use Complete Scenarios**: For tests involving multiple entities, use `create_complete_test_scenario()`
- **Test Relationships**: Validate entity relationships and constraints
- **Isolation**: Ensure tests don't interfere with each other's data

```rust
// ✅ Good - uses complete scenario for multi-entity tests
let (user, workspace, role, member) = test_app.create_complete_test_scenario().await.unwrap();

// Test business logic involving multiple entities
let result = add_user_to_workspace_with_role(&mut conn, user.id, workspace.id, role.name).await;
assert!(result.is_ok());

// Verify all relationships are maintained
assert!(test_app.is_workspace_member(workspace.id, user.id).await.unwrap());
```

## Troubleshooting

### Common Issues

1. **Test Fails Due to Data Conflicts**:
   - **Cause**: Tests not using proper prefixes or isolation
   - **Fix**: Ensure TestApp name matches test function name exactly

2. **Foreign Key Constraint Violations**:
   - **Cause**: Tests creating entities with non-existent related entities
   - **Fix**: Use `create_test_workspace_with_user()` or `create_complete_test_scenario()` helpers

3. **Service vs Query Layer Confusion**:
   - **Cause**: Using wrong test pattern for the layer being tested
   - **Fix**: Service tests should use service functions, query tests should use query functions

4. **Database Connection Errors**:
   - **Cause**: PostgreSQL not running or wrong credentials
   - **Fix**: Check `.env` file and ensure PostgreSQL is running

5. **Example Fails on Re-run**:
   - **Cause**: Previous run data not cleaned up
   - **Fix**: Examples are auto-cleaning, but manual cleanup may be needed

### Debugging Tests

**Enable Output**:
```bash
cargo test -- --nocapture
```

**Run Specific Test Categories**:
```bash
cargo test services  # Service layer tests only
cargo test queries   # Query layer tests only
cargo test users     # User module tests only
```

**Check Database State**:
Add debug prints to see what data exists:
```rust
println!("Test prefix: {}", test_app.test_prefix());
println!("User count: {}", test_app.count_test_users().await.unwrap());
println!("Workspace count: {}", test_app.count_test_workspaces().await.unwrap());
```

**Test Data Inspection**:
```rust
// Print generated test data
let user_data = test_app.generate_test_user();
println!("Generated user email: {}", user_data.email);
```

### Performance Considerations

- **Parallel Tests**: Tests are designed to run safely in parallel
- **Connection Pooling**: Uses efficient connection pooling
- **Batch Cleanup**: Cleanup operations are batched for efficiency
- **Memory Management**: Proper connection cleanup prevents resource leaks
- **Test Isolation**: Minimal cross-test data dependencies

## Conclusion

This testing and examples framework provides:

1. **Comprehensive Coverage**: Tests cover all major functionality across all modules
2. **Robust Isolation**: Tests can run safely in parallel without interference
3. **Clear Examples**: Working demonstrations of all features
4. **Best Practices**: Established patterns for new development
5. **Modular Organization**: Well-structured test suites that scale with the application
6. **Maintainable Code**: Well-documented, organized codebase

The framework is designed to be extensible - follow the established patterns when adding new tests or examples, and the system will maintain its reliability and performance characteristics as the application grows.