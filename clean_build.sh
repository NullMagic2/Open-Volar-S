#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KDIR="${KDIR:-/lib/modules/$(uname -r)/build}"
(cd "$ROOT" && cargo clean)
if [[ -d "$KDIR" ]]; then
  make -C "$ROOT/linux" KDIR="$KDIR" clean
else
  rm -f -- "$ROOT/linux/open_volar_s_usb.o" "$ROOT/linux/open_volar_s_usb.ko" \
    "$ROOT/linux/open_volar_s_usb.mod" "$ROOT/linux/open_volar_s_usb.mod.c" \
    "$ROOT/linux/open_volar_s_usb.mod.o" "$ROOT/linux/Module.symvers" \
    "$ROOT/linux/modules.order" "$ROOT/linux/.open_volar_s_usb.o.cmd" \
    "$ROOT/linux/.open_volar_s_usb.ko.cmd" "$ROOT/linux/.open_volar_s_usb.mod.cmd" \
    "$ROOT/linux/.open_volar_s_usb.mod.o.cmd"
  rm -rf -- "$ROOT/linux/.tmp_versions"
fi
