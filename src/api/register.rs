use rocket::{http::Status, State};
use rocket_contrib::json::Json;

use crate::{
    consts::USER_DATABASE_CONFIG,
    response::ApiResponse,
    user::{add_user, Credentials},
    Database,
};

#[post("/0/register", format = "json", data = "<credentials>")]
pub fn register(db: State<Database>, credentials: Json<Credentials>) -> ApiResponse {
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
