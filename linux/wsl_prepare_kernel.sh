#!/usr/bin/env bash
# Prepare exact Microsoft WSL kernel symbols, then build and load the A865R USB module.
set -euo pipefail
if [[ "$(id -u)" != 0 ]]; then
  echo "Run as root: sudo bash linux/wsl_prepare_kernel.sh" >&2
  exit 1
fi
release="$(uname -r)"
case "$release" in
  *-microsoft-standard-WSL2) ;;
  *) echo "This helper requires a Microsoft WSL2 kernel. Native Linux uses DKMS and distro headers." >&2; exit 1 ;;
esac
version="$(printf '%s' "$release" | sed 's/-microsoft-standard-WSL2$//')"
module_dir="$(cd "$(dirname "$0")" && pwd)"
saved_module="/var/lib/open-volar-s/wsl-modules/$release/open_volar_s_usb.ko"
load_saved() {
  if [[ ! -f "$saved_module" ]]; then
    echo "No cached Open Volar S module for $release. Run wsl_prepare_kernel.sh once to build it." >&2
    return 1
  fi
  install -Dm644 "$saved_module" "/lib/modules/$release/extra/open_volar_s_usb.ko"
  depmod -a "$release"
  modprobe open_volar_s_usb
}
if [[ "${1:-}" == --load-only ]]; then load_saved; exit; fi
cache="/var/lib/open-volar-s/wsl-kernel/$release"
tree="$(printenv OVS_WSL_KERNEL_TREE || true)"
if [[ -z "$tree" ]]; then
  tree="$cache/WSL2-Linux-Kernel-linux-msft-wsl-$version"
fi
if [[ ! -s "$tree/Module.symvers" ]]; then
  if command -v apt-get >/dev/null; then
    DEBIAN_FRONTEND=noninteractive apt-get install -y build-essential gcc-11 flex bison bc cpio curl libelf-dev libssl-dev dwarves
  else
    echo "Install GCC 11, flex, bison, bc, cpio, curl, libelf, OpenSSL headers, and dwarves first." >&2
    exit 1
  fi
  if [[ ! -f "$tree/Makefile" ]]; then
    mkdir -p "$cache"
    archive="$cache/linux-msft-wsl-$version.tar.gz"
    curl -fsSL --retry 2 "https://github.com/microsoft/WSL2-Linux-Kernel/archive/refs/tags/linux-msft-wsl-$version.tar.gz" -o "$archive"
    tar -xzf "$archive" -C "$cache"
  fi
  if [[ -r /proc/config.gz ]]; then
    zcat /proc/config.gz > "$tree/.config"
  else
    cp "$tree/Microsoft/config-wsl" "$tree/.config"
  fi
  # GCC 11 with newer distro headers detects this host-only libbpf const warning.
  sed -i -E 's/^([[:space:]]*)(const[[:space:]]+)*char \*next_path;/\1const char *next_path;/' "$tree/tools/lib/bpf/libbpf.c"
  make -C "$tree" CC=gcc-11 HOSTCC=gcc-11 olddefconfig
  built="$(make -s -C "$tree" CC=gcc-11 kernelrelease)"
  if [[ "$built" != "$release" ]]; then
    echo "Prepared kernel $built does not match running kernel $release" >&2
    exit 1
  fi
  make -j"$(nproc)" -C "$tree" CC=gcc-11 HOSTCC=gcc-11 vmlinux modules
fi
if [[ ! -s "$tree/Module.symvers" ]]; then
  echo "Matching Module.symvers was not generated." >&2
  exit 1
fi
ln -sfn "$tree" "/lib/modules/$release/build"
make -C "$tree" M="$module_dir" CC=gcc-11 modules
install -Dm644 "$module_dir/open_volar_s_usb.ko" "$saved_module"
load_saved
# WSL may recreate /lib/modules from its module VHD at startup. Keep the built
# module in the distro filesystem and restore it without rebuilding at boot.
install -Dm755 "$0" /usr/lib/open-volar-s/wsl_prepare_kernel.sh
cat > /etc/systemd/system/open-volar-s-wsl-module.service <<'SERVICE'
[Unit]
Description=Restore the cached Open Volar S module for the current WSL kernel
ConditionVirtualization=wsl
After=systemd-modules-load.service

[Service]
Type=oneshot
ExecStart=/usr/lib/open-volar-s/wsl_prepare_kernel.sh --load-only
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
SERVICE
systemctl daemon-reload
systemctl enable open-volar-s-wsl-module.service
echo "Loaded open_volar_s_usb for $release."
if ls /dev/open-volar-s* >/dev/null 2>&1; then
  ls -l /dev/open-volar-s*
else
  echo "No A865R device node yet. Attach USB ID 07ca:b865 with usbipd on Windows."
fi
