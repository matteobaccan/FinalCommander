use std::path::PathBuf;
use std::time::Duration;

use fcmd_core::{Scheme, VPath};

use crate::error::{NetError, Result};

/// How we prove who we are. Passwords and passphrases are never stored here: a site says
/// *how* to authenticate, and the secret is asked for when it is needed (`docs/spec/04-vfs.md`
/// §6).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Auth {
    /// The running SSH agent (`SSH_AUTH_SOCK`, or Pageant on Windows).
    #[default]
    Agent,
    /// A private key file. The passphrase, if the key has one, is asked for.
    Key { path: PathBuf },
    /// Ask for a password every time.
    Password,
}

impl Auth {
    /// `agent`, `key:~/.ssh/id_ed25519`, `ask`/`password`.
    pub fn parse(text: &str) -> Result<Auth> {
        let text = text.trim();
        if let Some(rest) = text.strip_prefix("key:") {
            let path = expand_tilde(rest.trim());
            if path.as_os_str().is_empty() {
                return Err(NetError::Connect {
                    host: "config".into(),
                    detail: "auth = \"key:\" needs a path".into(),
                });
            }
            return Ok(Auth::Key { path });
        }
        match text.to_ascii_lowercase().as_str() {
            "agent" | "" => Ok(Auth::Agent),
            "ask" | "password" => Ok(Auth::Password),
            other => Err(NetError::Connect {
                host: "config".into(),
                detail: format!("unknown auth `{other}` (agent | key:<path> | ask)"),
            }),
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Auth::Agent => "agent".into(),
            Auth::Key { path } => format!("key:{}", path.display()),
            Auth::Password => "ask".into(),
        }
    }
}

/// One entry of the connection manager's site list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    pub id: String,
    pub scheme: Scheme,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: Auth,
    /// Where the panel starts. `/` when the site does not say.
    pub start_dir: String,
    /// Group heading in the site list, if any.
    pub group: Option<String>,
    pub keepalive: Option<Duration>,
    pub compress: bool,
}

pub const DEFAULT_SSH_PORT: u16 = 22;

impl Site {
    pub fn new(host: impl Into<String>) -> Site {
        Site {
            id: String::new(),
            scheme: Scheme::Sftp,
            host: host.into(),
            port: DEFAULT_SSH_PORT,
            user: current_user(),
            auth: Auth::Agent,
            start_dir: "/".into(),
            group: None,
            keepalive: Some(Duration::from_secs(30)),
            compress: false,
        }
    }

    /// `user@host:port` — what a [`VPath`] carries.
    pub fn authority(&self) -> String {
        format!("{}@{}:{}", self.user, self.host, self.port)
    }

    /// `sftp://user@host:port` — the key a connection is pooled under.
    ///
    /// The scheme is part of it because the authority alone is not enough: a server may offer
    /// both `ftp` and `ftps` on port 21, and those are two connections, not one. It is also
    /// what the site list and the drive list show for a host connected by hand, which is why
    /// it is a URL and not a tuple.
    pub fn key(&self) -> String {
        format!("{}://{}", self.scheme.as_str(), self.authority())
    }

    /// The same, for a path rather than a site.
    pub fn key_of(path: &VPath) -> Option<String> {
        Some(format!(
            "{}://{}",
            path.scheme().as_str(),
            path.authority()?
        ))
    }

    /// The path a panel opens when the site is connected.
    pub fn start(&self) -> VPath {
        VPath::remote(self.scheme, self.authority(), &self.start_dir)
    }

    /// The root of the site's filesystem.
    pub fn root(&self) -> VPath {
        VPath::remote(self.scheme, self.authority(), "/")
    }

    /// `sftp://mb@host.example:2222/srv` — the whole site in one string.
    pub fn url(&self) -> String {
        format!(
            "{}://{}{}",
            self.scheme.as_str(),
            self.authority(),
            self.start_dir
        )
    }

    /// What the site list shows: the id when there is one, else the URL.
    pub fn label(&self) -> String {
        if self.id.is_empty() {
            self.url()
        } else {
            format!("{} — {}", self.id, self.url())
        }
    }

