//! Errors that can occur during the I/O operations.

use serialport::Error as SerialError;

/// Result typedef.
pub type Result<T> = std::result::Result<T, Error>;

/// Robonomics sensors errors.
#[derive(Debug, derive_more::Display, derive_more::From, Eq, PartialEq, Clone)]
pub enum Error {
    /// Too long work time (must be less than 30).
    TooLongWorkTime,
    /// Data length is 0.
    EmptyDataFrame,
    /// Checksum doesn't match.
    BadChecksum,
    /// Serial port read errors.
    ReadError(String),
}

impl From<SerialError> for Error {
    fn from(s: SerialError) -> Self {
        Self::ReadError(s.description)
    }
}

impl From<std::io::Error> for Error {
    fn from(s: std::io::Error) -> Self {
        Self::ReadError(format!("{:?}", s.raw_os_error()))
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            _ => None,
        }
    }
}
