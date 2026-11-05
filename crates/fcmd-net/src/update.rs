//! Which release of FinalCommander is the latest (SPEC 11 §5).
//!
//! One request, and only when the user asked for it — the menu's *Check for updates*, or
//! `general.update_check` turned on, which asks once a day. It is not GitHub's API: the
//! release page `…/releases/latest` answers with a redirect to `…/releases/tag/vX.Y.Z`, and
//! the tag is read from that `Location`. No token, no JSON, no rate limit worth the name, and
//! nothing is sent but the request itself and the user agent every client sends.

use std::time::Duration;

/// Where the releases are published.
pub const RELEASES: &str = "https://github.com/matteobaccan/FinalCommander/releases";

/// The newest published release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// The tag as it was published, `v0.32.0`.
    pub tag: String,
    /// Its page, where the archives and the changelog are.
    pub url: String,
}

/// Ask for the latest release. Errors come back as text: they are shown, never acted on.
pub async fn latest() -> Result<Release, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("fcmd/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(format!("{RELEASES}/latest"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let location = response
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| format!("{RELEASES}/latest: HTTP {}", response.status()))?;
    let tag = tag_of(location).ok_or_else(|| format!("no release tag in {location}"))?;
    Ok(Release {
        url: format!("{RELEASES}/tag/{tag}"),
        tag,
    })
}

/// The tag a release page's address ends in: `…/releases/tag/v0.32.0` gives `v0.32.0`.
pub fn tag_of(location: &str) -> Option<String> {
    let (_, tag) = location.rsplit_once("/tag/")?;
    let tag = tag.split(['?', '#', '/']).next()?;
    version_of(tag).map(|_| tag.to_string())
}

/// `v0.32.0` or `0.32.0` as three numbers; anything after a `-` (a pre-release) is ignored
/// for the comparison, which is right for the tags this project publishes.
pub fn version_of(text: &str) -> Option<(u64, u64, u64)> {
    let core = text.trim().trim_start_matches('v');
    let core = core.split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    let v = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(v)
}

/// Is the published tag newer than the version this binary was built as?
pub fn is_newer(tag: &str, current: &str) -> bool {
    matches!((version_of(tag), version_of(current)), (Some(t), Some(c)) if t > c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tag_is_read_from_the_redirect() {
        assert_eq!(
            tag_of("https://github.com/matteobaccan/FinalCommander/releases/tag/v0.32.0"),
            Some("v0.32.0".into())
        );
        assert_eq!(
            tag_of("https://github.com/x/releases/tag/v1.2.3?x=1"),
            Some("v1.2.3".into())
        );
        assert_eq!(tag_of("https://github.com/x/releases"), None);
        assert_eq!(tag_of("https://github.com/x/releases/tag/nightly"), None);
    }

    #[test]
    fn versions_compare_as_numbers_not_as_text() {
        assert!(is_newer("v0.32.0", "0.31.0"));
        assert!(is_newer("v0.31.10", "0.31.9"));
        assert!(is_newer("v1.0.0", "0.99.99"));
        assert!(!is_newer("v0.31.0", "0.31.0"));
        assert!(!is_newer("v0.30.5", "0.31.0"));
        assert!(!is_newer("garbage", "0.31.0"));
        assert_eq!(version_of("v0.32.0-rc1"), Some((0, 32, 0)));
        assert_eq!(version_of("0.32"), None);
    }
}
