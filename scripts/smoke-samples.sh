#!/usr/bin/env bash
# Headless smoke: scan samples + convert beside sources (no GUI dialog).
set -euo pipefail
# Optional samples dir for local HEIC fixtures (tests skip if missing).
SAMPLES="${1:-${HEIC_CONVERTER_SAMPLES:-}}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/src-tauri"
# shellcheck disable=SC1091
source "$HOME/.cargo/env"
if [[ -n "$SAMPLES" ]]; then
  export HEIC_CONVERTER_SAMPLES="$SAMPLES"
fi
cargo test --lib -- --nocapture
echo "OK: converter + job smoke tests passed${SAMPLES:+ for samples under $SAMPLES}"
