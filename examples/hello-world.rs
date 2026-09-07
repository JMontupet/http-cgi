use std::env::vars_os;
use std::error::Error;
use std::io::{stdin, stdout};

use http_cgi::request::{FromRequest, read_request};
use http_cgi::response::{IntoResponse, write_response};

fn main() -> Result<(), Box<dyn Error>> {
    let request = read_request(vars_os(), stdin().lock())?;
    let body: String = FromRequest::from_request(request)?;

    let response = format!("Hello, {body}!").into_response();
    write_response(response, stdout().lock())?;
    Ok(())
}
