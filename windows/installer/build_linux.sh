#!/usr/bin/env bash
# Compile the existing Inno installer using Wine and the MinGW cross-build.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
if [[ -z "${DISPLAY:-}" ]] && command -v xvfb-run >/dev/null; then
  exec xvfb-run -a bash "${BASH_SOURCE[0]}" "$@"
fi
case "${1:-}" in
  '') (cd "$ROOT" && cargo build --release --locked -p a865rctl); bash "$ROOT/windows/cross_build.sh"; bash "$ROOT/windows/cross_build.sh" i686-pc-windows-gnu ;;
  --no-build) ;;
  *) echo 'Usage: bash windows/installer/build_linux.sh [--no-build]' >&2; exit 2 ;;
esac
WINE="${WINE:-wine}"
: "${ISCC:?Set ISCC to the installed Inno Setup ISCC.exe path (Inno 7 x64 recommended)}"
command -v "$WINE" >/dev/null
export WINEDEBUG="${WINEDEBUG:--all}"
# Isolate both Windows state and Linux menu integration from the desktop user.
export WINEPREFIX="${WINEPREFIX:-$ROOT/.build-cache/inno-wine}"
source "$ROOT/windows/installer/wine_build_environment.sh"
STAGE="$(mktemp -d /tmp/open-volar-s-installer-XXXXXXXX)"
trap 'rm -rf -- "$STAGE"' EXIT
python3 "$ROOT/windows/installer/stage_cross_build.py" "$STAGE"
PAYLOAD_WINDOWS="$("$WINE" winepath -w "$STAGE" | tr -d '\r')"
SCRIPT_WINDOWS="$("$WINE" winepath -w "$ROOT/windows/installer/a865r.iss" | tr -d '\r')"
DIST="${OPEN_VOLAR_S_DIST:-$ROOT/dist}"
mkdir -p "$DIST"
OUTPUT_WINDOWS="$("$WINE" winepath -w "$DIST" | tr -d '\r')"
"$WINE" "$ISCC" "/DCrossPayload=$PAYLOAD_WINDOWS" "/O$OUTPUT_WINDOWS" "$SCRIPT_WINDOWS"
printf 'Installer output: %s/\n' "$DIST"
