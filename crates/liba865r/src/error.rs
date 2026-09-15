//! Summary: Defines structured errors shared by the A865R protocol, USB transport, and tools.

use std::fmt::{Display, Formatter};

/// Convenient result alias used throughout liba865r.
pub type Result<T> = std::result::Result<T, Error>;

/// Represents a recoverable driver-core failure without panicking the process.
#[derive(Debug)]
pub enum Error {
    /// The caller supplied an invalid value.
    InvalidArgument(String),
    /// A USB backend operation failed.
    Transport(String),
    /// A framed ITE-family command failed validation or returned an error status.
    Protocol(String),
    /// The requested operation requires a connected/probed device.
    NotConnected,
    /// The requested feature is intentionally not implemented for the current hardware state.
    Unsupported(String),
    /// A standard file or stream I/O operation failed.
    Io(std::io::Error),
}

impl Display for Error {
    /// Formats a concise human-readable error suitable for the CLI.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidArgument(message) => write!(formatter, "invalid argument: {message}"),
            Self::Transport(message) => write!(formatter, "USB transport error: {message}"),
            Self::Protocol(message) => write!(formatter, "protocol error: {message}"),
            Self::NotConnected => write!(formatter, "the A865R has not been connected and probed"),
            Self::Unsupported(message) => write!(formatter, "unsupported: {message}"),
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
        }
    }
}

impl std::error::Error for Error {
    /// Exposes the underlying I/O error when one exists.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    /// Converts standard I/O failures into the library's structured error type.
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
