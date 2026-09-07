mod cgi;
mod into;
mod into_parts;

pub use cgi::*;
pub use into::*;
pub use into_parts::*;

pub type Response = http::Response<bytes::Bytes>;
pub type Parts = http::response::Parts;
