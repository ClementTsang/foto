# foto

A demo image repository prototype as per [Shopify's backend coding challenge for Summer 2021](https://docs.google.com/document/d/1ZKRywXQLZWOqVOHC4JkF3LqdpO3Llpfk_CkZPR8bjak/edit).
Mainly built using Rust, Sled, and Rocket, with many other libraries to aid in implementation.

## Features

- Supports user registration and authentication (latter uses a _very_ basic JWT setup).
- Supports uploading images and storing into an S3 Bucket.
- Supports searching for images via perceptual hashes to find similar images.

## Installation

TL;DR: Install Rust, optionally set up S3, set up your `config.json` file, clone the project, and run!

1. Install Rust. You can do so with [rustup](https://www.rust-lang.org/tools/install). You can check if everything worked by doing

   ```bash
   rustc --version
   ```

   and see if you get some output. For more detailed instructions on how to install Rust, see [the Rust language book](https://doc.rust-lang.org/book/ch01-01-installation.html) for more details.

2. Clone this repository:

   ```bash
   git clone https://github.com/ClementTsang/foto.git
   ```

3. `cd`/open the repository directory.

4. Create a file called `config.json`, with the following fields:

   ```json
   {
     "salt": "someBase64String",
     "jwtSecret": "someBase64String",
     "hammingDistance": 10,
     "s3BucketName": "yourS3BucketHere"
   }
   ```

   where:

   - `"salt"` is a random base64 string to use as your salt for hashing passwords. I generally used 16-byte strings for testing.
   - `"jwtSecret"` is a random base64 string to use as your JWT secret for logins. I generally used 16-byte strings for testing.
   - `"hammingDistance"` is some unsigned 64 bit number, representing how far of a Hamming distance you want to still consider as "similar". A smaller value means requiring more similarity to be returned. This is an optional value, if you don't include it, it defaults to 10.
   - `"s3BucketName`" is your S3 bucket name. This is optional, if not included, it will simply just not upload anything.

5. Run in a terminal:

   ```bash
   cargo run --release
   ```

   This may take a while, there are quite a few dependencies to download and build. When it's done building, you should get some output that looks like:

   ```bash
   Compiling foto v0.1.0 (/home/.../foto)
    Finished release [optimized] target(s) in 11.23s
     Running `target/release/foto`
   ```

   If you see this, then you're done!

## Usage

This backend currently supports the following endpoints (replace `http://127.0.0.1:8000` appropriately if needed):

### `/api/0/register`

Adds a user to the database. Their username and hashed + salted passwords are stored.

```http
POST http://127.0.0.1:8000/api/0/register
content-type: application/json

{
    "username": "username",
    "password": "password"
}
```

- Registers a new user.

- Will return a 400 error if one tries to create an account with a duplicate username.

### `/api/0/login`

```http
POST http://127.0.0.1:8000/api/0/login
content-type: application/json

{
    "username": "username",
    "password": "password"
}
```

- Authenticates a user, and returns a JWT token lasting 30 minutes if successful:

  ```json
  {
    "message": "Successfully logged in",
    "token": "TOKEN"
  }
  ```

- An invalid login will return a 400 error.

### `/api/0/upload`

Uploads an image, adds it to the database, and uploads it to S3 if a bucket is provided. Requires a valid JWT token. Replace `TOKEN` with the JWT token.

```http
POST http://127.0.0.1:8000/api/0/upload
Content-Type: multipart/form-data; boundary=----Boundary
Authorization: Bearer TOKEN

------Boundary
Content-Disposition: form-data; name="image"; filename="test1.jpg"
Content-Type: image/jpeg

< ./images/test1.jpg
------Boundary
Content-Disposition: form-data; name="type"

File
------Boundary
Content-Disposition: form-data; name="title"

Goose 1 (Normal)
------Boundary
Content-Disposition: form-data; name="description"

A totally normal picture of a goose.
------Boundary--
```

- The `type` field supports three values (case insensitive):

  - `url`
  - `file`
  - `base64`

  Choose the appropriate value for the file type.

- Lacking a correct JWT token will throw a 401 error:

  ```json
  {
    "message": "please include a valid JWT token"
  }
  ```

- An invalid multipart form will throw a 400 error:

  ```json
  {
    "message": "please include a valid multipart form"
  }
  ```

- A form missing either the `image` or `type` fields will throw a 500 error.

- If the image fails to be uploaded for any other reason, it will also throw a 500 error.

### `/api/0/search`

Uploads an image similarly to the `/api/0/upload` endpoint, but returns a JSON with image results that are deemed similar to the uploaded image.

Similarity is calculated using an image procedural hash and comparing hashes via Hamming distance.

```http
POST http://127.0.0.1:8000/api/0/search
Content-Type: multipart/form-data; boundary=----Boundary

------Boundary
Content-Disposition: form-data; name="similar_image"; filename="test1.jpg"
Content-Type: image/jpeg

< ./images/test1.jpg
------Boundary
Content-Disposition: form-data; name="similar_image_type"

File
------Boundary--
```

- Example of output:

  ```json
  {
    "results": [
      {
        "datetime": 1610934842,
        "description": "A totally normal picture of a goose.",
        "height": 768,
        "id": "glooeluob4j",
        "imageType": "image/jpeg",
        "imageUrl": "https:/bucket.s3.amazonaws.com/glooeluob4j.jpg",
        "tags": [],
        "title": "Goose 1 (Normal)",
        "username": "username",
        "width": 576
      },
      {
        "datetime": 1610935216,
        "description": "A totally modified picture of a goose.",
        "height": 768,
        "id": "4Uh2jVenjbY",
        "imageType": "image/jpeg",
        "imageUrl": "https:/bucket.s3.amazonaws.com/4Uh2jVenjbY.jpg",
        "tags": [],
        "title": "Goose 1 (Modified)",
        "username": "username",
        "width": 576
      }
    ]
  }
  ```

- Similar to the `/api/0/upload` endpoint, it will fail if the multipart form is incorrect, or missing fields.

## Thanks

Thanks to _all_ the library authors whose work I was able to use.
