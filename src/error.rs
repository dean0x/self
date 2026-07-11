use std::{fmt, io};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Json(serde_json::Error),
    HomeNotSet,
    NoSelfDir,
    MalformedMarkers(String),
    InvalidJson(String),
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::Json(e) => write!(f, "JSON error: {e}"),
            Error::HomeNotSet => write!(
                f,
                "HOME environment variable is not set — cannot determine home directory"
            ),
            Error::NoSelfDir => write!(
                f,
                "~/.self does not exist or is not a git repository — run `self init` first"
            ),
            Error::MalformedMarkers(msg) => {
                write!(f, "malformed self markers in CLAUDE.md: {msg}")
            }
            Error::InvalidJson(msg) => write!(f, "invalid JSON: {msg}"),
            Error::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