    /// Parse `sftp://user@host:port/start/dir`; the scheme, the user and the port may be left
    /// out (`host`, `mb@host`, `sftp://host/srv` all work).
    pub fn parse_url(text: &str) -> Result<Site> {
        let text = text.trim();
        if text.is_empty() {
            return Err(NetError::NotRemote(text.to_string()));
        }
        let (scheme, rest) = match text.split_once("://") {
            Some((head, rest)) => {
                let scheme = Scheme::parse(head)
                    .filter(|s| s.is_remote())
                    .ok_or_else(|| NetError::NotRemote(text.to_string()))?;
                (scheme, rest)
            }
            None => (Scheme::Sftp, text),
        };
        let (authority, start_dir) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };
        let mut site = Site::new("");
        site.scheme = scheme;
        site.port = scheme.default_port().unwrap_or(DEFAULT_SSH_PORT);
        site.start_dir = if start_dir.is_empty() {
            "/".to_string()
        } else {
            start_dir.to_string()
        };
        let (user, host_port) = match authority.rsplit_once('@') {
            Some((user, host)) => (Some(user), host),
            None => (None, authority),
        };
        if let Some(user) = user {
            if user.is_empty() {
                return Err(NetError::NotRemote(text.to_string()));
            }
            site.user = user.to_string();
        }
        // An IPv6 literal is bracketed: [::1]:2222.
        let (host, port) = if let Some(rest) = host_port.strip_prefix('[') {
            match rest.split_once(']') {
                Some((host, tail)) => (host, tail.strip_prefix(':')),
                None => return Err(NetError::NotRemote(text.to_string())),
            }
        } else {
            match host_port.rsplit_once(':') {
                Some((host, port)) => (host, Some(port)),
                None => (host_port, None),
            }
        };
        if host.is_empty() {
            return Err(NetError::NotRemote(text.to_string()));
        }
        site.host = host.to_string();
        if let Some(port) = port {
            site.port = port.parse().map_err(|_| NetError::Connect {
                host: host.to_string(),
                detail: format!("`{port}` is not a port number"),
            })?;
        }
        Ok(site)
    }

    /// A site from a `[[site]]` block of `fcmd.toml`.
    pub fn from_config(cfg: &fcmd_config::SiteCfg) -> Result<Site> {
        let scheme = Scheme::parse(&cfg.scheme)
            .filter(|s| s.is_remote())
            .ok_or_else(|| NetError::Connect {
                host: cfg.host.clone(),
                detail: format!(
                    "unknown scheme `{}` (sftp | ftp | ftps | webdav | webdavs)",
                    cfg.scheme
                ),
            })?;
        if cfg.host.trim().is_empty() {
            return Err(NetError::Connect {
                host: cfg.id.clone(),
                detail: "a site needs a host".into(),
            });
        }
        let mut site = Site::new(cfg.host.trim());
        site.id = cfg.id.clone();
        site.scheme = scheme;
        site.port = scheme.default_port().unwrap_or(DEFAULT_SSH_PORT);
        if let Some(port) = cfg.port {
            site.port = port;
        }
        if let Some(user) = &cfg.user {
            if !user.trim().is_empty() {
                site.user = user.trim().to_string();
            }
        }
        if let Some(auth) = &cfg.auth {
            site.auth = Auth::parse(auth)?;
        }
        if let Some(dir) = &cfg.start_dir {
            if !dir.trim().is_empty() {
                site.start_dir = dir.trim().to_string();
            }
        }
        site.group = cfg.group.clone().filter(|g| !g.trim().is_empty());
        site.keepalive = match cfg.keepalive_secs {
            Some(0) => None,
            Some(secs) => Some(Duration::from_secs(secs)),
            None => site.keepalive,
        };
        site.compress = cfg.compress.unwrap_or(false);
        Ok(site)
    }

    /// Every site in the config, with the broken ones reported rather than dropped silently.
    pub fn list_from_config(cfg: &fcmd_config::Config) -> (Vec<Site>, Vec<String>) {
        let mut sites = Vec::new();
        let mut problems = Vec::new();
        for raw in &cfg.sites {
            match Site::from_config(raw) {
                Ok(site) => sites.push(site),
                Err(e) => problems.push(e.to_string()),
            }
        }
        (sites, problems)
    }

    /// Rebuild a site from a [`VPath`]'s authority, so a reconnect needs no site list.
    pub fn from_path(path: &VPath) -> Result<Site> {
        let authority = path
            .authority()
            .ok_or_else(|| NetError::NotRemote(path.to_string()))?;
        let mut site = Site::parse_url(&format!("{}://{}", path.scheme().as_str(), authority))?;
        site.start_dir = path.remote_path();
        Ok(site)
    }

    /// Does this site own that path?
    pub fn owns(&self, path: &VPath) -> bool {
        path.scheme() == self.scheme && path.authority() == Some(self.authority().as_str())
    }
}

/// `~/x` → `$HOME/x`. Done here rather than by the shell, because we never run a shell.
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(rest);
        }
    }
    if path == "~" {
        if let Some(home) = home_dir() {
            return home;
        }
    }
    PathBuf::from(path)
}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
}

