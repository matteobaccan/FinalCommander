//! `fcmd-secrets.vault`: the program's own encrypted file of keys and passwords.
//!
//! It sits beside the settings, so it travels with them: copy the folder to another machine
//! and the secrets come along, readable with the one password that opens them. That password
//! is asked for the first time a secret is needed, kept in memory while the program is in use,
//! and forgotten after five minutes without a key pressed (see [`Unlocked::expired`]); the next
//! secret needed asks for it again.
//!
//! The file is `FCMDVLT1`, a 16-byte salt, the PBKDF2 round count, a 12-byte nonce, and then
//! the secrets as JSON sealed with AES-256-GCM under a key derived from the password with
//! PBKDF2-HMAC-SHA256. A wrong password fails the tag check and says so; nothing is ever
//! written in clear. `ring` does the cryptography, and is already in the tree for SSH and S3.

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::path::Path;
use std::time::{Duration, Instant};

use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};

const MAGIC: &[u8; 8] = b"FCMDVLT1";
/// Beside `fcmd.toml`.
pub const FILE_NAME: &str = "fcmd-secrets.vault";
/// OWASP's figure for PBKDF2-HMAC-SHA256: slow enough to make guessing expensive, a fraction
/// of a second for the one person who knows the password.
pub const ROUNDS: u32 = 600_000;
/// How long the password is kept with no key pressed.
pub const IDLE: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, PartialEq, Eq)]
pub enum VaultError {
    WrongPassword,
    Damaged,
    Io(String),
}

impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VaultError::WrongPassword => f.write_str("wrong password"),
            VaultError::Damaged => f.write_str("the secrets file is damaged"),
            VaultError::Io(e) => f.write_str(e),
        }
    }
}

/// The secrets, open: what they are, and the key that seals them again.
pub struct Unlocked {
    key: [u8; 32],
    salt: [u8; 16],
    rounds: u32,
    pub secrets: BTreeMap<String, String>,
    last_used: Instant,
}

impl std::fmt::Debug for Unlocked {
    /// By hand, so that no derive ever prints a secret or the key.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Unlocked")
            .field("secrets", &self.secrets.len())
            .finish_non_exhaustive()
    }
}

impl Drop for Unlocked {
    fn drop(&mut self) {
        self.key.fill(0);
        for value in self.secrets.values_mut() {
            zero_string(value);
        }
    }
}

/// Overwrite a secret's bytes before its memory is given back: the string is taken, its bytes
/// zeroed, and the zeroed buffer put back where it was.
fn zero_string(value: &mut String) {
    let mut bytes = std::mem::take(value).into_bytes();
    bytes.fill(0);
    *value = String::from_utf8(bytes).unwrap_or_default();
}

fn derive(password: &str, salt: &[u8; 16], rounds: u32) -> [u8; 32] {
    let mut key = [0u8; 32];
    ring::pbkdf2::derive(
        ring::pbkdf2::PBKDF2_HMAC_SHA256,
        NonZeroU32::new(rounds.max(1)).expect("not zero"),
        salt,
        password.as_bytes(),
        &mut key,
    );
    key
}

impl Unlocked {
    /// A new, empty set of secrets under this password.
    pub fn create(password: &str) -> Unlocked {
        // The tests open and close many files; the cost is the point everywhere else.
        Self::create_with(password, if cfg!(test) { 1000 } else { ROUNDS })
    }

    pub fn create_with(password: &str, rounds: u32) -> Unlocked {
        let mut salt = [0u8; 16];
        let _ = SystemRandom::new().fill(&mut salt);
        Unlocked {
            key: derive(password, &salt, rounds),
            salt,
            rounds,
            secrets: BTreeMap::new(),
            last_used: Instant::now(),
        }
    }

    /// Open the file with this password.
    pub fn open(path: &Path, password: &str) -> Result<Unlocked, VaultError> {
        let bytes = std::fs::read(path).map_err(|e| VaultError::Io(e.to_string()))?;
        Self::open_bytes(&bytes, password)
    }

