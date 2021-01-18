use rocket::{http::Status, State};
use rocket_contrib::json::Json;

use crate::{
    consts::USER_DATABASE_CONFIG,
    response::ApiResponse,
    user::{verify_user, Credentials},
    Database,
};

#[post("/0/login", format = "json", data = "<credentials>")]
pub fn login(db: State<Database>, credentials: Json<Credentials>) -> ApiResponse {
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
