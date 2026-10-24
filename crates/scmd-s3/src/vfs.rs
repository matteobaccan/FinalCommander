//! A bucket as a panel.
//!
//! Object storage has no directories. A bucket is a flat map from a key to some bytes, and
//! `2024/08/13/access.log` is one key with slashes in it, not four nested things. What makes
//! it browsable is the `delimiter` parameter of a listing: ask for the keys under
//! `2024/08/` with `/` as the delimiter and the service answers with the objects directly
//! there and, separately, the set of prefixes one level down — which is a directory's files
//! and its subdirectories, computed by the service rather than by us.
//!
//! That is the whole trick, and it is what makes the size of the bucket beside the point. A
//! prefix with forty objects under it costs one request whether the bucket holds a thousand
//! keys or ten million, because the service never sends the rest. The cost only returns if
//! someone asks for a listing with no delimiter at all — every key under a prefix, however
//! deep — and that is what [`Service::listing_cap`] is for: it is a real listing of a real
//! bucket, it is a thousand requests for a million objects, and it stops at a number the
//! panel can show and says that it stopped.
//!
//! A path is `s3://[profile@]bucket/prefix`. The profile names a service in the configuration
//! — which host, which region, which credentials — because unlike SFTP the host is not in the
//! path: two buckets of the same account live at the same host, and the same bucket name at
//! two providers is two different things. An empty bucket (`s3://work@/`) lists the buckets
//! the credentials can see.

use std::collections::HashMap;
use std::ffi::OsString;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use scmd_core::{CancelToken, Entry, EntryFlags, EntryId, EntryKind, Result as CoreResult, VPath};
use scmd_vfs::{Caps, Dir, Vfs};

use crate::client::Client;
use crate::endpoint::Endpoint;
use crate::error::{Result, S3Error};
use crate::sign::Credentials;

/// How many rows a listing without a delimiter will gather before it stops. Six figures is
/// about a second of sorting and a quarter of a gigabyte of panel, and a hundred requests.
pub const DEFAULT_LISTING_CAP: usize = 100_000;

/// One configured service: where it is, who we are to it, and how far a flat listing may go.
#[derive(Debug, Clone)]
pub struct Service {
    pub name: String,
    pub endpoint: Endpoint,
    pub credentials: Option<Credentials>,
    pub listing_cap: usize,
}

impl Service {
    pub fn new(name: impl Into<String>, endpoint: Endpoint) -> Service {
        Service {
            name: name.into(),
            endpoint,
            credentials: None,
            listing_cap: DEFAULT_LISTING_CAP,
        }
    }

    pub fn with_credentials(mut self, credentials: Option<Credentials>) -> Service {
        self.credentials = credentials;
        self
    }

    pub fn with_listing_cap(mut self, cap: usize) -> Service {
        self.listing_cap = cap;
        self
    }
}

/// The provider. One of these holds every configured service and a client per bucket.
pub struct S3Vfs {
    services: HashMap<String, Service>,
    /// A bucket lives in one region and answers from no other, so when a service says which
    /// region it wanted, the client for that bucket is rebuilt there and kept.
    clients: Mutex<HashMap<(String, String), Arc<Client>>>,
}

