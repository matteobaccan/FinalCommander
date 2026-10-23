use std::io;

use crate::vpath::VPath;

pub type Result<T> = std::result::Result<T, Error>;

/// Every error carries the path it happened on, so the UI never has to guess.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{path}: {source}")]
    Io {
        path: VPath,
        #[source]
        source: io::Error,
    },

    #[error("{path}: not a directory")]
    NotADirectory { path: VPath },

    #[error("{path}: operation not supported by this filesystem")]
    Unsupported { path: VPath },

    #[error("cancelled")]
    Cancelled,

    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn io(path: &VPath, source: io::Error) -> Self {
        Error::Io {
            path: path.clone(),
            source,
        }
    }

    /// The OS error code, when there is one. Used by the job engine to decide whether an
    /// error is retryable.
    pub fn os_code(&self) -> Option<i32> {
        match self {
            Error::Io { source, .. } => source.raw_os_error(),
            _ => None,
        }
    }
}
