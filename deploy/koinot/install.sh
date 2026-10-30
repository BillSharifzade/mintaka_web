#!/bin/sh
# Moves production from the cron-started process of dchradmin to the sandboxed service
# (mintaka-prod.service), once, with sudo:
#
#   ssh -t dchradmin@192.168.1.156 sudo sh mintaka-prod/deploy-koinot/install.sh
#
# It creates the `mintaka` account (dchradmin joins its group, to fetch briefs), copies the
# process's data into /var/lib/mintaka, installs and starts the units, and checks the site
# answers. If anything fails on the way, or the site doesn't answer within 15 s, it puts the old
# process back. Running it again is safe: the data is copied only the first time. Undo:
# sudo systemctl disable --now mintaka-prod.service mintaka-prod-reload.path, then
# ~/mintaka-prod/run.sh.
set -eu
[ "$(id -u)" = 0 ] || { echo "run it with sudo" >&2; exit 1; }
deploy=dchradmin
prod=/home/$deploy/mintaka-prod
here=$(dirname "$(readlink -f "$0")")
state=/var/lib/mintaka
fresh=

id -u mintaka >/dev/null 2>&1 ||
  useradd --system --user-group --home-dir /nonexistent --no-create-home --shell /usr/sbin/nologin mintaka
usermod -a -G mintaka "$deploy"
install -d -m 755 /opt/mintaka /opt/mintaka/bin /opt/mintaka/dist
install -m 644 "$here/mintaka-prod.service" "$here/mintaka-prod-reload.path" \
  "$here/mintaka-prod-reload.service" /etc/systemd/system/
systemctl daemon-reload

rollback() {
  echo "the service did not start; the old process is back. Its log:" >&2
  journalctl -u mintaka-prod -n 30 --no-pager -o cat >&2 || true
  systemctl disable --now mintaka-prod-reload.path mintaka-prod.service || true
  # The old process goes on with ~/mintaka-prod/data; the next run copies it afresh.
  [ -z "$fresh" ] || mv "$state" "$state.failed-$(date +%s)" || true
  su - "$deploy" -c "$prod/run.sh" || true
}
# Once the unit is enabled, run.sh (and so the watchdog) starts nothing: from here on, every way
# out but success goes through the rollback.
trap 'rollback; exit 1' EXIT
systemctl enable mintaka-prod.service mintaka-prod-reload.path

# Stop the old process first, so no brief or chat lands in a directory that is no longer read.
pkill -x -u "$deploy" mintaka-prod || true
for _ in $(seq 75); do pgrep -x -u "$deploy" mintaka-prod >/dev/null || break; sleep 0.2; done
pkill -KILL -x -u "$deploy" mintaka-prod || true
if [ ! -d "$state" ]; then
  fresh=1
  install -d -m 750 "$state"
  cp -a "$prod/data/." "$state/"
fi
chown -R mintaka:mintaka "$state"
chmod 750 "$state"
if [ -d "$state/contact" ]; then
  chmod 750 "$state/contact"
  find "$state/contact" -type f -exec chmod 640 {} +
fi

systemctl restart mintaka-prod.service
systemctl start mintaka-prod-reload.path
for _ in $(seq 15); do
  if curl -fs -o /dev/null http://172.20.0.1:8790/mintaka/; then
    trap - EXIT
    journalctl -u mintaka-prod -n 4 --no-pager -o cat
    echo "mintaka-prod runs as the sandboxed service. Check it: systemd-analyze security mintaka-prod"
    exit 0
  fi
  sleep 1
done