impl std::fmt::Debug for S3Vfs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S3Vfs")
            .field("services", &self.services.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl S3Vfs {
    /// Build the provider from the configured services. The one named `""` is the default,
    /// used by a path that names no profile.
    pub fn new(services: Vec<Service>) -> S3Vfs {
        S3Vfs {
            services: services
                .into_iter()
                .map(|s| (s.name.clone(), s))
                .collect::<HashMap<_, _>>(),
            clients: Mutex::new(HashMap::new()),
        }
    }

    /// Nothing configured at all, which is what an installation without an `[s3]` section has.
    pub fn empty() -> S3Vfs {
        S3Vfs::new(Vec::new())
    }

    pub fn is_empty(&self) -> bool {
        self.services.is_empty()
    }

    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.services.keys().cloned().collect();
        names.sort();
        names
    }

    /// `profile@bucket` → the two halves, with an empty profile meaning the default one.
    pub fn split_authority(authority: &str) -> (&str, &str) {
        match authority.split_once('@') {
            Some((profile, bucket)) => (profile, bucket),
            None => ("", authority),
        }
    }

    fn service(&self, profile: &str) -> Result<&Service> {
        self.services.get(profile).ok_or_else(|| {
            let known = self.names().join(", ");
            S3Error::NoCredentials(if profile.is_empty() {
                format!("no default object storage service is configured (known: {known})")
            } else {
                format!(
                    "no object storage service named `{profile}` is configured (known: {known})"
                )
            })
        })
    }

    fn client(&self, profile: &str, bucket: &str) -> Result<Arc<Client>> {
        let key = (profile.to_string(), bucket.to_string());
        if let Some(client) = self.clients.lock().expect("the client map").get(&key) {
            return Ok(client.clone());
        }
        let service = self.service(profile)?;
        let client = Arc::new(Client::new(
            service.endpoint.clone(),
            service.credentials.clone(),
        )?);
        self.clients
            .lock()
            .expect("the client map")
            .insert(key, client.clone());
        Ok(client)
    }

    /// Remember that this bucket answers from `region` from now on.
    fn move_bucket(&self, profile: &str, bucket: &str, region: &str) -> Result<Arc<Client>> {
        let client = self.client(profile, bucket)?.in_region(region)?;
        let client = Arc::new(client);
        self.clients
            .lock()
            .expect("the client map")
            .insert((profile.to_string(), bucket.to_string()), client.clone());
        Ok(client)
    }

    /// The prefix a path names: no leading slash, and a trailing one unless it is the root,
    /// because a prefix that is a directory ends at the delimiter.
    pub fn prefix_of(path: &VPath) -> String {
        let text = path.remote_path();
        let trimmed = text.trim_start_matches('/');
        if trimmed.is_empty() {
            String::new()
        } else if trimmed.ends_with('/') {
            trimmed.to_string()
        } else {
            format!("{trimmed}/")
        }
    }

    /// The key a path names: the prefix without the trailing slash.
    pub fn key_of(path: &VPath) -> String {
        path.remote_path().trim_start_matches('/').to_string()
    }

    /// Read one directory, following the pages and stopping where it must.
    async fn read(&self, path: &VPath, cancel: &CancelToken, flat: bool) -> Result<Dir> {
        let authority = path.authority().unwrap_or("");
        let (profile, bucket) = S3Vfs::split_authority(authority);
        if bucket.is_empty() {
            return self.read_buckets(profile, cancel).await;
        }
        let cap = self.service(profile)?.listing_cap;
        let prefix = S3Vfs::prefix_of(path);
        let delimiter = if flat { None } else { Some("/") };

        let mut entries = vec![Entry::parent_entry()];
        let mut token: Option<String> = None;
        let mut client = self.client(profile, bucket)?;
        let mut requests = 0u32;
        loop {
            if cancel.is_cancelled() {
                return Err(S3Error::Cancelled);
            }
            let page = match client
                .list_page(bucket, &prefix, delimiter, token.as_deref(), cancel)
                .await
            {
                Ok(page) => page,
                // The bucket is in another region and the service said which. Follow it once,
                // and remember, rather than making the user find the setting.
                Err(error) if error.wanted_region().is_some() && requests == 0 => {
                    let region = error.wanted_region().expect("just checked").to_string();
                    client = self.move_bucket(profile, bucket, &region)?;
                    continue;
                }
                Err(error) => return Err(error),
            };
            requests += 1;
            for common in &page.prefixes {
                entries.push(prefix_entry(common));
            }
            for object in &page.objects {
                // A key that is the prefix itself is the marker some tools write for a
                // directory. It is not a row: it is this directory.
                if object.key == prefix {
                    continue;
                }
                entries.push(object_entry(&object.key, &prefix, object));
            }
            let more = page.truncated && page.next_token.is_some();
            if !more {
                return Ok(Dir::whole(entries));
            }
            if cap > 0 && entries.len() >= cap {
                let note = format!(
                    "the first {} of this prefix, in {requests} requests; there are more \
                     (s3.listing_cap)",
                    entries.len() - 1
                );
                return Ok(Dir::partial(entries, note));
            }
            token = page.next_token;
        }
    }

    async fn read_buckets(&self, profile: &str, cancel: &CancelToken) -> Result<Dir> {
        let client = self.client(profile, "")?;
        let buckets = client.list_buckets(cancel).await?;
        let mut entries = Vec::with_capacity(buckets.len());
        for bucket in buckets {
            entries.push(Entry {
                name: OsString::from(&bucket.name),
                kind: EntryKind::Dir,
                size: 0,
                mtime: bucket.created,
                btime: bucket.created,
                atime: None,
                flags: EntryFlags::empty(),
                id: EntryId::Name(name_hash(&bucket.name)),
                link_target: None,
            });
        }
        Ok(Dir::whole(entries))
    }

    /// What one path is. A prefix with anything under it is a directory; a key that answers a
    /// `HEAD` is a file; a bucket is a directory.
    async fn describe(&self, path: &VPath, cancel: &CancelToken) -> Result<Entry> {
        let authority = path.authority().unwrap_or("");
        let (profile, bucket) = S3Vfs::split_authority(authority);
        if bucket.is_empty() {
            return Ok(dir_entry("/"));
        }
        let key = S3Vfs::key_of(path);
        if key.is_empty() {
            return Ok(dir_entry(bucket));
        }
        let client = self.client(profile, bucket)?;
        match client.head(bucket, &key, cancel).await {
            Ok(meta) => Ok(Entry {
                name: OsString::from(last_segment(&key)),
                kind: EntryKind::File,
                size: meta.size,
                mtime: meta.modified,
                btime: None,
                atime: None,
                flags: EntryFlags::empty(),
                id: EntryId::Name(name_hash(&key)),
                link_target: None,
            }),
            Err(S3Error::NotFound(_)) => {
                // Not an object. It may still be a prefix, and the way to find out is to ask
                // for one key under it: a prefix nothing lives under does not exist at all.
                let prefix = format!("{key}/");
                let page = client
                    .list_page(bucket, &prefix, Some("/"), None, cancel)
                    .await?;
                if page.objects.is_empty() && page.prefixes.is_empty() {
                    Err(S3Error::NotFound(format!("{bucket}/{key}")))
                } else {
                    Ok(dir_entry(last_segment(&key)))
                }
            }
            Err(other) => Err(other),
        }
    }

    /// Read bytes of an object: the whole thing, or a window of it for the viewer.
    pub async fn read_object(
        &self,
        path: &VPath,
        range: Option<(u64, Option<u64>)>,
        cancel: &CancelToken,
    ) -> Result<Vec<u8>> {
        let authority = path.authority().unwrap_or("");
        let (profile, bucket) = S3Vfs::split_authority(authority);
        let key = S3Vfs::key_of(path);
        if bucket.is_empty() || key.is_empty() {
            return Err(S3Error::NotFound(path.to_string()));
        }
        self.client(profile, bucket)?
            .get(bucket, &key, range, cancel)
            .await
    }
}

