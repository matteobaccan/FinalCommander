//! Object storage: S3 and the services that speak its protocol.

pub mod client;
pub mod clock;
pub mod creds;
pub mod endpoint;
pub mod error;
pub mod jobs;
pub mod sign;
pub mod uri;
pub mod vfs;
pub mod xml;

/// A service that speaks S3, in this process, for the tests — this crate's own, and those of
/// the application above it, which is why it is behind a feature rather than `cfg(test)`:
/// a test in another crate cannot see a `cfg(test)` module of this one.
#[cfg(any(test, feature = "fake"))]
pub mod fake;
