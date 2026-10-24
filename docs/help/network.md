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

Objects are read-only today: `F3` and a copy out work, and nothing offers to write, rename or
delete, because the provider does not claim a capability it does not have.
