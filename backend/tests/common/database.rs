use backend::load_config;
use secrecy::ExposeSecret;
use sqlx::PgPool;
use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize test database
pub async fn init_test_db() -> PgPool {
    INIT.call_once(|| {
        dotenvy::dotenv().ok();
    });

    let config = load_config().expect("Failed to load config");
    let pool = PgPool::connect(config.database.connection_string().expose_secret())
        .await
        .expect("Failed to connect to database");

    // Run migrations if they exist
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .unwrap_or_else(|_| {
            // If no migrations exist, create basic tables
            tokio::spawn(create_test_tables(pool.clone()));
        });

    // Clean up any existing test data
    cleanup_test_data(&pool).await;

    pool
}

/// Create basic test tables if migrations don't exist
async fn create_test_tables(pool: PgPool) -> Result<(), sqlx::Error> {
    let create_users_table = r#"
        CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            email VARCHAR(255) UNIQUE NOT NULL,
            password_hash VARCHAR(255) NOT NULL,
            full_name VARCHAR(255),
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        )
    "#;

    sqlx::query(create_users_table).execute(&pool).await?;
    Ok(())
}

/// Clean up test data with hardcoded test prefix
async fn cleanup_test_data(pool: &PgPool) {
    // Delete test users with hardcoded test prefix
    let cleanup_query = r#"
        DELETE FROM users
        WHERE email LIKE 'test_backend_%'
    "#;

    sqlx::query(cleanup_query)
        .execute(pool)
        .await
        .expect("Failed to cleanup test data");
}

/// Test database wrapper for better test isolation
pub struct TestDb {
    pub pool: PgPool,
    test_prefix: String,
}

impl TestDb {
    /// Creates a new test database instance with isolated data namespace.
    ///
    /// # Arguments
    /// * `test_name` - The name of the test function (MUST match the test function name for consistency)
    ///
    /// # How it works:
    /// 1. Creates a database connection pool using the production config
    /// 2. Generates a unique prefix: `"test_{test_name}"` (e.g., "test_user_registration_success")
    /// 3. Automatically cleans up any existing data with this prefix (for test retries)
    /// 4. Returns a TestDb instance with this isolated namespace
    ///
    /// # Important Rules:
    /// - **ALWAYS use the test function name as `test_name`** - e.g., for `fn test_user_registration()`, use `"test_user_registration"`
    /// - This prevents conflicts when tests run in parallel
    /// - Each test gets its own database namespace
    /// - Easy debugging: database entries can be traced back to specific tests
    ///
    /// # Example Usage:
    /// ```rust
    /// #[tokio::test]
    /// async fn test_user_registration_success() {
    ///     let test_db = TestDb::new("test_user_registration_success").await;
    ///     // ... test logic
    /// }
    /// ```
    ///
    /// # Database Isolation:
    /// - Test data is stored with emails like: `"test_user_registration_success_<uuid>@example.com"`
    /// - `count_test_users()` only counts users with this test's prefix
    /// - Automatic cleanup happens when TestDb is dropped
    /// - No interference between parallel tests
    pub async fn new(test_name: &str) -> Self {
        let pool = init_test_db().await;
        let test_prefix = format!("test_{}", test_name);

        // Clean up any existing data with this specific prefix (handles test retries)
        Self::cleanup_prefix(&pool, &test_prefix).await;

        Self { pool, test_prefix }
    }

    pub async fn get_connection(&self) -> sqlx::pool::PoolConnection<sqlx::Postgres> {
        self.pool
            .acquire()
            .await
            .expect("Failed to get database connection")
    }

    /// Get the test prefix for this test instance
    pub fn test_prefix(&self) -> &str {
        &self.test_prefix
    }

    /// Clean up users with specific test prefix
    async fn cleanup_prefix(pool: &PgPool, prefix: &str) {
        let cleanup_query = "DELETE FROM users WHERE email LIKE $1";
        sqlx::query(cleanup_query)
            .bind(format!("{}%", prefix))
            .execute(pool)
            .await
            .expect("Failed to cleanup test data");
    }

    /// Get a count of users with test prefix
    pub async fn count_test_users(&self) -> Result<i64, sqlx::Error> {
        let count = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE email LIKE $1")
            .bind(format!("{}%", self.test_prefix))
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    /// Check if a user exists by email
    pub async fn user_exists(&self, email: &str) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
            .bind(email)
            .fetch_one(&self.pool)
            .await?;
        Ok(exists)
    }

