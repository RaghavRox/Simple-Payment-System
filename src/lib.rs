mod api_doc;
mod app_state;
mod config;
mod db;
mod error;

use anyhow::anyhow;
use api_doc::ApiDoc;
use app_state::AppState;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::{self, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, RequestPartsExt, Router,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use chrono::{Duration, Utc};
use config::config;
use db::Db;
use error::{AppError, AppResult};
use jsonwebtoken::{
    errors::{Error, ErrorKind},
    EncodingKey, Header, TokenData,
};
use serde::{Deserialize, Serialize};

use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;
use validator::Validate;
pub async fn get_router() -> anyhow::Result<Router> {
    //Construct App State
    let app_state = AppState::init().await?;

    let user_management_router = Router::new()
        .route("/signup", post(signup))
        .route("/login", post(login))
        .route("/whoami", get(whoami));

    let account_balance_router = Router::new()
        .route("/", get(get_balance))
        .route("/deposit", post(deposit));

    let transactions_router = Router::new().route("/", post(create_transaction));

    Ok(Router::new()
        .nest("/users", user_management_router)
        .nest("/transactions", transactions_router)
        .nest("/balance", account_balance_router)
        .merge(
            SwaggerUi::new("/docs")
                .url("/docs/openapi.json", ApiDoc::openapi())
                .config(
                    utoipa_swagger_ui::Config::default()
                        .doc_expansion(r#"["list"*,"full","none"]"#)
                        .request_snippets_enabled(true)
                        .persist_authorization(true),
                ),
        )
        .with_state(app_state))
}

#[derive(Serialize, Deserialize, Validate, ToSchema, Debug)]
struct UserCredentials {
    #[validate(length(min = 4, max = 16))]
    username: String,
    #[validate(length(min = 8, max = 100))]
    password: String,
}

struct HashedUserCredentials {
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

fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);

    let argon2 = Argon2::default();

    Ok(argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .to_string())
}

fn validate_password(password: &str, hash: &str) -> anyhow::Result<bool> {
    let parsed_hash = PasswordHash::new(hash).map_err(|e| anyhow!("{e}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[derive(Debug, Serialize, Deserialize)]
struct CustomClaims {
    sub: String,
    exp: i64,
}

fn encode_jwt(secret: &[u8], claims: &CustomClaims) -> Result<String, Error> {
    let header = Header::default();
    let key = EncodingKey::from_secret(secret);
    jsonwebtoken::encode(&header, claims, &key)
}

fn decode_jwt(secret: &[u8], token: &str) -> Result<TokenData<CustomClaims>, Error> {
    let key = jsonwebtoken::DecodingKey::from_secret(secret);
    let validation = Default::default();
    jsonwebtoken::decode::<CustomClaims>(token, &key, &validation)
}

fn generate_token(expiry_time: i64, user_id: String) -> Result<String, Error> {
    let secret_key = config().JWT_SECRET.as_bytes();
    let claims = CustomClaims {
        sub: user_id,
        exp: (Utc::now() + Duration::seconds(expiry_time)).timestamp(),
    };
    encode_jwt(secret_key, &claims)
}

//validate the token and also check if it is expired or not
async fn validate_token(token: &str) -> Result<String, Error> {
    let secret_key = config().JWT_SECRET.as_bytes();
    let token_data = decode_jwt(secret_key, token);
    let username = match token_data {
        Ok(data) => {
            if data.claims.exp < (Utc::now()).timestamp() {
                return Err(Error::from(ErrorKind::ExpiredSignature));
            }
            data.claims.sub
        }
        Err(e) => {
            return Err(e);
        }
    };

    Ok(username)
}

pub(crate) struct UserInfo {
    username: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for UserInfo {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;

        let username = validate_token(bearer.token())
            .await
            .map_err(|e| AppError::from(e).into_response())?;

        Ok(UserInfo { username })
    }
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

#[derive(Serialize, ToSchema, Deserialize)]
pub(crate) struct Transaction {
    transaction_id: Uuid,
    from_user: String,
    to_user: String,
    amount: i32,
    created_at: chrono::DateTime<Utc>,
}

#[derive(Deserialize, ToSchema, Validate)]
pub(crate) struct TransactionRequest {
    #[validate(length(min = 4, max = 16))]
    to_user: String,
    #[validate(range(min = 1))]
    amount: i32,
}
