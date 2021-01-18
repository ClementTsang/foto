use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub salt: String,
    pub jwt_secret: String,
    pub hamming_distance: Option<serde_json::Number>,
    pub s3_bucket_name: Option<String>,
}
