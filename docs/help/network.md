# Servers

`Alt+F1` and `Alt+F2` list the stored connections beside the drives. `n` types a URL:

    sftp://me@host/srv/www
    ftp://me@host/pub
    ftps://me@host/pub        the same server with TLS
    webdavs://me@host/remote.php/dav/files/me/
    webdav://me@host:8080/share

A connected panel is an ordinary panel: the keys are the same, a copy to or from it is an
ordinary job with an ordinary progress bar, and `F4` edits a remote file by bringing it here
and writing it back on save. The port follows the protocol when the URL does not say — 22 for
SFTP, 21 for both spellings of FTP — and one server offering `ftp` and `ftps` on port 21 is two
connections, not one.

## What FTP cannot do, and says so

SFTP is a filesystem; FTP is a file *transfer* protocol, and the difference shows:

* **No links.** Nothing in FTP makes one, and a listing only sometimes says where one points.
* **Nothing to seek with.** The viewer reads a remote file by fetching it first either way, so
  this is only visible on a very large file, where the wait is the whole file rather than the
  first page.
* **Times and modes are extensions.** `MFMT` sets a time and `SITE CHMOD` sets a mode; most
  servers have both, and one that has not says so in its own words rather than quietly losing
  the timestamp of a copy.
* **A date is only trustworthy from a modern server.** `MLSD`, which this asks for first,
  gives the time in UTC to the second. A server too old for it answers with the text its own
  `ls` prints, where the year is missing on recent files and the time zone is never stated —
  so those rows are shown with **no** date at all. A date wrong by a year is worse than none:
  sorting by it, and "only files newer than these" in the copy dialog, would both believe it.

A transfer opens a connection of its own and logs in again, which is what graphical FTP clients
do and what keeps a listing from waiting behind a download. A password is therefore kept in
memory while the connection is open — it is never written anywhere — and a user called
`anonymous` or `ftp` is not asked for one at all.

FTPS is explicit TLS: the connection starts in the clear and is secured with `AUTH TLS` before
the password is sent, and the data connections are protected too. The certificate is checked
against this machine's own authorities, which is what makes a server behind a company CA work
without anything being turned off. Implicit FTPS on port 990 is deprecated and is not here.

## WebDAV

`webdav://` is HTTP and `webdavs://` is HTTPS — the ports are 80 and 443 when the URL does not
say — and the path is the one the server publishes, which on Nextcloud and ownCloud is
`/remote.php/dav/files/<user>/` and on an Apache `mod_dav` is whatever the `Alias` says.

A listing is one request, a file is a `GET`, writing one is a `PUT` of the whole file, a
directory is a `MKCOL`, and deleting a directory takes its tree in a single request. What that
costs:

* **A file is written whole.** There is nothing to truncate and nothing to write into the
  middle of, so `F4` on a remote file saves it by sending all of it — which is what the
  editor does anyway, since it brings the file here and writes it back.
* **A time cannot be set and neither can a mode.** The server owns a file's modification time
  in WebDAV — it is a *live* property — and HTTP has no permission bits. A copy to a WebDAV
  panel therefore says that it could not carry the timestamp over, instead of reporting
  success and leaving the file dated now. (`Ctrl+A` has nothing to offer on such a path.)
* **A rename will not replace anything.** `F6` onto a name that is already taken is refused by
  the server rather than silently overwriting it.
* **A password is sent with every request**, as HTTP basic authentication over the connection —
  which is why `webdavs://` is the one to use and `webdav://` is for a server on this machine
  or behind a VPN. A share that needs no login is reached by leaving the user out of the URL.

What is not there yet: digest authentication (basic is what every server in practice accepts
over TLS), locking, and the quota properties.

A host key the program has never seen stops and shows it before anything is sent, and records
it only if you say so. A password is asked for when it is needed, never echoed and never
written anywhere.

`F7` in the connection manager stores a session, `F4` edits one, and `d` disconnects. What is
stored is the host, the user and the path — not the password.

## Tunnels

`F9 → Commands → Open a tunnel…` (or `net.tunnel` from the command palette) forwards a port
over the connection the panel is on — the same thing as `ssh -L`, without a second ssh and
without a second authentication:

    8080:db.internal:5432     this machine's 8080 reaches db.internal:5432 from the server
    db.internal:5432          the same port on both sides
    5432                      the server's own 5432

Anything that connects to the local port is carried over the session and comes out at the far
end as if it had been opened there — a database client, a browser, or this program's own `s3://`
panel pointed at a MinIO nobody outside the server room can reach.

The local port is bound on **127.0.0.1**, not on every interface: a tunnel is the server's
network handed to this machine, and binding it everywhere would hand it to the rest of the
office. `Tunnels…` in the same menu lists what is open with three counters — connections
carried, open now, refused by the far end — and `Enter` on a row closes it, which frees the
port at once.

## Buckets

An object storage service — S3 itself, or MinIO, Ceph, R2, Wasabi, anything that speaks the
same protocol — is a panel too, once `fcmd.toml` has an `[[s3]]` block saying where it is:

    s3://logs/2024/08/
    s3://work@logs/2024/08/     the service named `work`
    s3://work@/                 the buckets that service can see

A bucket has no directories in it. `2024/08/13/access.log` is one key with slashes in it, and
what the panel shows as a folder is every key that starts the same way up to the next slash —
which the service works out, not this program. That is why a prefix opens as fast in a bucket
with ten million objects as in one with ten.

Asking for everything under a prefix, however deep, is a different matter: that really is a
listing of the whole bucket, a thousand requests for a million keys, and it stops at
`listing_cap` rows. When it stops, the footer says `(partial)` for as long as that listing is
on screen, because a count that is not the whole count should not look like one.

The keys are never kept in `fcmd.toml`. They come from `AWS_ACCESS_KEY_ID` and
`AWS_SECRET_ACCESS_KEY`, or from `~/.aws/credentials` — the file the AWS tools share, profiles
and all. A bucket that anyone may read needs `credentials = "anonymous"` and no keys at all.

`F3` on an object brings it down to a temporary copy and shows you that, the way it does for a
file on a server, and `net.fetch_limit_mb` is the ceiling on what it will fetch. `F4` edits
that copy and saving sends it back as one write.

`F5`, `F6`, `F7` and `F8` work on a bucket as they do anywhere else, with the same dialog, the
same progress bar, the same overwrite question and the same `Esc`. A copy into a bucket is an
upload streamed off the disk; a copy out of one is a download written straight to the file; a
copy from one key to another inside the same service is done by the service, so the bytes never
come through here, which is what makes copying a large object affordable.

A few things are the nature of object storage rather than gaps:

* **There are no directories.** `a/b/c.txt` is one key, and the `a/` a panel shows is read out
  of the keys. `F7` writes the empty object named `a/b/` that every console uses to make a
  prefix visible before anything is in it, and the first real key under it makes that marker
  redundant.
* **A rename is a copy and a delete**, which the service does itself. It is not atomic: a
  failure between the two leaves both keys, and the program says so rather than hiding it.
* **Times and permissions are not kept.** An object carries the time the service wrote it and
  no other; a bucket's access is its policy, which is not a set of permission bits. A copy
  into a bucket therefore does not preserve the file's modification time, because there is
  nowhere to put it.
* **One write is one object.** There is no appending, no writing into the middle, and no
  resuming — and no half-written object either, which is the other side of the same coin. An
  object above five gigabytes needs a multipart upload, which is not here yet; the service's
  own refusal is what says so.
* **A service configured without credentials reads and nothing more.** Its rows are marked
  read-only, and the keys that would write are not offered.
