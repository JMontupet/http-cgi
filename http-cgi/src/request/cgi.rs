use std::ffi::OsString;
use std::io::Read;

use bytes::Bytes;
use http::request;

use crate::error::{Error, Result};
use crate::request::Request;

fn bad_request(context: &str, error: impl std::fmt::Display) -> Error {
    Error::BadRequest(format!("{context}: {error}"))
}

/// Maximum request body size accepted by [`read_request`].
///
/// Requests with a larger `CONTENT_LENGTH` are rejected with
/// [`Error::PayloadTooLarge`] before any buffer is allocated.
pub const MAX_BODY_SIZE: usize = 1024 * 1024;

pub fn read_request(
    env: impl Iterator<Item = (OsString, OsString)>,
    mut body: impl Read,
) -> Result<Request> {
    let mut builder = request::Builder::new();
    let mut content_length: Option<usize> = None;

    let mut raw_uri: Option<String> = None;
    let mut path_info = String::new();
    let mut query_string = String::new();
    let mut host_header: Option<String> = None;
    let mut server_name = String::new();
    let mut server_port = String::new();
    let mut is_https = false;

    for (key, value) in env {
        let key_str = match key.to_str() {
            Some(k) => k,
            None => continue,
        };

        match key_str {
            "REQUEST_METHOD" => {
                let method_str = value.to_string_lossy();
                let method = http::Method::from_bytes(method_str.as_bytes())
                    .map_err(|e| bad_request("invalid request method", e))?;
                builder = builder.method(method);
            }
            "REQUEST_URI" => {
                raw_uri = Some(value.to_string_lossy().into_owned());
            }
            "PATH_INFO" => {
                path_info = value.to_string_lossy().into_owned();
            }
            "QUERY_STRING" => {
                query_string = value.to_string_lossy().into_owned();
            }
            "HTTPS" => {
                let val = value.to_string_lossy();
                if val == "on" || val == "1" || val == "yes" {
                    is_https = true;
                }
            }
            "SERVER_NAME" => {
                server_name = value.to_string_lossy().into_owned();
            }
            "SERVER_PORT" => {
                server_port = value.to_string_lossy().into_owned();
            }
            "SERVER_PROTOCOL" => {
                let protocol_lossy = value.to_string_lossy();
                let version = match protocol_lossy.as_ref() {
                    "HTTP/0.9" => Ok(http::Version::HTTP_09),
                    "HTTP/1.0" => Ok(http::Version::HTTP_10),
                    "HTTP/1.1" => Ok(http::Version::HTTP_11),
                    "HTTP/2.0" => Ok(http::Version::HTTP_2),
                    "HTTP/3.0" => Ok(http::Version::HTTP_3),
                    _ => Err(Error::InvalidServerProtocol(protocol_lossy.to_string())),
                }?;
                builder = builder.version(version);
            }
            "CONTENT_LENGTH" => {
                let len_lossy = value.to_string_lossy();
                if !len_lossy.is_empty() {
                    let parsed_len = len_lossy
                        .parse()
                        .map_err(|_| Error::InvalidContentLength(len_lossy.to_string()))?;
                    content_length = Some(parsed_len);
                    builder = builder.header(
                        http::header::CONTENT_LENGTH,
                        http::header::HeaderValue::from(parsed_len),
                    );
                }
            }
            "CONTENT_TYPE" => {
                let val_lossy = value.to_string_lossy();
                if !val_lossy.is_empty() {
                    builder = builder.header(
                        http::header::CONTENT_TYPE,
                        http::header::HeaderValue::from_bytes(value.as_encoded_bytes())
                            .map_err(|e| bad_request("invalid content type", e))?,
                    );
                }
            }
            "HTTP_HOST" => {
                let host_val = value.to_string_lossy().into_owned();
                host_header = Some(host_val.clone());
                builder = builder.header(
                    http::header::HOST,
                    http::header::HeaderValue::from_bytes(value.as_encoded_bytes())
                        .map_err(|e| bad_request("invalid host header", e))?,
                );
            }
            key_str if key_str.starts_with("HTTP_") => {
                let header_name_str = key_str["HTTP_".len()..].replace('_', "-");

                builder = builder.header(
                    http::header::HeaderName::from_bytes(header_name_str.as_bytes())
                        .map_err(|e| bad_request("invalid header name", e))?,
                    http::header::HeaderValue::from_bytes(value.as_encoded_bytes())
                        .map_err(|e| bad_request("invalid header value", e))?,
                );
            }
            _ => {}
        }
    }

    let uri_string = match raw_uri {
        Some(uri) => uri,
        None => {
            let mut fallback = if path_info.is_empty() { "/".to_string() } else { path_info };
            if !query_string.is_empty() {
                fallback.push('?');
                fallback.push_str(&query_string);
            }
            fallback
        }
    };
    builder = builder.uri(uri_string);

    if host_header.is_none() && !server_name.is_empty() {
        let mut final_host = server_name;
        if ((!is_https && server_port != "80") || (is_https && server_port != "443"))
            && !server_port.is_empty()
        {
            final_host.push(':');
            final_host.push_str(&server_port);
        }
        builder = builder.header(
            http::header::HOST,
            http::header::HeaderValue::from_bytes(final_host.as_bytes())
                .map_err(|e| bad_request("invalid host header", e))?,
        );
    }

    let expected = content_length.unwrap_or_default();
    if expected > MAX_BODY_SIZE {
        return Err(Error::PayloadTooLarge);
    }

    let mut buf = vec![0u8; expected];
    if let Err(error) = body.read_exact(&mut buf) {
        return Err(match error.kind() {
            std::io::ErrorKind::UnexpectedEof => {
                Error::BadRequest(format!("incomplete request body: expected {expected} bytes"))
            }
            _ => Error::Io(error),
        });
    }

    let request = builder.body(Bytes::from(buf)).map_err(|e| bad_request("invalid request", e))?;
    Ok(request)
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

    use std::io::Cursor;

    use http::header::{CONTENT_LENGTH, CONTENT_TYPE, HOST};

    use super::*;

    fn env(pairs: &[(&str, &str)]) -> impl Iterator<Item = (OsString, OsString)> {
        pairs
            .iter()
            .map(|(key, value)| (OsString::from(key), OsString::from(value)))
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn reads_a_valid_request() {
        let request = read_request(
            env(&[
                ("REQUEST_METHOD", "POST"),
                ("REQUEST_URI", "/hello?name=world"),
                ("SERVER_PROTOCOL", "HTTP/1.1"),
                ("CONTENT_LENGTH", "5"),
                ("CONTENT_TYPE", "text/plain"),
            ]),
            Cursor::new(b"hello".to_vec()),
        )
        .unwrap();

        assert_eq!(request.method(), http::Method::POST);
        assert_eq!(request.uri().path(), "/hello");
        assert_eq!(request.uri().query(), Some("name=world"));
        assert_eq!(request.version(), http::Version::HTTP_11);
        assert_eq!(request.body().as_ref(), b"hello");
        assert_eq!(request.headers()[CONTENT_LENGTH], "5");
        assert_eq!(request.headers()[CONTENT_TYPE], "text/plain");
    }

    #[test]
    fn builds_uri_from_path_and_query_when_request_uri_is_missing() {
        let request = read_request(
            env(&[("PATH_INFO", "/fallback"), ("QUERY_STRING", "a=1")]),
            Cursor::new(Vec::new()),
        )
        .unwrap();

        assert_eq!(request.uri().path(), "/fallback");
        assert_eq!(request.uri().query(), Some("a=1"));
    }

    #[test]
    fn derives_host_header_from_server_name() {
        let request = read_request(
            env(&[
                ("SERVER_NAME", "example.test"),
                ("SERVER_PORT", "8080"),
                ("SERVER_PROTOCOL", "HTTP/1.1"),
            ]),
            Cursor::new(Vec::new()),
        )
        .unwrap();

        assert_eq!(request.headers()[HOST], "example.test:8080");
    }

    #[test]
    fn rejects_an_unknown_protocol() {
        let error = read_request(env(&[("SERVER_PROTOCOL", "HTTP/9.9")]), Cursor::new(Vec::new()))
            .unwrap_err();

        assert!(matches!(error, Error::InvalidServerProtocol(value) if value == "HTTP/9.9"));
    }

    #[test]
    fn rejects_an_invalid_content_length() {
        let error =
            read_request(env(&[("CONTENT_LENGTH", "not-a-number")]), Cursor::new(Vec::new()))
                .unwrap_err();

        assert!(matches!(error, Error::InvalidContentLength(value) if value == "not-a-number"));
    }

    #[test]
    fn rejects_an_invalid_method() {
        let error =
            read_request(env(&[("REQUEST_METHOD", "GE T")]), Cursor::new(Vec::new())).unwrap_err();

        assert!(
            matches!(error, Error::BadRequest(message) if message.contains("invalid request method"))
        );
    }

    #[test]
    fn rejects_an_invalid_uri() {
        let error =
            read_request(env(&[("REQUEST_URI", "/a b")]), Cursor::new(Vec::new())).unwrap_err();

        assert!(matches!(error, Error::BadRequest(message) if message.contains("invalid request")));
    }

    #[test]
    fn rejects_an_invalid_header_value() {
        let error = read_request(env(&[("HTTP_X_TEST", "bad\nvalue")]), Cursor::new(Vec::new()))
            .unwrap_err();

        assert!(
            matches!(error, Error::BadRequest(message) if message.contains("invalid header value"))
        );
    }

    #[test]
    fn rejects_a_truncated_body() {
        let error = read_request(env(&[("CONTENT_LENGTH", "10")]), Cursor::new(b"short".to_vec()))
            .unwrap_err();

        assert!(
            matches!(error, Error::BadRequest(message) if message.contains("incomplete request body"))
        );
    }

    #[test]
    fn rejects_an_oversized_body_before_reading() {
        let length = (MAX_BODY_SIZE + 1).to_string();
        let error = read_request(
            env(&[("CONTENT_LENGTH", length.as_str())]),
            Cursor::new(b"never read".to_vec()),
        )
        .unwrap_err();

        assert!(matches!(error, Error::PayloadTooLarge));
    }

    #[test]
    fn accepts_an_empty_body() {
        let request = read_request(env(&[]), Cursor::new(Vec::new())).unwrap();
        assert!(request.body().is_empty());
    }
}
