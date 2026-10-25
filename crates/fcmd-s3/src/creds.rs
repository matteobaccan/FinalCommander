//! Where the keys come from.
//!
//! Not from the configuration file. A secret access key is a password, and this program has
//! never written a password into `fcmd.toml` — the netbox asks for one or uses the SSH agent,
//! and object storage reads the two places the rest of the world already keeps these: the
//! `AWS_*` environment variables, and the `~/.aws/credentials` file every AWS tool shares. A
//! key in a configuration file would be a key in a backup, in a dotfiles repository and in a
//! screen share, and the file it belongs in is one that already exists and is already
//! expected to be private.

use std::path::{Path, PathBuf};

use crate::sign::Credentials;

/// Where to look.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` and, when it is there,
    /// `AWS_SESSION_TOKEN`. Falls back to the shared file, as every AWS tool does.
    Environment,
    /// A named profile of `~/.aws/credentials` (`default` when not given).
    AwsProfile(String),
    /// None at all, for a bucket with a public read policy.
    Anonymous,
}

impl Source {
    /// `env`, `aws`, `aws:work`, `anonymous`. Anything else is read as a profile name, which
    /// is what someone who wrote `credentials = "work"` meant.
    pub fn parse(text: &str) -> Source {
        let text = text.trim();
        match text.to_ascii_lowercase().as_str() {
            "" | "env" | "environment" | "auto" => Source::Environment,
            "anonymous" | "none" | "public" => Source::Anonymous,
            "aws" | "profile" | "default" => Source::AwsProfile("default".into()),
            _ => match text.split_once(':') {
                Some(("aws" | "profile", profile)) => {
                    Source::AwsProfile(profile.trim().to_string())
                }
                _ => Source::AwsProfile(text.to_string()),
            },
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Source::Environment => "env".into(),
            Source::AwsProfile(p) => format!("aws:{p}"),
            Source::Anonymous => "anonymous".into(),
        }
    }
}

/// Find the credentials, or none when the source says so and `None` when it cannot.
///
/// `home` is where `~` is, taken as an argument so the test does not depend on whose account
/// is running it.
pub fn resolve(source: &Source, home: Option<&Path>) -> Option<Credentials> {
    match source {
        Source::Anonymous => None,
        Source::Environment => from_env().or_else(|| {
            let profile = std::env::var("AWS_PROFILE").unwrap_or_else(|_| "default".into());
            from_aws_file(&credentials_file(home)?, &profile)
        }),
        Source::AwsProfile(profile) => from_aws_file(&credentials_file(home)?, profile),
    }
}

/// The region, from the environment, for a service whose configuration does not say.
pub fn region_from_env() -> Option<String> {
    std::env::var("AWS_REGION")
        .or_else(|_| std::env::var("AWS_DEFAULT_REGION"))
        .ok()
        .filter(|r| !r.trim().is_empty())
}

/// `AWS_ACCESS_KEY_ID` and friends. Both of the first two or nothing: half a credential is
/// not a credential, and falling back silently to the file would hide a typo in a variable.
pub fn from_env() -> Option<Credentials> {
    let id = std::env::var("AWS_ACCESS_KEY_ID").ok()?;
    let secret = std::env::var("AWS_SECRET_ACCESS_KEY").ok()?;
    if id.trim().is_empty() || secret.trim().is_empty() {
        return None;
    }
    let mut credentials = Credentials::new(id.trim(), secret.trim());
    if let Ok(token) = std::env::var("AWS_SESSION_TOKEN") {
        if !token.trim().is_empty() {
            credentials = credentials.with_session_token(token.trim());
        }
    }
    Some(credentials)
}

