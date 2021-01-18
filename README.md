# foto

A demo image repository prototype as per [Shopify's backend coding challenge for Summer 2021](https://docs.google.com/document/d/1ZKRywXQLZWOqVOHC4JkF3LqdpO3Llpfk_CkZPR8bjak/edit).
Mainly built using Rust, Sled, and Rocket, with many other libraries to aid in implementation.

## Features

- Supports searching for images via perceptual hashes to find similar images.
- Supports a basic implementation of user registration and logins.

## Installation

TL;DR: Install Rust, set up S3, clone the project, and run!

1. Install Rust. You can do so with [rustup](https://www.rust-lang.org/tools/install), or your system's package manager if possible. You can check if everything worked by doing

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
     "hammingDistance": 20,
     "s3BucketName": "yourS3BucketHere"
   }
   ```

   where:

   - `"salt"` is a random base64 string to use as your salt for hashing passwords. I generally used 16-byte strings for testing.
   - `"jwtSecret"` is a random base64 string to use as your JWT secret for logins. I generally used 16-byte strings for testing.
   - `"hammingDistance"` is some unsigned 64 bit number, representing how far of a Hamming distance you want to still consider as "similar". A smaller value means requiring more similarity to be returned. This is an optional value, if you don't include it, it defaults to 20.
   - `"s3BucketName`" is your S3 bucket name. This is optional, if not included, it will simply just not upload anything.

5. Run in a terminal:

   ```bash
   cargo run --release
   ```

   This may take a while and take some resources, there are quite a few dependencies to download and build. When it's done building, you should get some output that looks like:

   ```bash
   Compiling foto v0.1.0 (/home/.../foto)
    Finished release [optimized] target(s) in 11.23s
     Running `target/release/foto`
   ```

   If you see this, then you're done!

## Usage

This backend currently supports the following endpoints:
