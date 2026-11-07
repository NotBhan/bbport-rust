#!/usr/bin/env bash
# Run the Rust launcher. Arguments are forwarded to bbport-rs:
#   ./run-rust-launcher.sh status
#   ./run-rust-launcher.sh route
#   ./run-rust-launcher.sh launch
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

BIN="target/release/bbport-rs"

if [[ ! -x "$BIN" ]]; then
    echo "Building $BIN..." >&2
    cargo build --release
fi

# With no arguments, show the launcher readiness status instead of the usage text.
if (( $# == 0 )); then
    exec "$BIN" status
fi

exec "$BIN" "$@"
