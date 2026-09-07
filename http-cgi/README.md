# http-cgi

A bridge to handle CGI environments using standard Rust `http` crate types.

## Usage

```rust
#[http_cgi::main]
fn main(body: String) -> String {
    format!("Hello, {}!", body)
}
```
