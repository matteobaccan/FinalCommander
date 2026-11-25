//! A Link site opened from the netbox (SPEC 04 §9.8): the ID found on the local network, the
//! call made, the six digits shown while the person there decides, and the fingerprint kept
//! so that a machine whose key changes is refused the next time.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use super::client::call;
use super::discovery;
use super::identity::{fingerprint_text, parse_id, Identity};
use crate::error::{NetError, Result};
use crate::remote::Remote;
use crate::site::Site;

/// How long a broadcast waits for the machine to answer.
const FIND_WAIT: Duration = Duration::from_secs(2);

/// What the program around it gives the netbox to call with: who this machine is, where the
/// fingerprints of the machines met before are kept, and how to show the six digits.
pub struct Dialer {
    pub identity: Arc<Identity>,
    /// `<id> <fingerprint hex>` per line.
    pub known: PathBuf,
    /// Called with the site and the six digits once the other machine answers, while its
    /// person decides.
    pub show_code: Arc<dyn Fn(&Site, &str) + Send + Sync>,
}

impl std::fmt::Debug for Dialer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dialer")
            .field("identity", &self.identity)
            .field("known", &self.known)
            .finish_non_exhaustive()
    }
}

fn failed(site: &Site, detail: impl std::fmt::Display) -> NetError {
    NetError::Connect {
        host: format!("link {}", site.host),
        detail: detail.to_string(),
    }
}

impl Dialer {
    /// Open the Link site: find, call, ask.
    pub async fn connect(&self, site: &Site, _secret: Option<&str>) -> Result<Remote> {
        let id = parse_id(&site.host)
            .ok_or_else(|| failed(site, "not a FinalCommander ID: nine digits"))?;
        let found = discovery::find(discovery::broadcast(), Some(id), FIND_WAIT)
            .await
            .map_err(|e| failed(site, e))?;
        let Some(there) = found.into_iter().next() else {
            return Err(failed(
                site,
                "no FinalCommander with this ID answered on the local network — is Link \
                 turned on there?",
            ));
        };
        let pending = call(there.addr, &self.identity).await?;
        let theirs = pending.peer.fingerprint;
        if let Some(known) = self.known_fingerprint(id) {
            if known != theirs {
                return Err(failed(
                    site,
                    format!(
                        "the machine's key has CHANGED (now {}) — refusing to connect; if it \
                         was reinstalled, remove its line from {}",
                        fingerprint_text(&theirs),
                        self.known.display()
                    ),
                ));
            }
        }
        (self.show_code)(site, &pending.sas);
        let fs = pending.ask(&site.user, false).await?;
        let _ = self.remember(id, &theirs);
        Ok(Remote::new(site.clone(), Arc::new(fs)))
    }

    fn known_fingerprint(&self, id: u32) -> Option<[u8; 32]> {
        let text = fs::read_to_string(&self.known).ok()?;
        text.lines().find_map(|line| {
            let (who, hex) = line.split_once(' ')?;
            if who.trim().parse::<u32>().ok()? != id {
                return None;
            }
            from_hex(hex.trim())
        })
    }

    fn remember(&self, id: u32, fp: &[u8; 32]) -> io::Result<()> {
        if self.known_fingerprint(id).is_some() {
            return Ok(());
        }
        if let Some(dir) = self.known.parent() {
            fs::create_dir_all(dir)?;
        }
        let hex: String = fp.iter().map(|b| format!("{b:02x}")).collect();
        let mut text = fs::read_to_string(&self.known).unwrap_or_default();
        text.push_str(&format!("{id} {hex}\n"));
        fs::write(&self.known, text)
    }
}

fn from_hex(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fingerprint_is_kept_once_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let dialer = Dialer {
            identity: Arc::new(Identity::generate("here").unwrap()),
            known: dir.path().join("link-known"),
            show_code: Arc::new(|_, _| {}),
        };
        assert_eq!(dialer.known_fingerprint(123_456_789), None);
        dialer.remember(123_456_789, &[7u8; 32]).unwrap();
        dialer.remember(123_456_789, &[9u8; 32]).unwrap();
        assert_eq!(
            dialer.known_fingerprint(123_456_789),
            Some([7u8; 32]),
            "the first stays"
        );
        assert_eq!(dialer.known_fingerprint(987_654_321), None);
    }
}
