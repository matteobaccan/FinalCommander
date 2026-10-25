//! Reading a WebDAV `PROPFIND` answer.
//!
//! One document matters — RFC 4918's `multistatus` — and it is shallow and element-only. So
//! this is a scanner over the text rather than a dependency, in the same shape as the object
//! store's reader for S3's XML: it reports opening tags, closing tags and text, and the reader
//! below picks out the four properties a panel needs (is it a collection, how big, when was it
//! changed, what is it called) and ignores every other, which is what keeps a server that
//! answers with thirty properties from breaking a listing.
//!
//! Two things it does take seriously. **Namespace prefixes are stripped**: the DAV namespace
//! is conventionally `D:` or `d:` but a server may bind it to anything, and `<lp1:resourcetype>`
//! from an Apache `mod_dav` has to read the same as `<D:resourcetype>`. And **entities are
//! decoded**, because an `href` holds a percent-encoded path that may still arrive with `&amp;`
//! in it.

use crate::clock;

/// One `<response>` of a multistatus, as far as a panel cares.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DavRow {
    /// The `href`, exactly as the server wrote it: an absolute path or a whole URL, and still
    /// percent-encoded. Making sense of it against the collection that was asked for is the
    /// caller's job, because only the caller knows what it asked for.
    pub href: String,
    pub is_dir: bool,
    pub size: u64,
    /// Seconds since the epoch, from `getlastmodified`.
    pub mtime: Option<u32>,
    /// `displayname`, when the server sends one. A name is taken from the `href` by preference
    /// — that is the one the next request has to use — and this is only a fallback.
    pub display_name: Option<String>,
}

/// Every `<response>` in a `multistatus`, in the order the server wrote them.
///
/// A `propstat` whose `status` is not a success is skipped, which is how a server says "I do
/// not have that property for this resource": asking for `getcontentlength` on a collection
/// usually comes back as a `404` inside a `207`, and reading it as a zero-length file would
/// turn every directory into an empty file.
pub fn parse_multistatus(text: &str) -> Vec<DavRow> {
    let mut rows = Vec::new();
    let mut row = DavRow::default();
    let mut in_response = false;
    // What the scanner is inside, innermost last, with the prefixes already stripped.
    let mut stack: Vec<String> = Vec::new();
    // One `propstat` at a time: its properties are held here until its `status` says whether
    // they are real.
    let mut proposed = DavRow::default();
    let mut propstat_ok = true;
    let mut in_propstat = false;

    for event in Scan::new(text) {
        match event {
            Event::Open(tag) => {
                let name = local(tag);
                match name.as_str() {
                    "response" => {
                        in_response = true;
                        row = DavRow::default();
                    }
                    "propstat" if in_response => {
                        in_propstat = true;
                        propstat_ok = true;
                        proposed = DavRow::default();
                    }
                    "collection" if stack.iter().any(|t| t == "resourcetype") => {
                        proposed.is_dir = true;
                    }
                    _ => {}
                }
                stack.push(name);
            }
            Event::Empty(tag) => {
                let name = local(tag);
                // `<D:collection/>` is how nearly every server says "this is a directory".
                if name == "collection" && stack.iter().any(|t| t == "resourcetype") {
                    proposed.is_dir = true;
                }
            }
            Event::Text(text) => {
                let Some(tag) = stack.last() else { continue };
                match tag.as_str() {
                    "href" if in_response && !in_propstat => row.href = text.trim().to_string(),
                    "getcontentlength" => proposed.size = text.trim().parse().unwrap_or(0),
                    "getlastmodified" => proposed.mtime = http_time(text.trim()),
                    "displayname" => {
                        let name = text.trim();
                        if !name.is_empty() {
                            proposed.display_name = Some(name.to_string());
                        }
                    }
                    "status" if in_propstat => propstat_ok = is_success(&text),
                    _ => {}
                }
            }
            Event::Close(tag) => {
                let name = local(tag);
                while let Some(open) = stack.pop() {
                    if open == name {
                        break;
                    }
                }
                match name.as_str() {
                    "propstat" => {
                        if propstat_ok {
                            // Only what this propstat actually carried: a later one that holds
                            // the failures must not undo it.
                            row.is_dir |= proposed.is_dir;
                            if proposed.size > 0 {
                                row.size = proposed.size;
                            }
                            if proposed.mtime.is_some() {
                                row.mtime = proposed.mtime;
                            }
                            if proposed.display_name.is_some() {
                                row.display_name = proposed.display_name.clone();
                            }
                        }
                        in_propstat = false;
                    }
                    "response" => {
                        if !row.href.is_empty() {
                            // A collection has no length of its own; a server that sends one
                            // is reporting the size of its index page or of nothing at all.
                            if row.is_dir {
                                row.size = 0;
                            }
                            rows.push(std::mem::take(&mut row));
                        }
                        in_response = false;
                    }
                    _ => {}
                }
            }
        }
    }
    rows
}