/// The local account name, used when a site or a URL does not name a user.
pub fn current_user() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "root".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_url_parses_down_to_its_parts() {
        let s = Site::parse_url("sftp://mb@host.example:2222/srv/www").unwrap();
        assert_eq!(s.user, "mb");
        assert_eq!(s.host, "host.example");
        assert_eq!(s.port, 2222);
        assert_eq!(s.start_dir, "/srv/www");
        assert_eq!(s.authority(), "mb@host.example:2222");
        assert_eq!(s.start().to_string(), "sftp://mb@host.example:2222/srv/www");
    }

    #[test]
    fn the_short_forms_work_too() {
        let bare = Site::parse_url("host").unwrap();
        assert_eq!(bare.host, "host");
        assert_eq!(bare.port, DEFAULT_SSH_PORT);
        assert_eq!(bare.user, current_user());
        assert_eq!(bare.start_dir, "/");

        let with_user = Site::parse_url("mb@host/srv").unwrap();
        assert_eq!(
            (with_user.user.as_str(), with_user.host.as_str()),
            ("mb", "host")
        );
        assert_eq!(with_user.start_dir, "/srv");

        let v6 = Site::parse_url("sftp://mb@[fe80::1]:2200/").unwrap();
        assert_eq!(v6.host, "fe80::1");
        assert_eq!(v6.port, 2200);
    }

    #[test]
    fn rubbish_is_refused_with_the_text_that_was_given() {
        for text in ["", "sftp://", "sftp://@host", "nfs://host", "host:nonsense"] {
            assert!(Site::parse_url(text).is_err(), "{text} was accepted");
        }
    }

    #[test]
    fn every_scheme_a_site_can_name_knows_its_own_port_and_key() {
        for (url, port) in [
            ("sftp://mb@host/", 22),
            ("ftp://mb@host/", 21),
            ("ftps://mb@host/", 21),
            ("ftpes://mb@host/", 21),
            ("webdav://mb@host/", 80),
            ("webdavs://mb@host/", 443),
        ] {
            let site = Site::parse_url(url).unwrap_or_else(|e| panic!("{url}: {e}"));
            assert_eq!(site.port, port, "{url}");
            // A port in the URL still wins over the scheme's own.
            let explicit = Site::parse_url(&url.replace("host/", "host:8021/")).unwrap();
            assert_eq!(explicit.port, 8021, "{url}");
        }

        // The key carries the scheme, which the authority does not: one server offering both
        // spellings of FTP on port 21 is two connections and has to be two keys.
        let plain = Site::parse_url("ftp://mb@host/").unwrap();
        let secure = Site::parse_url("ftps://mb@host/").unwrap();
        assert_eq!(plain.authority(), secure.authority());
        assert_ne!(plain.key(), secure.key());
        assert_eq!(secure.key(), "ftps://mb@host:21");
        assert_eq!(
            Site::key_of(&secure.start()).as_deref(),
            Some(secure.key().as_str())
        );
        assert_eq!(Site::key_of(&VPath::local("/srv")), None);
    }

    #[test]
    fn a_site_round_trips_through_a_path() {
        let site = Site::parse_url("sftp://mb@host:22/srv").unwrap();
        let path = site.start().join("www");
        let back = Site::from_path(&path).unwrap();
        assert_eq!(back.authority(), site.authority());
        assert_eq!(back.start_dir, "/srv/www");
        assert!(site.owns(&path));
        assert!(!site.owns(&VPath::local("/srv")));
        assert!(Site::from_path(&VPath::local("/srv")).is_err());
    }

    #[test]
    fn sites_come_out_of_the_config_file() {
        let (cfg, warnings) = fcmd_config::Config::parse(
            "fcmd.toml",
            r#"
            [[site]]
            id = "jump"
            host = "jump.example.net"
            port = 2222
            user = "mb"
            auth = "agent"
            start_dir = "/srv"
            group = "work"
            keepalive_secs = 0

            [[site]]
            id = "broken"
            host = ""
            "#,
        )
        .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        let (sites, problems) = Site::list_from_config(&cfg);
        assert_eq!(sites.len(), 1);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("needs a host"), "{}", problems[0]);
        let site = &sites[0];
        assert_eq!(site.id, "jump");
        assert_eq!(site.authority(), "mb@jump.example.net:2222");
        assert_eq!(site.start_dir, "/srv");
        assert_eq!(site.group.as_deref(), Some("work"));
        assert_eq!(site.keepalive, None, "keepalive_secs = 0 turns it off");
        assert_eq!(site.auth, Auth::Agent);
        assert_eq!(
            cfg.site("JUMP").map(|s| s.host.as_str()),
            Some("jump.example.net")
        );
    }

    #[test]
    fn auth_is_parsed_from_the_config_spelling() {
        assert_eq!(Auth::parse("agent").unwrap(), Auth::Agent);
        assert_eq!(Auth::parse("ask").unwrap(), Auth::Password);
        assert_eq!(
            Auth::parse("key:/home/mb/.ssh/id_ed25519").unwrap(),
            Auth::Key {
                path: PathBuf::from("/home/mb/.ssh/id_ed25519")
            }
        );
        assert!(Auth::parse("magic").is_err());
        assert!(Auth::parse("key:").is_err());
        assert_eq!(Auth::Agent.describe(), "agent");
    }
}
