use crate::rocket_from_db;

use rocket::{
    http::{ContentType, Status},
    local::blocking::Client,
};

use once_cell::sync::Lazy;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

static DATABASE: Lazy<sled_extensions::Db> = Lazy::new(|| {
    sled_extensions::Config::default()
        .path("./sled_data")
        .open()
        .expect("Failed to open sled db")
});

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

#[test]
fn test_wrong_token() {}

#[test]
fn insert_one_image() {}

#[test]
fn insert_many_images() {}

#[test]
fn search_similar_image() {}
