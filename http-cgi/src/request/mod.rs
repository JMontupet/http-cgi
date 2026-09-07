mod cgi;
mod from;
mod from_parts;

pub use cgi::*;
pub use from::*;
pub use from_parts::*;

pub type Request = http::Request<bytes::Bytes>;
pub type Parts = http::request::Parts;
