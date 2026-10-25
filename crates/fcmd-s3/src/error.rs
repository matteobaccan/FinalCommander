//! What can go wrong, said in a way a status line can show.

use fcmd_core::VPath;

pub type Result<T> = std::result::Result<T, S3Error>;

#[derive(Debug, thiserror::Error)]
pub enum S3Error {
    /// The request never got an answer: no route, no name, a handshake that failed, a timeout.
    #[error("{host}: {detail}")]
    Transport { host: String, detail: String },

    /// The service answered, and refused. `code` is its own word for it, which is worth
    /// showing: `NoSuchBucket`, `AccessDenied`, `InvalidAccessKeyId`, `SignatureDoesNotMatch`
    /// and `RequestTimeTooSkewed` each mean something quite different to whoever has to fix it.
    #[error("{code}: {message}")]
    Service {
        status: u16,
        code: String,
        message: String,
        /// The region the service says it wanted, when it says so. A bucket lives in one
        /// region and answers from no other.
        region: Option<String>,
    },

    #[error("{0}: no such object")]
    NotFound(String),

    /// No credentials, and the bucket is not public.
    #[error("{0}")]
    NoCredentials(String),

    #[error("cancelled")]
    Cancelled,

    /// Something on this side of the wire: the file an upload was to read, a thread that
    /// would not join. It is not the service's fault and the message should not blame it.
    #[error("{0}")]
    Local(String),
}

impl S3Error {
    /// Is trying again the right answer? A throttle and a gateway error say yes; a signature
    /// that does not match will not match the second time either.
    pub fn retryable(&self) -> bool {
        match self {
            S3Error::Transport { .. } => true,
            S3Error::Service { status, code, .. } => {
                *status == 429 || *status >= 500 || code == "RequestTimeout" || code == "SlowDown"
            }
            _ => false,
        }
    }

    /// The region the service asked for, if the failure was about that.
    pub fn wanted_region(&self) -> Option<&str> {
        match self {
            S3Error::Service { region, .. } => region.as_deref(),
            _ => None,
        }
    }

    /// Carry it into the VFS layer, where the panel will show it against the path.
    pub fn into_core(self, path: &VPath) -> fcmd_core::Error {
        match self {
            S3Error::Cancelled => fcmd_core::Error::Cancelled,
            other => fcmd_core::Error::Other(format!("{path}: {other}")),
        }
    }
}
