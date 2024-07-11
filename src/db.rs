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

    pub async fn get_balance_of_user(&self, username: &str) -> sqlx::Result<i64> {
        sqlx::query!(
            "SELECT balance FROM user_credentials WHERE username = $1",
            username
        )
        .fetch_one(&self.pool)
        .await
        .map(|record| record.balance)
    }

    pub async fn deposit(self, username: &str, amount: i32) -> sqlx::Result<i64> {
        let mut transaction = self.pool.begin().await?;

        let balance = sqlx::query!(
            "SELECT balance FROM user_credentials WHERE username = $1",
            username
        )
        .fetch_one(&mut *transaction)
        .await?
        .balance;

        let new_balance = balance + amount as i64;

        let updated_balance = sqlx::query!(
            "UPDATE user_credentials SET balance = $1 WHERE username = $2 RETURNING balance",
            new_balance,
            username
        )
        .fetch_one(&mut *transaction)
        .await?
        .balance;

        transaction.commit().await?;

        Ok(updated_balance)
    }
}
