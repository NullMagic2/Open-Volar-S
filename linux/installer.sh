#!/usr/bin/env bash
# Build and open the Rust package installer without Python.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
(cd "$ROOT" && cargo build --release --locked -p open-volar-s-installer)
exec "$ROOT/target/release/open-volar-s-installer"