#!/bin/sh
# Takes the pictures of the manual (guide/img/) from the simulated MK3: the recipes of crates/octachrome/tests/manual.rs
# press what the manual says and save both screens at the end of each step. Run it after changing a screen.
set -e
cd "$(dirname "$0")/.."
raw=$(mktemp -d)
OCTACHROME_BENCH_DIR="$raw" cargo test -p octachrome --test manual -- --test-threads=1
rm -f guide/img/r*.png
python3 scripts/shrink-png.py "$raw" guide/img | tail -3
rm -rf "$raw"
