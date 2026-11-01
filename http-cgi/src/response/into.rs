use std::convert::Infallible;
use std::string::FromUtf8Error;

use bytes::Bytes;
use http::StatusCode;
use http::header::{CONTENT_TYPE, HeaderValue};

use crate::error::Error;
use crate::response::{IntoResponseParts, Response};

pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl<T: IntoResponseParts> IntoResponse for T {
    fn into_response(self) -> Response {
        let (parts, body) = ().into_response().into_parts();
        match self.into_response_parts(parts) {
            Ok(parts) => Response::from_parts(parts, body),
            Err(e) => e.into_response(),
        }
    }
}

impl IntoResponse for () {
    fn into_response(self) -> Response {
        let mut response = Response::new(Bytes::new());
        *response.status_mut() = StatusCode::NO_CONTENT;
        response
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

fn with_content_type(content_type: &'static str, body: Bytes) -> Response {
    let mut response = Response::new(body);
    response.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    response
}

impl IntoResponse for String {
    fn into_response(self) -> Response {
        with_content_type("text/plain", Bytes::from(self))
    }
}

impl IntoResponse for &str {
    fn into_response(self) -> Response {
        self.to_owned().into_response()
    }
}

impl IntoResponse for Vec<u8> {
    fn into_response(self) -> Response {
        with_content_type("application/octet-stream", Bytes::from(self))
    }
}

impl IntoResponse for Bytes {
    fn into_response(self) -> Response {
        with_content_type("application/octet-stream", self)
    }
}

impl IntoResponse for Infallible {
    fn into_response(self) -> Response {
        match self {}
    }
}

impl IntoResponse for FromUtf8Error {
    fn into_response(self) -> Response {
        Error::BadRequest(format!("invalid UTF-8 body: {self}")).into_response()
    }
}

impl<T, E> IntoResponse for std::result::Result<T, E>
where
    T: IntoResponse,
    E: IntoResponse,
{
    fn into_response(self) -> Response {
        match self {
            Ok(t) => t.into_response(),
            Err(e) => e.into_response(),
        }
    }
}

macro_rules! impl_into_response_tuples {
    ($( $( $P:ident )* => $R:ident ),*) => {
        $(
            impl<$R, $( $P ),*> IntoResponse for ($( $P, )* $R,)
            where
                $R: IntoResponse,
                $( $P: IntoResponseParts, )*
            {
                #[allow(non_snake_case)]
                fn into_response(self) -> Response {
                    let ($( $P, )* R,) = self;
                    let (parts, body) = R.into_response().into_parts();
                    $(
                        let parts = match $P.into_response_parts(parts) {
                            Ok(parts) => parts,
                            Err(e) => return e.into_response(),
                        };
                    )*
                    Response::from_parts(parts, body)
                }
            }
        )*
    };
}
impl_into_response_tuples! {
    P1 => R,
    P1 P2 => R,
    P1 P2 P3 => R,
    P1 P2 P3 P4 => R,
    P1 P2 P3 P4 P5 => R,
    P1 P2 P3 P4 P5 P6 => R,
    P1 P2 P3 P4 P5 P6 P7 => R,
    P1 P2 P3 P4 P5 P6 P7 P8 => R
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

    use bytes::Bytes;
    use http::header::CONTENT_TYPE;

    use super::*;
    use crate::response::Parts;

    fn body(response: &Response) -> &[u8] {
        response.body().as_ref()
    }

    struct FailingParts;

    impl IntoResponseParts for FailingParts {
        type Error = Error;

        fn into_response_parts(self, _parts: Parts) -> std::result::Result<Parts, Self::Error> {
            Err(Error::BadRequest("bad parts".to_owned()))
        }
    }

    #[test]
    fn unit_response_is_empty_no_content() {
        let response = ().into_response();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(body(&response).is_empty());
    }

    #[test]
    fn string_responses_are_plain_text() {
        for response in ["hi".into_response(), String::from("hi").into_response()] {
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(body(&response), b"hi");
            assert_eq!(response.headers()[CONTENT_TYPE], "text/plain");
        }
    }

    #[test]
    fn byte_responses_are_octets() {
        for response in
            [vec![1u8, 2, 3].into_response(), Bytes::from_static(&[1, 2, 3]).into_response()]
        {
            assert_eq!(body(&response), &[1, 2, 3]);
            assert_eq!(response.headers()[CONTENT_TYPE], "application/octet-stream");
        }
    }

    #[test]
    fn existing_responses_are_returned_unchanged() {
        let mut original = Response::new(Bytes::from_static(b"body"));
        original.headers_mut().insert("x-test", "value".parse().unwrap());

        let response = original.into_response();
        assert_eq!(body(&response), b"body");
        assert_eq!(response.headers()["x-test"], "value");
    }

    #[test]
    fn parts_errors_replace_the_response() {
        let response = FailingParts.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body(&response), b"Bad request: bad parts");
    }

    #[test]
    fn tuple_parts_errors_replace_the_response() {
        let response = (StatusCode::CREATED, FailingParts, "created").into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body(&response), b"Bad request: bad parts");
    }

    #[test]
    fn tuples_apply_status_and_headers() {
        let mut headers = http::HeaderMap::new();
        headers.insert("x-test", "value".parse().unwrap());

        let response = (StatusCode::CREATED, headers, "created").into_response();
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(response.headers()["x-test"], "value");
        assert_eq!(body(&response), b"created");
    }

    #[test]
    fn result_handlers_propagate_both_arms() {
        let ok: std::result::Result<&str, Error> = Ok("ok");
        assert_eq!(ok.into_response().status(), StatusCode::OK);

        let err: std::result::Result<&str, Error> = Err(Error::BadRequest("nope".to_owned()));
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body(&response), b"Bad request: nope");
    }

    #[test]
    fn infallible_results_never_fail() {
        let ok: std::result::Result<(), Infallible> = Ok(());
        assert_eq!(ok.into_response().status(), StatusCode::NO_CONTENT);
    }

    #[test]
    fn invalid_utf8_is_a_bad_request() {
        let error = String::from_utf8(vec![0xff]).unwrap_err();
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(body(&response).starts_with(b"Bad request: invalid UTF-8 body:"));
    }
}
