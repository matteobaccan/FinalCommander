# FinalCommander Link without a screen

`fcmd --link-serve [FOLDER]` accepts Link calls (SPEC 04 §9) on a machine nobody sits at — a
router, a home server, a Raspberry Pi — and offers one folder read only. With no person there
to answer, only a caller that gives the machine's PIN is let in, so a PIN is set first:

    fcmd --link-pin

It asks for the PIN twice without showing it (at least six characters) and keeps a salted,
stretched value of it in `link-pin` beside the settings — never the PIN itself. An empty PIN
removes it.

`--link-serve` prints the machine's ID and TCP port, then one line for each call: refused
without a PIN, refused for a wrong one, in, left. Five wrong PINs from one address lock it out
for ten minutes. The folder is the path given, or `link.share`; `link.name` is the name callers
see and `link.port` fixes the TCP port when a firewall has to be opened for it (UDP 47831, for
being found, as well).

On the calling machine, the netbox session for it has protocol *Link*, the ID as the host, and
*ask* as the authentication: the password asked for is the PIN.

- `fcmd-link`: an OpenWrt service (procd), for `/etc/init.d/`.
- `../systemd/fcmd-link.service`: the same as a systemd user service.
