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
| `s3` | `aws-sdk-s3` (any S3-compatible endpoint) | READ WRITE CREATE DELETE SERVER_SIDE_COPY, no RENAME | M3 |
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
