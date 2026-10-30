# Production on the Koinot host

Until the switch below, `~/mintaka-prod/run.sh` starts the server as the `dchradmin` account,
restarted by the cron watchdog. That account is in the `docker` group, so a hole in our server
would hand an attacker the whole machine (docs/SECURITY-REVIEW.md, finding M2). These units run
it under an account of its own, `mintaka`, in a sandbox: read-only system, no capabilities, no
view of `/home`, a syscall filter (`systemd-analyze security` exposure 1.3).

**The switch** takes one `sudo` session. Copy this directory to `~/mintaka-prod/deploy-koinot/`
first, then, from a terminal that can type the password:

```sh
ssh -t dchradmin@192.168.1.156 sudo sh mintaka-prod/deploy-koinot/install.sh
```

`install.sh` creates the account, adds `dchradmin` to its group, stops the old process, copies
`~/mintaka-prod/data` (briefs, the console-challenge and bot keys, the bot's chat memory) to
`/var/lib/mintaka`, installs and starts the units, and checks the site answers. If anything
fails, it puts the old process back. After it:

- **Deploying** stays the same: copy `dist/` and the binary into `~/mintaka-prod/` (binary
  last); `mintaka-prod-reload.path` restarts the service, as it does when `~/mintaka-prod/env`
  changes (a new bot token needs no sudo either). `run.sh` and the watchdog leave the service
  alone.
- **Briefs** are fetched from the new place:
  `rsync -a <server>:/var/lib/mintaka/contact/ briefs/`. The briefs are encrypted, and readable
  by the `mintaka` group; the keys and the chat memory stay readable only by the service.
- **The log** is in the journal: `journalctl -u mintaka-prod`.
- **Undo:** `sudo systemctl disable --now mintaka-prod.service mintaka-prod-reload.path`; bring
  the data back with
  `sudo rsync -a --chown=dchradmin:dchradmin /var/lib/mintaka/ ~/mintaka-prod/data/`; then
  `~/mintaka-prod/run.sh`.
