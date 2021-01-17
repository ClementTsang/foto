use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rocket::http::hyper::header::AUTHORIZATION;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::consts;

#[derive(Debug, Deserialize, Serialize)]
struct Claims {
    sub: String,
    exp: u64,
}

#[derive(Error, Debug)]
/// An error while trying to authorize a user.
pub enum AuthError {
    #[error("Could not find authorization header")]
    NoAuthHeader,
    #[error("Invalid auth header format")]
    InvalidAuthHeader,
}

pub struct UserId {
    pub username: String,
}

const BEARER: &str = "Bearer ";

/// Creates a JWT given a UID.
pub fn create_jwt(username: &str) -> anyhow::Result<String> {
    let expiration_time = Utc::now()
        .checked_add_signed(chrono::Duration::minutes(60))
        .ok_or(anyhow::format_err!("Could not add time to JWT timestamp."))?
        .timestamp();

    let claims = Claims {
        sub: username.to_owned(),
        exp: expiration_time as u64,
    };

    let header = Header::new(Algorithm::HS512);
    encode(
        &header,
        &claims,
        &EncodingKey::from_secret(&consts::JWT_SECRET),
    )
    .map_err(|_| anyhow::format_err!("Could not encode JWT"))
}

#[rocket::async_trait]
impl<'a, 'r> FromRequest<'a, 'r> for UserId {
    type Error = AuthError;

    async fn from_request(req: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        let uid_result = authorize(req.headers());

        match uid_result {
            Ok(username) => Outcome::Success(UserId { username }),
            Err(err) => Outcome::Failure((Status::BadRequest, err)),
        }
    }
}

fn authorize(headers: &rocket::http::HeaderMap) -> Result<String, AuthError> {
    let jwt = get_jwt(headers)?;

    let decoded_jwt = decode::<Claims>(
        &jwt,
        &DecodingKey::from_secret(&consts::JWT_SECRET),
        &Validation::new(Algorithm::HS512),
    )
    .map_err(|_| AuthError::InvalidAuthHeader)?;

    Ok(decoded_jwt.claims.sub)
}

fn get_jwt(headers: &rocket::http::HeaderMap) -> Result<String, AuthError> {
    let auth_header = headers
        .get_one(AUTHORIZATION.as_str())
        .ok_or(AuthError::NoAuthHeader)?;

    if !auth_header.starts_with(BEARER) {
        Err(AuthError::InvalidAuthHeader)
    } else {
        Ok(auth_header.trim_start_matches(BEARER).to_owned())
    }
}
