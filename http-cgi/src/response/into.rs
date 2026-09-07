use bytes::Bytes;
use http::StatusCode;
use http::header::CONTENT_TYPE;

use crate::response::Response;

pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl IntoResponse for () {
    fn into_response(self) -> Response {
        http::Response::builder().status(StatusCode::NO_CONTENT).body(Bytes::new()).unwrap()
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Response {
        http::Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "text/plain")
            .body(Bytes::from(self))
            .unwrap()
    }
}

impl IntoResponse for &str {
    fn into_response(self) -> Response {
        self.to_owned().into_response()
    }
}

impl<T> IntoResponse for (http::StatusCode, T)
where
    T: IntoResponse,
{
    fn into_response(self) -> Response {
        let (status, body) = self;
        let mut res = body.into_response();
        *res.status_mut() = status;
        res
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
