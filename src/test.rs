use crate::rocket_from_db;

use rocket::{
    http::{ContentType, Status},
    local::blocking::Client,
};

use once_cell::sync::Lazy;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

static DATABASE: Lazy<sled_extensions::Db> = Lazy::new(|| {
    sled_extensions::Config::default()
        .path("./sled_data")
        .open()
        .expect("Failed to open sled db")
});

#[allow(dead_code)]
const TEST_USERNAME: &str = "clement_testing_testing";

#[allow(dead_code)]
const TEST_PASSWORD: &str = "bad_password123";

#[allow(dead_code)]
fn create_or_do_nothing(client: &Client, username: &str, password: &str) {
    let response = client
        .post("/api/0/register")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{ 
            "username": "{}",
            "password": "{}"
        }}"#,
            username, password
        ))
        .dispatch();

    println!("Account creation response: {}", response.status());
}

#[derive(Serialize, Deserialize)]
struct LoginResponse {
    pub message: String,
    pub token: Option<String>,
}

#[allow(dead_code)]
fn login_get_json(client: &Client, username: &str, password: &str) -> LoginResponse {
    let response = client
        .post("/api/0/login")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{ 
                "username": "{}",
                "password": "{}"
            }}"#,
            username, password
        ))
        .dispatch();

    println!("Login status: {}", response.status());

    serde_json::from_str(&response.into_string().unwrap()).unwrap()
}

#[test]
fn test_account_creation() {
    let client = Client::tracked(rocket_from_db(&DATABASE)).expect("Valid rocket instance...");

    let rand_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(15)
        .map(char::from)
        .collect();

    let response = client
        .post("/api/0/register")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{ 
                "username": "test_user_{}",
                "password": "123456789"
            }}"#,
            rand_string
        ))
        .dispatch();

    assert_eq!(response.status(), Status::Ok);
}

#[test]
fn test_dupe_account() {
    let client = Client::tracked(rocket_from_db(&DATABASE)).expect("Valid rocket instance...");

    let rand_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(15)
        .map(char::from)
        .collect();

    let response = client
        .post("/api/0/register")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{ 
                "username": "test_user_{}",
                "password": "123456789"
            }}"#,
            rand_string
        ))
        .dispatch();

    assert_eq!(response.status(), Status::Ok);

    let response = client
        .post("/api/0/register")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{ 
                "username": "test_user_{}",
                "password": "987654321"
            }}"#,
            rand_string
        ))
        .dispatch();

    assert_eq!(response.status(), Status::BadRequest);
}

#[test]
fn login() {
    let client = Client::tracked(rocket_from_db(&DATABASE)).expect("Valid rocket instance...");

    let rand_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(15)
        .map(char::from)
        .collect();

    let response = client
        .post("/api/0/register")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{ 
                "username": "test_user_{}",
                "password": "123456789"
            }}"#,
            rand_string
        ))
        .dispatch();

    assert_eq!(response.status(), Status::Ok);

    let response = client
        .post("/api/0/login")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{ 
            "username": "test_user_{}",
            "password": "123456789"
        }}"#,
            rand_string
        ))
        .dispatch();

    assert_eq!(response.status(), Status::Ok);
}
