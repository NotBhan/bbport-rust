#!/usr/bin/env bash
# Run the native Rust GTK4 launcher.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

BIN="target/release/bbport-gui"

if [[ ! -x "$BIN" ]]; then
    echo "Building $BIN..." >&2
    cargo build --release --bin bbport-gui
fi

exec "$BIN" "$@"
