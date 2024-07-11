mod db;

use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

use crate::{app_state::AppState, db::Db, error::AppResult, utils::UserInfo};

pub(super) fn get_router(app_state: AppState) -> Router {
    Router::new()
        .route("/", get(get_balance))
        .route("/deposit", post(deposit))
        .with_state(app_state)
}

#[derive(Deserialize, Validate, ToSchema)]
pub(crate) struct DepositAmount {
    #[validate(range(min = 1))]
    deposit_amount: i32,
}

#[utoipa::path(
    post,
    path = "/balance/deposit",
    tag = "Account Balance Management",
    request_body =  DepositAmount,
    responses(
        (status = 200, description = "Successfully deposited money", body = i64),
        (status = 401, description = "Incorrect Credentials"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("USER_JWT" = [])
    )
)]
async fn deposit(
    State(db): State<Db>,
    UserInfo { username }: UserInfo,
    Json(deposit_amount): Json<DepositAmount>,
) -> AppResult<impl IntoResponse> {
    deposit_amount.validate()?;
    Ok(db
        .deposit(&username, deposit_amount.deposit_amount)
        .await?
        .to_string())
}

#[utoipa::path(
    get,
    path = "/balance",
    tag = "Account Balance Management",
    responses(
        (status = 200, description = "Current balance of user"),
        (status = 401, description = "Invalid bearer token"),
    ),
    security(
        ("USER_JWT" = [])
    )
)]
async fn get_balance(
    State(db): State<Db>,
    UserInfo { username }: UserInfo,
) -> AppResult<impl IntoResponse> {
    Ok((db.get_balance_of_user(&username).await?.to_string()).into_response())
}
