#!/usr/bin/env bash
# Linux -> Windows GNU build. Install rustup + MinGW-w64 first.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="${1:-x86_64-pc-windows-gnu}"
case "$TARGET" in
  x86_64-pc-windows-gnu) PREFIX=x86_64-w64-mingw32 ;;
  i686-pc-windows-gnu) PREFIX=i686-w64-mingw32 ;;
  *) echo 'Use x86_64-pc-windows-gnu or i686-pc-windows-gnu.' >&2; exit 2 ;;
esac
command -v "$PREFIX-g++" >/dev/null
command -v "$PREFIX-windres" >/dev/null
cd "$ROOT"
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
export CARGO_TARGET_I686_PC_WINDOWS_GNU_LINKER=i686-w64-mingw32-gcc
export RC="$PREFIX-windres"
if [[ "$TARGET" == i686-pc-windows-gnu ]]; then
  cargo build --release --locked --target "$TARGET" -p a865r-bda
else
  cargo build --release --locked --target "$TARGET" -p a865rctl -p a865r-debug -p a865r-bda -p a865r-tv
fi
printf 'Windows outputs: %s/%s/release\n' "${CARGO_TARGET_DIR:-$ROOT/target}" "$TARGET"
