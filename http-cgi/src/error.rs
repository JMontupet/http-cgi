use std::borrow::Cow;

use bytes::Bytes;
use http::StatusCode;
use http::header::{CONTENT_TYPE, HeaderValue};
use thiserror::Error;

use crate::response::{IntoResponse, Response};

/// A convenience type alias for results that return an [`crate::Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur while handling a CGI request or response.
///
/// Errors caused by the client (malformed CGI environment, invalid body) use
/// the `400`/`413` variants. Errors caused by the server or the process
/// environment use the `500` variants and are not exposed to the client.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum Error {
    /// The `SERVER_PROTOCOL` value is not a supported HTTP version.
    #[error("Invalid server protocol: {0}")]
    InvalidServerProtocol(String),

    /// The `CONTENT_LENGTH` value is not a valid length.
    #[error("Invalid content length: {0}")]
    InvalidContentLength(String),

    /// The request could not be built from the CGI environment.
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// The request body exceeds the maximum accepted size.
    #[error("Payload too large")]
    PayloadTooLarge,

    /// An unexpected internal error occurred.
    #[error("Internal server error")]
    Internal,

    /// An I/O error occurred.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// A value could not be serialized to JSON.
    #[cfg(feature = "json")]
    #[error("JSON error: {0}")]
    Json(serde_json::Error),
}

impl Error {
    /// Returns the HTTP status code that should be sent to the client.
    pub fn status(&self) -> StatusCode {
        match self {
            Error::InvalidServerProtocol(_)
            | Error::InvalidContentLength(_)
            | Error::BadRequest(_) => StatusCode::BAD_REQUEST,
            Error::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Error::Internal | Error::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            #[cfg(feature = "json")]
            Error::Json(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Returns the message that is safe to send to the client.
    ///
    /// Client errors expose their context; server errors are replaced with a
    /// generic message so internal details never leak into the response.
    pub fn public_message(&self) -> Cow<'static, str> {
        if self.status().is_client_error() {
            Cow::Owned(self.to_string())
        } else {
            Cow::Borrowed("Internal Server Error")
        }
    }

    /// Converts the error into an HTTP response.
    ///
    /// Server errors are also written to stderr (the CGI error log) with their
    /// full debug representation.
    pub fn into_response(self) -> Response {
        self.respond()
    }

    fn respond(self) -> Response {
        let status = self.status();
        if status.is_server_error() {
            eprintln!("http-cgi: {self:?}");
        }

        let mut response = Response::new(Bytes::from(self.public_message().into_owned()));
        *response.status_mut() = status;
        response
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=utf-8"));
        response
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        self.respond()
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

    use super::*;

    fn body(response: Response) -> String {
        String::from_utf8(response.body().to_vec()).unwrap()
    }

    #[test]
    fn status_map_classifies_client_and_server_errors() {
        assert_eq!(
            Error::InvalidServerProtocol("HTTP/9.9".to_owned()).status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            Error::InvalidContentLength("nope".to_owned()).status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(Error::BadRequest("missing field".to_owned()).status(), StatusCode::BAD_REQUEST);
        assert_eq!(Error::PayloadTooLarge.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(Error::Internal.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            Error::Io(std::io::Error::other("disk")).status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        #[cfg(feature = "json")]
        assert_eq!(
            Error::Json(serde_json::from_str::<i32>("nope").unwrap_err()).status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn client_errors_expose_their_message() {
        let response = Error::BadRequest("name must not be empty".to_owned()).into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body(response), "Bad request: name must not be empty");

        let response = Error::InvalidServerProtocol("HTTP/9.9".to_owned()).into_response();
        assert_eq!(body(response), "Invalid server protocol: HTTP/9.9");

        let response = Error::InvalidContentLength("nope".to_owned()).into_response();
        assert_eq!(body(response), "Invalid content length: nope");
    }

    #[test]
    fn payload_too_large_is_a_client_error() {
        let response = Error::PayloadTooLarge.into_response();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(body(response), "Payload too large");
    }

    #[test]
    fn server_errors_are_generic() {
        for error in [Error::Internal, Error::Io(std::io::Error::other("disk"))] {
            let response = error.into_response();
            assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body(response), "Internal Server Error");
        }
    }

    #[cfg(feature = "json")]
    #[test]
    fn json_errors_are_generic() {
        let error = Error::Json(serde_json::from_str::<i32>("nope").unwrap_err());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body(response), "Internal Server Error");
    }

    #[test]
    fn responses_are_plain_text() {
        let response = Error::BadRequest("oops".to_owned()).into_response();
        assert_eq!(
            response.headers()[CONTENT_TYPE],
            HeaderValue::from_static("text/plain; charset=utf-8")
        );
    }

    #[test]
    fn trait_and_inherent_conversions_agree() {
        let inherent = Error::PayloadTooLarge.into_response();
        let through_trait = IntoResponse::into_response(Error::PayloadTooLarge);
        assert_eq!(inherent.status(), through_trait.status());
        assert_eq!(inherent.body(), through_trait.body());
    }
}
