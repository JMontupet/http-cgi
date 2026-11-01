use core::fmt::NumBuffer;
use std::io::Write;

use http::header::CONTENT_LENGTH;

use crate::error::Result;
use crate::response::Response;

pub fn write_response(response: Response, mut writer: impl Write) -> Result<()> {
    let (parts, body) = response.into_parts();

    // Build the complete response in memory first so a header or status
    // problem cannot leave a partially written response on stdout.
    let mut buffer = Vec::new();

    buffer.write_all(b"Status: ")?;
    buffer.write_all(parts.status.as_str().as_bytes())?;
    if let Some(reason) = parts.status.canonical_reason() {
        buffer.write_all(b" ")?;
        buffer.write_all(reason.as_bytes())?;
    }
    buffer.write_all(b"\r\n")?;

    // Iterating a `HeaderMap` by reference yields repeated names for repeated
    // values, so multi-valued headers (e.g. `Set-Cookie`) need no special case.
    // Content-Length is always derived from the body, never trusted from the
    // handler.
    for (name, value) in parts.headers.iter() {
        if name == CONTENT_LENGTH {
            continue;
        }
        buffer.write_all(name.as_ref())?;
        buffer.write_all(b": ")?;
        buffer.write_all(value.as_ref())?;
        buffer.write_all(b"\r\n")?;
    }

    let status = parts.status.as_u16();
    let body_forbidden = status == 204 || status == 304 || status < 200;
    if !body_forbidden {
        let mut len_buf = NumBuffer::new();
        buffer.write_all(CONTENT_LENGTH.as_ref())?;
        buffer.write_all(b": ")?;
        buffer.write_all(body.len().format_into(&mut len_buf).as_bytes())?;
        buffer.write_all(b"\r\n")?;
    }

    buffer.write_all(b"\r\n")?;

    if !body_forbidden {
        buffer.write_all(&body)?;
    }

    writer.write_all(&buffer)?;
    writer.flush()?;
    Ok(())
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
    use http::StatusCode;
    use http::header::{HeaderValue, SET_COOKIE};

    use super::*;
    use crate::error::Error;

    #[derive(Default)]
    struct RecordingWriter {
        wrote: Vec<u8>,
        fail_on_write: bool,
        fail_on_flush: bool,
    }

    impl Write for RecordingWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            if self.fail_on_write {
                return Err(std::io::Error::other("write failed"));
            }
            self.wrote.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            if self.fail_on_flush {
                return Err(std::io::Error::other("flush failed"));
            }
            Ok(())
        }
    }

    fn render(response: Response) -> String {
        let mut writer = RecordingWriter::default();
        write_response(response, &mut writer).unwrap();
        String::from_utf8(writer.wrote).unwrap()
    }

    fn body_and_headers(body: &'static [u8], status: StatusCode) -> Response {
        let mut response = Response::new(Bytes::from_static(body));
        *response.status_mut() = status;
        response
    }

    #[test]
    fn writes_status_line_and_body() {
        assert_eq!(
            render(body_and_headers(b"hello", StatusCode::OK)),
            "Status: 200 OK\r\ncontent-length: 5\r\n\r\nhello"
        );
    }

    #[test]
    fn writes_headers_in_order() {
        let mut response = body_and_headers(b"hello", StatusCode::OK);
        response.headers_mut().insert("x-test", HeaderValue::from_static("value"));

        assert_eq!(
            render(response),
            "Status: 200 OK\r\nx-test: value\r\ncontent-length: 5\r\n\r\nhello"
        );
    }

    #[test]
    fn writes_repeated_headers() {
        let mut response = body_and_headers(b"", StatusCode::OK);
        response.headers_mut().append(SET_COOKIE, HeaderValue::from_static("first=1; Path=/"));
        response.headers_mut().append(SET_COOKIE, HeaderValue::from_static("second=2; Path=/"));

        assert_eq!(
            render(response),
            "Status: 200 OK\r\nset-cookie: first=1; Path=/\r\nset-cookie: second=2; Path=/\r\ncontent-length: 0\r\n\r\n"
        );
    }

    #[test]
    fn derives_content_length_from_the_body() {
        let mut response = body_and_headers(b"hi", StatusCode::OK);
        response.headers_mut().insert(CONTENT_LENGTH, HeaderValue::from_static("999"));

        let rendered = render(response);
        assert!(rendered.contains("content-length: 2\r\n"));
        assert!(!rendered.contains("999"));
        assert_eq!(rendered.matches("content-length").count(), 1);
    }

    #[test]
    fn suppresses_the_body_for_no_content_and_not_modified() {
        assert_eq!(
            render(body_and_headers(b"ignored", StatusCode::NO_CONTENT)),
            "Status: 204 No Content\r\n\r\n"
        );
        assert_eq!(
            render(body_and_headers(b"ignored", StatusCode::NOT_MODIFIED)),
            "Status: 304 Not Modified\r\n\r\n"
        );
    }

    #[test]
    fn keeps_the_status_line_for_unknown_codes() {
        let status = StatusCode::from_u16(599).unwrap();
        assert_eq!(
            render(body_and_headers(b"", status)),
            "Status: 599\r\ncontent-length: 0\r\n\r\n"
        );
    }

    #[test]
    fn reports_write_failures_without_partial_output() {
        let mut writer = RecordingWriter { fail_on_write: true, ..RecordingWriter::default() };

        let error =
            write_response(body_and_headers(b"hello", StatusCode::OK), &mut writer).unwrap_err();

        assert!(matches!(error, Error::Io(_)));
        assert!(writer.wrote.is_empty());
    }

    #[test]
    fn reports_flush_failures() {
        let mut writer = RecordingWriter { fail_on_flush: true, ..RecordingWriter::default() };

        let error =
            write_response(body_and_headers(b"hello", StatusCode::OK), &mut writer).unwrap_err();

        assert!(matches!(error, Error::Io(_)));
    }
}
