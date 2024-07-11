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

        let updated_receiver_balance = receiver_balance + transaction_request.amount as i64;

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
