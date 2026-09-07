use std::convert::Infallible;

use http::{HeaderMap, HeaderName};

use crate::response::{IntoResponse, Parts};

pub trait IntoResponseParts {
    type Error: IntoResponse;

    fn into_response_parts(self, res: Parts) -> Result<Parts, Self::Error>;
}

impl IntoResponseParts for HeaderMap {
    type Error = Infallible;

    fn into_response_parts(self, mut res: Parts) -> Result<Parts, Self::Error> {
        let mut last_key: Option<HeaderName> = None;
        for (k, v) in self {
            if let Some(k) = k {
                last_key = Some(k.clone());
                res.headers.insert(k, v);
            } else if let Some(k) = &last_key {
                res.headers.append(k.clone(), v);
            }
        }
        Ok(res)
    }
}

impl IntoResponseParts for http::StatusCode {
    type Error = Infallible;

    fn into_response_parts(self, mut res: Parts) -> Result<Parts, Self::Error> {
        res.status = self;
        Ok(res)
    }
}
