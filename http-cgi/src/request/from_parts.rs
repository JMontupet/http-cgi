use std::convert::Infallible;

use http::HeaderMap;

use crate::request::Parts;

pub trait FromRequestParts: Sized {
    type Error;
    fn from_request_parts(parts: &Parts) -> Result<Self, Self::Error>;
}

impl FromRequestParts for HeaderMap {
    type Error = Infallible;

    fn from_request_parts(parts: &Parts) -> Result<Self, Self::Error> {
        Ok(parts.headers.clone())
    }
}
