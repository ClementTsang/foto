#[macro_use]
extern crate rocket;

#[macro_use]
extern crate rocket_contrib;

#[cfg(test)]
mod test;

mod auth;
mod config;
mod consts;
mod images;
mod response;
mod search;
mod upload;
mod user;

use images::Image;
use rocket::{http::Status, State};
use rocket_contrib::json::Json;

use consts::*;
use response::*;
use sled_extensions::{bincode::Tree, DbExt};
use user::*;

#[post("/api/0/register", format = "json", data = "<credentials>")]
fn register(db: State<Database>, credentials: Json<Credentials>) -> ApiResponse {
    match add_user(credentials.0, &USER_DATABASE_CONFIG, &db) {
        Ok(_) => ApiResponse {
            json: json!({
                "message": "Successfully created a new user"
            }),
            status: Status::Ok,
        },
        Err(_err) => ApiResponse {
            json: json!({
                "message": "Could not create a new user, please try again" // TODO: Make this more descriptive.
            }),
            status: Status::BadRequest,
        },
    }
}

#[post("/api/0/login", format = "json", data = "<credentials>")]
fn login(db: State<Database>, credentials: Json<Credentials>) -> ApiResponse {
    match verify_user(credentials.0, &USER_DATABASE_CONFIG, &db) {
        Ok(token) => ApiResponse {
            json: json!({
                "message": "Successfully logged in",
                "token": token
            }),
            status: Status::Ok,
        },
        Err(_err) => ApiResponse {
            json: json!({
                "message": "Wrong username or password, please try again"
            }),
            status: Status::BadRequest,
        },
    }
}

#[launch]
fn rocket() -> rocket::Rocket {
    let db = sled_extensions::Config::default()
        .path("./sled_data")
        .open()
        .expect("Failed to open sled db");

    rocket::ignite()
        .mount(
            "/",
            routes![search::search, upload::upload, register, login],
        )
        .manage(Database {
            users: db.open_bincode_tree("users").unwrap(),
            images: db.open_bincode_tree("images").unwrap(),
        })
}

pub struct Database {
    users: Tree<User>,
    images: Tree<Image>,
}
