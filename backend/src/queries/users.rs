use crate::{
    error::{Error, Result, ValidationErrors},
    models::users::{NewUser, User},
};
use uuid::Uuid;

use crate::DbConn;

/// Creates a new user in the database.
pub async fn create_user(conn: &mut DbConn, new_user: NewUser) -> Result<User> {
    let user = sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (email, password_hash, full_name)
        VALUES ($1, $2, $3)
        RETURNING id, email, password_hash, full_name, created_at, updated_at
        "#,
        new_user.email,
        new_user.password_hash,
        new_user.full_name
    )
    .fetch_one(conn)
    .await
    .map_err(|e| {
        let error_msg = e.to_string().to_lowercase();

        // Check for unique constraint violations
        // Generic error message to prevent user enumeration
        if error_msg.contains("unique")
            || error_msg.contains("duplicate key")
            || error_msg.contains("users_email_key") // PostgreSQL specific constraint name
        {
            Error::Validation(ValidationErrors::Single {
                field: "email".to_string(),
                message: "Registration failed. Please try again.".to_string(),
            })
        } else {
            Error::Sqlx(e)
        }
    })?;

    Ok(user)
}

/// Gets a single user by their ID. The user may not exist.
pub async fn get_user_by_id(conn: &mut DbConn, id: Uuid) -> Result<Option<User>> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, email, password_hash, full_name, created_at, updated_at
        FROM users
        WHERE id = $1
        "#,
        id,
    )
    .fetch_optional(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(user)
}

/// Gets a single user by their email address. The user may not exist.
pub async fn get_user_by_email(conn: &mut DbConn, email: &str) -> Result<Option<User>> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, email, password_hash, full_name, created_at, updated_at
        FROM users
        WHERE email = $1
        "#,
        email
    )
    .fetch_optional(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(user)
}

/// Lists all users in the database.
pub async fn list_users(conn: &mut DbConn) -> Result<Vec<User>> {
    let users = sqlx::query_as!(
        User,
        r#"
        SELECT id, email, password_hash, full_name, created_at, updated_at
        FROM users
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(users)
}

/// Updates an existing user's details.
pub async fn update_user(conn: &mut DbConn, user: &User) -> Result<User> {
    let updated_user = sqlx::query_as!(
        User,
        r#"
        UPDATE users
        SET email = $1, password_hash = $2, full_name = $3, updated_at = now()
        WHERE id = $4
        RETURNING id, email, password_hash, full_name, created_at, updated_at
        "#,
        &user.email,
        user.password_hash.as_deref(),
        user.full_name,
        user.id
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(updated_user)
}

/// Updates a user's password hash.
pub async fn update_user_password(conn: &mut DbConn, user_id: Uuid, password_hash: &str) -> Result<()> {
    let rows_affected = sqlx::query(
        r#"
        UPDATE users
        SET password_hash = $1, updated_at = now()
        WHERE id = $2
        "#,
    )
    .bind(password_hash)
    .bind(user_id)
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?
    .rows_affected();

    if rows_affected == 0 {
        return Err(Error::NotFound(format!("User with ID {} not found", user_id)));
    }

    Ok(())
}

/// Deletes a user by their ID.
pub async fn delete_user(conn: &mut DbConn, id: Uuid) -> Result<u64> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM users
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?
    .rows_affected();

    Ok(rows_affected)
}
