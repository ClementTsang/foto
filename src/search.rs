use multer::{Constraints, Multipart, SizeLimit};
use rocket::response::{Responder, Response};
use rocket::{data::ByteUnit, request::Request};

use rocket::{data::ToByteUnit, http::Status};
use rocket::{http::ContentType, Data};
use rocket_contrib::{databases::rusqlite, json::JsonValue};
use rusqlite::NO_PARAMS;
use thiserror::Error;

use crate::FotoDB;
use crate::{images::*, upload::Boundary};

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
    data: Data,
    boundary: Boundary,
    conn: FotoDB,
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
    if !vec![image.is_some() && image_type.is_some()]
        .iter()
        .any(|element| *element)
    {
        return Err(SearchError::MissingFields);
    }

    let mut query = "SELECT id, image_url, user_id, title, description FROM".to_string();
    if let (Some(image), Some(image_type)) = (image, image_type) {
        // Search for similar images...
        let image = get_image_from_type_and_bytes(&image_type, &image)
            .await
            .map_err(|err| SearchError::FailedToSearch(err.to_string()))?;

        let hash = get_image_hash(&image);

        query = format!(
            "{} (SELECT id, image_url, user_id, title, description, HAMMINGDISTANCE({}, {}, {}, {}, {}, {}, {}, {}, hash1, hash2, hash3, hash4, hash5, hash6, hash7, hash8) AS distance FROM images) WHERE distance < 10",
            query, hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7]
        );
    }

    println!("Query: {:?}", query);

    // TODO: search by tag

    let results = conn
        .run(move |connection| return_search_results(query.as_str(), connection))
        .await
        .map_err(|err| SearchError::FailedToSearch(err.to_string()))?;

    Ok(json!({ "results": results }))
}

fn return_search_results(
    query: &str,
    connection: &rusqlite::Connection,
) -> anyhow::Result<Vec<SendableImage>> {
    use rusqlite::functions::FunctionFlags;

    connection
        .create_scalar_function(
            "HAMMINGDISTANCE",
            16,
            FunctionFlags::SQLITE_UTF8
                | FunctionFlags::SQLITE_DETERMINISTIC
                | FunctionFlags::SQLITE_DIRECTONLY,
            move |ctx| {
                assert_eq!(ctx.len(), 16, "called with unexpected number of arguments");

                let distance = (ctx.get_raw(0).as_i64()? ^ ctx.get_raw(08).as_i64()?)
                    + (ctx.get_raw(1).as_i64()? ^ ctx.get_raw(09).as_i64()?)
                    + (ctx.get_raw(2).as_i64()? ^ ctx.get_raw(10).as_i64()?)
                    + (ctx.get_raw(3).as_i64()? ^ ctx.get_raw(11).as_i64()?)
                    + (ctx.get_raw(4).as_i64()? ^ ctx.get_raw(12).as_i64()?)
                    + (ctx.get_raw(5).as_i64()? ^ ctx.get_raw(13).as_i64()?)
                    + (ctx.get_raw(6).as_i64()? ^ ctx.get_raw(14).as_i64()?)
                    + (ctx.get_raw(7).as_i64()? ^ ctx.get_raw(15).as_i64()?);

                println!("Hamming distance: {}", distance);

                Ok(distance)
            },
        )
        .unwrap();

    let mut statement = connection.prepare(query)?;

    // TODO: Also grab metadata.
    let images = statement.query_map(NO_PARAMS, |row| {
        Ok(SendableImage {
            id: row.get(0)?,
            image_url: row.get(1)?,
            user_id: row.get(2)?,
            title: row.get(3)?,
            description: row.get(4)?,
        })
    })?;

    Ok(images.filter_map(Result::ok).collect::<Vec<_>>())
}
