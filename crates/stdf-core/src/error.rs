use std::fmt;

/// All the ways decoding an STDF byte stream can fail.
///
/// Kept as plain data (no external error-handling crate) so this stays
/// a zero-dependency core crate - callers in Rust or via PyO3 can map
/// this to whatever error type fits their layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StdfError {
    /// Fewer bytes were available than the format requires at this point.
    UnexpectedEof { needed: usize, available: usize },
    /// The first record in the file was not a valid FAR (File Attributes
    /// Record) - every valid STDF file must begin with one.
    InvalidFar,
    /// CPU_TYPE values outside the v0.1 support matrix are rejected
    /// explicitly rather than silently mis-decoded.
    UnsupportedCpuType(u8),
    /// A record type/subtype pair this decoder doesn't yet implement.
    /// Carried as data, not a hard error, so callers can choose to skip it.
    UnknownRecordType { rec_typ: u8, rec_sub: u8 },
    /// A record's header declared a length longer than the remaining bytes.
    /// Common at the end of a file left behind by a crashed test program.
    Truncated,
}

impl fmt::Display for StdfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StdfError::UnexpectedEof { needed, available } => write!(
                f,
                "unexpected end of file: needed {needed} bytes, only {available} available"
            ),
            StdfError::InvalidFar => {
                write!(f, "invalid or missing FAR record at start of file")
            }
            StdfError::UnsupportedCpuType(cpu_type) => {
                write!(f, "unsupported FAR CPU_TYPE {cpu_type}")
            }
            StdfError::UnknownRecordType { rec_typ, rec_sub } => {
                write!(f, "unknown record type {rec_typ}/{rec_sub}")
            }
            StdfError::Truncated => {
                write!(f, "record truncated before its declared length")
            }
        }
    }
}

impl std::error::Error for StdfError {}
