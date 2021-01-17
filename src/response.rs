use rocket::http::ContentType;
use rocket::http::Status;
use rocket::response::{Responder, Response};
use rocket_contrib::json::JsonValue;

#[derive(Debug)]
pub struct ApiResponse {
    pub json: JsonValue,
    pub status: Status,
}

impl<'r> Responder<'r, 'static> for ApiResponse {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        Response::build_from(self.json.respond_to(&req).unwrap())
            .status(self.status)
            .header(ContentType::JSON)
            .ok()
    }
}