    pub fn open_bytes(bytes: &[u8], password: &str) -> Result<Unlocked, VaultError> {
        if bytes.len() < 8 + 16 + 4 + 12 + 16 || &bytes[..8] != MAGIC {
            return Err(VaultError::Damaged);
        }
        let mut salt = [0u8; 16];
        salt.copy_from_slice(&bytes[8..24]);
        let rounds = u32::from_le_bytes(bytes[24..28].try_into().map_err(|_| VaultError::Damaged)?);
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&bytes[28..40]);
        let key = derive(password, &salt, rounds);
        let sealing =
            LessSafeKey::new(UnboundKey::new(&AES_256_GCM, &key).map_err(|_| VaultError::Damaged)?);
        let mut body = bytes[40..].to_vec();
        let plain = sealing
            .open_in_place(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(MAGIC),
                &mut body,
            )
            .map_err(|_| VaultError::WrongPassword)?;
        let secrets: BTreeMap<String, String> =
            serde_json::from_slice(plain).map_err(|_| VaultError::Damaged)?;
        body.fill(0);
        Ok(Unlocked {
            key,
            salt,
            rounds,
            secrets,
            last_used: Instant::now(),
        })
    }

    /// The file's bytes: sealed again, with a fresh nonce every time.
    pub fn seal(&self) -> Vec<u8> {
        let mut nonce = [0u8; 12];
        let _ = SystemRandom::new().fill(&mut nonce);
        let sealing = LessSafeKey::new(UnboundKey::new(&AES_256_GCM, &self.key).expect("32 bytes"));
        let mut body = serde_json::to_vec(&self.secrets).unwrap_or_default();
        sealing
            .seal_in_place_append_tag(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(MAGIC),
                &mut body,
            )
            .expect("sealing a buffer in memory");
        let mut out = Vec::with_capacity(40 + body.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&self.salt);
        out.extend_from_slice(&self.rounds.to_le_bytes());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&body);
        out
    }

    /// Write the file, through a temporary beside it so a crash never leaves half of one.
    pub fn save(&self, path: &Path) -> Result<(), VaultError> {
        let io = |e: std::io::Error| VaultError::Io(format!("{}: {e}", path.display()));
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(io)?;
        }
        let tmp = path.with_extension("vault.part");
        std::fs::write(&tmp, self.seal()).map_err(io)?;
        std::fs::rename(&tmp, path).map_err(io)
    }

    pub fn get(&mut self, name: &str) -> Option<String> {
        self.last_used = Instant::now();
        self.secrets.get(name).cloned()
    }

    pub fn set(&mut self, name: &str, value: &str) {
        self.last_used = Instant::now();
        self.secrets.insert(name.to_string(), value.to_string());
    }

    /// Make the last use this much older, for the tests of forgetting.
    #[cfg(test)]
    pub fn age(&mut self, by: Duration) {
        self.last_used -= by;
    }

    /// Note that the program is in use: a key was pressed.
    pub fn touch(&mut self) {
        self.last_used = Instant::now();
    }

    /// Has the program gone unused for longer than `idle`? Then the password is to be
    /// forgotten — the whole `Unlocked` dropped, which zeroes the key.
    pub fn expired(&self, now: Instant, idle: Duration) -> bool {
        now.duration_since(self.last_used) >= idle
    }

    /// How long until it expires, for the loop's wait.
    pub fn left(&self, now: Instant, idle: Duration) -> Duration {
        idle.saturating_sub(now.duration_since(self.last_used))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_round_trip_and_a_wrong_password_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fcmd-secrets.vault");
        let mut v = Unlocked::create_with("correct horse", 1000);
        v.set("ai:claude", "sk-ant-123");
        v.save(&path).unwrap();
        let raw = std::fs::read(&path).unwrap();
        assert!(
            !raw.windows(7).any(|w| w == b"sk-ant-"),
            "nothing is written in clear"
        );
        let mut again = Unlocked::open(&path, "correct horse").unwrap();
        assert_eq!(again.get("ai:claude").as_deref(), Some("sk-ant-123"));
        assert_eq!(
            Unlocked::open(&path, "wrong").unwrap_err(),
            VaultError::WrongPassword
        );
        assert_eq!(
            Unlocked::open_bytes(b"not a vault at all, no", "x").unwrap_err(),
            VaultError::Damaged
        );
        // Sealed twice, two different files: the nonce is fresh each time.
        assert_ne!(again.seal(), again.seal());
    }

    #[test]
    fn the_password_is_forgotten_after_five_idle_minutes() {
        let mut v = Unlocked::create_with("p", 1);
        let now = Instant::now();
        assert!(!v.expired(now, IDLE));
        assert!(v.expired(now + IDLE, IDLE));
        v.touch();
        assert!(v.left(Instant::now(), IDLE) > Duration::from_secs(299));
    }
}
