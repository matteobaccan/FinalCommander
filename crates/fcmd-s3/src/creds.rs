//! Where the keys come from.
//!
//! Not from the configuration file. A secret access key is a password, and this program has
//! never written a password into `fcmd.toml` — the netbox asks for one or uses the SSH agent,
//! and object storage reads the two places the rest of the world already keeps these: the
//! `AWS_*` environment variables, and the `~/.aws/credentials` file every AWS tool shares. A
//! key in a configuration file would be a key in a backup, in a dotfiles repository and in a
//! screen share, and the file it belongs in is one that already exists and is already
//! expected to be private.
//!
//! Beside it sits `~/.aws/config`, which holds what is not a secret about each profile — its
//! region above all — under `[profile name]` headings. The two are read together
//! ([`SharedFiles`]): the names in both are the profiles there are to choose from, the keys
//! come from whichever holds them, the region from the configuration. Both are INI as the
//! AWS tools write it, read by the few lines of [`parse_ini`] rather than by a crate: the
//! format is a heading, a key and a comment, and what it leaves out — nesting, quoting,
//! continuation — is exactly what a general parser would get differently from `botocore`.
//!
//! Only static keys are read. A profile that signs in another way — single sign-on, an
//! assumed role, a `credential_process` — is listed as a profile and said to hold no keys:
//! each of those is a protocol of its own, and the AWS command line is the tool that speaks
//! them (`aws configure export-credentials` writes the keys this program can then read from
//! the environment).

use std::path::{Path, PathBuf};

use crate::sign::Credentials;

/// Where to look.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` and, when it is there,
    /// `AWS_SESSION_TOKEN`. Falls back to the shared file, as every AWS tool does.
    Environment,
    /// A named profile of `~/.aws/credentials` and `~/.aws/config` (`default` when not
    /// given).
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
        // Not worth opening two files to be told there is nothing to find in them.
        Source::Anonymous => None,
        _ => SharedFiles::discover(home).resolve(source),
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

/// `$AWS_CONFIG_FILE`, else `~/.aws/config`: where a profile keeps what is not a secret —
/// its region above all.
pub fn config_file(home: Option<&Path>) -> Option<PathBuf> {
    if let Ok(path) = std::env::var("AWS_CONFIG_FILE") {
        if !path.trim().is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    Some(home?.join(".aws").join("config"))
}

/// One section of an INI file as the AWS tools write it: the heading as written between the
/// brackets, and its `key = value` lines in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub heading: String,
    pub pairs: Vec<(String, String)>,
}

impl Section {
    /// The value of a key, whatever case it was written in; the last one wins, as it does
    /// for the tools that write these files.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.pairs
            .iter()
            .rev()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }
}

/// Read an INI file the way the AWS tools do.
///
/// `[heading]` lines, `key = value` lines, and comments: a whole line beginning with `#` or
/// `;`, and whatever follows a `#` or `;` that has white space before it at the end of a
/// value (`region = eu-west-1  # Dublin`). An indented line under a key with nothing after
/// its `=` is a nested setting (`s3 =` followed by `  addressing_style = path`), which is
/// nobody's business here and is skipped rather than read as a key of the profile. Lines
/// before the first heading belong to no section and are dropped. Nothing is ever an error:
/// a file somebody else's tool wrote is read for what can be read in it.
pub fn parse_ini(text: &str) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    // Is the last key one with an empty value, so that indented lines under it are its own?
    let mut nested = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[') {
            // A comment may follow the closing bracket.
            if let Some((heading, _)) = rest.split_once(']') {
                sections.push(Section {
                    heading: heading.split_whitespace().collect::<Vec<_>>().join(" "),
                    pairs: Vec::new(),
                });
                nested = false;
                continue;
            }
        }
        let indented = raw.starts_with([' ', '\t']);
        if indented && nested {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = strip_comment(value).trim().to_string();
        nested = value.is_empty();
        if let Some(section) = sections.last_mut() {
            section.pairs.push((key.trim().to_string(), value));
        }
    }
    sections
}

/// A value without the comment after it. Only a `#` or `;` with white space in front begins
/// one: a secret key or a token may hold either character, and never holds a space.
fn strip_comment(value: &str) -> &str {
    let bytes = value.as_bytes();
    for at in 1..bytes.len() {
        if matches!(bytes[at], b'#' | b';') && matches!(bytes[at - 1], b' ' | b'\t') {
            return &value[..at];
        }
    }
    value
}

