use axum::extract::FromRef;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{HashedUserCredentials, Transaction, TransactionRequest};

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

    // Scope for optimization :  Instead of running 4 queries, a single sql function can be written and called from application.
    /// If the transaction fails to insufficent balance this method returns false
    pub async fn process_transaction(
        &self,
        username: &str,
        transaction_request: TransactionRequest,
    ) -> sqlx::Result<bool> {
        let mut transaction = self.pool.begin().await?;

        let balance = sqlx::query!(
            "SELECT balance FROM user_credentials WHERE username = $1",
            username
        )
        .fetch_one(&mut *transaction)
        .await?
        .balance;

        let new_balance = balance - transaction_request.amount as i64;

        // check if balance is sufficient
        if new_balance < 0 {
            transaction.rollback().await?;
            return Ok(false);
        }

        let receiver_balance = sqlx::query!(
            "SELECT balance FROM user_credentials WHERE username = $1",
            transaction_request.to_user
        )
        .fetch_one(&mut *transaction)
        .await?
        .balance;

        let updated_receiver_balance = receiver_balance - transaction_request.amount as i64;

        sqlx::query!(
            "UPDATE user_credentials SET balance = $1 WHERE username = $2 ",
            new_balance,
            username
        )
        .execute(&mut *transaction)
        .await?;

        sqlx::query!(
            "UPDATE user_credentials SET balance = $1 WHERE username = $2 ",
            updated_receiver_balance,
            transaction_request.to_user
        )
        .execute(&mut *transaction)
        .await?;

        Ok(true)
    }

    pub async fn get_transaction(&self, id: Uuid) -> sqlx::Result<Transaction> {
        sqlx::query_as!(
            Transaction,
            "SELECT * FROM transactions WHERE transaction_id = $1",
            id
        )
        .fetch_one(&self.pool)
        .await
    }

    pub async fn get_transactions_list(&self, username: &str) -> sqlx::Result<Vec<Transaction>> {
        sqlx::query_as!(
            Transaction,
            "SELECT * FROM transactions WHERE from_user = $1 or to_user = $2",
            username,
            username
        )
        .fetch_all(&self.pool)
        .await
    }
}
