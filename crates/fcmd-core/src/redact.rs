//! Secrets taken out of a text before it leaves the machine (SPEC 06 §14.2).
//!
//! What is sent to a language model is whatever the files hold, and files hold keys: an
//! `.env` beside the sources, a token pasted into a script, a private key in a folder that
//! was selected whole. This reads a text for the shapes such things have and writes
//! `[REDACTED:kind]` where each one was, counting them, so the dialog that says what goes can
//! also say how much of it was masked.
//!
//! It errs towards masking. A value that only looks like a secret costs the model a word it
//! could have read; a secret that looked like a value is on somebody else's disk for good.
//! It is still a net and not a guarantee: a password that is an ordinary word in an ordinary
//! sentence has no shape to find, which is why the patterns can be added to
//! (`[ai_guard] patterns`).

use std::collections::BTreeMap;

use regex::{Captures, Regex};

/// What a text became: itself with the secrets masked, how many there were, and how many of
/// each kind — `aws-key`, `private-key`, `url-password`, … — for a line that says so.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Redacted {
    pub text: String,
    pub count: usize,
    pub kinds: BTreeMap<String, usize>,
}

impl Redacted {
    /// `2 aws-key, 1 private-key`: the kinds with their counts, for the dialog and the
    /// preview.
    pub fn summary(kinds: &BTreeMap<String, usize>) -> String {
        kinds
            .iter()
            .map(|(kind, n)| format!("{n} {kind}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// One shape of secret: what it is called, how it is found, and which group of the match is
/// the secret itself — 0 for all of it, so that `password=` stays and only its value goes.
#[derive(Debug, Clone)]
struct Rule {
    kind: String,
    find: Regex,
    group: usize,
}

/// The reader: the shapes this program knows, and the user's own after them.
#[derive(Debug, Clone)]
pub struct Redactor {
    rules: Vec<Rule>,
    assignment: Regex,
}

/// What stands where a secret was. The kind is said, because `[REDACTED]` alone in the
/// middle of a configuration leaves the model — and the reader of the preview — guessing
/// what was there.
fn mask(kind: &str) -> String {
    format!("[REDACTED:{kind}]")
}

/// The tokens that say what they are by how they start. Each of these is the service's own
/// documented prefix, so a match is a key and not a word.
const TOKENS: &[(&str, &str)] = &[
    // An unfinished block — a file cut where it was read — is masked to its end.
    (
        "private-key",
        r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY(?: BLOCK)?-----[\s\S]*?(?:-----END [A-Z0-9 ]*PRIVATE KEY(?: BLOCK)?-----|\z)",
    ),
    (
        "aws-key",
        r"\b(?:AKIA|ASIA|AGPA|AIDA|AROA|ANPA)[0-9A-Z]{16}\b",
    ),
    ("github-token", r"\bgh[pousr]_[A-Za-z0-9]{36,255}\b"),
    ("github-token", r"\bgithub_pat_[A-Za-z0-9_]{22,255}\b"),
    ("gitlab-token", r"\bglpat-[A-Za-z0-9_\-]{20,}"),
    ("slack-token", r"\bxox[abeoprs]-[A-Za-z0-9\-]{10,}"),
    ("stripe-key", r"\b[sr]k_(?:live|test)_[0-9A-Za-z]{16,}\b"),
    // OpenAI's `sk-…` and `sk-proj-…`, Anthropic's `sk-ant-…`, and the many that copy them.
    ("api-key", r"\bsk-[A-Za-z0-9_\-]{20,}"),
    ("google-key", r"\bAIza[0-9A-Za-z_\-]{35}"),
    ("npm-token", r"\bnpm_[A-Za-z0-9]{36}\b"),
    // Three base64url parts, the first two starting as `{"` encodes.
    (
        "jwt",
        r"\beyJ[A-Za-z0-9_\-]{8,}\.eyJ[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{8,}",
    ),
];

impl Redactor {
    /// The built-in shapes, and `extra` — the user's own regular expressions — after them.
    /// A pattern of the user's masks its first group when it has one, and otherwise all it
    /// matched. One that is not a regular expression is left out and named in the second
    /// part, for a warning; the rest still work.
    pub fn new(extra: &[String]) -> (Redactor, Vec<String>) {
        let built = |pattern: &str| Regex::new(pattern).expect("a pattern of our own");
        let mut rules: Vec<Rule> = TOKENS
            .iter()
            .map(|(kind, pattern)| Rule {
                kind: (*kind).to_string(),
                find: built(pattern),
                group: 0,
            })
            .collect();
        // `scheme://user:password@host`: the password, and only it — the host and the user
        // are what make the line worth sending.
        rules.push(Rule {
            kind: "url-password".into(),
            find: built(r"[A-Za-z][A-Za-z0-9+.\-]*://[^\s/:@]*:([^\s/@]+)@"),
            group: 1,
        });
        // A header as it is written in a script or a log: `Authorization: Bearer …`.
        rules.push(Rule {
            kind: "auth-header".into(),
            find: built(
                r"(?i)\bauthorization\b[\x22']?\s*[:=]\s*[\x22']?(?:bearer|basic|token)\s+([A-Za-z0-9._~+/=\-]{8,})",
            ),
            group: 1,
        });
        rules.push(Rule {
            kind: "auth-header".into(),
            find: built(r"(?i)\bbearer\s+([A-Za-z0-9._~+/=\-]{20,})"),
            group: 1,
        });
        let mut bad = Vec::new();
        for pattern in extra {
            let pattern = pattern.trim();
            if pattern.is_empty() {
                continue;
            }
            match Regex::new(pattern) {
                Ok(find) => {
                    let group = usize::from(find.captures_len() > 1);
                    rules.push(Rule {
                        kind: "custom".into(),
                        find,
                        group,
                    });
                }
                Err(e) => bad.push(format!("{pattern}: {e}")),
            }
        }
        // `KEY=value`, `key: value` and `"key": "value"` at the start of a line, where the
        // key's name says the value is a secret. The name must end in the word (or its
        // plural), so that `TOKENIZER` and `password_hint_shown` are left alone.
        let assignment = built(
            r#"(?im)^([ \t]*(?:export[ \t]+|set[ \t]+|setx[ \t]+)?["']?([A-Za-z0-9_.\-]*?(?:password|passwd|passphrase|pwd|secret|token|api[_\-]?key|access[_\-]?key|private[_\-]?key|auth[_\-]?key|credential)s?)["']?[ \t]*[:=][ \t]*)([^\r\n]+)"#,
        );
        (Redactor { rules, assignment }, bad)
    }

    /// The text with every secret found in it masked.
    pub fn redact(&self, text: &str) -> Redacted {
        let mut out = Redacted {
            text: text.to_string(),
            ..Redacted::default()
        };
        for rule in &self.rules {
            // The common case is a text with nothing in it: no copy is made for a rule that
            // finds nothing.
            if !rule.find.is_match(&out.text) {
                continue;
            }
            let mut found = 0;
            let masked = rule
                .find
                .replace_all(&out.text, |caps: &Captures| {
                    let all = caps.get(0).expect("group 0 is the match");
                    let Some(secret) = caps.get(rule.group) else {
                        return all.as_str().to_string();
                    };
                    // Masked already by a rule before this one: counted once.
                    if secret.as_str().contains("[REDACTED:") || secret.as_str().is_empty() {
                        return all.as_str().to_string();
                    }
                    found += 1;
                    let (from, to) = (secret.start() - all.start(), secret.end() - all.start());
                    format!(
                        "{}{}{}",
                        &all.as_str()[..from],
                        mask(&rule.kind),
                        &all.as_str()[to..]
                    )
                })
                .into_owned();
            if found > 0 {
                out.text = masked;
                out.count += found;
                *out.kinds.entry(rule.kind.clone()).or_default() += found;
            }
        }
        self.assignments(&mut out);
        out
    }

    /// The `.env` shape, which needs more than a pattern: the value is looked at before it
    /// is masked, because `password = prompt()` is a line of a program and `PWD=/home/mb`
    /// is where the shell is.
    fn assignments(&self, out: &mut Redacted) {
        if !self.assignment.is_match(&out.text) {
            return;
        }
        let mut found = 0;
        let masked = self
            .assignment
            .replace_all(&out.text, |caps: &Captures| {
                let (head, name, rest) = (&caps[1], &caps[2], &caps[3]);
                let Some((before, after)) = secret_value(name, rest) else {
                    return caps[0].to_string();
                };
                found += 1;
                format!("{head}{before}{}{after}", mask("assignment"))
            })
            .into_owned();
        if found > 0 {
            out.text = masked;
            out.count += found;
            *out.kinds.entry("assignment".into()).or_default() += found;
        }
    }
}

/// The value of an assignment, when it is one to mask: what stands before it on the line
/// (an opening quote) and what follows it (the closing quote, a comma, a comment).
/// `None` for what is not a secret written down: nothing at all, a reference to a variable,
/// a call, a number, a word like `true`.
fn secret_value<'a>(name: &str, rest: &'a str) -> Option<(&'a str, &'a str)> {
    // The shell's own, which every dump of an environment has.
    if matches!(name.to_ascii_uppercase().as_str(), "PWD" | "OLDPWD") {
        return None;
    }
    // The blanks at the end of the line stay where they were, after whatever follows the
    // value.
    let line = rest.trim_end();
    let (before, value, after) = match line.chars().next()? {
        quote @ ('"' | '\'') => {
            let end = line[1..].find(quote)?;
            (&rest[..1], &line[1..1 + end], &rest[1 + end..])
        }
        _ => {
            // Up to a trailing comma or semicolon, or a comment after a blank.
            let cut = line.find(" #").unwrap_or(line.len());
            let value = line[..cut].trim_end().trim_end_matches([',', ';']);
            (&rest[..0], value, &rest[value.len()..])
        }
    };
    let quoted = !before.is_empty();
    if value.is_empty() || value.contains("[REDACTED:") {
        return None;
    }
    // A reference says where the secret is, not what it is.
    if value.starts_with(['$', '%', '<']) || value.starts_with("{{") {
        return None;
    }
    let lower = value.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "true" | "false" | "null" | "none" | "nil" | "yes" | "no" | "on" | "off" | "changeme"
    ) {
        return None;
    }
    // A count, a port, a length: `max_tokens = 4096`.
    if value.len() < 8 && value.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    // Not quoted and it reads as a line of a program: a call, a path through a module, a
    // type with its parameters, several words.
    if !quoted
        && (value.contains(['(', ')', '{', '}', '<', '>'])
            || value.contains("::")
            || value.contains("->")
            || value.split_whitespace().count() > 1)
    {
        return None;
    }
    Some((before, after))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain() -> Redactor {
        Redactor::new(&[]).0
    }

    /// Built here rather than written out, so that this file has no line in it that a
    /// scanner of repositories would take for a leaked key.
    fn aws() -> String {
        format!("AKIA{}", "IOSFODNN7EXAMPLE")
    }
    fn github() -> String {
        format!("ghp_{}", "a1B2c3D4e5F6g7H8i9J0k1L2m3N4o5P6q7R8")
    }

    #[test]
    fn a_text_with_no_secret_in_it_comes_back_as_it_was() {
        let text = "fn main() {\n    println!(\"hello\");\n}\n# a heading\nport = 8080\n";
        let got = plain().redact(text);
        assert_eq!(got.text, text);
        assert_eq!(got.count, 0);
        assert!(got.kinds.is_empty());
    }

    #[test]
    fn the_keys_that_say_what_they_are_are_masked_by_their_kind() {
        let text = format!(
            "aws {}\ngithub {}\nopenai sk-proj-{}\nanthropic sk-ant-api03-{}\n\
             google AIza{}\nslack xoxb-1234567890-abcdefghij\nstripe sk_live_{}\n\
             gitlab glpat-{}\n",
            aws(),
            github(),
            "Zx9".repeat(12),
            "Qw8".repeat(14),
            "SyB".repeat(11) + "ab",
            "4eC39HqLyjWDarjtT1zdp7dc",
            "xY7".repeat(7),
        );
        let got = plain().redact(&text);
        assert_eq!(got.count, 8, "{}", got.text);
        for kind in [
            "aws-key",
            "github-token",
            "api-key",
            "google-key",
            "slack-token",
            "stripe-key",
            "gitlab-token",
        ] {
            assert!(got.text.contains(&mask(kind)), "{kind}: {}", got.text);
        }
        assert_eq!(got.kinds["api-key"], 2);
        assert!(!got.text.contains(&aws()) && !got.text.contains(&github()));
        // What says which line is which is still there for the model to read.
        assert!(got.text.contains("aws [REDACTED:aws-key]"));
    }

    #[test]
    fn a_private_key_block_is_masked_whole_and_counted_once() {
        let text = "before\n-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAA\n\
                    QyNTUxOQAAACD\n-----END OPENSSH PRIVATE KEY-----\nafter\n";
        let got = plain().redact(text);
        assert_eq!(got.text, "before\n[REDACTED:private-key]\nafter\n");
        assert_eq!(got.count, 1);
        // And one the file was cut in the middle of, to its end.
        let cut = "x\n-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA\n";
        assert_eq!(plain().redact(cut).text, "x\n[REDACTED:private-key]");
        // A public key is not a secret.
        let public = "-----BEGIN PUBLIC KEY-----\nMIIB\n-----END PUBLIC KEY-----\n";
        assert_eq!(plain().redact(public).count, 0);
    }

    #[test]
    fn the_password_in_an_address_goes_and_the_rest_of_the_address_stays() {
        let text = "DATABASE_URL=postgres://app:s3cr3t-pw@db.internal:5432/shop\n\
                    git clone https://mb:hunter2hunter2@example.org/repo.git\n\
                    see https://example.org/a:b and mailto:someone@example.org\n";
        let got = plain().redact(text);
        assert_eq!(got.count, 2, "{}", got.text);
        assert!(got
            .text
            .contains("postgres://app:[REDACTED:url-password]@db.internal:5432/shop"));
        assert!(got
            .text
            .contains("https://mb:[REDACTED:url-password]@example.org/repo.git"));
        assert!(got.text.contains("https://example.org/a:b"));
    }

    #[test]
    fn an_env_file_loses_the_values_of_its_secret_looking_keys_only() {
        let text = "# settings\n\
                    APP_NAME=shop\n\
                    DB_PASSWORD=correct-horse\n\
                    export API_KEY=\"abc123def456\"  \n\
                    SECRET_TOKEN='tok tok'\n\
                    aws_secret_access_key = wJalrXUtnFEMIK7MDENGbPxRfiCY\n\
                    PORT=8080\n\
                    PWD=/home/mb\n\
                    MAX_TOKENS=4096\n\
                    TOKENIZER=bpe\n\
                    EMPTY_PASSWORD=\n\
                    FROM_VAULT_PASSWORD=${VAULT_PW}\n";
        let got = plain().redact(text);
        assert_eq!(got.count, 4, "{}", got.text);
        assert!(got.text.contains("DB_PASSWORD=[REDACTED:assignment]\n"));
        assert!(got
            .text
            .contains("export API_KEY=\"[REDACTED:assignment]\""));
        assert!(got.text.contains("SECRET_TOKEN='[REDACTED:assignment]'"));
        assert!(got
            .text
            .contains("aws_secret_access_key = [REDACTED:assignment]"));
        for kept in [
            "APP_NAME=shop",
            "PORT=8080",
            "PWD=/home/mb",
            "MAX_TOKENS=4096",
            "TOKENIZER=bpe",
            "EMPTY_PASSWORD=\n",
            "FROM_VAULT_PASSWORD=${VAULT_PW}",
        ] {
            assert!(got.text.contains(kept), "{kept}: {}", got.text);
        }
    }

    #[test]
    fn yaml_and_json_keys_are_read_the_same_way() {
        let text = "db:\n  password: hunter2hunter2\n  user: app\n{\n  \"api_key\": \"k-123456789\",\n  \"name\": \"x\"\n}\n";
        let got = plain().redact(text);
        assert_eq!(got.count, 2, "{}", got.text);
        assert!(got.text.contains("  password: [REDACTED:assignment]\n"));
        assert!(got
            .text
            .contains("  \"api_key\": \"[REDACTED:assignment]\",\n"));
        assert!(got.text.contains("user: app") && got.text.contains("\"name\": \"x\""));
    }

    #[test]
    fn a_line_of_a_program_about_a_password_is_not_a_password() {
        let text = "let password = prompt();\n\
                    password = read_line(stdin)\n\
                    self.token = Token::new()\n\
                    token: Option<String>,\n\
                    password = some words here\n";
        let got = plain().redact(text);
        assert_eq!(got.count, 0, "{}", got.text);
        assert_eq!(got.text, text);
    }

    #[test]
    fn an_authorization_header_keeps_its_scheme_and_loses_its_credential() {
        let text = "curl -H \"Authorization: Bearer abcdefgh12345678\" https://example.org\n\
                    Authorization: Basic dXNlcjpwYXNzd29yZA==\n\
                    the bearer of this letter\n";
        let got = plain().redact(text);
        assert_eq!(got.count, 2, "{}", got.text);
        assert!(got
            .text
            .contains("Authorization: Bearer [REDACTED:auth-header]\""));
        assert!(got
            .text
            .contains("Authorization: Basic [REDACTED:auth-header]"));
        assert!(got.text.contains("the bearer of this letter"));
    }

    #[test]
    fn a_secret_two_rules_would_find_is_counted_once() {
        // A known token as the value of a secret-looking key, and a JWT after `Bearer`.
        let jwt = format!(
            "eyJ{}.eyJ{}.{}",
            "hbGciOiJIUzI1NiJ9", "zdWIiOiIxMjM0NTY3ODkwIn0", "dozjgNryP4J3jVmNHl0w5N"
        );
        let text = format!("GITHUB_TOKEN={}\nAuthorization: Bearer {jwt}\n", github());
        let got = plain().redact(&text);
        assert_eq!(got.count, 2, "{}", got.text);
        assert!(got.text.contains("GITHUB_TOKEN=[REDACTED:github-token]"));
        assert!(got.text.contains("Bearer [REDACTED:jwt]"));
    }

    #[test]
    fn the_users_own_patterns_are_added_and_a_broken_one_is_named() {
        let (r, bad) = Redactor::new(&[
            r"ACME-[0-9]{6}".to_string(),
            r"customer id: (\d+)".to_string(),
            r"(unclosed".to_string(),
            "  ".to_string(),
        ]);
        assert_eq!(bad.len(), 1);
        assert!(bad[0].starts_with("(unclosed"), "{bad:?}");
        let got = r.redact("licence ACME-123456, customer id: 9981\n");
        assert_eq!(
            got.text,
            "licence [REDACTED:custom], customer id: [REDACTED:custom]\n"
        );
        assert_eq!(got.count, 2);
        assert_eq!(Redacted::summary(&got.kinds), "2 custom");
    }

    #[test]
    fn the_summary_names_each_kind_with_its_count() {
        let text = format!("{}\n{}\nPASSWORD=longenough\n", aws(), aws());
        let got = plain().redact(&text);
        assert_eq!(Redacted::summary(&got.kinds), "1 assignment, 2 aws-key");
    }
}
