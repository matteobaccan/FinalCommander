//! FinalCommander Link: another machine's folder in a panel, on the local network
//! (SPEC 04 §9).
//!
//! Two FinalCommanders talk TLS 1.3 to each other, each showing a self-signed certificate of
//! its own; neither trusts an authority, each checks the other's fingerprint. The first time
//! two machines meet, both screens show the same six digits for the people in front of them
//! to compare; then the one called says yes or no, and a yes opens a share — a folder, read
//! only in this first step — to the caller, whose panel reads it through [`client::LinkFs`],
//! a [`crate::RemoteFs`] like SFTP's.
//!
//! - [`identity`]: the nine-digit ID, the key and certificate, fingerprints, the six digits;
//! - [`tls`]: the two TLS configurations;
//! - [`wire`]: frames on the stream;
//! - [`share`]: a caller's path resolved under a shared folder, and refused if it climbs out;
//! - [`server`]: the called side;
//! - [`client`]: the calling side;
//! - [`dial`]: a Link site opened from the netbox;
//! - [`discovery`]: who is on the network, by UDP broadcast;
//! - [`net`]: which addresses count as the local network.

pub mod client;
pub mod dial;
pub mod discovery;
pub mod identity;
pub mod net;
pub mod pin;
pub mod server;
pub mod share;
pub mod tls;
pub mod wire;

pub use client::{call, LinkFs, Pending};
pub use dial::Dialer;
pub use identity::{format_id, parse_id, Identity};
pub use server::{serve, Admission, Caller, Gate, HangUp};
pub use share::Share;

/// The protocol this build speaks, sent first by each side.
pub const VERSION: u32 = 1;

/// How long the person called has to answer before the call counts as refused.
pub const ANSWER_SECONDS: u64 = 60;