/// `HTTP/1.1 200 OK` → true. Anything from 200 to 299 counts; everything else, and anything
/// unreadable, does not.
fn is_success(status: &str) -> bool {
    status
        .split_whitespace()
        .find_map(|word| word.parse::<u16>().ok())
        .map(|code| (200..300).contains(&code))
        .unwrap_or(false)
}

/// `Mon, 01 Jan 2026 12:00:00 GMT` — RFC 1123, which is the only form `getlastmodified` takes.
///
/// A date in any other shape is read as no date at all. The alternative is guessing, and a
/// timestamp that is wrong is worse than one that is missing: the panel sorts by it and the
/// copy dialog's "only newer files" believes it.
pub fn http_time(text: &str) -> Option<u32> {
    // Drop the weekday, which is redundant and which some servers spell out in full.
    let rest = text.split_once(',').map(|(_, rest)| rest).unwrap_or(text);
    let mut parts = rest.split_whitespace();
    let day: i64 = parts.next()?.parse().ok()?;
    let month = clock::month_of(parts.next()?)?;
    let year: i64 = parts.next()?.parse().ok()?;
    let clock_text = parts.next()?;
    let mut hms = clock_text.split(':');
    let hour: i64 = hms.next()?.parse().ok()?;
    let min: i64 = hms.next()?.parse().ok()?;
    let sec: i64 = hms.next()?.parse().ok()?;
    // The zone must be GMT; RFC 7231 says a server has to send it that way, and a server that
    // sends an offset instead is not one to take a timestamp from.
    match parts.next() {
        Some(zone) if zone.eq_ignore_ascii_case("GMT") || zone.eq_ignore_ascii_case("UTC") => {}
        _ => return None,
    }
    clock::seconds_from_utc(year, month, day, hour, min, sec)
}

/// `D:href` → `href`, `{DAV:}href` → `href`. Case is folded: XML is case-sensitive, but no
/// server spells these properties in anything but lower case and a reader that broke on one
/// that did would be answering the wrong question.
fn local(tag: &str) -> String {
    let name = tag.split_whitespace().next().unwrap_or(tag);
    let name = match name.rfind(['}', ':']) {
        Some(cut) => &name[cut + 1..],
        None => name,
    };
    name.to_ascii_lowercase()
}

