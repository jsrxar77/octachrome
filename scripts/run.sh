#!/usr/bin/env bash
# Start OP-8 on the Maschine MK3 with one command, from anywhere.
#
#   scripts/run.sh                     # build, free the MK3, play
#   scripts/run.sh -l                  # also print every control event (is PLAY arriving?)
#   scripts/run.sh -s                  # also print how much the screens send each second
#   scripts/run.sh -r                  # reset the MK3 on the USB bus first (a stuck device)
#   scripts/run.sh -t chrome            # anything else goes to `op8 run` (a theme, --script f.op8)
#
# It closes what gets in the way of the MK3 (a previous op8, and NI's background
# services, which hold the screens), builds, waits for the MK3 to appear on USB,
# and starts. Ctrl-C closes OP-8. To get NI's software back afterwards, open
# Native Access or Maschine again.
set -euo pipefail
cd "$(dirname "$0")/.."

log=0 stats=0 reset=0
# Our options come first; everything after them (-t chrome, --script f) goes to op8.
while [ $# -gt 0 ]; do
  case "$1" in
    -l) log=1; shift ;;
    -s) stats=1; shift ;;
    -r) reset=1; shift ;;
    -h|--help) awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "$0"; exit 0 ;;
    *) break ;;
  esac
done

# Rust: rustup puts it here, and a fresh terminal may not have it on the PATH.
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found. Install Rust first: https://rustup.rs" >&2
  exit 1
fi

step() { printf '\n== %s\n' "$1"; }

step "Freeing the MK3"
if pgrep -x op8 >/dev/null 2>&1; then
  echo "closing a running op8"
  pkill -x op8 || true
  sleep 1
fi
ni="NIHardwareAgent NIHostIntegrationAgent NTKDaemon"
for p in $ni; do
  if pgrep -x "$p" >/dev/null 2>&1; then
    echo "closing $p (it holds the MK3's screens)"
    pkill -x "$p" || true
  fi
done

step "Building"
cargo build --release 2>&1 | grep -E "^(error|warning: unused)|Compiling op8|Finished" || true
[ -x target/release/op8 ] || { echo "the build failed; run: cargo build --release" >&2; exit 1; }

if [ "$reset" = 1 ]; then
  step "Resetting the MK3 on USB"
  target/release/op8 usb-reset || echo "reset failed (is the MK3 plugged in?)" >&2
fi

# macOS: wait for the MK3 to be on the bus (it takes a few seconds after a reset).
if command -v ioreg >/dev/null 2>&1; then
  step "Waiting for the MK3"
  for _ in $(seq 1 15); do
    ioreg -p IOUSB -l -w0 2>/dev/null | grep -q "Maschine MK3@" && break
    sleep 1
  done
  if ioreg -p IOUSB -l -w0 2>/dev/null | grep -q "Maschine MK3@"; then
    echo "found it"
  else
    echo "no Maschine MK3 on USB. Plug it in (straight into the Mac, if you can) and run this again." >&2
    exit 1
  fi
fi

step "Starting OP-8 (Ctrl-C to close)"
[ "$log" = 1 ] && export OP8_LOG_EVENTS=1
[ "$stats" = 1 ] && export OP8_DISPLAY_STATS=1
exec target/release/op8 run "$@"
