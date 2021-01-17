use std::{convert::TryInto, str::FromStr, time::Duration};

use anyhow::Result;
use image::DynamicImage;
use img_hash::{image, HasherConfig};
use reqwest::ClientBuilder;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Database;

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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    id: u64,
    image_url: String,
    #[serde(skip)]
    hash: [u8; 8],
    username: String,
    title: String,
    tags: Vec<String>,
    description: String,

    // Ideally you flatten this, but this is broken as per https://github.com/servo/bincode/issues/245
    // Instead, we'll just manually inline the data ourselves.
    // #[serde(flatten)]
    // metadata: ImageMetadata,
    image_type: String,
    width: u32,
    height: u32,
    /// Unix timestamp
    datetime: i64,
}

// #[derive(Debug, Serialize, Deserialize)]
// pub struct ImageMetadata {
//     image_type: String,
//     width: u32,
//     height: u32,
//     /// Unix timestamp
//     datetime: i64,
// }

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

pub async fn build_image_for_foto(image_form: ImageForm, username: &str) -> Result<Image> {
    let image = get_image_from_type_and_bytes(&image_form.image_type, &image_form.image).await;

    match image {
        Ok(image) => {
            let hash = get_image_hash(&image);

            // TODO: Upload to S3
            let image_url = "".to_string();

            let rgba16_img = image.into_rgba16();

            Ok(Image {
                id: 0, // FIXME
                image_url,
                hash: hash
                    .try_into()
                    .map_err(|_| anyhow::format_err!("Could not get 8 bytes from hash..."))?,
                username: username.to_string(),
                tags: vec![], // FIXME
                title: image_form.title,
                description: image_form.description,
                image_type: image_form.mime,
                width: rgba16_img.width(),
                height: rgba16_img.height(),
                datetime: chrono::Utc::now().timestamp(),
            })
        }
        Err(err) => Err(err)?,
    }
}

pub fn add_image_to_db(image: Image, db: &Database) -> Result<()> {
    let hash = image.hash.to_vec();

    let value = match db.images.get(hash.clone())? {
        Some(mut images) => {
            images.push(image);

            images
        }
        None => {
            vec![image]
        }
    };

    db.images.insert(hash, value)?;

    Ok(())
}

pub fn get_image_hash(image: &DynamicImage) -> Vec<u8> {
    let hasher = HasherConfig::new().to_hasher();
    let hash = hasher.hash_image(image);

    hash.as_bytes().to_vec()
}
