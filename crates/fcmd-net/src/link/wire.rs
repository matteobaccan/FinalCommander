//! Frames on the Link's TLS stream (SPEC 04 §9.7).
//!
//! A frame is a kind byte, a length (four bytes, big-endian) and that many bytes: JSON for
//! every message, raw bytes for the contents of a file. A frame larger than [`MAX_FRAME`] ends
//! the connection, so a broken or hostile peer cannot make the other side allocate without
//! bound.

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::ops::RemoteEntry;
use fcmd_core::EntryKind;

/// The largest frame either side accepts: a chunk of a file plus room.
pub const MAX_FRAME: usize = 1024 * 1024;

/// What a side sends first.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hello {
    pub version: u32,
    pub id: u32,
    pub name: String,
}

/// What the caller asks for, after the six digits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ask {
    /// The share by name; empty for the one the called side offers first.
    pub share: String,
    pub write: bool,
    /// The caller has a PIN to prove (SPEC 04 §9.5). Absent from an older caller.
    #[serde(default)]
    pub pin: bool,
}

/// The called side's first word to a caller with a PIN: the salt to stretch it with, or
/// none when this machine has no PIN (the call then goes to the person), or why not at all.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PinOffer {
    pub salt: Option<String>,
    pub refused: Option<String>,
}

/// A SPAKE2 message, hex.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PinMessage {
    pub message: String,
}

/// A side's proof that it holds the key the PIN gave, hex.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PinProof {
    pub proof: String,
}

/// The called side's answer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Answer {
    pub ok: bool,
    pub write: bool,
    /// The share's name, as the caller's panel shows it.
    pub share: String,
    pub reason: Option<String>,
}

/// One request; the reply carries the same number.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Request {
    pub n: u64,
    pub op: Op,
}

/// The operations (SPEC 04 §9.7). Paths are `/`-separated, absolute within the share.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    ReadDir {
        path: String,
    },
    Stat {
        path: String,
        follow: bool,
    },
    ReadLink {
        path: String,
    },
    Canonicalize {
        path: String,
    },
    /// Open a file for reading: the reply is a handle and its size.
    Open {
        path: String,
    },
    /// Up to `len` bytes at `offset`: the reply is a data frame.
    Read {
        handle: u32,
        offset: u64,
        len: u32,
    },
    Close {
        handle: u32,
    },
    // Writing (step 2). Refused by a read-only session, whatever the caller sends.
    CreateDir {
        path: String,
    },
    RemoveFile {
        path: String,
    },
    RemoveDir {
        path: String,
    },
    Rename {
        from: String,
        to: String,
    },
    SetMeta {
        path: String,
        mode: Option<u32>,
        mtime: Option<u32>,
    },
    Create {
        path: String,
    },
    Write {
        handle: u32,
        offset: u64,
    },
    Commit {
        handle: u32,
    },
}

impl Op {
    /// Whether the operation changes anything on the called side.
    pub fn writes(&self) -> bool {
        matches!(
            self,
            Op::CreateDir { .. }
                | Op::RemoveFile { .. }
                | Op::RemoveDir { .. }
                | Op::Rename { .. }
                | Op::SetMeta { .. }
                | Op::Create { .. }
                | Op::Write { .. }
                | Op::Commit { .. }
        )
    }
}

/// A reply, as JSON; a read's bytes come in a data frame instead.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Reply {
    pub n: u64,
    pub result: Result<Payload, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Payload {
    Done,
    Entries { entries: Vec<WireEntry> },
    Entry { entry: WireEntry },
    Path { path: String },
    Opened { handle: u32, size: u64 },
}

/// A directory entry on the wire.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WireEntry {
    pub name: String,
    /// `file`, `dir` or `link`.
    pub kind: String,
    pub size: u64,
    pub mtime: Option<u32>,
    pub mode: Option<u32>,
    pub link_target: Option<String>,
}

