use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub salt: String,
    pub jwt_secret: String,
}
