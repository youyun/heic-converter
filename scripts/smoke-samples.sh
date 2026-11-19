#!/usr/bin/env bash
# Headless smoke: scan samples + convert beside sources (no GUI dialog).
set -euo pipefail
SAMPLES="${1:-/Users/yuanda/Downloads/heic-samples}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/src-tauri"
source "$HOME/.cargo/env"
cargo test --lib -- --nocapture
echo "OK: converter + job smoke tests passed for samples under $SAMPLES"