impl From<&RemoteEntry> for WireEntry {
    fn from(e: &RemoteEntry) -> WireEntry {
        WireEntry {
            name: e.name.clone(),
            kind: match e.kind {
                EntryKind::Dir => "dir",
                EntryKind::Symlink => "link",
                _ => "file",
            }
            .to_string(),
            size: e.size,
            mtime: e.mtime,
            mode: e.mode,
            link_target: e.link_target.clone(),
        }
    }
}

impl From<WireEntry> for RemoteEntry {
    fn from(w: WireEntry) -> RemoteEntry {
        RemoteEntry {
            name: w.name,
            kind: match w.kind.as_str() {
                "dir" => EntryKind::Dir,
                "link" => EntryKind::Symlink,
                _ => EntryKind::File,
            },
            size: w.size,
            mtime: w.mtime,
            mode: w.mode,
            link_target: w.link_target,
        }
    }
}

/// A frame as read.
#[derive(Debug)]
pub enum Frame {
    Json(Vec<u8>),
    Data(Vec<u8>),
}

const KIND_JSON: u8 = 0;
const KIND_DATA: u8 = 1;

/// Write a message as a JSON frame.
pub async fn send<W: AsyncWrite + Unpin, T: Serialize>(w: &mut W, msg: &T) -> std::io::Result<()> {
    let body = serde_json::to_vec(msg).map_err(std::io::Error::other)?;
    write_frame(w, KIND_JSON, &body).await
}

/// Write bytes as a data frame.
pub async fn send_data<W: AsyncWrite + Unpin>(w: &mut W, bytes: &[u8]) -> std::io::Result<()> {
    write_frame(w, KIND_DATA, bytes).await
}

async fn write_frame<W: AsyncWrite + Unpin>(
    w: &mut W,
    kind: u8,
    body: &[u8],
) -> std::io::Result<()> {
    if body.len() > MAX_FRAME {
        return Err(std::io::Error::other("frame too large"));
    }
    let mut head = [0u8; 5];
    head[0] = kind;
    head[1..].copy_from_slice(&(body.len() as u32).to_be_bytes());
    w.write_all(&head).await?;
    w.write_all(body).await?;
    w.flush().await
}

/// Read one frame.
pub async fn read_frame<R: AsyncRead + Unpin>(r: &mut R) -> std::io::Result<Frame> {
    let mut head = [0u8; 5];
    r.read_exact(&mut head).await?;
    let len = u32::from_be_bytes([head[1], head[2], head[3], head[4]]) as usize;
    if len > MAX_FRAME {
        return Err(std::io::Error::other("frame too large"));
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body).await?;
    match head[0] {
        KIND_JSON => Ok(Frame::Json(body)),
        KIND_DATA => Ok(Frame::Data(body)),
        k => Err(std::io::Error::other(format!("unknown frame kind {k}"))),
    }
}

/// Read a JSON frame as `T`; a data frame here is an error.
pub async fn recv<R: AsyncRead + Unpin, T: for<'de> Deserialize<'de>>(
    r: &mut R,
) -> std::io::Result<T> {
    match read_frame(r).await? {
        Frame::Json(body) => serde_json::from_slice(&body).map_err(std::io::Error::other),
        Frame::Data(_) => Err(std::io::Error::other("bytes where a message was expected")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_message_and_bytes_go_round_and_a_huge_frame_is_refused() {
        let (mut a, mut b) = tokio::io::duplex(4 * MAX_FRAME);
        let req = Request {
            n: 7,
            op: Op::Read {
                handle: 1,
                offset: 9,
                len: 10,
            },
        };
        send(&mut a, &req).await.unwrap();
        send_data(&mut a, b"bytes").await.unwrap();
        let got: Request = recv(&mut b).await.unwrap();
        assert_eq!(got, req);
        match read_frame(&mut b).await.unwrap() {
            Frame::Data(d) => assert_eq!(d, b"bytes"),
            Frame::Json(_) => panic!("data expected"),
        }
        let mut head = [0u8; 5];
        head[1..].copy_from_slice(&((MAX_FRAME as u32) + 1).to_be_bytes());
        a.write_all(&head).await.unwrap();
        assert!(read_frame(&mut b).await.is_err());
    }
}
