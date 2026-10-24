//! Reading the XML an S3 service answers with.
//!
//! Three documents matter — a page of a listing, the list of buckets, and an error — and all
//! three are shallow, element-only and attribute-free. So this is a scanner over the text
//! rather than a dependency: it reports opening tags, closing tags and text, and the three
//! readers below pick out the elements they know and ignore every other, which is what keeps
//! a service that adds a field from breaking a listing.
//!
//! Two things it does take seriously. Entities are decoded, because a key may hold an `&` or a
//! `<` and arrive as `&amp;` or `&lt;`. And every listing is asked for with
//! `encoding-type=url`, so keys come back percent-encoded and a key holding a newline, a
//! control character or an unpaired surrogate reaches us intact; the readers decode them.

use std::time::SystemTime;

use crate::{clock, uri};

/// What the scanner reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event<'a> {
    Open(&'a str),
    Close(&'a str),
    Text(String),
    /// A tag that opens and closes at once, `<IsTruncated/>`.
    Empty(&'a str),
}

/// A scan over an XML document. Declarations, comments and doctypes are skipped; `CDATA` is
/// text; attributes are read past and dropped, since none of these documents uses them for
/// anything but the namespace.
pub struct Scan<'a> {
    rest: &'a str,
}

impl<'a> Scan<'a> {
    pub fn new(text: &'a str) -> Scan<'a> {
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
            let empty = tag.ends_with('/');
            let tag = tag.trim_end_matches('/').trim();
            let name = tag.split_whitespace().next().unwrap_or(tag);
            return Some(if empty {
                Event::Empty(name)
            } else {
                Event::Open(name)
            });
        }
    }
}

/// The five entities XML defines, and numeric references in both bases.
pub fn unescape(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let end = match rest.find(';') {
            Some(e) if e <= 12 => e,
            // A bare `&` that is not an entity at all: keep it, do not eat the rest.
            _ => {
                out.push('&');
                rest = &rest[1..];
                continue;
            }
        };
        let name = &rest[1..end];
        let decoded = match name {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            other => other
                .strip_prefix('#')
                .and_then(|n| match n.strip_prefix(['x', 'X']) {
                    Some(h) => u32::from_str_radix(h, 16).ok(),
                    None => n.parse().ok(),
                })
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// One object in a listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectRow {
    pub key: String,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub etag: Option<String>,
    pub storage_class: Option<String>,
}

/// One page of `ListObjectsV2`. A bucket is not handed over in one piece: a page carries at
/// most a thousand rows by default and says whether there are more.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Page {
    pub name: String,
    pub prefix: String,
    pub objects: Vec<ObjectRow>,
    /// The `CommonPrefixes`: everything between the prefix asked for and the next delimiter,
    /// which is what makes a flat namespace look like a tree of directories.
    pub prefixes: Vec<String>,
    pub next_token: Option<String>,
    pub truncated: bool,
}

/// One bucket of `ListBuckets`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BucketRow {
    pub name: String,
    pub created: Option<SystemTime>,
}

/// What a service says when it refuses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServiceError {
    pub code: String,
    pub message: String,
    /// `AuthorizationHeaderMalformed` and a redirect both name the region they wanted.
    pub region: Option<String>,
    pub request_id: Option<String>,
}

