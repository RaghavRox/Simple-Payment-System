use anyhow::anyhow;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    RequestPartsExt,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{
    errors::{Error, ErrorKind},
    EncodingKey, Header, TokenData,
};
use serde::{Deserialize, Serialize};

use crate::{config::config, error::AppError};

pub(crate) fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);

    let argon2 = Argon2::default();

    Ok(argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .to_string())
}

pub(crate) fn validate_password(password: &str, hash: &str) -> anyhow::Result<bool> {
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

pub(crate) fn generate_token(expiry_time: i64, user_id: String) -> Result<String, Error> {
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
    pub username: String,
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
