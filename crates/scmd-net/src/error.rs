use std::fmt;
use std::io;

/// Everything that can go wrong on the way to (or on) a remote filesystem.
///
/// Every variant names the host, and the ones that happen on a path name the path: a netbox
/// error that says only "connection failed" is a bug.
#[derive(Debug, thiserror::Error)]
pub enum NetError {
    #[error("{host}: {detail}")]
    Connect { host: String, detail: String },

    #[error("{user}@{host}: authentication failed ({detail})")]
    Auth {
        user: String,
        host: String,
        detail: String,
    },

    /// The server's key is not in `known_hosts`. The fingerprint is shown to the user, who
    /// decides; nothing is written to `known_hosts` without that decision.
    #[error("{host}: unknown host key {fingerprint}")]
    UnknownHostKey { host: String, fingerprint: String },

    /// The key changed. This is the interesting one: we refuse, loudly, and never offer to
    /// "just accept it" in the same breath.
    #[error("{host}: the host key has CHANGED (known_hosts line {line}) — refusing to connect")]
    ChangedHostKey { host: String, line: usize },

    /// The connection needs something we were not given (a password, a key passphrase).
    #[error("{user}@{host}: {} required", kind.as_str())]
    NeedsSecret {
        user: String,
        host: String,
        kind: SecretKind,
    },

    #[error("{path}: {detail}")]
    Remote { path: String, detail: String },

    #[error("{path}: not supported by this provider ({detail})")]
    Unsupported { path: String, detail: String },

    #[error("{0}: not a remote path")]
    NotRemote(String),

    #[error("no connection to {0}")]
    NotConnected(String),

    #[error("cancelled")]
    Cancelled,

    /// Anything the SSH library itself reports (protocol, cipher, disconnect).
    #[error("ssh: {0}")]
    Ssh(String),
}

impl From<russh::Error> for NetError {
    fn from(e: russh::Error) -> NetError {
        NetError::Ssh(e.to_string())
    }
}

/// What a connection is waiting for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKind {
    Password,
    Passphrase,
}

impl SecretKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SecretKind::Password => "a password",
            SecretKind::Passphrase => "a key passphrase",
        }
    }
}

impl fmt::Display for SecretKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub type Result<T> = std::result::Result<T, NetError>;

impl NetError {
    pub fn remote(path: impl Into<String>, detail: impl fmt::Display) -> NetError {
        NetError::Remote {
            path: path.into(),
            detail: detail.to_string(),
        }
    }

    /// Turn a provider error into the `io::Error` the job engine speaks, keeping the text.
    pub fn into_io(self) -> io::Error {
        let kind = match &self {
            NetError::Remote { detail, .. } => {
                let lower = detail.to_ascii_lowercase();
                if lower.contains("no such file") || lower.contains("not found") {
                    io::ErrorKind::NotFound
                } else if lower.contains("permission") {
                    io::ErrorKind::PermissionDenied
                } else if lower.contains("exists") {
                    io::ErrorKind::AlreadyExists
                } else {
                    io::ErrorKind::Other
                }
            }
            NetError::Cancelled => io::ErrorKind::Interrupted,
            NetError::Unsupported { .. } => io::ErrorKind::Unsupported,
            _ => io::ErrorKind::Other,
        };
        io::Error::new(kind, self.to_string())
    }
}

impl From<NetError> for io::Error {
    fn from(e: NetError) -> io::Error {
        e.into_io()
    }
}

impl From<NetError> for scmd_core::Error {
    fn from(e: NetError) -> scmd_core::Error {
        scmd_core::Error::Other(e.to_string())
    }
}
