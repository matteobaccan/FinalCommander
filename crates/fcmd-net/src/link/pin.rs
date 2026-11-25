//! The Link's PIN (SPEC 04 §9.5): a caller that knows it is let in without the dialog.
//!
//! The PIN never crosses the network. Both sides stretch it with PBKDF2 and a salt the
//! called side keeps, and run SPAKE2 with the result, the two machines' fingerprints as the
//! identities: a listener learns nothing, and a machine in the middle — or one pretending to
//! be the called side — gets exactly one guess per call, which [`Lockout`] then limits. Each
//! side proves it holds the same key with an HMAC of a label of its own before anything is
//! opened.
//!
//! The called side keeps the stretched value, not the PIN, in `link-pin` beside the identity,
//! readable by the user alone.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::path::Path;
use std::time::{Duration, Instant};

use ring::rand::{SecureRandom, SystemRandom};
use ring::{hmac, pbkdf2};
use spake2::{Ed25519Group, Identity as Side, Password, Spake2};

use super::identity::write_private;

/// The file beside the identity.
pub const PIN_FILE: &str = "link-pin";

/// The shortest PIN accepted when one is set.
pub const MIN_LEN: usize = 6;

/// PBKDF2 rounds: a second or so on a router, nothing on a desktop, and done once per call.
const ROUNDS: u32 = 100_000;

/// Wrong PINs from one address before it is locked out, and for how long.
pub const MAX_WRONG: u32 = 5;
pub const LOCKED_FOR: Duration = Duration::from_secs(10 * 60);

const CALLER_LABEL: &[u8] = b"FCMD-LINK/1 pin caller";
const CALLED_LABEL: &[u8] = b"FCMD-LINK/1 pin called";

/// What the called side keeps: the salt and the stretched PIN.
#[derive(Clone, PartialEq, Eq)]
pub struct Pin {
    pub salt: [u8; 16],
    pub verifier: [u8; 32],
}

impl std::fmt::Debug for Pin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Pin(…)")
    }
}

/// Stretch `pin` with `salt`: what both sides feed SPAKE2.
pub fn stretch(pin: &str, salt: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        NonZeroU32::new(ROUNDS).expect("rounds"),
        salt,
        pin.trim().as_bytes(),
        &mut out,
    );
    out
}

impl Pin {
    /// A new PIN with a fresh salt. Refused when shorter than [`MIN_LEN`].
    pub fn new(pin: &str) -> io::Result<Pin> {
        if pin.trim().chars().count() < MIN_LEN {
            return Err(io::Error::other(format!(
                "a PIN has at least {MIN_LEN} characters"
            )));
        }
        let mut salt = [0u8; 16];
        SystemRandom::new()
            .fill(&mut salt)
            .map_err(|_| io::Error::other("no random numbers"))?;
        Ok(Pin {
            salt,
            verifier: stretch(pin, &salt),
        })
    }

    /// The PIN kept in `dir`, if one was set.
    pub fn load(dir: &Path) -> io::Result<Option<Pin>> {
        let text = match fs::read_to_string(dir.join(PIN_FILE)) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let bad = || io::Error::other(format!("{} is damaged", dir.join(PIN_FILE).display()));
        let (salt, verifier) = text.trim().split_once(' ').ok_or_else(bad)?;
        Ok(Some(Pin {
            salt: from_hex(salt).ok_or_else(bad)?,
            verifier: from_hex(verifier).ok_or_else(bad)?,
        }))
    }

    pub fn save(&self, dir: &Path) -> io::Result<()> {
        fs::create_dir_all(dir)?;
        let line = format!("{} {}\n", hex(&self.salt), hex(&self.verifier));
        write_private(&dir.join(PIN_FILE), line.as_bytes())
    }

