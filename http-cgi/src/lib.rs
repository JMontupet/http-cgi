//! A bridge to handle CGI environments using standard Rust `http` crate types.
//!
//! # Usage
//!
//!   ```rust,ignore
//!   #[http_cgi::main]
//!   fn main(body: Json<Payload>) -> Result<Response, Error> {
//!       if payload.name.is_empty() {
//!           return Err(Error::BadRequest("name must not be empty".to_owned()));
//!       }
//!       Ok((StatusCode::CREATED, out_headers, Json(reply)).into_response())
//!   }
//!   ```

// Failures are reported through `Result`, never through panics. The generated
// entry point catches unplanned panics at the boundary; the library itself must
// not create them.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unreachable,
    clippy::unimplemented
)]

pub mod error;
#[cfg(feature = "json")]
pub mod json;
pub mod request;
pub mod response;
pub mod run;

pub use error::{Error, Result};
pub use http;
#[cfg(feature = "macros")]
pub use http_cgi_macros::*;
#[cfg(feature = "json")]
pub use json::*;
