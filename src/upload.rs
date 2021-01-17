use multer::{parse_boundary, Constraints, Multipart, SizeLimit};
use rocket::{
    data::ByteUnit,
    request::{FromRequest, Outcome, Request},
};
use rocket::{
    response::{Responder, Response},
    State,
};

use rocket::{
    data::ToByteUnit,
    http::{hyper::header::CONTENT_TYPE, Status},
};
use rocket::{http::ContentType, Data};
use thiserror::Error;

use crate::images::*;
use crate::{auth::UserId, Database};

#[derive(Error, Debug)]
pub enum UploadError {
    #[error("Failed to parse field")]
    ParseError(#[from] ImageUploadTypeError),
    #[error("Failed to read multipart form properly")]
    MultipartError(#[from] multer::Error),
    #[error("Missing fields")]
    MissingFields,
    #[error("Failed to add image")]
    FailedToAdd(String),
}

#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for UploadError {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'static> {
        println!("Error while uploading: {:?}", self);

        Response::build_from(
            json!({
                "message": "Failed to upload image"
            })
            .respond_to(&req)
            .unwrap(),
        )
        .status(Status::InternalServerError)
        .header(ContentType::JSON)
        .ok()
    }
}

pub struct Boundary {
    pub val: String,
}

#[rocket::async_trait]
impl<'a, 'r> FromRequest<'a, 'r> for Boundary {
    type Error = ();

    async fn from_request(req: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        if let Some(content_type) = req.headers().get_one(CONTENT_TYPE.as_str()) {
            let boundary = parse_boundary(content_type);

            match boundary {
                Ok(val) => Outcome::Success(Boundary { val }),
                Err(_) => Outcome::Failure((Status::BadRequest, ())),
            }
        } else {
            Outcome::Failure((Status::BadRequest, ()))
        }
    }
}

#[post("/0/upload", format = "multipart/form-data", data = "<data>")]
pub async fn upload(
    db: State<'_, Database>,
    data: Data,
    boundary: Boundary,
    user_id: UserId,
) -> Result<(), UploadError> {
    use futures::stream::once;

    let limit: ByteUnit = 15.mebibytes();
    let constraints = Constraints::new()
        .allowed_fields(vec!["image", "type", "title", "description"])
        .size_limit(
            SizeLimit::new()
                // Set 15mb as size limit for the whole stream body.
                .whole_stream(15 * 1024 * 1024)
                // Set 10mb as size limit for all fields.
                .per_field(10 * 1024 * 1024)
                .for_field("image_type", 100)
                .for_field("name", 30 * 1024)
                .for_field("title", 30 * 1024)
                .for_field("description", 30 * 1024),
        );

    let reader = once(async move { data.open(limit).stream_to_vec().await });
    let mut multipart = Multipart::new_with_constraints(reader, boundary.val, constraints);

    let mut image: Option<Vec<u8>> = None;
    let mut image_type: Option<ImageUploadType> = None;
    let mut title: Option<String> = None;
    let mut description: Option<String> = None;
    let mut content_type: Option<String> = None;
    // let mut file_name: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        if let Some(field_name) = field.name() {
            match field_name {
                "image" => {
                    // file_name = field.file_name().and_then(|s| Some(s.to_string()));
                    content_type = field.content_type().and_then(|mime| Some(mime.to_string()));
                    image = Some(field.bytes().await?.to_vec());
                }
                "type" => {
                    image_type = Some(field.text().await?.parse::<ImageUploadType>()?);
                }
                "title" => {
                    title = Some(field.text().await?);
                }
                "description" => {
                    description = Some(field.text().await?);
                }
                _ => {}
            }
        }
    }

    if let (Some(image), Some(image_type)) = (image, image_type) {
        let image_form = ImageForm {
            image,
            image_type,
            title: title.unwrap_or_default(),
            description: description.unwrap_or_default(),
            mime: content_type.unwrap_or_default(),
        };

        let image = build_image_for_foto(image_form, &user_id.username)
            .await
            .map_err(|err| UploadError::FailedToAdd(err.to_string()))?;

        add_image_to_db(image, &db).map_err(|err| UploadError::FailedToAdd(err.to_string()))?;
    } else {
        return Err(UploadError::MissingFields);
    }

    Ok(())
}
