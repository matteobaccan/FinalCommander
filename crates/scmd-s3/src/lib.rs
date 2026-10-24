//! Object storage: S3 and the services that speak its protocol.

pub mod client;
pub mod clock;
pub mod creds;
pub mod endpoint;
pub mod error;
pub mod sign;
pub mod uri;
pub mod vfs;
pub mod xml;

#[cfg(test)]
pub(crate) mod fake;
