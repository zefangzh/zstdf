use thiserror::Error;

/// All errors produced by stdf-core.
#[derive(Error, Debug)]
pub enum StdfError {
    #[error("resource limit exceeded for {resource}: requested {requested}, limit {limit}")]
    ResourceLimit {
        resource: &'static str,
        requested: usize,
        limit: usize,
    },
    #[error("unexpected EOF at byte {position}: needed {expected} more bytes")]
    UnexpectedEof { position: usize, expected: usize },

    #[error("invalid FAR: expected (typ=0, sub=10), got ({typ}, {sub})")]
    InvalidFar { typ: u8, sub: u8 },

    #[error("unsupported STDF version {0} (only V4 supported)")]
    UnsupportedVersion(u8),

    #[error("invalid field in {record}.{field}: {msg}")]
    InvalidField {
        record: &'static str,
        field: &'static str,
        msg: String,
    },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, StdfError>;