/// What the scanner reports.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Event<'a> {
    Open(&'a str),
    Close(&'a str),
    Text(String),
    /// A tag that opens and closes at once: `<D:collection/>`.
    Empty(&'a str),
}

/// A scan over an XML document. Declarations, comments and doctypes are skipped, `CDATA` is
/// text, and attributes are read past: none of this document's elements uses one for anything
/// but a namespace declaration.
struct Scan<'a> {
    rest: &'a str,
}

impl<'a> Scan<'a> {
    fn new(text: &'a str) -> Scan<'a> {
        Scan { rest: text }
    }
}

impl<'a> Iterator for Scan<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Event<'a>> {
        loop {
            if self.rest.is_empty() {
                return None;
            }
            if !self.rest.starts_with('<') {
                let cut = self.rest.find('<').unwrap_or(self.rest.len());
                let (text, rest) = self.rest.split_at(cut);
                self.rest = rest;
                if text.trim().is_empty() {
                    continue;
                }
                return Some(Event::Text(unescape(text)));
            }
            if let Some(after) = self.rest.strip_prefix("<![CDATA[") {
                let cut = after.find("]]>").unwrap_or(after.len());
                let text = &after[..cut];
                self.rest = after.get(cut + 3..).unwrap_or("");
                return Some(Event::Text(text.to_string()));
            }
            if self.rest.starts_with("<!--") {
                let after = &self.rest[4..];
                let cut = after.find("-->").map(|i| i + 3).unwrap_or(after.len());
                self.rest = &after[cut.min(after.len())..];
                continue;
            }
            let cut = match self.rest.find('>') {
                Some(i) => i,
                None => {
                    self.rest = "";
                    return None;
                }
            };
            let tag = &self.rest[1..cut];
            self.rest = &self.rest[cut + 1..];
            if tag.starts_with('?') || tag.starts_with('!') {
                continue;
            }
            if let Some(name) = tag.strip_prefix('/') {
                return Some(Event::Close(name.trim()));
            }
            if let Some(name) = tag.strip_suffix('/') {
                return Some(Event::Empty(name.trim()));
            }
            return Some(Event::Open(tag));
        }
    }
}

/// The five entities XML always has. A numeric reference is decoded too: a `displayname` may
/// carry one.
fn unescape(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(cut) = rest.find('&') {
        out.push_str(&rest[..cut]);
        rest = &rest[cut..];
        let Some(end) = rest.find(';') else {
            out.push_str(rest);
            return out;
        };
        let entity = &rest[1..end];
        match entity {
            "amp" => out.push('&'),
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            other => {
                let code = other
                    .strip_prefix("#x")
                    .or_else(|| other.strip_prefix("#X"))
                    .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                    .or_else(|| other.strip_prefix('#').and_then(|d| d.parse().ok()));
                match code.and_then(char::from_u32) {
                    Some(c) => out.push(c),
                    // Not an entity we know: left exactly as it arrived rather than dropped.
                    None => out.push_str(&rest[..=end]),
                }
            }
        }
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTING: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/dav/srv/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype><D:collection/></D:resourcetype>
        <D:getlastmodified>Thu, 01 Jan 2026 12:00:00 GMT</D:getlastmodified>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
    <D:propstat>
      <D:prop><D:getcontentlength/></D:prop>
      <D:status>HTTP/1.1 404 Not Found</D:status>
    </D:propstat>
  </D:response>
  <D:response>
    <D:href>/dav/srv/readme%20notes.md</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype/>
        <D:getcontentlength>13</D:getcontentlength>
        <D:getlastmodified>Thu, 01 Jan 2026 12:00:00 GMT</D:getlastmodified>
        <D:displayname>readme notes.md</D:displayname>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#;

    #[test]
    fn a_listing_says_which_rows_are_directories_and_how_big_the_files_are() {
        let rows = parse_multistatus(LISTING);
        assert_eq!(rows.len(), 2);

        assert_eq!(rows[0].href, "/dav/srv/");
        assert!(rows[0].is_dir);
        assert_eq!(rows[0].size, 0);
        assert_eq!(rows[0].mtime, Some(1_767_268_800));

        assert_eq!(rows[1].href, "/dav/srv/readme%20notes.md");
        assert!(!rows[1].is_dir);
        assert_eq!(rows[1].size, 13);
        assert_eq!(rows[1].display_name.as_deref(), Some("readme notes.md"));
    }

    #[test]
    fn a_property_the_server_says_it_does_not_have_is_not_read_as_a_zero() {
        // The directory above has `getcontentlength` in a 404 propstat. A reader that took it
        // anyway would show every directory as an empty file.
        let rows = parse_multistatus(LISTING);
        assert_eq!(rows[0].size, 0);
        assert!(rows[0].is_dir, "and it is still a directory");
    }

    #[test]
    fn whatever_prefix_the_server_binds_the_namespace_to_is_read_the_same() {
        // Apache's mod_dav answers with `lp1:` for the live properties and no prefix at all
        // for the namespace it declares as the default.
        let apache = r#"<multistatus xmlns="DAV:">
            <response>
              <href>/x/notes.txt</href>
              <propstat><prop>
                <lp1:resourcetype xmlns:lp1="DAV:"/>
                <lp1:getcontentlength xmlns:lp1="DAV:">7</lp1:getcontentlength>
              </prop><status>HTTP/1.1 200 OK</status></propstat>
            </response>
          </multistatus>"#;
        let rows = parse_multistatus(apache);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].href, "/x/notes.txt");
        assert_eq!(rows[0].size, 7);
        assert!(!rows[0].is_dir);
    }

    #[test]
    fn an_href_with_an_entity_in_it_comes_out_whole() {
        let doc = r#"<d:multistatus xmlns:d="DAV:"><d:response>
            <d:href>/x/a%20&amp;%20b.txt</d:href>
            <d:propstat><d:prop><d:getcontentlength>1</d:getcontentlength></d:prop>
            <d:status>HTTP/1.1 200 OK</d:status></d:propstat>
          </d:response></d:multistatus>"#;
        let rows = parse_multistatus(doc);
        assert_eq!(rows[0].href, "/x/a%20&%20b.txt");
    }

    #[test]
    fn an_http_date_is_read_in_the_one_form_the_rfc_allows() {
        assert_eq!(
            http_time("Thu, 01 Jan 2026 12:00:00 GMT"),
            Some(1_767_268_800)
        );
        assert_eq!(http_time("Thu, 01 Jan 1970 00:00:00 GMT"), Some(0));
        // No zone, a zone that is not GMT, and rubbish: no date rather than a wrong one.
        assert_eq!(http_time("Thu, 01 Jan 2026 12:00:00"), None);
        assert_eq!(http_time("Thu, 01 Jan 2026 12:00:00 +0100"), None);
        assert_eq!(http_time("whenever"), None);
        assert_eq!(http_time(""), None);
    }

    #[test]
    fn a_document_that_is_not_one_gives_nothing_rather_than_panicking() {
        for text in [
            "",
            "<multistatus>",
            "not xml at all",
            "<a><b>c</b></a>",
            "<<<",
        ] {
            assert!(parse_multistatus(text).is_empty(), "{text}");
        }
    }
}
