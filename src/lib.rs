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
use axum::{extract::State, http, response::IntoResponse, routing::post, Json, Router};
use chrono::{Duration, Utc};
use config::config;
use db::Db;
use error::AppResult;
use jsonwebtoken::{
    errors::{Error, ErrorKind},
    EncodingKey, Header, TokenData,
};
use serde::{Deserialize, Serialize};

use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;
use validator::Validate;
pub async fn get_router() -> anyhow::Result<Router> {
    //Construct App State
    let app_state = AppState::init().await?;

    let user_management_router = Router::new()
        .route("/signup", post(signup))
        .route("/login", post(login));

    Ok(Router::new()
        .nest("/users", user_management_router)
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
    if let Err(e) = user_credentials.validate() {
        return Ok((http::StatusCode::UNPROCESSABLE_ENTITY, format!("{e}")).into_response());
    }

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
