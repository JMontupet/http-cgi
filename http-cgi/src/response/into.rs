use bytes::Bytes;
use http::StatusCode;
use http::header::CONTENT_TYPE;

use crate::response::{IntoResponseParts, Response};

pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl<T: IntoResponseParts> IntoResponse for T {
    fn into_response(self) -> Response {
        let (parts, body) = ().into_response().into_parts();
        let Ok(parts) = self.into_response_parts(parts) else { todo!() };
        Response::from_parts(parts, body)
    }
}

impl IntoResponse for () {
    fn into_response(self) -> Response {
        http::Response::builder().status(StatusCode::NO_CONTENT).body(Bytes::new()).unwrap()
    }
}

impl IntoResponse for ! {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, "Never").into_response()
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

impl IntoResponse for Vec<u8> {
    fn into_response(self) -> Response {
        http::Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(Bytes::from(self))
            .unwrap()
    }
}

impl IntoResponse for Bytes {
    fn into_response(self) -> Response {
        http::Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(self)
            .unwrap()
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
                        let Ok(parts) = $P.into_response_parts(parts) else { todo!() };
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
