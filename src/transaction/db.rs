use uuid::Uuid;

use crate::db::Db;

use super::{Transaction, TransactionRequest};

impl Db {
    // Scope for optimization :  Instead of running 5 queries, a single sql function can be written and called from application.
    /// If the transaction fails to insufficent balance this method returns false
    pub async fn process_transaction(
        &self,
        username: &str,
        transaction_request: TransactionRequest,
    ) -> sqlx::Result<bool> {
        let mut transaction = self.pool.begin().await?;

        //lock the rows
        sqlx::query!(
            "SELECT balance FROM user_credentials WHERE username IN ($1, $2) FOR UPDATE;",
            username,
            transaction_request.to_user
        )
        .fetch_all(&mut *transaction)
        .await?;

        let balance = sqlx::query!(
            "SELECT balance FROM user_credentials WHERE username = $1",
            username
        )
        .fetch_one(&mut *transaction)
        .await?
        .balance;

        // check if balance is sufficient
        if balance < transaction_request.amount as i64 {
            transaction.rollback().await?;
            return Ok(false);
        }

        sqlx::query!(
            "UPDATE user_credentials SET balance = balance - $1 WHERE username = $2 ",
            transaction_request.amount as i64,
            username
        )
        .execute(&mut *transaction)
        .await?;

        sqlx::query!(
            "UPDATE user_credentials SET balance = balance + $1 WHERE username = $2 ",
            transaction_request.amount as i64,
            transaction_request.to_user
        )
        .execute(&mut *transaction)
        .await?;

        sqlx::query!(
            "INSERT INTO transactions(from_user, to_user, amount) VALUES($1, $2, $3)",
            username,
            transaction_request.to_user,
            transaction_request.amount
        )
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

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
