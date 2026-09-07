use core::fmt::NumBuffer;
use std::io::Write;

use http::header::CONTENT_LENGTH;

use crate::error::Result;
use crate::response::Response;

pub fn write_response(response: Response, mut writer: impl Write) -> Result<()> {
    let (parts, body) = response.into_parts();

    writer.write_all(b"Status: ")?;
    writer.write_all(parts.status.as_str().as_bytes())?;
    if let Some(reason) = parts.status.canonical_reason() {
        writer.write_all(b" ")?;
        writer.write_all(reason.as_bytes())?;
    }
    writer.write_all(b"\r\n")?;

    // Headers (Content-Length is always derived from body, never trusted from handler)
    let mut current_name: Option<http::header::HeaderName> = None;
    for (name, value) in parts.headers {
        let name = match name {
            Some(n) => {
                current_name = Some(n.clone());
                n
            }
            None => {
                current_name.clone().expect("HeaderMap iterator yielded None without prior name")
            }
        };
        if name == CONTENT_LENGTH {
            continue;
        }
        writer.write_all(name.as_ref())?;
        writer.write_all(b": ")?;
        writer.write_all(value.as_ref())?;
        writer.write_all(b"\r\n")?;
    }

    let status = parts.status.as_u16();
    let body_forbidden = status == 204 || status == 304 || status < 200;
    if !body_forbidden {
        let mut len_buf = NumBuffer::new();
        writer.write_all(CONTENT_LENGTH.as_ref())?;
        writer.write_all(b": ")?;
        writer.write_all(body.len().format_into(&mut len_buf).as_bytes())?;
        writer.write_all(b"\r\n")?;
    }

    writer.write_all(b"\r\n")?;

    if !body_forbidden {
        writer.write_all(&body)?;
    }

    writer.flush()?;
    Ok(())
}
