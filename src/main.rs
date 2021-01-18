#[macro_use]
extern crate rocket;

#[macro_use]
extern crate rocket_contrib;

#[cfg(test)]
mod test;

mod api;
mod auth;
mod config;
mod consts;
mod images;
mod page;
mod response;
mod user;

use images::Image;
use rusoto_core;
use rusoto_s3::S3Client;
use sled_extensions::{bincode::Tree, DbExt};
use user::*;

#[launch]
fn rocket() -> rocket::Rocket {
    let db = sled_extensions::Config::default()
        .path("./sled_data")
        .open()
        .expect("Failed to open sled db");

    rocket_from_db(&db)
}

/// Builds a rocket given a sled_embedded database reference.  The reason this is pulled out from the main [`rocket`] function
/// is mostly for testing purposes, as the testing client will use its own database connection across all clients.
fn rocket_from_db(db: &sled_extensions::Db) -> rocket::Rocket {
    let s3_client = S3Client::new(rusoto_core::Region::UsEast1);

    rocket::ignite()
        .mount(
            "/api/",
            routes![
                api::search::search,
                api::search::search_invalid_form,
                api::upload::upload,
                api::upload::upload_no_auth,
                api::upload::upload_invalid_form,
                api::register::register,
                api::login::login
            ],
        )
        .mount("/", routes![page::login::login])
        .manage(Database {
            users: db.open_bincode_tree("users").unwrap(),
            image_hashes: db.open_bincode_tree("image_hashes").unwrap(),
            images: db.open_bincode_tree("images").unwrap(),
        })
        .manage(s3_client)
}

pub struct Database {
    users: Tree<User>,
    image_hashes: Tree<Vec<Image>>,
    images: Tree<Image>,
}