/// Which file a section was read from, because the two name their profiles differently:
/// `[work]` in the credentials file is `[profile work]` in the configuration, where a bare
/// `[work]` is not a profile at all — and `[sso-session …]` and `[services …]` never are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shared {
    Credentials,
    Config,
}

/// The profile a heading names in that file, if it names one.
fn profile_of(heading: &str, file: Shared) -> Option<&str> {
    let heading = heading.trim();
    match heading.strip_prefix("profile ") {
        // People copy sections from one file into the other, so `[profile x]` is read in
        // the credentials file too, as `from_ini` always has.
        Some(name) => Some(name.trim()).filter(|n| !n.is_empty()),
        None if heading.contains(' ') => None,
        None => match file {
            Shared::Credentials => Some(heading),
            Shared::Config => (heading == "default").then_some(heading),
        },
    }
    .filter(|n| !n.is_empty())
}

/// One profile of the shared files, as a row to choose from: never its keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub name: String,
    /// The region its section of the configuration names.
    pub region: Option<String>,
    /// Does it hold an access key and its secret? One that signs in another way — single
    /// sign-on, a role, a program asked for the keys — is a profile all the same and is
    /// listed, but there is nothing in it this program can sign with.
    pub has_keys: bool,
}

/// What the two shared files hold, read once.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SharedFiles {
    credentials: Vec<Section>,
    config: Vec<Section>,
}

impl SharedFiles {
    /// From the text of the two files; either may be empty, which is a file that is not
    /// there.
    pub fn parse(credentials: &str, config: &str) -> SharedFiles {
        SharedFiles {
            credentials: parse_ini(credentials),
            config: parse_ini(config),
        }
    }

    /// From the two files themselves. One that is missing or unreadable is an empty one.
    pub fn read(credentials: Option<&Path>, config: Option<&Path>) -> SharedFiles {
        let text = |path: Option<&Path>| {
            path.and_then(|p| std::fs::read_to_string(p).ok())
                .unwrap_or_default()
        };
        SharedFiles::parse(&text(credentials), &text(config))
    }

    /// From where the AWS tools keep them: `AWS_SHARED_CREDENTIALS_FILE` and
    /// `AWS_CONFIG_FILE` when they are set, `~/.aws/` under `home` otherwise.
    pub fn discover(home: Option<&Path>) -> SharedFiles {
        SharedFiles::read(
            credentials_file(home).as_deref(),
            config_file(home).as_deref(),
        )
    }

    fn sections<'a>(&'a self, profile: &'a str) -> impl Iterator<Item = &'a Section> + 'a {
        let of = move |sections: &'a [Section], file: Shared| {
            sections
                .iter()
                .filter(move |s| profile_of(&s.heading, file) == Some(profile))
        };
        // The credentials file first: where both hold a key, it is the one that wins.
        of(&self.credentials, Shared::Credentials).chain(of(&self.config, Shared::Config))
    }

    fn value(&self, profile: &str, key: &str) -> Option<String> {
        self.sections(profile)
            .filter_map(|s| s.get(key))
            .map(str::to_string)
            .find(|v| !v.is_empty())
    }

    /// The keys of a profile: from the credentials file, else from the configuration, where
    /// the AWS tools accept them too. Both halves from the same section, or nothing — an id
    /// from one file signed with a secret from the other is a request refused far away.
    pub fn credentials(&self, profile: &str) -> Option<Credentials> {
        self.sections(profile).find_map(|section| {
            let id = section.get("aws_access_key_id").filter(|v| !v.is_empty())?;
            let secret = section
                .get("aws_secret_access_key")
                .filter(|v| !v.is_empty())?;
            let credentials = Credentials::new(id, secret);
            Some(
                match section.get("aws_session_token").filter(|t| !t.is_empty()) {
                    Some(token) => credentials.with_session_token(token),
                    None => credentials,
                },
            )
        })
    }

    /// The region a profile names, which is the configuration's to say.
    pub fn region(&self, profile: &str) -> Option<String> {
        self.value(profile, "region")
    }

    /// Every profile either file names: `default` first when it is there, the others by
    /// name.
    pub fn profiles(&self) -> Vec<Profile> {
        let mut names: Vec<&str> = self
            .credentials
            .iter()
            .filter_map(|s| profile_of(&s.heading, Shared::Credentials))
            .chain(
                self.config
                    .iter()
                    .filter_map(|s| profile_of(&s.heading, Shared::Config)),
            )
            .collect();
        names.sort_by_key(|n| (*n != "default", *n));
        names.dedup();
        names
            .into_iter()
            .map(|name| Profile {
                name: name.to_string(),
                region: self.region(name),
                has_keys: self.credentials(name).is_some(),
            })
            .collect()
    }

    /// The profile a source stands for, when it stands for one: the one it names, or for the
    /// environment the one `AWS_PROFILE` names — `default` when it names none.
    fn profile_for(source: &Source) -> Option<String> {
        match source {
            Source::Anonymous => None,
            Source::AwsProfile(profile) => Some(profile.clone()),
            Source::Environment => Some(
                std::env::var("AWS_PROFILE")
                    .ok()
                    .filter(|p| !p.trim().is_empty())
                    .unwrap_or_else(|| "default".into()),
            ),
        }
    }

    /// The keys a source gives: the environment's own when that is the source and it has
    /// them, a profile's otherwise.
    pub fn resolve(&self, source: &Source) -> Option<Credentials> {
        match source {
            Source::Anonymous => None,
            Source::Environment => {
                from_env().or_else(|| self.credentials(&SharedFiles::profile_for(source)?))
            }
            Source::AwsProfile(profile) => self.credentials(profile),
        }
    }

    /// The region to sign for when the service's own block names none: the environment's,
    /// then the profile's own from the configuration file — the order the AWS tools keep, so
    /// a shell that sets `AWS_REGION` means here what it means to them.
    pub fn region_for(&self, source: &Source) -> Option<String> {
        self.region_with(source, region_from_env())
    }

    /// The same with the environment's answer handed in, for a test that must not depend on
    /// the shell it runs in.
    pub fn region_with(&self, source: &Source, env: Option<String>) -> Option<String> {
        env.or_else(|| SharedFiles::profile_for(source).and_then(|p| self.region(&p)))
    }
}

