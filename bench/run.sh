#!/usr/bin/env bash
# The published benchmark, as one command on the machine being measured (see bench/README.md).
#
#   bench/run.sh [--label NAME] [--rates "20000 50000"] [--seconds 10] [--conns 64] [--dist DIR]
#
# Builds the release server (and, without --dist, the release site), serves it on loopback, and
# drives GET /en/ at fixed request rates with oha's latency correction — over plain HTTP/1.1 and
# over TLS 1.3 + HTTP/2 — while recording the server's own /_telemetry ticks. Writes
# bench/results/<date>-<label>.md with the machine's facts, the commands and both views.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
label="local" rates="20000 50000" seconds=10 conns=64 dist=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --label) label="$2"; shift 2 ;;
    --rates) rates="$2"; shift 2 ;;
    --seconds) seconds="$2"; shift 2 ;;
    --conns) conns="$2"; shift 2 ;;
    --dist) dist="$2"; shift 2 ;;
    *) echo "unknown option $1" >&2; exit 2 ;;
  esac
done
[[ "$label" =~ ^[a-zA-Z0-9._-]+$ ]] || { echo "--label: letters, digits, . _ - only" >&2; exit 2; }
oha="$(command -v oha || echo "$HOME/.cargo/bin/oha")"
[[ -x "$oha" ]] || { echo "needs oha (cargo install oha)" >&2; exit 1; }

cd "$root"
cargo build --quiet --release -p server
if [[ -z "$dist" ]]; then
  cargo xtask build --release >/dev/null
  dist="dist"
fi

work="$(mktemp -d)"
cleanup() { kill $(jobs -p) 2>/dev/null || true; wait 2>/dev/null || true; rm -rf "$work"; }
trap cleanup EXIT

free_port() { python3 -I -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])'; }
http_port="$(free_port)" tls_port="$(free_port)"
RUST_LOG=info ./target/release/mintaka-server --dist "$dist" --listen "127.0.0.1:$http_port" \
  --tls-self-signed --tls-listen "127.0.0.1:$tls_port" --data-dir "$work/data" >"$work/server.log" 2>&1 &
for _ in $(seq 50); do curl -sf -o /dev/null "http://127.0.0.1:$http_port/_ping" && break; sleep 0.1; done
curl -sN "http://127.0.0.1:$http_port/_telemetry" >"$work/telemetry.txt" &
sleep 2

# A browser-like User-Agent matters: curl/… would get the curl-mode text instead of the page.
headers=(-H 'accept-encoding: br' -H 'user-agent: Mozilla/5.0 (mintaka benchmark)')
runs=()
for scenario in h1 h2tls; do
  for rate in $rates; do
    if [[ "$scenario" == h1 ]]; then
      url="http://127.0.0.1:$http_port/en/" extra=()
    else
      url="https://127.0.0.1:$tls_port/en/" extra=(--http2 --insecure)
    fi
    start="$(date +%s)"
    "$oha" -z "${seconds}s" -q "$rate" -c "$conns" --latency-correction --no-tui --output-format json \
      "${headers[@]}" "${extra[@]}" "$url" >"$work/$scenario-$rate.json"
    runs+=("$scenario $rate $start $(date +%s)")
    sleep 2
  done
done

mkdir -p bench/results
out="bench/results/$(date -u +%F)-$label.md"
python3 -I - "$work" "$out" "$seconds" "$conns" "$oha" "$dist" "${runs[@]}" <<'PY'
import json, os, platform, re, subprocess, sys
work, out, seconds, conns, oha, dist, *runs = sys.argv[1:]

def sh(*cmd):
    try:
        return subprocess.run(cmd, capture_output=True, text=True, timeout=10).stdout.strip()
    except Exception:
        return ""

def first(pattern, text, default="—"):
    m = re.search(pattern, text, re.M)
    return m.group(1).strip() if m else default

lscpu = sh("lscpu")
meminfo = open("/proc/meminfo").read() if os.path.exists("/proc/meminfo") else ""
governor = sh("cat", "/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor") or "—"
ticks = []
for block in open(os.path.join(work, "telemetry.txt")).read().split("\n\n"):
    if "event: tick" in block:
        ticks.append(json.loads(block.split("data: ", 1)[1]))

