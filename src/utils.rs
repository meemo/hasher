use std::fmt;
use std::io;

use sqlx;
use tokio::task::JoinError;

#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    ThreadPanic,
    Database(sqlx::Error),
    FileChanged,
    DiskSpace,
    DbLocked,
    Config(String),
    Download(String),
    Join(String),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        match e.kind() {
            io::ErrorKind::OutOfMemory => Error::DiskSpace,
            _ => Error::IO(e),
        }
    }
}

impl From<hasher::Error> for Error {
    fn from(e: hasher::Error) -> Self {
        match e {
            hasher::Error::Io(e) => Self::from(e),
            hasher::Error::ThreadPanic => Error::ThreadPanic,
            hasher::Error::FileChanged => Error::FileChanged,
            hasher::Error::InvalidInput(msg) => Error::Config(msg.to_string()),
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::Database(e) if e.code().as_deref() == Some("SQLITE_BUSY") => {
                Error::DbLocked
            }
            _ => Error::Database(e),
        }
    }
}

impl From<walkdir::Error> for Error {
    fn from(e: walkdir::Error) -> Self {
        if let Some(inner) = e.io_error() {
            Error::IO(io::Error::new(inner.kind(), inner.to_string()))
        } else {
            Error::IO(io::Error::new(io::ErrorKind::Other, e))
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IO(e) => write!(f, "IO error: {}", e),
            Error::ThreadPanic => write!(f, "Thread panic occurred"),
            Error::Database(e) => write!(f, "Database error: {}", e),
            Error::FileChanged => write!(f, "File was modified during reading"),
            Error::DiskSpace => write!(f, "Out of disk space"),
            Error::DbLocked => write!(f, "Database is locked"),
            Error::Config(e) => write!(f, "Configuration error: {}", e),
            Error::Download(e) => write!(f, "Download error: {}", e),
            Error::Join(e) => write!(f, "Join error: {}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::IO(e) => Some(e),
            Error::Database(e) => Some(e),
            _ => None,
        }
    }
}

impl From<JoinError> for Error {
    fn from(e: JoinError) -> Self {
        if e.is_panic() {
            Error::ThreadPanic
        } else {
            Error::Join(e.to_string())
        }
    }
}
