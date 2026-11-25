//! Who a FinalCommander is on the Link (SPEC 04 §9.1): an ID to read out over the phone, and a
//! key with a self-signed certificate whose SHA-256 is what the other side really checks.

use std::fs;
use std::io;
use std::path::Path;

use ring::rand::{SecureRandom, SystemRandom};
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

/// The files kept in the configuration folder.
const ID_FILE: &str = "link-id";
const CERT_FILE: &str = "link-cert.der";
const KEY_FILE: &str = "link-key.der";

/// This installation on the Link.
pub struct Identity {
    /// Nine digits, 100 000 000 to 999 999 999.
    pub id: u32,
    /// What the other side shows beside the ID: the machine's name.
    pub name: String,
    cert: Vec<u8>,
    key: Vec<u8>,
}

impl std::fmt::Debug for Identity {
    // The key never reaches a log.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Identity")
            .field("id", &format_id(self.id))
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl Identity {
    /// A new identity, in memory: a random ID and a new key.
    pub fn generate(name: &str) -> io::Result<Identity> {
        let certified = rcgen::generate_simple_self_signed(vec!["finalcommander-link".to_string()])
            .map_err(io::Error::other)?;
        Ok(Identity {
            id: random_id()?,
            name: name.to_string(),
            cert: certified.cert.der().to_vec(),
            key: certified.signing_key.serialize_der(),
        })
    }

    /// The identity kept in `dir`, made and kept there the first time. The key is written
    /// readable by the user alone.
    pub fn load_or_create(dir: &Path, name: &str) -> io::Result<Identity> {
        let id_path = dir.join(ID_FILE);
        let cert_path = dir.join(CERT_FILE);
        let key_path = dir.join(KEY_FILE);
        if let (Ok(id), Ok(cert), Ok(key)) = (
            fs::read_to_string(&id_path),
            fs::read(&cert_path),
            fs::read(&key_path),
        ) {
            if let Some(id) = parse_id(&id) {
                return Ok(Identity {
                    id,
                    name: name.to_string(),
                    cert,
                    key,
                });
            }
        }
        let made = Identity::generate(name)?;
        fs::create_dir_all(dir)?;
        write_private(&key_path, &made.key)?;
        fs::write(&cert_path, &made.cert)?;
        fs::write(&id_path, format!("{}\n", made.id))?;
        Ok(made)
    }

    /// The certificate, as TLS sends it.
    pub fn certificate(&self) -> CertificateDer<'static> {
        CertificateDer::from(self.cert.clone())
    }

    /// The private key, for the TLS configurations alone.
    pub(crate) fn private_key(&self) -> PrivateKeyDer<'static> {
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(self.key.clone()))
    }

    /// This machine's fingerprint.
    pub fn fingerprint(&self) -> [u8; 32] {
        fingerprint(&self.cert)
    }
}

/// The SHA-256 of a certificate.
pub fn fingerprint(cert: &[u8]) -> [u8; 32] {
    let digest = ring::digest::digest(&ring::digest::SHA256, cert);
    let mut out = [0u8; 32];
    out.copy_from_slice(digest.as_ref());
    out
}

/// A fingerprint as hex in groups of four, for the screen.
pub fn fingerprint_text(fp: &[u8; 32]) -> String {
    fp.chunks(2)
        .map(|c| format!("{:02x}{:02x}", c[0], c[1]))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The six digits both screens show the first time two machines meet: the same on both
/// sides whichever called, and different if anything stands between them with a key of its
/// own (SPEC 04 §9.4).
pub fn sas(a: &[u8; 32], b: &[u8; 32]) -> String {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    let mut ctx = ring::digest::Context::new(&ring::digest::SHA256);
    ctx.update(b"FinalCommander Link SAS 1");
    ctx.update(lo);
    ctx.update(hi);
    let d = ctx.finish();
    let n = u32::from_be_bytes([d.as_ref()[0], d.as_ref()[1], d.as_ref()[2], d.as_ref()[3]]);
    let six = n % 1_000_000;
    format!("{:03} {:03}", six / 1000, six % 1000)
}

/// `123456789` as `123 456 789`.
pub fn format_id(id: u32) -> String {
    format!(
        "{:03} {:03} {:03}",
        id / 1_000_000,
        id / 1000 % 1000,
        id % 1000
    )
}

/// An ID as a person types it: nine digits, spaces and dashes ignored.
pub fn parse_id(text: &str) -> Option<u32> {
    let digits: String = text
        .chars()
        .filter(|c| !matches!(c, ' ' | '-' | '\n' | '\r' | '\t'))
        .collect();
    if digits.len() != 9 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let id: u32 = digits.parse().ok()?;
    (100_000_000..=999_999_999).contains(&id).then_some(id)
}

fn random_id() -> io::Result<u32> {
    let mut bytes = [0u8; 4];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| io::Error::other("no random numbers"))?;
    Ok(100_000_000 + u32::from_le_bytes(bytes) % 900_000_000)
}

pub(super) fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(bytes)
    }
    #[cfg(not(unix))]
    {
        fs::write(path, bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_is_nine_digits_read_the_way_a_person_types_it() {
        assert_eq!(format_id(123_456_789), "123 456 789");
        assert_eq!(parse_id("123 456 789"), Some(123_456_789));
        assert_eq!(parse_id("123-456-789"), Some(123_456_789));
        assert_eq!(parse_id("12345678"), None);
        assert_eq!(parse_id("012345678"), None);
        assert_eq!(parse_id("12345678x"), None);
    }

    #[test]
    fn the_six_digits_are_the_same_on_both_sides_and_change_with_either_key() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        let c = [3u8; 32];
        assert_eq!(sas(&a, &b), sas(&b, &a));
        assert_ne!(sas(&a, &b), sas(&a, &c));
        assert_eq!(sas(&a, &b).len(), 7);
    }

    #[test]
    fn an_identity_is_kept_and_read_back_the_same() {
        let dir = tempfile::tempdir().unwrap();
        let first = Identity::load_or_create(dir.path(), "here").unwrap();
        let again = Identity::load_or_create(dir.path(), "here").unwrap();
        assert_eq!(first.id, again.id);
        assert_eq!(first.fingerprint(), again.fingerprint());
        assert!((100_000_000..=999_999_999).contains(&first.id));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(dir.path().join(KEY_FILE))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600, "the key is the user's alone");
        }
    }
}
