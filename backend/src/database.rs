/// Database connection pool type
pub type DbPool = sqlx::PgPool;

/// Database connection type - supports both pool connections and transactions
/// Use `conn.as_mut()` for pool connections, `tx.as_mut()` for transactions
pub type DbConn = sqlx::PgConnection;
