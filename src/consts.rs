use once_cell::sync::Lazy;

use crate::user::UserDataBaseConfig;

static CONFIG: Lazy<crate::config::Config> = Lazy::new(|| {
    let config: crate::config::Config = serde_json::from_str(
        std::fs::read_to_string("./config.json")
            .expect("Could not find config.json file.")
            .as_str(),
    )
    .expect("Could not parse config.json file.");

    config
});

pub static USER_DATABASE_CONFIG: Lazy<UserDataBaseConfig> = Lazy::new(|| UserDataBaseConfig {
    pbkdf2_iterations: std::num::NonZeroU32::new(100_000).unwrap(),
    db_salt_component: base64::decode(CONFIG.salt.clone()).unwrap(),
});

pub static JWT_SECRET: Lazy<Vec<u8>> =
    Lazy::new(|| base64::decode(CONFIG.jwt_secret.clone()).unwrap());

/// Defaults to 20.
pub static HAMMING_DISTANCE: Lazy<u64> = Lazy::new(|| {
    if let Some(distance) = &CONFIG.hamming_distance {
        distance.as_u64().expect("Hamming distance must be a u64.")
    } else {
        20
    }
});
