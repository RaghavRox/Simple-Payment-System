use axum::extract::FromRef;
use sqlx::PgPool;

#[derive(FromRef, Clone)]
pub(crate) struct Db {
    pub(crate) pool: PgPool,
}

impl Db {
    pub fn init(pool: PgPool) -> Self {
        Db { pool }
    }
}