    /// Take the PIN away: calls go back to the dialog.
    pub fn remove(dir: &Path) -> io::Result<()> {
        match fs::remove_file(dir.join(PIN_FILE)) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn from_hex<const N: usize>(text: &str) -> Option<[u8; N]> {
    let bytes = from_hex_vec(text)?;
    bytes.try_into().ok()
}

pub fn from_hex_vec(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
        .collect()
}

/// One side of the exchange, started.
pub struct Exchange {
    state: Spake2<Ed25519Group>,
    /// The message to send.
    pub message: Vec<u8>,
    caller: bool,
}

impl Exchange {
    /// The caller's side: `stretched` from [`stretch`], the fingerprints of both machines.
    pub fn caller(stretched: &[u8; 32], caller: &[u8; 32], called: &[u8; 32]) -> Exchange {
        let (state, message) = Spake2::<Ed25519Group>::start_a(
            &Password::new(stretched),
            &Side::new(caller),
            &Side::new(called),
        );
        Exchange {
            state,
            message,
            caller: true,
        }
    }

    pub fn called(stretched: &[u8; 32], caller: &[u8; 32], called: &[u8; 32]) -> Exchange {
        let (state, message) = Spake2::<Ed25519Group>::start_b(
            &Password::new(stretched),
            &Side::new(caller),
            &Side::new(called),
        );
        Exchange {
            state,
            message,
            caller: false,
        }
    }

    /// With the other side's message: the proof this side sends, and the proof it expects.
    /// A malformed message is an error; a wrong PIN shows only as proofs that differ.
    pub fn finish(self, theirs: &[u8]) -> io::Result<Proofs> {
        let key = self
            .state
            .finish(theirs)
            .map_err(|_| io::Error::other("a broken PIN exchange"))?;
        let key = hmac::Key::new(hmac::HMAC_SHA256, &key);
        let (mine, theirs) = match self.caller {
            true => (CALLER_LABEL, CALLED_LABEL),
            false => (CALLED_LABEL, CALLER_LABEL),
        };
        Ok(Proofs {
            mine: hmac::sign(&key, mine).as_ref().to_vec(),
            key,
            theirs,
        })
    }
}

pub struct Proofs {
    /// What this side sends.
    pub mine: Vec<u8>,
    key: hmac::Key,
    theirs: &'static [u8],
}

impl Proofs {
    /// Does the other side's proof show it holds the same key? In constant time.
    pub fn check(&self, proof: &[u8]) -> bool {
        hmac::verify(&self.key, self.theirs, proof).is_ok()
    }
}

/// Wrong PINs counted by address: [`MAX_WRONG`] of them lock the address out for
/// [`LOCKED_FOR`]. Kept by the server for as long as it runs.
#[derive(Debug, Default)]
pub struct Lockout {
    wrong: HashMap<IpAddr, (u32, Option<Instant>)>,
}

impl Lockout {
    /// `Err` with the time left while `ip` is locked out.
    pub fn check(&mut self, ip: IpAddr, now: Instant) -> Result<(), Duration> {
        if let Some((_, Some(until))) = self.wrong.get(&ip) {
            if now < *until {
                return Err(*until - now);
            }
            self.wrong.remove(&ip);
        }
        Ok(())
    }

    pub fn failed(&mut self, ip: IpAddr, now: Instant) {
        let entry = self.wrong.entry(ip).or_insert((0, None));
        entry.0 += 1;
        if entry.0 >= MAX_WRONG {
            entry.1 = Some(now + LOCKED_FOR);
        }
    }

    pub fn succeeded(&mut self, ip: IpAddr) {
        self.wrong.remove(&ip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_pin_proves_itself_and_another_does_not() {
        let caller = [1u8; 32];
        let called = [2u8; 32];
        let pin = Pin::new("246810").unwrap();
        let run = |typed: &str| {
            let a = Exchange::caller(&stretch(typed, &pin.salt), &caller, &called);
            let b = Exchange::called(&pin.verifier, &caller, &called);
            let (ma, mb) = (a.message.clone(), b.message.clone());
            let pa = a.finish(&mb).unwrap();
            let pb = b.finish(&ma).unwrap();
            (pb.check(&pa.mine), pa.check(&pb.mine))
        };
        assert_eq!(run("246810"), (true, true));
        assert_eq!(run("246811"), (false, false));
        // Bound to the machines: the same PIN between other fingerprints does not agree.
        let a = Exchange::caller(&stretch("246810", &pin.salt), &[9u8; 32], &called);
        let b = Exchange::called(&pin.verifier, &caller, &called);
        let (ma, mb) = (a.message.clone(), b.message.clone());
        assert!(!b.finish(&ma).unwrap().check(&a.finish(&mb).unwrap().mine));
    }

    #[test]
    fn a_pin_is_kept_stretched_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(Pin::load(dir.path()).unwrap(), None);
        assert!(Pin::new("12345").is_err(), "too short");
        let pin = Pin::new("secret-pin").unwrap();
        pin.save(dir.path()).unwrap();
        let text = fs::read_to_string(dir.path().join(PIN_FILE)).unwrap();
        assert!(!text.contains("secret-pin"));
        assert_eq!(Pin::load(dir.path()).unwrap(), Some(pin));
        Pin::remove(dir.path()).unwrap();
        assert_eq!(Pin::load(dir.path()).unwrap(), None);
    }

    #[test]
    fn five_wrong_pins_lock_an_address_out_for_ten_minutes() {
        let mut lock = Lockout::default();
        let ip: IpAddr = "192.168.1.9".parse().unwrap();
        let t = Instant::now();
        for _ in 0..MAX_WRONG - 1 {
            lock.failed(ip, t);
            assert!(lock.check(ip, t).is_ok());
        }
        lock.failed(ip, t);
        assert!(lock.check(ip, t + Duration::from_secs(60)).is_err());
        assert!(lock.check(ip, t + LOCKED_FOR).is_ok());
        lock.failed(ip, t + LOCKED_FOR);
        assert!(
            lock.check(ip, t + LOCKED_FOR).is_ok(),
            "the count started again"
        );
    }
}