#[async_trait]
impl Vfs for S3Vfs {
    fn scheme(&self) -> &'static str {
        "s3"
    }

    /// Read, and seek. An object is immutable from this program's point of view today: there
    /// is no partial write, no rename that is not a copy, and no timestamp to set — so the
    /// capabilities say read and nothing else, and the UI offers nothing it cannot do.
    fn caps(&self) -> Caps {
        Caps::READ | Caps::SEEK | Caps::CASE_SENSITIVE
    }

    async fn list(&self, path: &VPath, cancel: CancelToken) -> CoreResult<Vec<Entry>> {
        self.read(path, &cancel, false)
            .await
            .map(|dir| dir.entries)
            .map_err(|e| e.into_core(path))
    }

    async fn list_dir(&self, path: &VPath, cancel: CancelToken) -> CoreResult<Dir> {
        self.read(path, &cancel, false)
            .await
            .map_err(|e| e.into_core(path))
    }

    async fn stat(&self, path: &VPath) -> CoreResult<Entry> {
        self.describe(path, &CancelToken::new())
            .await
            .map_err(|e| e.into_core(path))
    }
}

/// A row for a common prefix: a directory, named by its last segment.
fn prefix_entry(prefix: &str) -> Entry {
    dir_entry(last_segment(prefix.trim_end_matches('/')))
}

