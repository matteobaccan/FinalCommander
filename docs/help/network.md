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
