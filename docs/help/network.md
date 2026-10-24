# Servers

`Alt+F1` and `Alt+F2` list the stored connections beside the drives. `n` types a URL:

    sftp://me@host/srv/www

A connected panel is an ordinary panel: the keys are the same, a copy to or from it is an
ordinary job with an ordinary progress bar, and `F4` edits a remote file by bringing it here
and writing it back on save.

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
same protocol — is a panel too, once `scmd.toml` has an `[[s3]]` block saying where it is:

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

The keys are never kept in `scmd.toml`. They come from `AWS_ACCESS_KEY_ID` and
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
