use bytes::Bytes;
use http::StatusCode;
use http::header::CONTENT_TYPE;
use serde::de::DeserializeOwned;

use crate::request::{FromRequest, Request};
use crate::response::{IntoResponse, Response};

pub struct Json<T>(pub T);

impl<T: serde::Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        match serde_json::to_vec(&self.0) {
            Ok(body) => http::Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, "application/json")
                .body(Bytes::from(body))
                .unwrap(),
            Err(_) => http::Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Bytes::new())
                .unwrap(),
        }
    }
}

impl<T: DeserializeOwned> FromRequest for Json<T> {
    type Error = serde_json::Error;

    fn from_request(req: Request) -> Result<Self, Self::Error> {
        let body = req.into_body();
        Ok(Json(serde_json::from_slice(&body)?))
    }
}
