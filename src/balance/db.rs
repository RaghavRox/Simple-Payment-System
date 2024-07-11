use crate::db::Db;

impl Db {
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