/// Read a page of a listing. Keys are percent-decoded, because the request asked for
/// `encoding-type=url`.
pub fn parse_page(text: &str) -> Page {
    let mut page = Page::default();
    let mut path: Vec<&str> = Vec::new();
    let mut object: Option<ObjectRow> = None;
    for event in Scan::new(text) {
        match event {
            Event::Open("Contents") => {
                object = Some(ObjectRow {
                    key: String::new(),
                    size: 0,
                    modified: None,
                    etag: None,
                    storage_class: None,
                });
                path.push("Contents");
            }
            Event::Close("Contents") => {
                path.pop();
                if let Some(row) = object.take() {
                    if !row.key.is_empty() {
                        page.objects.push(row);
                    }
                }
            }
            Event::Open(name) => path.push(name),
            Event::Close(_) => {
                path.pop();
            }
            Event::Empty(_) => {}
            Event::Text(text) => {
                let depth = path.len();
                let here = path.last().copied().unwrap_or("");
                let parent = if depth >= 2 { path[depth - 2] } else { "" };
                match (parent, here) {
                    ("Contents", "Key") => {
                        if let Some(row) = object.as_mut() {
                            row.key = uri::decode(&text);
                        }
                    }
                    ("Contents", "Size") => {
                        if let Some(row) = object.as_mut() {
                            row.size = text.trim().parse().unwrap_or(0);
                        }
                    }
                    ("Contents", "LastModified") => {
                        if let Some(row) = object.as_mut() {
                            row.modified = clock::parse_iso8601(&text);
                        }
                    }
                    ("Contents", "ETag") => {
                        if let Some(row) = object.as_mut() {
                            row.etag = Some(text.trim().trim_matches('"').to_string());
                        }
                    }
                    ("Contents", "StorageClass") => {
                        if let Some(row) = object.as_mut() {
                            row.storage_class = Some(text.trim().to_string());
                        }
                    }
                    ("CommonPrefixes", "Prefix") => page.prefixes.push(uri::decode(&text)),
                    (_, "Prefix") => page.prefix = uri::decode(&text),
                    (_, "Name") => page.name = text.trim().to_string(),
                    (_, "IsTruncated") => page.truncated = text.trim() == "true",
                    // Opaque, and not subject to `encoding-type`: it goes back as it came.
                    (_, "NextContinuationToken") => page.next_token = Some(text.trim().to_string()),
                    _ => {}
                }
            }
        }
    }
    page
}

/// Read the list of buckets.
pub fn parse_buckets(text: &str) -> Vec<BucketRow> {
    let mut out = Vec::new();
    let mut path: Vec<&str> = Vec::new();
    let mut row: Option<BucketRow> = None;
    for event in Scan::new(text) {
        match event {
            Event::Open("Bucket") => {
                row = Some(BucketRow {
                    name: String::new(),
                    created: None,
                });
                path.push("Bucket");
            }
            Event::Close("Bucket") => {
                path.pop();
                if let Some(bucket) = row.take() {
                    if !bucket.name.is_empty() {
                        out.push(bucket);
                    }
                }
            }
            Event::Open(name) => path.push(name),
            Event::Close(_) => {
                path.pop();
            }
            Event::Empty(_) => {}
            Event::Text(text) => {
                if path.last().copied() == Some("Name") {
                    if let Some(bucket) = row.as_mut() {
                        bucket.name = text.trim().to_string();
                    }
                } else if path.last().copied() == Some("CreationDate") {
                    if let Some(bucket) = row.as_mut() {
                        bucket.created = clock::parse_iso8601(&text);
                    }
                }
            }
        }
    }
    out
}

