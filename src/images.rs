use std::{str::FromStr, time::Duration};

use anyhow::Result;
use image::DynamicImage;
use img_hash::{image, HasherConfig};
use reqwest::ClientBuilder;
use rocket_contrib::databases::rusqlite;
use serde::Serialize;
use thiserror::Error;

pub struct ImageForm {
    pub image: Vec<u8>,

    pub image_type: ImageUploadType,

    pub title: String,

    pub description: String,

    pub mime: String,
}

#[derive(Debug)]
pub enum ImageUploadType {
    Base64,
    Url,
    File,
}

#[derive(Error, Debug)]
pub enum ImageUploadTypeError {
    #[error("Failed to parse field")]
    ImageUploadTypeParseError,
}

impl FromStr for ImageUploadType {
    type Err = ImageUploadTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "base64" => Ok(ImageUploadType::Base64),
            "url" => Ok(ImageUploadType::Url),
            "file" => Ok(ImageUploadType::File),
            _ => Err(ImageUploadTypeError::ImageUploadTypeParseError),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendableImage {
    pub id: i64,
    pub image_url: String,
    pub user_id: i64,
    pub title: String,
    pub description: String,
    // pub image_type: String,
    // pub width: u32,
    // pub height: u32,
    // pub datetime: i64,
}

#[derive(Debug)]
pub struct Image {
    image_url: String,
    hash: Vec<u8>,
    user_id: i64,
    title: String,
    // tags: Vec<String>,
    description: String,
    metadata: ImageMetadata,
}

#[derive(Debug)]
pub struct ImageMetadata {
    image_type: String,
    width: u32,
    height: u32,
    /// Unix timestamp
    datetime: i64,
}

pub async fn get_image_from_type_and_bytes(
    image_type: &ImageUploadType,
    image_byes: &[u8],
) -> Result<DynamicImage> {
    match image_type {
        ImageUploadType::Base64 => {
            let decoded_image = base64::decode(std::str::from_utf8(image_byes)?)?;
            Ok(image::load_from_memory(&decoded_image)?)
        }
        ImageUploadType::Url => {
            let url = std::str::from_utf8(image_byes)?;

            println!("Downloading url: {:?}", url);

            let client = ClientBuilder::new().timeout(Duration::new(10, 0)).build()?;
            let response = client.get(url).send().await?;

            if response.status().is_success() {
                let downloaded_image = response.bytes().await?;
                Ok(image::load_from_memory(&downloaded_image)?)
            } else {
                Err(anyhow::format_err!("Could not download image..."))?
            }
        }
        ImageUploadType::File => Ok(image::load_from_memory(image_byes)?),
    }
}

pub async fn build_image_for_foto(image_form: ImageForm, user_id: i64) -> Result<Image> {
    let image = get_image_from_type_and_bytes(&image_form.image_type, &image_form.image).await;

    match image {
        Ok(image) => {
            let hash = get_image_hash(&image);

            // TODO: Upload to S3
            let image_url = "".to_string();

            let rgba16_img = image.into_rgba16();

            Ok(Image {
                image_url,
                hash,
                user_id,
                title: image_form.title,
                description: image_form.description,
                metadata: ImageMetadata {
                    image_type: image_form.mime,
                    width: rgba16_img.width(),
                    height: rgba16_img.height(),
                    datetime: chrono::Utc::now().timestamp(),
                },
            })
        }
        Err(err) => Err(err)?,
    }
}

pub fn add_image_to_db(image: Image, connection: &rusqlite::Connection) -> Result<()> {
    assert!(image.hash.len() == 8);
    connection.execute(
        "INSERT INTO images (image_url, hash1, hash2, hash3, hash4, hash5, hash6, hash7, hash8, user_id, title, description) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        rusqlite::params![
            image.image_url,
            image.hash[0],
            image.hash[1],
            image.hash[2],
            image.hash[3],
            image.hash[4],
            image.hash[5],
            image.hash[6],
            image.hash[7],
            image.user_id,
            image.title,
            image.description
        ],
    )?;

    let image_id = connection.last_insert_rowid();

    connection.execute(
        "INSERT INTO image_metadata (image_id, type, width, height, datetime) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            image_id,
            image.metadata.image_type,
            image.metadata.width,
            image.metadata.height,
            image.metadata.datetime
        ],
    )?;
    Ok(())
}

pub fn get_image_hash(image: &DynamicImage) -> Vec<u8> {
    let hasher = HasherConfig::new().to_hasher();
    let hash = hasher.hash_image(image);

    hash.as_bytes().to_vec()
}
