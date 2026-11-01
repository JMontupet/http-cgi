use bytes::Bytes;
use http::header::{CONTENT_TYPE, HeaderValue};
use serde::de::DeserializeOwned;

use crate::error::Error;
use crate::request::{FromRequest, Request};
use crate::response::{IntoResponse, Response};

pub struct Json<T>(pub T);

impl<T: serde::Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        match serde_json::to_vec(&self.0) {
            Ok(body) => {
                let mut response = Response::new(Bytes::from(body));
                response
                    .headers_mut()
                    .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                response
            }
            Err(e) => Error::Json(e).into_response(),
        }
    }
}

impl IntoResponse for serde_json::Error {
    fn into_response(self) -> Response {
        Error::BadRequest(format!("invalid JSON body: {self}")).into_response()
    }
}

impl<T: DeserializeOwned> FromRequest for Json<T> {
    type Error = serde_json::Error;

    fn from_request(req: Request) -> Result<Self, Self::Error> {
        let body = req.into_body();
        serde_json::from_slice(&body).map(Json)
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unreachable,
        clippy::unimplemented
    )]

    use http::StatusCode;
    use serde::Serialize;

    use super::*;

    #[derive(serde::Deserialize, serde::Serialize)]
    struct Payload {
        name: String,
    }

    struct Failing;

    impl Serialize for Failing {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            Err(serde::ser::Error::custom("cannot serialize"))
        }
    }

    fn body(response: &Response) -> &[u8] {
        response.body().as_ref()
    }

    #[test]
    fn serializes_json() {
        let response = Json(Payload { name: "World".to_owned() }).into_response();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_TYPE], "application/json");
        assert_eq!(body(&response), br#"{"name":"World"}"#);
    }

    #[test]
    fn encode_failure_is_a_generic_server_error() {
        let response = Json(Failing).into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body(&response), b"Internal Server Error");
    }

    #[test]
    fn decodes_json() {
        let request = Request::new(Bytes::from_static(br#"{"name":"World"}"#));
        let Json(payload) = Json::<Payload>::from_request(request).unwrap();
        assert_eq!(payload.name, "World");
    }

    #[test]
    fn decode_failure_is_a_bad_request() {
        let request = Request::new(Bytes::from_static(b"{not json}"));
        let error = Json::<Payload>::from_request(request).err().unwrap();
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(body(&response).starts_with(b"Bad request: invalid JSON body:"));
    }
}
