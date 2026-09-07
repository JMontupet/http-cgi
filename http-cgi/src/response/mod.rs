mod cgi;
mod into;

pub use cgi::*;
pub use into::*;

pub type Response = http::Response<bytes::Bytes>;
