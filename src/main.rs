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
mod search;
mod upload;
mod user;

use rocket::http::ContentType;
use rocket::http::Status;
use rocket::response::{Responder, Response};
use rocket_contrib::{
    databases::rusqlite,
    json::{Json, JsonValue},
};

use consts::*;
use user::*;

#[database("foto_db")]
pub struct FotoDB(rusqlite::Connection);

#[derive(Debug)]
struct ApiResponse {
    json: JsonValue,
    status: Status,
}

impl<'r> Responder<'r, 'static> for ApiResponse {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        Response::build_from(self.json.respond_to(&req).unwrap())
            .status(self.status)
            .header(ContentType::JSON)
            .ok()
    }
}

#[post("/api/0/register", format = "json", data = "<credentials>")]
async fn register(credentials: Json<Credentials>, conn: FotoDB) -> ApiResponse {
    match conn
        .run(move |connection| add_user(credentials.0, connection, &USER_DATABASE_CONFIG))
        .await
    {
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
async fn login(credentials: Json<Credentials>, conn: FotoDB) -> ApiResponse {
    match conn
        .run(move |connection| verify_user(credentials.0, connection, &USER_DATABASE_CONFIG))
        .await
    {
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
    // Initialize sqlite table if required.  This must succeed.
    init();

    rocket::ignite()
        .mount(
            "/",
            routes![search::search, upload::upload, register, login],
        )
        .attach(FotoDB::fairing())
}

fn init() {
    use rusqlite::{Connection, NO_PARAMS};

    let conn = Connection::open("foto.sqlite").unwrap();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL unique,
            password TEXT NOT NULL
        )",
        NO_PARAMS,
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS images (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            image_url TEXT NOT NULL,
            hash1 INTEGER NOT NULL,
            hash2 INTEGER NOT NULL,
            hash3 INTEGER NOT NULL,
            hash4 INTEGER NOT NULL,
            hash5 INTEGER NOT NULL,
            hash6 INTEGER NOT NULL,
            hash7 INTEGER NOT NULL,
            hash8 INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            title TEXT NOT NULL,
            description TEXT NOT NULL
        )",
        NO_PARAMS,
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS image_metadata (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            image_id INTEGER NOT NULL,
            type TEXT NOT NULL,
            width INTEGER NOT NULL,
            height INTEGER NOT NULL,
            datetime INTEGER NOT NULL
        )",
        NO_PARAMS,
    )
    .unwrap();

    // Close.
    conn.close().unwrap();
}
