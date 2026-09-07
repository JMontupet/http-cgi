#[http_cgi::main]
fn main(body: String) -> String {
    format!("Hello, {}!", body)
}
