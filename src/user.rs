use anyhow::Context;
use ring::{digest, pbkdf2};
use rocket_contrib::databases::rusqlite::{self, params};
use serde::Deserialize;
use std::num::NonZeroU32;
use thiserror::Error;

use crate::auth::create_jwt;

static PBKDF2_ALG: pbkdf2::Algorithm = pbkdf2::PBKDF2_HMAC_SHA256;
const CREDENTIAL_LEN: usize = digest::SHA256_OUTPUT_LEN;
pub type HashedCredential = [u8; CREDENTIAL_LEN];

#[derive(Debug, Deserialize)]
pub struct Credentials {
    username: String,
    password: String,
}

/// A simple user database config.
pub struct UserDataBaseConfig {
    pub pbkdf2_iterations: NonZeroU32,
    pub db_salt_component: Vec<u8>,
}

#[derive(Error, Debug)]
/// An error while trying to verify a user's login.
pub enum VerifyError {
    #[error("Incorrect username or password")]
    IncorrectUsernameOrPassword,
}

struct User {
    id: i32,
    /// In base64.
    password: String,
}

/// Creates a new user and stores it given a set of [`Credentials`].
pub fn add_user(
    credentials: Credentials,
    connection: &rusqlite::Connection,
    config: &UserDataBaseConfig,
) -> anyhow::Result<()> {
    let users = get_users_by_credentials(&credentials, connection)?;

    if !users.is_empty() {
        Err(anyhow::format_err!("User already exists!"))
    } else {
        store_credentials(credentials, connection, config)
            .context("Failed to store credentials.")?;
        Ok(())
    }
}

/// Stores a set of [`Credentials`].
fn store_credentials(
    credentials: Credentials,
    connection: &rusqlite::Connection,
    config: &UserDataBaseConfig,
) -> anyhow::Result<()> {
    let salt = salt(credentials.username.as_str(), config);
    let mut hashed_credential: HashedCredential = [0u8; CREDENTIAL_LEN];
    pbkdf2::derive(
        PBKDF2_ALG,
        config.pbkdf2_iterations,
        &salt,
        credentials.password.as_bytes(),
        &mut hashed_credential,
    );

    connection.execute(
        "INSERT INTO users (username, password) VALUES (?1, ?2)",
        params![credentials.username, base64::encode(hashed_credential)],
    )?;

    Ok(())
}

/// Verifies a user given a set of [`Credentials`].
pub fn verify_user(
    credentials: Credentials,
    connection: &rusqlite::Connection,
    config: &UserDataBaseConfig,
) -> anyhow::Result<String> {
    let attempt_pw = &credentials.password;

    let users = get_users_by_credentials(&credentials, connection)?;

    if users.len() == 1 {
        if let Some(Ok(user)) = users.get(0) {
            if let Ok(actual_pw_hash) = base64::decode(user.password.clone()) {
                let salt = salt(credentials.username.as_str(), config);
                pbkdf2::verify(
                    PBKDF2_ALG,
                    config.pbkdf2_iterations,
                    &salt,
                    attempt_pw.as_bytes(),
                    &actual_pw_hash,
                )
                .map_err(|_| VerifyError::IncorrectUsernameOrPassword)?;
            }

            return Ok(create_jwt(user.id.to_string().as_str())?);
        }
    }

    // Return an error otherwise.
    Err(VerifyError::IncorrectUsernameOrPassword)?
}

/// Returns a salt given a username.
fn salt(username: &str, config: &UserDataBaseConfig) -> Vec<u8> {
    let mut salt = Vec::with_capacity(config.db_salt_component.len() + username.as_bytes().len());
    salt.extend(config.db_salt_component.clone());
    salt.extend(username.as_bytes());
    salt
}

fn get_users_by_credentials(
    credentials: &Credentials,
    connection: &rusqlite::Connection,
) -> anyhow::Result<Vec<rusqlite::Result<User>>> {
    let mut query = connection
        .prepare(format!("SELECT id, password FROM users WHERE username = (?1)").as_str())?;

    let iter = query
        .query_map(params![credentials.username], |row| {
            Ok(User {
                id: row.get(0)?,
                password: row.get(1)?,
            })
        })?
        .collect::<Vec<_>>();

    Ok(iter)
}
