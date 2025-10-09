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
   - [Database Layer Tests](#database-layer-tests)
5. [Writing New Tests](#writing-new-tests)
   - [Service Test Example](#service-test-example)
   - [Database Test Example](#database-test-example)
6. [Best Practices](#best-practices)
7. [Troubleshooting](#troubleshooting)

## Overview

This backend project demonstrates a complete user management system with:
- **Robust testing** with 23 comprehensive tests
- **Working examples** showcasing all features
- **Database integration** with PostgreSQL
- **Proper error handling** and validation
- **Password security** with Argon2 hashing
- **Transaction management** and isolation

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

**Key Features Demonstrated**:

1. **User Registration**:
   ```rust
   let user = register_user(&mut conn, RegisterUser {
       email: "user@example.com".to_string(),
       password: "password123".to_string(),
       confirm_password: "password123".to_string(),
   }).await?;
   ```

2. **Password Verification**:
   ```rust
   let is_valid = verify_password("password123", &user.password_hash)?;
   ```

3. **Database Transactions**:
   ```rust
   let mut tx = pool.begin().await?;
   // Operations within transaction
   tx.commit().await?;
   ```

4. **Direct Database Operations**:
   ```rust
   let user = create_user(&mut conn, NewUser {
       email: "direct@example.com".to_string(),
       password_hash: "hash".to_string(),
       full_name: Some("Direct User".to_string()),
   }).await?;
   ```

**Safety Features**:
- **Idempotent**: Safe to run multiple times
- **Auto-cleanup**: Automatically cleans up previous runs
- **Prefix Isolation**: Uses `example_02_users_management` prefix
- **Error Resilience**: Handles missing tables gracefully

## Testing Framework

### Test Infrastructure

The testing framework is built around a robust isolation system:

```
tests/
├── common/
│   ├── database.rs    # Test utilities (TestDb, TestApp)
│   └── mod.rs         # Module exports
├── user_services_tests.rs  # Service layer tests
└── user_queries_tests.rs   # Database layer tests
```

### Test Isolation Strategy

**Prefix-Based Isolation**: Each test uses a unique database namespace to prevent conflicts:

- **Service Tests**: `test_{function_name}` prefix (e.g., `test_user_registration_success`)
- **Database Tests**: `test_{function_name}` prefix (e.g., `test_create_user_query`)
- **Examples**: `example_02_users_management` prefix

**Data Isolation**:
- Test emails: `test_user_registration_success_<uuid>@example.com`
- Automatic cleanup before and after each test
- Parallel test execution safety

### Running Tests

**All Tests**:
```bash
cargo test
```

**Specific Test Files**:
```bash
cargo test --test user_services_tests
cargo test --test user_queries_tests
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

### Service Layer Tests (`user_services_tests.rs`)

**Purpose**: Test business logic and service layer functionality.

**Coverage**:
- ✅ User registration workflow
- ✅ Password validation (length, matching)
- ✅ Password hashing and verification
- ✅ Email validation and edge cases
- ✅ Database constraints
- ✅ Transaction isolation
- ✅ User lookup operations
- ✅ Multiple user scenarios

**Key Tests**:

1. **User Registration**:
   ```rust
   let test_app = TestApp::new("test_user_registration_success").await;
   let mut conn = test_app.get_connection().await;
   let user_data = test_app.generate_test_user();
   let user = register_user(&mut conn, user_data).await.unwrap();
   ```

2. **Password Validation**:
   ```rust
   // Test password mismatch
   let mut user_data = test_app.generate_test_user();
   user_data.confirm_password = "different".to_string();
   let result = register_user(&mut conn, user_data).await;
   assert!(result.is_err()); // Should fail
   ```

3. **Transaction Isolation**:
   ```rust
   let mut tx = test_app.test_db.pool.begin().await.unwrap();
   let user = register_user(tx.as_mut(), user_data).await.unwrap();
   // User exists in transaction but not outside
   tx.commit().await?;
   // Now user exists outside
   ```

### Database Layer Tests (`user_queries_tests.rs`)

**Purpose**: Test direct database operations and query layer.

**Coverage**:
- ✅ Direct user creation
- ✅ User retrieval by ID
- ✅ User updates (full and partial)
- ✅ User deletion
- ✅ User listing and ordering
- ✅ Database constraint enforcement
- ✅ Error handling for non-existent records

**Key Tests**:

1. **Direct Database Operations**:
   ```rust
   let new_user = NewUser {
       email: "test@example.com".to_string(),
       password_hash: "hash".to_string(),
       full_name: Some("Test User".to_string()),
   };
   let user = create_user(&mut conn, new_user).await.unwrap();
   ```

2. **User Updates**:
   ```rust
   let mut user_to_update = created_user.clone();
   user_to_update.email = "updated@example.com".to_string();
   let updated = update_user(&mut conn, &user_to_update).await.unwrap();
   ```

3. **Database Constraints**:
   ```rust
   // First user succeeds
   create_user(&mut conn, user1).await.unwrap();
   // Duplicate email fails
   let result = create_user(&mut conn, user2).await;
   assert!(result.is_err()); // Should fail due to unique constraint
   ```

## Writing New Tests

### Service Test Example

When writing new service layer tests, follow this pattern:

```rust
#[tokio::test]
async fn test_new_feature() {
    // 1. Create test app with unique name
    let test_app = TestApp::new("test_new_feature").await;
    let mut conn = test_app.get_connection().await;

    // 2. Generate test data using TestApp helpers
    let user_data = test_app.generate_test_user();

    // 3. Execute test logic
    let result = your_service_function(&mut conn, user_data).await;

    // 4. Assert results
    assert!(result.is_ok(), "Feature should work correctly");

    // 5. Verify database state if needed
    let final_count = test_app.count_test_users().await.unwrap();
    assert_eq!(final_count, 1, "Should have created one user");
}
```

### Database Test Example

For database layer tests, use TestDb directly:

```rust
#[tokio::test]
async fn test_new_query_function() {
    // 1. Create test database
    let test_db = TestDb::new("test_new_query_function").await;
    let mut conn = test_db.get_connection().await;

    // 2. Create test data directly
    let new_user = NewUser {
        email: format!("{}_test@example.com", test_db.test_prefix()),
        password_hash: "test_hash".to_string(),
        full_name: None,
    };
    let user = create_user(&mut conn, new_user).await.unwrap();

    // 3. Test your query function
    let found = your_query_function(&mut conn, user.id).await.unwrap();

    // 4. Assert results
    assert_eq!(found.id, user.id, "Should find correct user");
}
```

## Best Practices

### Test Naming

- **Function Name**: Use descriptive names that explain what's being tested
- **Test Prefix**: Always use `test_` prefix for TestApp/TestDb construction
- **Consistency**: Match the test function name with the TestApp/TestDb name

```rust
#[tokio::test]
async fn test_user_email_validation() {  // ✅ Good
    let test_app = TestApp::new("test_user_email_validation").await; // ✅ Matches
    // ...
}
```

### Data Management

- **Use TestApp Helpers**: Always use `test_app.generate_test_user()` for data creation
- **Prefix Consistency**: Never hardcode emails without the test prefix
- **Cleanup**: Let the automatic cleanup handle data removal

```rust
// ✅ Good - uses TestApp helper
let user_data = test_app.generate_test_user();

// ❌ Bad - hardcoded email
let user_data = RegisterUser {
    email: "test@example.com".to_string(), // No prefix!
    password: "password".to_string(),
    confirm_password: "password".to_string(),
};
```

### Error Testing

- **Test Success and Failure**: Test both valid and invalid scenarios
- **Error Messages**: Assert on specific error messages when relevant
- **Database Constraints**: Test constraint violations

```rust
// ✅ Good - tests both success and failure
let valid_result = register_user(&mut conn, valid_data).await;
assert!(valid_result.is_ok());

let invalid_result = register_user(&mut conn, invalid_data).await;
assert!(invalid_result.is_err());
assert!(invalid_result.unwrap_err().to_string().contains("Passwords do not match"));
```

### Transaction Testing

- **Isolation Verification**: Test that transactions properly isolate data
- **Commit/Rollback**: Test both successful commits and rollbacks
- **Visibility**: Test data visibility inside vs outside transactions

## Troubleshooting

### Common Issues

1. **Test Fails Due to Data Conflicts**:
   - **Cause**: Tests not using proper prefixes
   - **Fix**: Ensure TestApp/TestDb name matches test function name

2. **Database Connection Errors**:
   - **Cause**: PostgreSQL not running or wrong credentials
   - **Fix**: Check `.env` file and ensure PostgreSQL is running

3. **Example Fails on Re-run**:
   - **Cause**: Previous run data not cleaned up
   - **Fix**: Examples are auto-cleaning, but you can manually clean:
     ```sql
     DELETE FROM users WHERE email LIKE 'example_02_users_management%';
     ```

4. **Clippy Warnings**:
   - **Cause**: Code quality issues
   - **Fix**: Run `cargo clippy --fix` or address warnings manually

### Debugging Tests

**Enable Output**:
```bash
cargo test -- --nocapture
```

**Run Single Test**:
```bash
cargo test test_specific_function
```

**Check Database State**:
Add debug prints to see what data exists:
```rust
println!("Test prefix: {}", test_app.test_prefix());
println!("User count: {}", test_app.count_test_users().await.unwrap());
```

### Performance Considerations

- **Parallel Tests**: Tests are designed to run in parallel safely
- **Connection Pooling**: Uses efficient connection pooling
- **Batch Cleanup**: Cleanup is done in batches for efficiency
- **Memory Management**: Proper connection cleanup prevents leaks

## Conclusion

This testing and examples framework provides:

1. **Comprehensive Coverage**: Tests cover all major functionality
2. **Robust Isolation**: Tests can run safely in parallel
3. **Clear Examples**: Working demonstrations of all features
4. **Best Practices**: Established patterns for new development
5. **Maintainable Code**: Well-documented, organized codebase

The framework is designed to be extensible - follow the established patterns when adding new tests or examples, and the system will maintain its reliability and performance characteristics.