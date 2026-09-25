#!/usr/bin/env bash
# Called automatically after a local build, or directly to repair an installation.
set -euo pipefail
LINUX="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if (( EUID != 0 )); then
  echo 'Installing Open Volar S device permissions (administrator access required).'
  if command -v sudo >/dev/null 2>&1; then
    exec sudo -- bash "$LINUX/install_permissions.sh"
  elif command -v pkexec >/dev/null 2>&1; then
    exec pkexec bash "$LINUX/install_permissions.sh"
  else
    echo "Run as root: bash '$LINUX/install_permissions.sh'" >&2
    exit 1
  fi
fi
install -Dm644 "$LINUX/70-open-volar-s.rules" /etc/udev/rules.d/70-open-volar-s.rules
# A normal source build also installs the broker needed by DVB applications.
if [[ -x "$LINUX/../target/release/open-volar-s-dvb-bridge" ]]; then
  install -Dm755 "$LINUX/../target/release/open-volar-s-dvb-bridge" /usr/bin/open-volar-s-dvb-bridge
  install -Dm644 "$LINUX/open-volar-s-dvb@.service" /usr/lib/systemd/system/open-volar-s-dvb@.service
  systemctl daemon-reload
fi
udevadm control --reload-rules
udevadm trigger --action=add --subsystem-match=usbmisc --sysname-match='open-volar-s[0-9]*'
udevadm trigger --action=add --subsystem-match=misc --sysname-match='open-volar-dvb[0-9]*'
udevadm trigger --action=add --subsystem-match=dvb
udevadm settle --timeout=10
echo 'Open Volar S device permissions installed and applied.'
