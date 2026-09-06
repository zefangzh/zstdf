use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum StdfError {
    #[error("I/O error: {0}")]
    Io(String), // wrapped so StdfError can be PartialEq in tests

    #[error(
        "{record} @ file+{file_offset:#010x}: \
         need {needed}B at body+{local_offset}, only {available} remain"
    )]
    Truncated {
        record:       &'static str,
        file_offset:  u64,
        local_offset: usize,
        needed:       usize,
        available:    usize,
    },

    #[error(
        "{record} @ file+{file_offset:#010x}: \
         declared body={declared}B but consumed {consumed}B"
    )]
    LengthMismatch {
        record:      &'static str,
        file_offset: u64,
        declared:    u16,
        consumed:    usize,
    },

    #[error("typ={typ} sub={sub} — unknown record (this is handled as Unknown variant, not an error)")]
    _UnknownRecord { typ: u8, sub: u8 },

    #[error("FAR must be first record; got typ={typ} sub={sub}")]
    MissingFar { typ: u8, sub: u8 },

    #[error("FAR CPU_TYP={0}: expected 1 (big-endian) or 2 (little-endian)")]
    InvalidCpuType(u8),

    #[error("STDF_VER={0}: only version 4 is supported")]
    UnsupportedVersion(u8),

    #[error(
        "{record} @ file+{file_offset:#010x} body+{local_offset}: \
         Cn field contains invalid UTF-8"
    )]
    InvalidString {
        record:       &'static str,
        file_offset:  u64,
        local_offset: usize,
    },

    #[error("Unexpected end of file reading record header")]
    UnexpectedEof,
}

impl From<std::io::Error> for StdfError {
    fn from(e: std::io::Error) -> Self {
        StdfError::Io(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, StdfError>;
