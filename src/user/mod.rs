mod db;

use axum::{
    extract::State,
    http,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    app_state::AppState,
    db::Db,
    error::{self, AppResult},
    utils::{generate_token, hash_password, validate_password, UserInfo},
};
use validator::Validate;

pub(super) fn get_router(app_state: AppState) -> Router {
    Router::new()
        .route("/signup", post(signup))
        .route("/login", post(login))
        .route("/whoami", get(whoami))
        .with_state(app_state)
}

#[utoipa::path(
    post,
    path = "/users/signup",
    tag = "User Management",
    request_body =  UserCredentials,
    responses(
        (status = 201, description = "User succesfully signed up"),
        (status = 409, description = "Username already exists"),
        (status = 500, description = "Internal Server Error"),
    ),
)]
async fn signup(
    State(db): State<Db>,
    Json(user_credentials): Json<UserCredentials>,
) -> AppResult<impl IntoResponse> {
    user_credentials.validate()?;

    if db
        .check_if_username_exists(&user_credentials.username)
        .await?
    {
        return Ok((http::StatusCode::CONFLICT).into_response());
    }

    db.signup_user(user_credentials.try_into()?).await?;
    Ok((http::StatusCode::CREATED).into_response())
}

#[utoipa::path(
    post,
    path = "/users/login",
    tag = "User Management",
    request_body =  UserCredentials,
    responses(
        (status = 200, description = "User succesfully logged in"),
        (status = 401, description = "Incorrect Credentials"),
        (status = 500, description = "Internal Server Error"),
    ),
)]
async fn login(
    State(db): State<Db>,
    Json(user_credentials): Json<UserCredentials>,
) -> AppResult<impl IntoResponse> {
    let hashed_password = db
        .get_hashed_password_of_user(&user_credentials.username)
        .await?;

    if !validate_password(&user_credentials.password, &hashed_password)? {
        return Err(error::AppError::Unauthorized);
    }

    Ok(generate_token(3600, user_credentials.username)?)
}

#[utoipa::path(
    get,
    path = "/users/whoami",
    tag = "User Management",
    responses(
        (status = 200, description = "User is logged in"),
        (status = 401, description = "Invalid bearer token"),
    ),
    security(
        ("USER_JWT" = [])
    )
)]
async fn whoami(UserInfo { username }: UserInfo) -> AppResult<impl IntoResponse> {
    Ok(username.into_response())
}
#[derive(Serialize, Deserialize, Validate, ToSchema, Debug)]
pub(crate) struct UserCredentials {
    #[validate(length(min = 4, max = 16))]
    username: String,
    #[validate(length(min = 8, max = 100))]
    password: String,
}

pub(crate) struct HashedUserCredentials {
    username: String,
    hashed_password: String,
}

impl TryFrom<UserCredentials> for HashedUserCredentials {
    type Error = anyhow::Error;

    fn try_from(value: UserCredentials) -> Result<Self, Self::Error> {
        Ok(HashedUserCredentials {
            username: value.username,
            hashed_password: hash_password(&value.password)?,
        })
    }
}