fn dir_entry(name: &str) -> Entry {
    Entry {
        name: OsString::from(name),
        kind: EntryKind::Dir,
        size: 0,
        mtime: None,
        btime: None,
        atime: None,
        flags: EntryFlags::empty(),
        id: EntryId::Name(name_hash(name)),
        link_target: None,
    }
}

/// A row for an object. The name is what is left of the key after the prefix, which for a
/// listing with a delimiter is one segment and for a flat listing is the whole path below it —
/// and that is the right answer for both, since a flat listing is asking to see the depth.
fn object_entry(key: &str, prefix: &str, object: &crate::xml::ObjectRow) -> Entry {
    let name = key.strip_prefix(prefix).unwrap_or(key);
    let mut flags = EntryFlags::empty();
    // Nothing here is writable through this program, and a panel that says so is honest.
    flags.insert(EntryFlags::READONLY);
    if name.starts_with('.') {
        flags.insert(EntryFlags::HIDDEN);
    }
    Entry {
        name: OsString::from(name),
        kind: EntryKind::File,
        size: object.size,
        mtime: object.modified,
        btime: None,
        atime: None,
        flags,
        id: EntryId::Name(name_hash(key)),
        link_target: None,
    }
}

fn last_segment(key: &str) -> &str {
    match key.rsplit_once('/') {
        Some((_, last)) => last,
        None => key,
    }
}