/// Read one profile out of an `aws` credentials file.
///
/// The format is INI as the AWS tools write it: `[profile]` headings, `key = value` lines,
/// `#` and `;` comments. A profile in that file may be written `[work]` or `[profile work]`,
/// the second being what `~/.aws/config` uses, and both are accepted because people copy one
/// into the other.
pub fn from_ini(text: &str, profile: &str) -> Option<Credentials> {
    SharedFiles::parse(text, "").credentials(profile)
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

    const CONFIG: &str = r#"
[default]
region = eu-south-1
output = json

[profile work]
region = eu-west-1   # Dublin
s3 =
    addressing_style = path
    region = not-a-region

[profile sso]
sso_session = corp
sso_account_id = 123456789012
region = us-west-2

[profile keys-here]
aws_access_key_id = AKIACONFIGCONFIGCONF
aws_secret_access_key = configconfigconfigconfigconfigconfigconf

; not profiles, whatever they are called
[sso-session corp]
sso_region = us-east-1
[services local]
[stray]
region = nowhere
"#;

    #[test]
    fn an_ini_file_is_read_section_by_section() {
        let sections = parse_ini(
            "orphan = dropped\n# comment\n[ one ]\nA = 1\na = 2 ; twice\n\n; x\n[profile   two] # tail\nkey=va=lue\nnothing\nempty =\n",
        );
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].heading, "one");
        // The last of two wins, in whatever case the key was written.
        assert_eq!(sections[0].get("a"), Some("2"));
        assert_eq!(sections[1].heading, "profile two");
        // Only the first `=` separates; a line with none is not a key.
        assert_eq!(sections[1].get("KEY"), Some("va=lue"));
        assert_eq!(sections[1].get("nothing"), None);
        assert_eq!(sections[1].get("empty"), Some(""));
        assert!(parse_ini("").is_empty());
        assert!(parse_ini("[never closed\nx = 1\n").is_empty());
    }

    /// A secret may hold a `#` or a `;`, and only one with a blank before it starts a comment.
    #[test]
    fn a_comment_after_a_value_needs_a_blank_before_it() {
        let sections = parse_ini("[a]\nsecret = ab#cd;ef\nregion = eu-west-1\t; Dublin\n");
        assert_eq!(sections[0].get("secret"), Some("ab#cd;ef"));
        assert_eq!(sections[0].get("region"), Some("eu-west-1"));
    }

    #[test]
    fn the_profiles_of_both_files_are_one_list_default_first() {
        let shared = SharedFiles::parse(FILE, CONFIG);
        let profiles = shared.profiles();
        let names: Vec<&str> = profiles.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            ["default", "archive", "broken", "keys-here", "sso", "work"],
            "no `sso-session`, no `services`, and no bare `[stray]` of the configuration"
        );
        let of = |name: &str| profiles.iter().find(|p| p.name == name).unwrap();
        assert_eq!(of("default").region.as_deref(), Some("eu-south-1"));
        assert!(of("default").has_keys);
        // The nested `region` under `s3 =` is not the profile's.
        assert_eq!(of("work").region.as_deref(), Some("eu-west-1"));
        assert!(of("work").has_keys);
        // A profile that signs in some other way is listed, and said to have no keys.
        assert!(!of("sso").has_keys);
        assert_eq!(of("sso").region.as_deref(), Some("us-west-2"));
        assert!(!of("broken").has_keys);
        assert_eq!(of("archive").region, None);
        // Keys kept in the configuration file are keys.
        assert!(of("keys-here").has_keys);
        assert_eq!(
            shared.credentials("keys-here").unwrap().access_key_id,
            "AKIACONFIGCONFIGCONF"
        );
        assert!(SharedFiles::default().profiles().is_empty());
    }

    /// Where both files hold keys for a profile the credentials file wins, and the two
    /// halves always come from one section.
    #[test]
    fn the_credentials_file_wins_and_a_key_is_never_assembled_from_two_places() {
        let credentials = "[work]\naws_access_key_id = FROM_CREDENTIALS\naws_secret_access_key = S1\n[half]\naws_access_key_id = ONLY_ID\n";
        let config = "[profile work]\naws_access_key_id = FROM_CONFIG\naws_secret_access_key = S2\n[profile half]\naws_secret_access_key = ONLY_SECRET\n";
        let shared = SharedFiles::parse(credentials, config);
        assert_eq!(
            shared.credentials("work").unwrap().access_key_id,
            "FROM_CREDENTIALS"
        );
        assert!(shared.credentials("half").is_none());
    }

    #[test]
    fn the_region_is_the_environments_and_then_the_profiles() {
        let shared = SharedFiles::parse(FILE, CONFIG);
        let work = Source::AwsProfile("work".into());
        assert_eq!(
            shared.region_with(&work, None).as_deref(),
            Some("eu-west-1")
        );
        assert_eq!(
            shared
                .region_with(&work, Some("ap-south-1".into()))
                .as_deref(),
            Some("ap-south-1"),
            "the order the AWS tools keep"
        );
        assert_eq!(
            shared.region_with(&Source::AwsProfile("archive".into()), None),
            None
        );
        assert_eq!(shared.region_with(&Source::Anonymous, None), None);
        assert_eq!(
            shared
                .resolve(&Source::AwsProfile("work".into()))
                .unwrap()
                .access_key_id,
            "AKIAWORKWORKWORKWORK"
        );
        assert!(shared.resolve(&Source::AwsProfile("sso".into())).is_none());
        assert!(shared.resolve(&Source::Anonymous).is_none());
    }

    /// The files are read from where they are said to be — a temporary folder here, never
    /// this account's own — and one that is not there is an empty one.
    #[test]
    fn the_two_files_are_read_from_the_paths_they_are_given() {
        let home = tempfile::tempdir().unwrap();
        let credentials = home.path().join("creds-elsewhere");
        let config = home.path().join("config-elsewhere");
        std::fs::write(&credentials, FILE).unwrap();
        std::fs::write(&config, CONFIG).unwrap();
        let shared = SharedFiles::read(Some(&credentials), Some(&config));
        assert_eq!(shared.profiles().len(), 6);
        assert_eq!(shared.region("default").as_deref(), Some("eu-south-1"));

        let only_keys = SharedFiles::read(Some(&credentials), Some(&home.path().join("absent")));
        let names: Vec<String> = only_keys.profiles().into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["default", "archive", "broken", "work"]);
        assert!(only_keys.profiles().iter().all(|p| p.region.is_none()));
        assert!(SharedFiles::read(None, None).profiles().is_empty());
    }

    #[test]
    fn the_configuration_file_is_found_under_the_home_it_is_given() {
        let home = tempfile::tempdir().unwrap();
        // `AWS_CONFIG_FILE` would win, and the test must not depend on whether this account
        // has it set.
        if std::env::var_os("AWS_CONFIG_FILE").is_none() {
            assert_eq!(
                config_file(Some(home.path())),
                Some(home.path().join(".aws").join("config"))
            );
            assert_eq!(config_file(None), None);
        }
    }
}
