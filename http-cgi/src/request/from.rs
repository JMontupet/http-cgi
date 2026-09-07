use std::convert::Infallible;
use std::string::FromUtf8Error;

use bytes::Bytes;

use crate::request::{FromRequestParts, Request};

pub trait FromRequest: Sized {
    type Error;
    fn from_request(req: Request) -> Result<Self, Self::Error>;
}

impl<T: FromRequestParts> FromRequest for T {
    type Error = T::Error;

    fn from_request(req: Request) -> Result<Self, Self::Error> {
        let (parts, _) = req.into_parts();
        T::from_request_parts(&parts)
    }
}

impl FromRequest for Request {
    type Error = Infallible;

    fn from_request(req: Request) -> Result<Self, Self::Error> {
        Ok(req)
    }
}

impl FromRequest for Bytes {
    type Error = Infallible;

    fn from_request(req: Request) -> Result<Self, Self::Error> {
        Ok(req.into_body())
    }
}

impl FromRequest for String {
    type Error = FromUtf8Error;

    fn from_request(req: Request) -> Result<Self, Self::Error> {
        let body_bytes = req.into_body();
        Ok(String::from_utf8(body_bytes.to_vec())?)
    }
}