/// FNV-1a. Object storage gives no inode, so the id is the hash of the key — which is stable
/// across a refresh, and that is what the selection needs it for.
fn name_hash(name: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in name.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeS3;
    use std::collections::BTreeMap;

    async fn provider() -> (FakeS3, S3Vfs) {
        let mut buckets = BTreeMap::new();
        let mut logs = BTreeMap::new();
        for key in [
            "2024/08/13/access.log",
            "2024/08/13/error.log",
            "2024/08/14/access.log",
            "2024/09/01/access.log",
            "readme.txt",
            ".hidden",
            "2024/",
        ] {
            logs.insert(key.to_string(), format!("contents of {key}").into_bytes());
        }
        buckets.insert("logs".to_string(), logs);
        buckets.insert("other".to_string(), BTreeMap::new());
        let fake = FakeS3::start(buckets).await;
        let vfs = S3Vfs::new(vec![
            Service::new("", fake.endpoint()).with_credentials(Some(fake.credentials.clone()))
        ]);
        (fake, vfs)
    }

    fn names(entries: &[Entry]) -> Vec<String> {
        entries
            .iter()
            .map(|e| e.name_str().into_owned())
            .collect::<Vec<_>>()
    }

    #[tokio::test]
    async fn a_bucket_is_a_directory_tree() {
        let (_fake, vfs) = provider().await;
        let root = VPath::parse("s3://logs/");
        assert_eq!(root.scheme(), scmd_core::Scheme::S3);

        let top = vfs.list(&root, CancelToken::new()).await.unwrap();
        assert_eq!(names(&top), vec!["..", "2024", ".hidden", "readme.txt"]);
        // The parent row is there, and the rest carry their sizes.
        assert_eq!(top[0].kind, EntryKind::Parent);
        assert_eq!(top[1].kind, EntryKind::Dir);
        assert_eq!(top[3].size, "contents of readme.txt".len() as u64);
        // A key beginning with a dot is hidden, as it would be anywhere else.
        assert!(top[2].flags.contains(EntryFlags::HIDDEN));

        let deeper = vfs
            .list(&VPath::parse("s3://logs/2024/08/"), CancelToken::new())
            .await
            .unwrap();
        assert_eq!(names(&deeper), vec!["..", "13", "14"]);

        let leaf = vfs
            .list(&VPath::parse("s3://logs/2024/08/13/"), CancelToken::new())
            .await
            .unwrap();
        assert_eq!(names(&leaf), vec!["..", "access.log", "error.log"]);
    }

    /// Some tools write a zero-length object whose key is the prefix itself to stand for a
    /// directory. It is this directory, not a file in it.
    #[tokio::test]
    async fn a_directory_marker_is_not_a_row() {
        let (_fake, vfs) = provider().await;
        let rows = vfs
            .list(&VPath::parse("s3://logs/2024/"), CancelToken::new())
            .await
            .unwrap();
        assert_eq!(names(&rows), vec!["..", "08", "09"]);
    }

    #[tokio::test]
    async fn the_buckets_are_the_root() {
        let (_fake, vfs) = provider().await;
        let rows = vfs
            .list(&VPath::parse("s3://@/"), CancelToken::new())
            .await
            .unwrap();
        assert_eq!(names(&rows), vec!["logs", "other"]);
        assert!(rows.iter().all(|r| r.kind == EntryKind::Dir));
    }

    #[tokio::test]
    async fn one_path_at_a_time_can_be_asked_about() {
        let (_fake, vfs) = provider().await;
        let file = vfs
            .stat(&VPath::parse("s3://logs/readme.txt"))
            .await
            .unwrap();
        assert_eq!(file.kind, EntryKind::File);
        assert_eq!(file.name_str(), "readme.txt");
        assert_eq!(file.size, 22);

        let prefix = vfs.stat(&VPath::parse("s3://logs/2024/08")).await.unwrap();
        assert_eq!(prefix.kind, EntryKind::Dir);
        assert_eq!(prefix.name_str(), "08");

        let bucket = vfs.stat(&VPath::parse("s3://logs/")).await.unwrap();
        assert_eq!(bucket.kind, EntryKind::Dir);
        assert_eq!(bucket.name_str(), "logs");

        let missing = vfs
            .stat(&VPath::parse("s3://logs/nothing/here"))
            .await
            .unwrap_err();
        assert!(missing.to_string().contains("no such object"), "{missing}");
    }

    #[tokio::test]
    async fn an_object_can_be_read_whole_or_in_part() {
        let (_fake, vfs) = provider().await;
        let path = VPath::parse("s3://logs/readme.txt");
        let cancel = CancelToken::new();
        assert_eq!(
            vfs.read_object(&path, None, &cancel).await.unwrap(),
            b"contents of readme.txt"
        );
        assert_eq!(
            vfs.read_object(&path, Some((12, Some(6))), &cancel)
                .await
                .unwrap(),
            b"readme"
        );
    }

    /// The flat listing: every key under the prefix, which is what a bucket of a million
    /// objects is, and the cap is what keeps the panel answering.
    #[tokio::test]
    async fn a_flat_listing_stops_at_the_cap_and_says_so() {
        let mut buckets = BTreeMap::new();
        buckets.insert("big".to_string(), FakeS3::many("data/", 40));
        let fake = FakeS3::start(buckets).await;
        fake.page_limit(10);
        let vfs = S3Vfs::new(vec![Service::new("", fake.endpoint())
            .with_credentials(Some(fake.credentials.clone()))
            .with_listing_cap(25)]);

        let path = VPath::parse("s3://big/data/");
        let capped = vfs.read(&path, &CancelToken::new(), true).await.unwrap();
        assert!(!capped.complete);
        let note = capped.note.clone().expect("a note about the cap");
        assert!(note.contains("there are more"), "{note}");
        assert!(note.contains("requests"), "{note}");
        // It stopped at the first page boundary at or past the cap, and not before it.
        assert!(capped.entries.len() >= 26, "{}", capped.entries.len());
        assert!(capped.entries.len() < 40, "{}", capped.entries.len());

        // With no cap at all it reads the lot, in four pages of ten.
        let whole = S3Vfs::new(vec![Service::new("", fake.endpoint())
            .with_credentials(Some(fake.credentials.clone()))
            .with_listing_cap(0)])
        .read(&path, &CancelToken::new(), true)
        .await
        .unwrap();
        assert!(whole.complete);
        assert_eq!(whole.entries.len(), 41);
        assert!(whole.note.is_none());
        // And a flat row is named by its whole path below the prefix, not by its last segment.
        assert_eq!(whole.entries[1].name_str(), "000000.txt");
    }

    #[tokio::test]
    async fn a_listing_with_a_delimiter_is_one_request_whatever_the_bucket_holds() {
        let mut buckets = BTreeMap::new();
        let mut big = FakeS3::many("data/2024/", 500);
        big.extend(FakeS3::many("data/2025/", 500));
        buckets.insert("big".to_string(), big);
        let fake = FakeS3::start(buckets).await;
        let vfs = S3Vfs::new(vec![
            Service::new("", fake.endpoint()).with_credentials(Some(fake.credentials.clone()))
        ]);

        let rows = vfs
            .list(&VPath::parse("s3://big/data/"), CancelToken::new())
            .await
            .unwrap();
        assert_eq!(names(&rows), vec!["..", "2024", "2025"]);
        // A thousand objects, and the service was asked once.
        assert_eq!(fake.requests().len(), 1);
    }

    #[tokio::test]
    async fn a_profile_that_is_not_configured_says_which_are() {
        let vfs = S3Vfs::new(vec![Service::new("work", Endpoint::aws("eu-west-1"))]);
        let err = vfs
            .list(&VPath::parse("s3://other@bucket/"), CancelToken::new())
            .await
            .unwrap_err();
        let text = err.to_string();
        assert!(text.contains("`other`"), "{text}");
        assert!(text.contains("work"), "{text}");

        // And a path with no profile at all, where no default is configured.
        let err = vfs
            .list(&VPath::parse("s3://bucket/"), CancelToken::new())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("no default"), "{err}");
    }

    #[tokio::test]
    async fn a_cancelled_listing_does_not_finish_the_pages() {
        let mut buckets = BTreeMap::new();
        buckets.insert("big".to_string(), FakeS3::many("data/", 40));
        let fake = FakeS3::start(buckets).await;
        fake.page_limit(5);
        let vfs = S3Vfs::new(vec![
            Service::new("", fake.endpoint()).with_credentials(Some(fake.credentials.clone()))
        ]);
        let cancel = CancelToken::new();
        cancel.cancel();
        let err = vfs
            .list(&VPath::parse("s3://big/data/"), cancel)
            .await
            .unwrap_err();
        assert!(matches!(err, scmd_core::Error::Cancelled), "{err:?}");
    }

    /// What the service says when it refuses is what the panel shows: `InvalidAccessKeyId`
    /// and `AccessDenied` send whoever hits them to two different places.
    #[tokio::test]
    async fn a_refusal_arrives_with_the_service_own_words() {
        let (fake, vfs) = provider().await;
        fake.refuse(403, "InvalidAccessKeyId", "the access key does not exist");
        let err = vfs
            .list(&VPath::parse("s3://logs/"), CancelToken::new())
            .await
            .unwrap_err();
        let text = err.to_string();
        assert!(text.contains("InvalidAccessKeyId"), "{text}");
        assert!(text.contains("the access key does not exist"), "{text}");
        // And the path it was about, since a panel shows one line.
        assert!(text.contains("s3://logs"), "{text}");
    }

    #[test]
    fn a_path_says_which_service_and_which_bucket() {
        assert_eq!(S3Vfs::split_authority("logs"), ("", "logs"));
        assert_eq!(S3Vfs::split_authority("work@logs"), ("work", "logs"));
        assert_eq!(S3Vfs::split_authority("work@"), ("work", ""));
        assert_eq!(S3Vfs::split_authority(""), ("", ""));

        let path = VPath::parse("s3://work@logs/2024/08/");
        assert_eq!(path.authority(), Some("work@logs"));
        assert_eq!(S3Vfs::prefix_of(&path), "2024/08/");
        // A path does not keep a trailing slash — it is a `PathBuf` underneath, and nothing
        // in it distinguishes `2024/08` from `2024/08/`. Which is why a prefix is derived by
        // appending the delimiter rather than by trusting what was typed, and why asking what
        // a path *is* takes a request.
        assert_eq!(S3Vfs::key_of(&path), "2024/08");
        let no_slash = VPath::parse("s3://work@logs/2024/08");
        assert_eq!(S3Vfs::prefix_of(&no_slash), "2024/08/");
        assert_eq!(S3Vfs::key_of(&no_slash), "2024/08");
        assert_eq!(S3Vfs::prefix_of(&VPath::parse("s3://logs/")), "");
    }
}
