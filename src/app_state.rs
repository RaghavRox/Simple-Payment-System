use axum::extract::FromRef;
use sqlx::postgres::PgPoolOptions;

use crate::{config::config, db::Db};

#[derive(FromRef, Clone)]
pub(crate) struct AppState {
    pub db: Db,
}

impl AppState {
    pub async fn init() -> sqlx::Result<Self> {
        let pool = PgPoolOptions::new()
            .connect(&config().DATABASE_URL)
            .await
            .expect("failed to connect to Database");

        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(AppState { db: Db::init(pool) })
    }
}
