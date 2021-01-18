use std::{convert::TryInto, str::FromStr, time::Duration};

use crate::{consts, Database};
use anyhow::Result;
use img_hash::{
    image::{self, DynamicImage, ImageFormat},
    HasherConfig,
};
use nanoid::nanoid;
use reqwest::ClientBuilder;
use rocket::http::hyper::Bytes;
use rusoto_s3::{PutObjectRequest, S3Client, S3};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub struct ImageForm {
    pub image: Vec<u8>,

    pub image_type: ImageUploadType,

    pub title: String,

    pub description: String,

    pub mime: String,

    pub image_name: String,
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
    id: String,
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
    image_bytes: &[u8],
) -> Result<(DynamicImage, ImageFormat, Option<Bytes>)> {
    match image_type {
        ImageUploadType::Base64 => {
            let decoded_image = base64::decode(std::str::from_utf8(image_bytes)?)?;
            Ok((
                image::load_from_memory(&decoded_image)?,
                image::guess_format(&decoded_image)?,
                Some(decoded_image.clone().into()),
            ))
        }
        ImageUploadType::Url => {
            let url = std::str::from_utf8(image_bytes)?;

            println!("Downloading url: {:?}", url);

            let client = ClientBuilder::new().timeout(Duration::new(10, 0)).build()?;
            let response = client.get(url).send().await?;

            if response.status().is_success() {
                let downloaded_image = response.bytes().await?;

                Ok((
                    image::load_from_memory(&downloaded_image)?,
                    image::guess_format(&downloaded_image)?,
                    Some(downloaded_image),
                ))
            } else {
                Err(anyhow::format_err!("Could not download image..."))?
            }
        }
        ImageUploadType::File => Ok((
            image::load_from_memory(image_bytes)?,
            image::guess_format(image_bytes)?,
            None,
        )),
    }
}

pub async fn build_image_for_foto(
    mut image_form: ImageForm,
    username: &str,
    s3_client: &S3Client,
) -> Result<Image> {
    let image_result =
        get_image_from_type_and_bytes(&image_form.image_type, &image_form.image).await;

    match image_result {
        Ok((image, image_type, bytes)) => {
            let id = nanoid!(11);
            if let Some(extension_str) = image_type.extensions_str().get(0) {
                image_form.image_name = format!("{}.{}", id, extension_str);
            }

            let hash = get_image_hash(&image);
            let mut image_url = String::default();

            if let Some(bucket_location) = consts::CONFIG.s3_bucket_name.clone() {
                let put_request = PutObjectRequest {
                    bucket: bucket_location.to_string(),
                    key: image_form.image_name.clone(),
                    body: Some(match bytes {
                        Some(bytes) => bytes.to_vec().into(),
                        None => image_form.image.into(),
                    }),
                    ..Default::default()
                };

                s3_client.put_object(put_request).await?;

                image_url = format!(
                    "https:/{}.s3.amazonaws.com/{}",
                    bucket_location, image_form.image_name
                );

                // let image_url = format!(
                //     "https:/s3-{}.amazonaws.com/{}/{}",
                //     s3_client
                //         .get_bucket_location(GetBucketLocationRequest {
                //             bucket: (*consts::CONFIG.s3_bucket_name).to_string(),
                //             expected_bucket_owner: None,
                //         })
                //         .await?
                //         .location_constraint
                //         .unwrap_or_default(),
                //     (*consts::CONFIG.s3_bucket_name).to_string(),
                //     image_form.image_name
                // );

                println!("Storing image at: {}", image_url);
            } else {
                println!("No S3 bucket provided, skipping upload.");
            }

            let rgba16_img = image.into_rgba16();

            Ok(Image {
                id,
                image_url,
                hash: hash
                    .try_into()
                    .map_err(|_| anyhow::format_err!("Could not get 8 bytes from hash..."))?,
                username: username.to_string(),
                tags: vec![],
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

pub fn add_image_to_db(mut image: Image, db: &Database) -> Result<()> {
    let mut id = image.id.clone();

    while db.images.contains_key(&id)? {
        id = nanoid!(11);
    }

    image.id = id.clone();

    {
        let image = image.clone();
        db.images.insert(id.as_bytes().to_vec(), image)?;
    }

    {
        db.image_hashes
            .transaction(move |tx_db| {
                let hash = image.hash.to_vec();

                let get = tx_db.get(&hash)?;
                let value = match get {
                    Ok(get) => match get {
                        Some(mut images) => {
                            images.push(image.clone());
                            images
                        }
                        None => {
                            vec![image.clone()]
                        }
                    },
                    Err(_) => {
                        vec![image.clone()]
                    }
                };

                tx_db.insert(hash.clone(), value)?.unwrap(); // Yes, unwrap is bad, but I have no idea how to process this second error...?

                Ok(Ok(()))
            })
            .map_err(|err| anyhow::format_err!("Transaction error: {:?}", err))??;
    }

    Ok(())
}

pub fn get_image_hash(image: &DynamicImage) -> Vec<u8> {
    let hasher = HasherConfig::new().to_hasher();
    let hash = hasher.hash_image(image);

    hash.as_bytes().to_vec()
}
