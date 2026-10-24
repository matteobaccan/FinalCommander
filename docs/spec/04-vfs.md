# SPEC 04 — Virtual filesystem

**Implemented today**: the trait and the capability flags; the `file` provider; and the
**netbox**'s `sftp` provider (`scmd-net`) with the connection pool, the site list from
`[[site]]` blocks, host-key verification against `known_hosts` with a trust-on-first-use
prompt, agent / key-file / password authentication, listing, stat, mkdir (including `mkdir -p`,
which SFTP has no call for), delete, rename, symlinks, mode and mtime, and transfers in every
direction (up, down, server-to-server on one connection and between two) driven by the job
engine with progress and cancellation. A `Router` sends each path to the provider that owns it,
so nothing above the VFS knows whether a panel is local or remote.
Editing a remote file works too: `F4` fetches it, the editor edits the copy, and `F2` writes it
back with a change check and a rename into place (§5).
**Archives** (0.0.5): `scmd-arc` reads zip and the tar family — plain, gzip, bzip2, xz and
zstd — with every decoder pure Rust, so the binary stays one file that cross-compiles without a
C toolchain. The format comes from the bytes first and the name second. `Enter` on an archive
opens it as a panel (§4's `!` paths), `F3` reads an entry, `F5` copies out through the job
engine, `Shift+F1` builds an archive and `Shift+F2` extracts one. The provider reports `READ`
and `SEEK` only: writing *into* an existing container is §4's rebuild and is not here yet, and
`F4` inside an archive says so rather than editing a copy nobody can save.
**Still to come**: `temp`, `trash`, the other network schemes, nesting and archives over a
remote filesystem (§4 — both want the container copied to a temporary file first), `watch`,
and `copy_within` as a server-side operation.

## 1. The trait

```rust
#[async_trait]
pub trait Vfs: Send + Sync {
    fn scheme(&self) -> &'static str;
    fn caps(&self) -> Caps;                 // bitflags, see §2

    async fn list(&self, p: &VPath, sink: &mut dyn ListSink, ct: &CancelToken) -> Result<()>;
    async fn stat(&self, p: &VPath) -> Result<Meta>;
    async fn open_read(&self, p: &VPath) -> Result<Box<dyn AsyncRead + Unpin + Send>>;
    async fn open_write(&self, p: &VPath, o: WriteOpts) -> Result<Box<dyn AsyncWrite + Unpin + Send>>;
    async fn mkdir(&self, p: &VPath) -> Result<()>;
    async fn remove(&self, p: &VPath, recursive: bool) -> Result<()>;
    async fn rename(&self, from: &VPath, to: &VPath) -> Result<()>;
    async fn set_meta(&self, p: &VPath, m: &MetaPatch) -> Result<()>;
    async fn copy_within(&self, from: &VPath, to: &VPath) -> Result<CopyOutcome>; // server-side
    async fn watch(&self, p: &VPath, tx: WatchSender) -> Result<WatchHandle>;
    fn random_access(&self) -> bool;        // can we seek cheaply? (viewer/hex mode)
}
```

`ListSink` receives chunks so panels paint progressively. Everything is cancellable.

## 2. Capabilities

```
READ WRITE CREATE DELETE RENAME SEEK TRUNCATE
META_TIMES META_MODE META_OWNER XATTR STREAMS LINKS HARDLINKS
WATCH SERVER_SIDE_COPY RESUME RANDOM_WRITE CASE_SENSITIVE ATOMIC_RENAME
```

The UI *asks* before it offers: no "Set permissions" entry on a FAT or S3 path, no `F6` rename
inside a solid 7z, no hard-link command where `HARDLINKS` is absent. A missing capability
produces a precise message ("SFTP server does not support atomic rename"), never a generic
failure.

## 3. Providers

Split of responsibility: the **core** owns `file`, the archive schemes and `temp`/`trash`;
**all network schemes are delivered by the `netbox` first-party plugin** (SPEC 08 §8), which is
also the reference implementation of `vfs-provider`. The table's "crate" column therefore names
where the code lives, not whether it is linked into the binary.

| Scheme | Crate / mechanism | Caps | Milestone |
| --- | --- | --- | --- |
| `file` | `std::fs` + `scmd-platform` | all native | M1 |
| `zip` | `zip` (**implemented, read + create**, deflate/deflate64/bzip2/lzma/xz, zip64) | READ SEEK, CREATE as a whole archive | M2 |
| `tar` family | `tar` + `flate2`/`lzma-rust2`/`ruzstd`/`bzip2`, all pure Rust (**implemented, read + create**) | READ SEEK (plain tar and zip only), CREATE as a whole archive | M2 |
| `7z` | `sevenz-rust2` | READ, WRITE (M3), no random write in solid blocks | M2/M3 |
| `rar` | `unrar` (read-only, non-free licence, optional feature, off by default in distro builds) | READ SEEK | M3 |
| `iso`, `cab`, `ar`/`deb`, `rpm`, `cpio`, `lha`, `arj`, `squashfs` | `libarchive` binding as one fallback provider | READ | M3 |
| `sftp` | `russh` + `russh-sftp` (**implemented**, `scmd-net`) | READ WRITE CREATE DELETE RENAME SEEK TRUNCATE META_TIMES META_MODE META_OWNER LINKS RESUME RANDOM_WRITE CASE_SENSITIVE | M1 |
| `ftp`/`ftps` | `suppaftp` | READ WRITE CREATE DELETE RENAME RESUME | M3 |
| `webdav` | `reqwest-dav` | READ WRITE CREATE DELETE RENAME | M3 |
| `s3` | hand-written client over `reqwest` + `ring` (**implemented, read**, `scmd-s3`; any S3-compatible endpoint) | READ SEEK CASE_SENSITIVE; WRITE CREATE DELETE SERVER_SIDE_COPY to come, never RENAME | M3 |
| `smb` | `pavao`/system mount | varies | M3 |
| `adb`, `mtp` | external `adb` binary / `libmtp` | READ WRITE DELETE | M4 |
| `rclone` | `rclone rcd` JSON API bridge — one provider, dozens of backends | READ WRITE DELETE RENAME | M3 |
| `temp` | in-memory collection of real paths | pass-through | M2 |
| `trash` | XDG/Finder/Recycle Bin listing + restore | READ DELETE, restore verb | M3 |
| `proc`, `reg`, `git` | special views | READ (+ verbs) | M4 |

## 4. Nesting and the archive cache

`sftp://srv/backups/x.tar.gz!/etc!/nginx.conf` — each `!` opens a nested provider.

- A **container cache** keyed by (scheme, root, size, mtime) holds an open provider instance
  with its index; LRU-evicted by count and total memory, closed on idle timeout.
  **Implemented**: four archives, keyed by the file's path, size and modification time, so
  stepping in and out of one reads its index once and an archive rewritten under us is read
  afresh rather than answered from a stale index. One cache serves the panels and the job
  engine, so the index a panel read to draw itself is the one an unpack then reads from.
- Non-seekable containers (`.tar.gz` over SFTP) are materialised to a temp file under a size
  limit (configurable, default 512 MiB) with a visible progress bar; above the limit the panel
  shows a streaming listing and random access is refused with a clear message.
- Nesting depth limit (default 4) to stop zip-bomb recursion; entry-count and expansion-ratio
  guards on every archive read, with the limits in config and the refusal explained.
  **Implemented** as `[arc]` in the config file (`max_entries`, `max_total_mb`, `max_ratio`,
  `max_depth`): the refusal names the limit that fired and what the archive asked for, because
  a user who meets one has to decide whether to raise it. An entry whose name climbs out of the
  archive (`../../etc/passwd`) is pinned at the root rather than trusted, so no extraction can
  be talked into writing outside where it was told to.
- Writes to a nested container rebuild the outer container; the UI warns and the job engine
  treats it as one transactional unit (write to temp, verify, atomic replace).

## 5. Editing inside a VFS

**Implemented today for `sftp`**: `F4` on a remote file fetches it into
`<temp>/scmd-<pid>/<name>`, opens the editor on that copy (the header and every message show
the remote path, not the temp one), and `F2` writes it back. The write-back uploads to
`name.scmd-part` and renames it over the target, so the file on the server keeps its old bytes
until the new ones are completely there. Before writing, the server's size and mtime are
compared with what they were when the file was fetched: a file someone else has changed brings
up a confirmation instead of being overwritten. If the write-back fails, the error names both
the reason and the local copy, so the edits are never lost silently. `Shift+F2` onto an
`sftp://` path writes the buffer up the same way, and `Shift+F4` on a remote panel plus a save
creates a file on the server. SFTP version 3's rename cannot replace an existing file and
`russh-sftp` has no `posix-rename` extension yet, so the old file is removed between the upload
and the rename; if the rename then fails the bytes are still on the server under the
`.scmd-part` name, and the message says so.
**Still to come**: the same thing for archives, and watching the temp file so an external
program's save is picked up.

`F4` on `zip://a.zip!/b.txt`: extract to a temp dir, open the editor, watch the temp file, on
save write back into the container through the job engine, then refresh. On failure the temp
file is kept and its path is shown. Same mechanism for "open with" external programs, with a
"wait for the program to exit, then write back" dialog (FAR behaviour).

## 6. Connection manager

`[[site]]` blocks in `scmd.toml` (no secrets). The spec first said `sites.toml`; the
single-file rule (SPEC 07 §1) moved them into the one config file, and the OS keyring is not
used yet — see the note below.

```toml
[[site]]
id = "jump"
scheme = "sftp"
host = "jump.example.net"
port = 22
user = "mb"
auth = "agent"              # agent | key:<path> | keyring | ask
charset = "utf-8"
start_dir = "/srv"
keepalive = "30s"
compress = true
```

Features: folder grouping, per-site keymap of a start action, host-key verification with a
TOFU prompt and a known-hosts file, proxy/jump-host chains (`ProxyJump`), connection reuse
across panels and jobs, automatic reconnect with the operation queue paused, and an explicit
"disconnect" command. Passwords/passphrases are only ever stored in the OS keyring, never in
config; `--no-keyring` forces prompting.

**Implemented today** (`Alt+N` opens the list, `Alt+Shift+N` closes the active panel's
connection, and a `sftp://user@host:port/dir` URL typed into `Ctrl+L` connects): the site list
with groups and a connected marker, `auth = "agent" | "key:<path>" | "ask"`, one pooled
connection per `user@host:port` shared by both panels and every job, host-key verification
with the fingerprint shown before anything is recorded, a changed host key refused outright,
and the password or key passphrase asked for in a masked prompt and kept only for the
connection attempt. **Not yet**: the OS keyring (so a password is asked for every connection),
`ProxyJump` chains, automatic reconnect, and `charset`.

### 6.1 How the netbox is tested

Two layers, because a provider that can only be tested against someone's server is not tested:

- everything above the wire (paths, listings, transfers, the job engine's view of a remote
  filesystem) runs against `scmd_net::memory::MemorySftp`, an in-memory SFTP server;
- the wire itself runs against a **real SSH server started in the test process** on a loopback
  port (`crates/scmd-net/tests/live_sftp.rs`): key exchange, the host-key decision including
  trust-on-first-use writing to a `known_hosts` file, password authentication, the `sftp`
  subsystem, and every call the provider makes.

## 7. Performance rules

- `list` must yield the first chunk in under 100 ms on a warm local directory; providers that
  cannot (remote) show the spinner immediately and never block the UI thread.
- `stat` storms are avoided: listing uses the directory entry's own type/size where the OS
  provides it (`d_type`, `FindFirstFileEx` info level), and extra metadata is a second pass.
- Remote providers batch and pipeline (SFTP: concurrent reads with a window; S3: list
  continuation prefetch).
- Every provider has a conformance test and a latency-injection test (500 ms per call) that the
  UI must survive without a dropped frame.

## 8. Object storage

A bucket is not a filesystem, and the provider does not pretend otherwise. There are no
directories in it: `2024/08/13/access.log` is **one key with slashes in it**, not four nested
things, and nothing in the service holds a list of what is "in" `2024/08/`.

What makes it browsable is one parameter of a listing. Ask for the keys under `2024/08/` with
`/` as the **delimiter** and the service answers with two lists: the objects directly there,
and the set of prefixes one level below — which is a directory's files and its subdirectories,
computed by the service rather than by us. That is the whole trick, and it is what makes the
size of the bucket beside the point: a prefix with forty objects under it costs **one request**
whether the bucket holds a thousand keys or ten million, because the service never sends the
rest.

### 8.1 A bucket with 1.7 million objects in it

The question is worth answering with figures rather than with confidence, so the figures are
measured — `cargo test --release -p scmd-core --lib a_bucket_sized_listing -- --ignored
--nocapture` builds a listing of 1.7 million rows whose names are shaped like the keys such a
bucket really holds (`2024/08/13/customer-00012345/event-0000000678.json`) and times it.

| | |
| --- | --- |
| Memory for the rows | ~310 MiB (136 bytes per row, 81 MiB of names, 6 MiB of order) |
| Ordering them by name | ~6 s |
| Ordering them by size or date | ~0.7 s |
| Requests to list them | ~1700, one per thousand keys |
| Wall clock for those requests | one to three minutes, and no way around it |

So the honest answer has two halves:

- **Browsing that bucket by prefix is instant**, and stays instant at ten times the size. Each
  level is one request and a few dozen rows. This is how the panel works by default, and it is
  the shape the service is built for.
- **A flat listing of all 1.7 million keys is a different thing**: minutes of requests, a third
  of a gigabyte, and six seconds every time the sort order changes. It is allowed, because
  sometimes it is what someone needs, and it is capped: `listing_cap` (default 100 000 rows)
  stops it at a number the panel can work with, and the panel then says `(partial)` in the
  footer for as long as that listing is on screen, with a line explaining why. A listing that
  was cut short and does not say so is the only unacceptable outcome.

The cap is enforced through [`Dir`], which is what a provider returns to the panel: the rows,
whether that is all of them, and a line to show when it is not. Every other caller uses `list`
and gets the rows alone, because a local directory is always read whole and should not pay for
the question.

### 8.2 The shape of a path

`s3://[profile@]bucket/prefix`. The profile names an `[[s3]]` block of the configuration —
which host, which region, which credentials — because unlike SFTP the host is not part of the
path: two buckets of one account live at the same host, and the same bucket name at two
providers is two different things. An empty bucket (`s3://work@/`) lists the buckets the
credentials can see.

A path cannot say whether it means a prefix or a key — underneath it is a `PathBuf`, and
nothing in it survives the difference between `2024/08` and `2024/08/`. So a prefix is derived
by appending the delimiter, and asking what one path *is* takes a request: a `HEAD` that
answers makes it an object, and a `HEAD` that does not is followed by a one-key listing, since
a prefix nothing lives under does not exist at all.

### 8.3 Addressing, and the services that are not AWS

AWS wants the bucket in front of the host (`logs.s3.eu-west-1.amazonaws.com`); MinIO, Ceph,
Garage, a LocalStack in a container and an IP address want it at the front of the path. The
endpoint says which, and two cases override it:

- a bucket whose name is not a DNS label — a dot in it, an upper-case letter, over 63
  characters — goes in the path whatever the endpoint prefers, because `my.bucket.s3.amazonaws.com`
  is a host no certificate covers and the request would fail at the handshake, where no error
  message can explain itself;
- a service that answers "this bucket is in another region" and names it is followed once, and
  the answer remembered for that bucket, rather than leaving someone to find the setting.

### 8.4 Credentials

**Not in `scmd.toml`.** A secret access key is a password, and this program has never written a
password into its configuration file. An `[[s3]]` block says *where* its keys come from and the
provider reads them from the two places the rest of the machine already keeps them: the `AWS_*`
environment variables, and `~/.aws/credentials` — the file every AWS tool shares, in the INI
format they all write, profiles and session tokens included. `credentials = "anonymous"` signs
nothing, which is how a bucket with a public read policy is browsed.

### 8.5 What is not there yet

Writing. The capabilities say `READ | SEEK | CASE_SENSITIVE` and nothing else, so the UI offers
nothing it cannot do: no `F6` rename (object storage has no rename — it is a copy and a
delete), no permissions, no timestamps. Upload, delete and server-side copy are the next
increment; they are a different kind of operation (a multipart upload is a protocol of its own)
and they are not worth shipping half-done, because a half-finished upload of a large object is
a charge on someone's account.

### 8.6 How it is tested

Without an AWS account, and without pretending. There is an S3 service in the test process —
`scmd_s3::fake` — with a map of buckets behind it, and it is stricter than the real thing in
two ways that matter:

- **It verifies the signature**, recomputed from the request line it received rather than from
  anything the client passed in. That catches the whole class of bug where the URL and the
  canonical request disagree — a key encoded twice, a query sorted differently, a port in the
  host header — which against a real service is a `SignatureDoesNotMatch` with no indication of
  which byte was wrong.
- **It pages**, and the page size can be set to three, so a listing that forgets to follow the
  continuation token fails in a test rather than in a bucket with a million objects in it. It
  caught exactly that: a token that was percent-encoded on the way out and so asked for the
  same page for ever.

The signature itself is tested against `botocore`, the library the AWS command line signs with,
run against a frozen clock — and one of those vectors is the worked example AWS publishes the
signature of, so the chain is anchored at both ends.
