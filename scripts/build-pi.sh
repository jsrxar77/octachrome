#!/usr/bin/env bash
# Build OP-3 for a Raspberry Pi 4/5 and (optionally) deploy it.
#
#   scripts/build-pi.sh                 # build only
#   scripts/build-pi.sh pi@op3.local    # build, copy, install, restart
#
# Needs `cross` (cargo install cross) and Docker/Podman. On the Pi itself you
# can simply run `cargo build --release` instead.
set -euo pipefail
TARGET=aarch64-unknown-linux-gnu
cd "$(dirname "$0")/.."

cross build --release --target "$TARGET" -p op3
BIN="target/$TARGET/release/op3"
echo "built $BIN ($(du -h "$BIN" | cut -f1))"

HOST="${1:-}"
[ -z "$HOST" ] && exit 0
scp "$BIN" deploy/op3.service deploy/99-maschine-mk3.rules deploy/pi-setup.sh "$HOST":/tmp/
ssh "$HOST" 'sudo bash /tmp/pi-setup.sh /tmp'
