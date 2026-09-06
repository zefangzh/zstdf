use thiserror::Error;

#[derive(Error, Debug)]
pub enum IoError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("STDF decode error: {0}")]
    Decode(#[from] stdf_core::StdfError),

    #[error("not a valid STDF file: cannot detect byte order from FAR")]
    InvalidFile,

    #[error("gzip decompression error: {0}")]
    Gzip(String),
}

pub type IoResult<T> = std::result::Result<T, IoError>;