    /// Get user password hash for testing
    #[allow(dead_code)]
    pub async fn get_user_password_hash(&self, email: &str) -> Result<Option<String>, sqlx::Error> {
        let hash = sqlx::query_scalar("SELECT password_hash FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;
        Ok(hash)
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        // Cleanup test data when TestDb is dropped
        let pool = self.pool.clone();
        let prefix = self.test_prefix.clone();
        tokio::spawn(async move {
            let cleanup_query = "DELETE FROM users WHERE email LIKE $1";
            let _ = sqlx::query(cleanup_query)
                .bind(format!("{}%", prefix))
                .execute(&pool)
                .await;
        });
    }
}

/// Test application wrapper that manages test data creation
pub struct TestApp {
    pub test_db: TestDb,
}

impl TestApp {
    /// Creates a new test app with isolated data namespace
    pub async fn new(test_name: &str) -> Self {
        let test_db = TestDb::new(test_name).await;
        Self { test_db }
    }

    /// Get the test database connection
    pub async fn get_connection(&self) -> sqlx::pool::PoolConnection<sqlx::Postgres> {
        self.test_db.get_connection().await
    }

    /// Get the test prefix
    pub fn test_prefix(&self) -> &str {
        self.test_db.test_prefix()
    }

    /// Generate a unique test user data with proper prefix
    pub fn generate_test_user(&self) -> backend::models::users::RegisterUser {
        let email = self.generate_test_email();
        backend::models::users::RegisterUser {
            email,
            password: "testpassword123".to_string(),
            confirm_password: "testpassword123".to_string(),
        }
    }

    /// Generate a unique test user data with custom password
    #[allow(dead_code)]
    pub fn generate_test_user_with_password(
        &self,
        password: &str,
    ) -> backend::models::users::RegisterUser {
        let mut user = self.generate_test_user();
        user.password = password.to_string();
        user.confirm_password = password.to_string();
        user
    }

    /// Generate a unique test user data with custom email
    #[allow(dead_code)]
    pub fn generate_test_user_with_email(
        &self,
        email: &str,
    ) -> backend::models::users::RegisterUser {
        let mut user = self.generate_test_user();
        user.email = email.to_string();
        user
    }

    /// Generate test users for list testing with proper prefix
    #[allow(dead_code)]
    pub fn generate_list_test_users(
        &self,
        count: usize,
    ) -> Vec<backend::models::users::RegisterUser> {
        (0..count)
            .map(|i| {
                let mut user = self.generate_test_user();
                user.email = format!("{}_test_list_{}@example.com", self.test_prefix(), i);
                user
            })
            .collect()
    }

    /// Generate test users for edge case email testing
    #[allow(dead_code)] // Actually used in user_services_tests.rs, clippy false positive
    pub fn generate_edge_case_users(&self) -> Vec<backend::models::users::RegisterUser> {
        vec![
            format!("{}_user+tag@example.com", self.test_prefix()),
            format!("{}_user.name@example.com", self.test_prefix()),
            format!("{}_user123@example.com", self.test_prefix()),
            format!("{}_UPPERCASE@EXAMPLE.COM", self.test_prefix()),
        ]
        .into_iter()
        .map(|email| self.generate_test_user_with_email(&email))
        .collect()
    }

    /// Generate a unique test email with proper prefix
    pub fn generate_test_email(&self) -> String {
        let uuid = uuid::Uuid::now_v7();
        format!("{}_{}@example.com", self.test_prefix(), uuid)
    }

    /// Get a count of users with test prefix
    #[allow(dead_code)] // Actually used in user_services_tests.rs and user_queries_tests.rs, clippy false positive
    pub async fn count_test_users(&self) -> Result<i64, sqlx::Error> {
        self.test_db.count_test_users().await
    }

    /// Check if a user exists by email
    #[allow(dead_code)] // Actually used in user_services_tests.rs and user_queries_tests.rs, clippy false positive
    pub async fn user_exists(&self, email: &str) -> Result<bool, sqlx::Error> {
        self.test_db.user_exists(email).await
    }

    /// Get user password hash for testing
    #[allow(dead_code)]
    pub async fn get_user_password_hash(&self, email: &str) -> Result<Option<String>, sqlx::Error> {
        self.test_db.get_user_password_hash(email).await
    }
}
