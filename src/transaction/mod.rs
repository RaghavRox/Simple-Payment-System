mod db;

use axum::{
    extract::{Path, State},
    http,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::{app_state::AppState, db::Db, error::AppResult, utils::UserInfo};

pub(super) fn get_router(app_state: AppState) -> Router {
    Router::new()
        .route("/", post(create_transaction))
        .route("/:id", get(get_transaction_by_id))
        .route("/", get(transactions_list))
        .with_state(app_state)
}
#[utoipa::path(
    post,
    path = "/transactions",
    tag = "Transactions",
    request_body = TransactionRequest,
    responses(
        (status = 200, description = "Transacion successfully executed"),
        (status = 402, description = "Insufficient balance"),
        (status = 401, description = "Incorrect Credentials"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("USER_JWT" = [])
    )
)]
async fn create_transaction(
    State(db): State<Db>,
    UserInfo { username }: UserInfo,
    Json(transaciton_request): Json<TransactionRequest>,
) -> AppResult<impl IntoResponse> {
    transaciton_request.validate()?;

    if !db
        .process_transaction(&username, transaciton_request)
        .await?
    {
        return Ok((
            http::StatusCode::PAYMENT_REQUIRED,
            "Insufficent balance in user account",
        )
            .into_response());
    }

    Ok(().into_response())
}

#[utoipa::path(
    get,
    path = "/transactions/{id}",
    tag = "Transactions",
    params(
        ("id" = Utoipa::Openapi::Schema::Uuid, Path, description = "Transaction id")
    ),
    responses(
        (status = 200, description = "Transacion successfully retreived", body = Transaction),
        (status = 403, description = "User is not authorized to view this transaction"),
        (status = 401, description = "Incorrect Credentials"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("USER_JWT" = [])
    )
)]
async fn get_transaction_by_id(
    State(db): State<Db>,
    UserInfo { username }: UserInfo,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let transaction = db.get_transaction(id).await?;

    if (transaction.to_user != username) && (transaction.from_user != username) {
        return Ok((
            http::StatusCode::FORBIDDEN,
            "User is not allowed to view this transaction",
        )
            .into_response());
    }

    Ok(Json(transaction).into_response())
}

///Transactions list belonging to a User
#[utoipa::path(
    get,
    path = "/transactions",
    tag = "Transactions",
    responses(
        (status = 200, description = "Transacions list  successfully retreived", body = Vec<Transaction>),
        (status = 401, description = "Incorrect Credentials"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("USER_JWT" = [])
    )
)]
async fn transactions_list(
    State(db): State<Db>,
    UserInfo { username }: UserInfo,
) -> AppResult<impl IntoResponse> {
    Ok(Json(db.get_transactions_list(&username).await?).into_response())
}

#[derive(Serialize, ToSchema, Deserialize)]
pub(crate) struct Transaction {
    pub transaction_id: Uuid,
    pub from_user: String,
    pub to_user: String,
    pub amount: i32,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Deserialize, ToSchema, Validate)]
pub(crate) struct TransactionRequest {
    #[validate(length(min = 4, max = 16))]
    pub to_user: String,
    #[validate(range(min = 1))]
    pub amount: i32,
}