ms = lambda s: f"{s * 1000:.3f} ms"
def span(values, fmt):
    values = [v for v in values if v is not None]
    if not values:
        return "—"
    lo, hi = fmt(min(values)), fmt(max(values))
    return lo if lo == hi else f"{lo}–{hi}"

client_rows, server_rows = [], []
names = {"h1": "HTTP/1.1, plain", "h2tls": "HTTP/2, TLS 1.3"}
for run in runs:
    scenario, rate, start, end = run.split()
    d = json.load(open(os.path.join(work, f"{scenario}-{rate}.json")))
    p = d["latencyPercentiles"]
    s = d["summary"]
    ok = d.get("statusCodeDistribution", {}).get("200", 0)
    errors = ", ".join(f"{k}: {v}" for k, v in (d.get("errorDistribution") or {}).items()) or "none"
    client_rows.append(f"| {names[scenario]} | {int(rate):,} /s | {s['requestsPerSec']:,.0f} /s | {ok:,} | "
                       f"{ms(p['p50'])} | {ms(p['p99'])} | {ms(p['p99.9'])} | {ms(p.get('p99.99', s['slowest']))} | {errors} |")
    # Steady state: skip the first two seconds of each run.
    steady = [t for t in ticks if int(start) + 2 <= t["t"] <= int(end)]
    us = lambda v: f"{v} µs"
    server_rows.append(f"| {names[scenario]} | {int(rate):,} /s | {span([t['p50_us'] for t in steady], us)} | "
                       f"{span([t['p99_us'] for t in steady], us)} | {span([t['p999_us'] for t in steady], us)} | "
                       f"{span([t['cpu_pct'] for t in steady], lambda v: f'{v:.0f} %')} | "
                       f"{span([t['rss_bytes'] for t in steady], lambda v: f'{v / 1048576:.0f} MB')} |")

first_run = runs[0].split()
page = json.load(open(os.path.join(work, f"{first_run[0]}-{first_run[1]}.json")))["summary"]["sizePerRequest"]
cpu = first(r"^Model name:\s*(.+)$", lscpu)
cpus = first(r"^CPU\(s\):\s*(.+)$", lscpu)
ram_gib = int(first(r"^MemTotal:\s*(\d+)", meminfo, "0")) / 1048576
rustc, oha_version, title = sh("rustc", "-V"), sh(oha, "--version"), os.path.basename(out)[:-3]
report = f"""# Benchmark run · {title}

Written by `bench/run.sh`; method and how to read it in `bench/README.md`. The load generator ran
on the **same machine** as the server, so both compete for the same cores.

## Machine

| | |
|---|---|
| CPU | {cpu} ({cpus} logical CPUs), governor {governor} |
| RAM | {ram_gib:.1f} GiB (MemTotal) |
| OS | {platform.system()} {platform.release()} |
| Toolchain | {rustc}, release profile (`lto = "fat"`, `codegen-units = 1`, `panic = "abort"`) |
| Load generator | {oha_version}, {conns} connections, {seconds} s per rate, latency correction on |
| Page | `GET /en/` from `{dist}`, brotli, {page:,} B per response |

## Client view (oha, end to end over loopback)

| Protocol | Rate (fixed) | Achieved | 200 OK | p50 | p99 | p99.9 | p99.99 | Errors |
|---|---|---|---|---|---|---|---|---|
""" + "\n".join(client_rows) + """

## Server view (`app` from Server-Timing, aggregated by the server in /_telemetry)

Ranges over the one-second ticks of each run's steady part (its first two seconds skipped);
each tick's percentiles cover the last 10 s.

| Protocol | Rate | `app` p50 | `app` p99 | `app` p99.9 | process CPU | RSS |
|---|---|---|---|---|---|---|
""" + "\n".join(server_rows) + "\n"
open(out, "w").write(report)
print(report)
print(f"wrote {out}")
PY
