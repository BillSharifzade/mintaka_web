#!/bin/sh
# Restarts the production site, https://dchr.koinotinav.com/mintaka/ — plain HTTP on the docker
# gateway 172.20.0.1:8790 behind the host's Caddy (route /mintaka/*), itself behind the provider's
# HTTPS edge. Settings and secrets (age recipient, Telegram): ./env (mode 600).
# Log: ~/mintaka-prod/server.log. The binary is named mintaka-prod so it can't be confused with
# the LAN preview (~/mintaka-web, mintaka-server). Started by hand after a deploy, and by the
# cron watchdog when it isn't running — until install.sh moves it to systemd, after which this
# does nothing (the service restarts itself after a deploy or a change to ./env).
if systemctl is-enabled --quiet mintaka-prod.service 2>/dev/null; then
  echo "mintaka-prod runs as a systemd service: journalctl -u mintaka-prod" >&2
  exit 0
fi
cd "$HOME/mintaka-prod" || exit 1
pkill -x mintaka-prod && sleep 1
set -a; . ./env; set +a
RUST_LOG=info setsid nohup ./bin/mintaka-prod \
  --dist dist --data-dir data \
  --listen 172.20.0.1:8790 \
  --trusted-proxy 172.20.0.0/16 --client-ip-header x-mintaka-client-ip \
  --region dyu-1 --location "Dushanbe,TJ,38.5598,68.7870" \
  >> server.log 2>&1 < /dev/null &
