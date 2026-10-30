//! Single error type used across the crate.

use std::{fmt, io};

#[derive(Debug)]
pub enum Error {
    /// Bad command line.
    Usage(String),
    /// Filesystem error with context.
    Io { context: String, source: io::Error },
    /// A Win32 API call failed.
    Os { context: &'static str, code: u32 },
    /// Waited too long for something (game process, ...).
    Timeout(String),
    /// The game exited before we were done. Not reported to the user.
    ProcessExited(String),
    /// Memory signature not found / did not resolve.
    PatternNotFound(String),
    /// Data was present but malformed.
    InvalidData(String),
    /// A file / registry key / value that must exist does not.
    NotFound(String),
    Sqlite(rusqlite::Error),
    Json(serde_json::Error),
    /// Operation not available on this platform.
    Unsupported(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn io(context: impl Into<String>, source: io::Error) -> Self {
        Error::Io { context: context.into(), source }
    }

    /// Process exit code. Stable, documented in docs/GAME_SUPPORT.md.
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::Usage(_) => 2,
            Error::NotFound(_) | Error::Timeout(_) => 3,
            Error::PatternNotFound(_) => 4,
            Error::ProcessExited(_) => 5,
            _ => 1,
        }
    }

    /// Errors that are expected during normal shutdown and need no dialog.
    pub fn is_silent(&self) -> bool {
        matches!(self, Error::ProcessExited(_))
    }

    pub fn user_message(&self) -> String {
        match self {
            Error::Usage(_) => self.to_string(),
            _ => format!("{self}\n\nThe game will start without an unlocked FPS."),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { context, source } => write!(f, "{context}: {source}"),
            Error::Os { context, code } => write!(f, "{context} failed (Win32 error {code})"),
            Error::Sqlite(e) => write!(f, "database error: {e}"),
            Error::Json(e) => write!(f, "JSON error: {e}"),
            Error::Usage(m)
            | Error::Timeout(m)
            | Error::ProcessExited(m)
            | Error::PatternNotFound(m)
            | Error::InvalidData(m)
            | Error::NotFound(m) => write!(f, "{m}"),
            Error::Unsupported(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::Sqlite(e) => Some(e),
            Error::Json(e) => Some(e),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Sqlite(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}
