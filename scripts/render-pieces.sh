#!/bin/sh
# Renders the eight pieces that come with OP-8 to WAV files (12 seconds each), so they can be heard without the MK3:
#   scripts/render-pieces.sh            # writes out/pieces/piece-1-house.wav ... piece-8-breaks.wav and opens the folder
set -e
cd "$(dirname "$0")/.."
mkdir -p out/pieces
OP8_BENCH_DIR="$(pwd)/out/pieces" cargo test --release -p op8 --test songs -- --nocapture 2>&1 | grep -E "^[0-9] |test result"
ls out/pieces/piece-*.wav
command -v open >/dev/null 2>&1 && open out/pieces || true
