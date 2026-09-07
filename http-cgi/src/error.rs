pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("InvalidServerProtocol")]
    InvalidServerProtocol,
    #[error("InvalidContentLength")]
    InvalidContentLength,
    #[error("HTTP Error")]
    Http {
        #[from]
        source: http::Error,
    },
    #[error("IO Error")]
    IO {
        #[from]
        source: std::io::Error,
    },
}
