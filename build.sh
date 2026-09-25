#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODE=all
PERMISSIONS=1
KDIR="${KDIR:-/lib/modules/$(uname -r)/build}"
for arg in "$@"; do
  case "$arg" in
    all|--userspace-only|--kernel-only) MODE="$arg" ;;
    --no-permissions) PERMISSIONS=0 ;;
    *) echo "Usage: bash build.sh [--userspace-only|--kernel-only] [--no-permissions]" >&2; exit 2 ;;
  esac
done
if [[ "$MODE" != "--kernel-only" ]]; then
  (cd "$ROOT" && cargo build --release --locked -p a865rctl -p a865r-debug -p open-volar-s-installer -p open-volar-s-live-tv -p open-volar-s-dvb-bridge -p open-volar-s-player)
fi
if [[ "$MODE" != "--userspace-only" ]]; then
  if [[ ! -d "$KDIR" ]]; then
    echo "Kernel headers are missing at $KDIR" >&2
    echo "Install headers for $(uname -r), or build a matching WSL kernel and set KDIR." >&2
    exit 1
  fi
  make -C "$ROOT/linux" KDIR="$KDIR" all
fi
if (( PERMISSIONS )); then
  bash "$ROOT/linux/install_permissions.sh"
fi
