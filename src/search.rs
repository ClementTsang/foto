use multer::{Constraints, Multipart, SizeLimit};
use rocket::{data::ByteUnit, request::Request};
use rocket::{
    response::{Responder, Response},
    State,
};

use rocket::{data::ToByteUnit, http::Status};
use rocket::{http::ContentType, Data};
use rocket_contrib::json::JsonValue;
use thiserror::Error;

use crate::{consts::HAMMING_DISTANCE, images::*, upload::Boundary, Database};

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Failed to parse field")]
    ParseError(#[from] ImageUploadTypeError),
    #[error("Failed to read multipart form properly")]
    MultipartError(#[from] multer::Error),
    #[error("Missing fields")]
    MissingFields,
    #[error("Failed to search for image")]
    FailedToSearch(String),
}

#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for SearchError {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'static> {
        println!("Error while searching: {:?}", self);

        Response::build_from(
            json!({
                "message": "Failed to search for image"
            })
            .respond_to(&req)
            .unwrap(),
        )
        .status(Status::InternalServerError)
        .header(ContentType::JSON)
        .ok()
    }
}

#[post("/api/0/search", format = "multipart/form-data", data = "<data>")]
pub async fn search(
    db: State<'_, Database>,
    data: Data,
    boundary: Boundary,
) -> Result<JsonValue, SearchError> {
    use futures::stream::once;

    let limit: ByteUnit = 15.mebibytes();
    let constraints = Constraints::new()
        .allowed_fields(vec!["similar_image", "similar_image_type"])
        .size_limit(
            SizeLimit::new()
                // Set 15mb as size limit for the whole stream body.
                .whole_stream(15 * 1024 * 1024)
                // Set 10mb as size limit for all fields.
                .per_field(10 * 1024 * 1024)
                .for_field("similar_image_type", 100),
        );

    let reader = once(async move { data.open(limit).stream_to_vec().await });
    let mut multipart = Multipart::new_with_constraints(reader, boundary.val, constraints);

    let mut image: Option<Vec<u8>> = None;
    let mut image_type: Option<ImageUploadType> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        if let Some(field_name) = field.name() {
            match field_name {
                "similar_image" => {
                    image = Some(field.bytes().await?.to_vec());
                }
                "similar_image_type" => {
                    image_type = Some(field.text().await?.parse::<ImageUploadType>()?);
                }
                _ => {}
            }
        }
    }

    // Expand with more types as needed.
    if !vec![(image.is_some() && image_type.is_some())]
        .iter()
        .any(|element| *element)
    {
        return Err(SearchError::MissingFields);
    }

    // TODO: search by tag

    let mut results: Vec<Image> = vec![];

    if let (Some(image), Some(image_type)) = (image, image_type) {
        // Search for similar images...
        let image = get_image_from_type_and_bytes(&image_type, &image)
            .await
            .map_err(|err| SearchError::FailedToSearch(err.to_string()))?;

        let hash = get_image_hash(&image);
        let correct_keys = db
            .images
            .iter()
            .keys()
            .filter_map(|key| {
                // Calculate Hamming distance...

                match key {
                    Ok(key) => {
                        let hamming_distance: u64 = hash
                            .iter()
                            .zip(key.iter())
                            .map(|(&hash_1, &hash_2)| (hash_1 ^ hash_2) as u64)
                            .sum::<u64>();

                        if hamming_distance <= *HAMMING_DISTANCE {
                            Some(key)
                        } else {
                            None
                        }
                    }
                    Err(_) => None,
                }
            })
            .collect::<Vec<_>>();

        for key in correct_keys {
            results.push(db.images.get(key).unwrap().unwrap());
        }
    }

    Ok(json!({ "results": results }))
}