/// `$AWS_SHARED_CREDENTIALS_FILE`, else `~/.aws/credentials`.
pub fn credentials_file(home: Option<&Path>) -> Option<PathBuf> {
    if let Ok(path) = std::env::var("AWS_SHARED_CREDENTIALS_FILE") {
        if !path.trim().is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    Some(home?.join(".aws").join("credentials"))
}

pub fn from_aws_file(path: &Path, profile: &str) -> Option<Credentials> {
    let text = std::fs::read_to_string(path).ok()?;
    from_ini(&text, profile)
}

/// Read one profile out of an `aws` credentials file.
///
/// The format is INI as the AWS tools write it: `[profile]` headings, `key = value` lines,
/// `#` and `;` comments. A profile in that file may be written `[work]` or `[profile work]`,
/// the second being what `~/.aws/config` uses, and both are accepted because people copy one
/// into the other.
pub fn from_ini(text: &str, profile: &str) -> Option<Credentials> {
    let mut inside = false;
    let mut id = None;
    let mut secret = None;
    let mut token = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(heading) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            let heading = heading.trim();
            let heading = heading.strip_prefix("profile ").unwrap_or(heading).trim();
            if inside {
                // The next heading ends the profile we wanted.
                break;
            }
            inside = heading == profile;
            continue;
        }
        if !inside {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().to_string();
        match key.trim().to_ascii_lowercase().as_str() {
            "aws_access_key_id" => id = Some(value),
            "aws_secret_access_key" => secret = Some(value),
            "aws_session_token" => token = Some(value),
            _ => {}
        }
    }
    let (id, secret) = (id?, secret?);
    if id.is_empty() || secret.is_empty() {
        return None;
    }
    let mut credentials = Credentials::new(id, secret);
    if let Some(token) = token.filter(|t| !t.is_empty()) {
        credentials = credentials.with_session_token(token);
    }
    Some(credentials)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = r#"
# the one every tool uses
[default]
aws_access_key_id = AKIAIOSFODNN7EXAMPLE
aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY

[work]
aws_access_key_id = AKIAWORKWORKWORKWORK
aws_secret_access_key = workworkworkworkworkworkworkworkworkwork
aws_session_token = FwoGZXIvYXdzEExample

; a profile written the way ~/.aws/config writes it
[profile archive]
aws_access_key_id = AKIAARCHIVEARCHIVEAR
aws_secret_access_key = archivearchivearchivearchivearchivearch

[broken]
aws_access_key_id = AKIABROKEN
"#;

    #[test]
    fn a_profile_is_read_out_of_the_shared_file() {
        let default = from_ini(FILE, "default").unwrap();
        assert_eq!(default.access_key_id, "AKIAIOSFODNN7EXAMPLE");
        assert_eq!(
            default.secret_access_key,
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
        );
        assert_eq!(default.session_token, None);

        let work = from_ini(FILE, "work").unwrap();
        assert_eq!(work.access_key_id, "AKIAWORKWORKWORKWORK");
        assert_eq!(work.session_token.as_deref(), Some("FwoGZXIvYXdzEExample"));

        // `[profile archive]` is the same profile as `[archive]`.
        assert_eq!(
            from_ini(FILE, "archive").unwrap().access_key_id,
            "AKIAARCHIVEARCHIVEAR"
        );

        // Half a credential is none.
        assert!(from_ini(FILE, "broken").is_none());
        assert!(from_ini(FILE, "absent").is_none());
        assert!(from_ini("", "default").is_none());
    }

    /// The keys of one profile must not leak into another: the heading ends the block.
    #[test]
    fn a_profile_ends_where_the_next_begins() {
        let text = "[a]\naws_access_key_id = ID_A\n[b]\naws_secret_access_key = SECRET_B\n";
        assert!(from_ini(text, "a").is_none());
        assert!(from_ini(text, "b").is_none());
    }

    #[test]
    fn the_source_is_read_the_way_people_write_it() {
        assert_eq!(Source::parse(""), Source::Environment);
        assert_eq!(Source::parse("env"), Source::Environment);
        assert_eq!(Source::parse("ENV"), Source::Environment);
        assert_eq!(Source::parse("anonymous"), Source::Anonymous);
        assert_eq!(Source::parse("public"), Source::Anonymous);
        assert_eq!(
            Source::parse("aws"),
            Source::AwsProfile("default".to_string())
        );
        assert_eq!(
            Source::parse("aws:work"),
            Source::AwsProfile("work".to_string())
        );
        // A bare name is a profile name, which is what someone writing it means.
        assert_eq!(
            Source::parse("work"),
            Source::AwsProfile("work".to_string())
        );
        assert_eq!(Source::parse("aws:work").describe(), "aws:work");
        assert_eq!(Source::parse("env").describe(), "env");
    }

    #[test]
    fn an_anonymous_source_asks_for_nothing() {
        assert!(resolve(&Source::Anonymous, None).is_none());
        // And a profile with no file to read it from is not a crash.
        assert!(resolve(&Source::AwsProfile("default".into()), None).is_none());
    }

    #[test]
    fn a_file_on_disk_is_found_from_the_home_it_is_given() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join(".aws")).unwrap();
        std::fs::write(home.path().join(".aws").join("credentials"), FILE).unwrap();
        // `AWS_SHARED_CREDENTIALS_FILE` would win, and the test must not depend on whether
        // this account has it set.
        if std::env::var_os("AWS_SHARED_CREDENTIALS_FILE").is_none() {
            let found = resolve(&Source::AwsProfile("work".into()), Some(home.path())).unwrap();
            assert_eq!(found.access_key_id, "AKIAWORKWORKWORKWORK");
        }
        assert_eq!(
            credentials_file(Some(home.path())).unwrap().file_name(),
            Some(std::ffi::OsStr::new("credentials"))
        );
    }
}
