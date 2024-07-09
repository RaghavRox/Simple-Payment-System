use axum::extract::FromRef;
use sqlx::PgPool;

use crate::HashedUserCredentials;

#[derive(FromRef, Clone)]
pub(crate) struct Db {
    pool: PgPool,
}

impl Db {
    pub fn init(pool: PgPool) -> Self {
        Db { pool }
    }

    pub async fn signup_user(
        &self,
        hashed_user_credentials: HashedUserCredentials,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO user_credentials(username, password) VALUES($1, $2)",
            hashed_user_credentials.username,
            hashed_user_credentials.hashed_password
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_hashed_password_of_user(&self, username: &str) -> sqlx::Result<String> {
        sqlx::query!(
            "SELECT password FROM user_credentials WHERE username = $1",
            username
        )
        .fetch_one(&self.pool)
        .await
        .map(|record| record.password)
    }

    pub async fn check_if_username_exists(&self, username: &str) -> sqlx::Result<bool> {
        match sqlx::query!(
            "SELECT password FROM user_credentials WHERE username = $1",
            username
        )
        .fetch_one(&self.pool)
        .await
        {
            Err(sqlx::Error::RowNotFound) => return Ok(false),
            Err(e) => return Err(e),
            _ => (),
        }

        Ok(true)
    }
}