/// Read an error document. `None` when the body is not one — a proxy's HTML page, say, or
/// nothing at all, which is what a bare `403` from a bucket policy gives.
pub fn parse_error(text: &str) -> Option<ServiceError> {
    let mut error = ServiceError::default();
    let mut path: Vec<&str> = Vec::new();
    let mut inside = false;
    for event in Scan::new(text) {
        match event {
            Event::Open("Error") => {
                inside = true;
                path.push("Error");
            }
            Event::Open(name) => path.push(name),
            Event::Close(_) => {
                path.pop();
            }
            Event::Empty(_) => {}
            Event::Text(text) => {
                if !inside {
                    continue;
                }
                match path.last().copied().unwrap_or("") {
                    "Code" => error.code = text.trim().to_string(),
                    "Message" => error.message = text.trim().to_string(),
                    "Region" => error.region = Some(text.trim().to_string()),
                    "RequestId" => error.request_id = Some(text.trim().to_string()),
                    _ => {}
                }
            }
        }
    }
    if error.code.is_empty() && error.message.is_empty() {
        None
    } else {
        Some(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTING: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <Name>logs</Name>
  <Prefix>2024%2F08%2F</Prefix>
  <KeyCount>3</KeyCount>
  <MaxKeys>1000</MaxKeys>
  <Delimiter>%2F</Delimiter>
  <IsTruncated>true</IsTruncated>
  <NextContinuationToken>1ueGcxLPRx1Tr/XYExHnhbYLgveDs2J/wm36Hy4vbOwM=</NextContinuationToken>
  <Contents>
    <Key>2024%2F08%2Faccess.log</Key>
    <LastModified>2024-08-13T10:11:12.000Z</LastModified>
    <ETag>&quot;d41d8cd98f00b204e9800998ecf8427e&quot;</ETag>
    <Size>1234567</Size>
    <StorageClass>STANDARD</StorageClass>
  </Contents>
  <Contents>
    <Key>2024%2F08%2Fa%20%26%20b%0Anewline.txt</Key>
    <LastModified>2024-08-14T00:00:00.000Z</LastModified>
    <ETag>"abc"</ETag>
    <Size>0</Size>
    <StorageClass>GLACIER</StorageClass>
  </Contents>
  <CommonPrefixes><Prefix>2024%2F08%2F13%2F</Prefix></CommonPrefixes>
  <CommonPrefixes><Prefix>2024%2F08%2F14%2F</Prefix></CommonPrefixes>
</ListBucketResult>"#;

    #[test]
    fn a_page_of_a_listing_is_read_whole() {
        let page = parse_page(LISTING);
        assert_eq!(page.name, "logs");
        assert_eq!(page.prefix, "2024/08/");
        assert!(page.truncated);
        assert_eq!(
            page.next_token.as_deref(),
            Some("1ueGcxLPRx1Tr/XYExHnhbYLgveDs2J/wm36Hy4vbOwM=")
        );
        assert_eq!(page.prefixes, vec!["2024/08/13/", "2024/08/14/"]);
        assert_eq!(page.objects.len(), 2);
        assert_eq!(page.objects[0].key, "2024/08/access.log");
        assert_eq!(page.objects[0].size, 1_234_567);
        assert_eq!(
            page.objects[0].etag.as_deref(),
            Some("d41d8cd98f00b204e9800998ecf8427e")
        );
        assert_eq!(page.objects[0].storage_class.as_deref(), Some("STANDARD"));
        assert_eq!(
            page.objects[0].modified,
            clock::parse_iso8601("2024-08-13T10:11:12Z")
        );
        // The second key holds an ampersand and a newline, and arrives whole.
        assert_eq!(page.objects[1].key, "2024/08/a & b\nnewline.txt");
        assert_eq!(page.objects[1].size, 0);
    }

    #[test]
    fn a_last_page_says_so() {
        let xml = "<ListBucketResult><Name>b</Name><IsTruncated>false</IsTruncated>\
                   <Contents><Key>x</Key><Size>1</Size></Contents></ListBucketResult>";
        let page = parse_page(xml);
        assert!(!page.truncated);
        assert_eq!(page.next_token, None);
        assert_eq!(page.objects.len(), 1);
        // No date and no ETag: the row still arrives, with what the service gave.
        assert_eq!(page.objects[0].modified, None);
        assert_eq!(page.objects[0].etag, None);
    }

    #[test]
    fn an_empty_prefix_is_a_page_with_nothing_in_it() {
        let page = parse_page(
            "<ListBucketResult><Name>b</Name><Prefix></Prefix><KeyCount>0</KeyCount>\
             <IsTruncated>false</IsTruncated></ListBucketResult>",
        );
        assert!(page.objects.is_empty() && page.prefixes.is_empty());
        assert_eq!(page.name, "b");
    }

    /// A field this version has never heard of must not derail the page.
    #[test]
    fn an_unknown_element_is_ignored() {
        let page = parse_page(
            "<ListBucketResult><Name>b</Name><SomethingNew><Nested>1</Nested></SomethingNew>\
             <Contents><Key>k</Key><Size>7</Size><ChecksumAlgorithm>CRC32</ChecksumAlgorithm>\
             </Contents><IsTruncated>false</IsTruncated></ListBucketResult>",
        );
        assert_eq!(page.objects.len(), 1);
        assert_eq!(page.objects[0].key, "k");
        assert_eq!(page.objects[0].size, 7);
    }

    #[test]
    fn the_buckets_are_read_with_their_dates() {
        let xml = r#"<?xml version="1.0"?>
<ListAllMyBucketsResult><Owner><ID>x</ID><DisplayName>Name</DisplayName></Owner><Buckets>
  <Bucket><Name>logs</Name><CreationDate>2019-04-01T12:00:00.000Z</CreationDate></Bucket>
  <Bucket><Name>backups</Name><CreationDate>2021-11-30T08:30:00.000Z</CreationDate></Bucket>
</Buckets></ListAllMyBucketsResult>"#;
        let buckets = parse_buckets(xml);
        assert_eq!(buckets.len(), 2);
        assert_eq!(buckets[0].name, "logs");
        assert_eq!(buckets[1].name, "backups");
        assert!(buckets[0].created.is_some());
        // The owner's name is not a bucket.
        assert!(!buckets.iter().any(|b| b.name == "Name"));
    }

    #[test]
    fn an_error_is_read_for_what_it_says() {
        let xml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Error><Code>NoSuchBucket</Code>\
                   <Message>The specified bucket does not exist</Message><BucketName>nope\
                   </BucketName><RequestId>ABC123</RequestId></Error>";
        let error = parse_error(xml).unwrap();
        assert_eq!(error.code, "NoSuchBucket");
        assert_eq!(error.message, "The specified bucket does not exist");
        assert_eq!(error.request_id.as_deref(), Some("ABC123"));

        // The one that matters for a misconfigured region: the service names the right one.
        let moved = parse_error(
            "<Error><Code>AuthorizationHeaderMalformed</Code><Message>the region 'us-east-1' \
             is wrong; expecting 'eu-west-1'</Message><Region>eu-west-1</Region></Error>",
        )
        .unwrap();
        assert_eq!(moved.region.as_deref(), Some("eu-west-1"));

        // And a body that is not an error document at all is not read as one.
        assert!(parse_error("").is_none());
        assert!(parse_error("<html><body>502 Bad Gateway</body></html>").is_none());
        assert!(parse_error("<ListBucketResult><Name>b</Name></ListBucketResult>").is_none());
    }

    #[test]
    fn the_entities_are_the_five_and_the_numbers() {
        assert_eq!(unescape("a &amp; b"), "a & b");
        assert_eq!(unescape("&lt;tag&gt;"), "<tag>");
        assert_eq!(unescape("&quot;q&quot; &apos;a&apos;"), "\"q\" 'a'");
        assert_eq!(unescape("&#65;&#x42;&#x43;"), "ABC");
        assert_eq!(unescape("&#x1F600;"), "\u{1F600}");
        // Nothing to decode, and nothing lost either way.
        assert_eq!(unescape("plain"), "plain");
        assert_eq!(unescape("50% & more"), "50% & more");
        assert_eq!(unescape("&notanentity;"), "&notanentity;");
        assert_eq!(unescape("&"), "&");
        assert_eq!(unescape("a&"), "a&");
    }

    #[test]
    fn the_scanner_skips_what_is_not_content() {
        let events: Vec<_> = Scan::new(
            "<?xml version=\"1.0\"?><!-- a comment --><!DOCTYPE x><a><b>text</b><c/>\
             <d><![CDATA[<raw & unescaped>]]></d></a>",
        )
        .collect();
        assert_eq!(
            events,
            vec![
                Event::Open("a"),
                Event::Open("b"),
                Event::Text("text".into()),
                Event::Close("b"),
                Event::Empty("c"),
                Event::Open("d"),
                Event::Text("<raw & unescaped>".into()),
                Event::Close("d"),
                Event::Close("a"),
            ]
        );
    }

    /// Truncated and malformed documents must end the scan, not spin in it.
    #[test]
    fn a_half_arrived_document_stops() {
        for bad in [
            "<ListBucketResult><Contents><Key>k</Key",
            "<ListBucketResult",
            "<!-- unclosed comment",
            "<![CDATA[unclosed",
            "<a><b>&amp",
            "<",
        ] {
            let events: Vec<_> = Scan::new(bad).collect();
            assert!(events.len() < 10, "{bad:?} gave {events:?}");
            // And the readers survive the same input.
            let _ = parse_page(bad);
            let _ = parse_buckets(bad);
            let _ = parse_error(bad);
        }
    }
}
