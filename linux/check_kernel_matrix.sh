#!/usr/bin/env bash
# Compile the module against a release's published generic kernel headers.
set -euo pipefail
SUITE="${1:?Usage: bash linux/check_kernel_matrix.sh <ubuntu-codename> <kernel-family>}"
FAMILY="${2:?Usage: bash linux/check_kernel_matrix.sh <ubuntu-codename> <kernel-family>}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TMP="$(mktemp -d /tmp/ovs-kernel-matrix-XXXXXXXX)"
trap 'case "$TMP" in /tmp/ovs-kernel-matrix-*) rm -rf -- "$TMP" ;; esac' EXIT
mkdir -p "$TMP/lists/partial" "$TMP/root"
MIRROR=""
for candidate in https://archive.ubuntu.com/ubuntu https://old-releases.ubuntu.com/ubuntu; do
  if curl -4fsSI --max-time 10 "$candidate/dists/$SUITE/Release" >/dev/null; then
    MIRROR="$candidate"
    break
  fi
done
if [[ -z "$MIRROR" ]]; then
  echo "No official Ubuntu archive found for $SUITE" >&2
  exit 1
fi
printf 'deb [arch=amd64] %s %s main\n' "$MIRROR" "$SUITE" > "$TMP/sources.list"
APT_OPTS=(
  -o "Dir::State::lists=$TMP/lists"
  -o "Dir::Etc::sourcelist=$TMP/sources.list"
  -o "Dir::Etc::sourceparts=-"
  -o "Acquire::ForceIPv4=true"
  -o "Acquire::Check-Valid-Until=false"
)
apt-get update -qq "${APT_OPTS[@]}"
PKG="$(apt-cache "${APT_OPTS[@]}" search "^linux-headers-${FAMILY//./\.}.*-generic$" |
  awk '{print $1}' | sort -V | tail -1)"
if [[ -z "$PKG" ]]; then
  echo "No generic Linux $FAMILY headers found for $SUITE" >&2
  exit 1
fi
COMMON="${PKG%-generic}"
(cd "$TMP" && apt-get "${APT_OPTS[@]}" download "$COMMON" "$PKG")
for package in "$TMP"/*.deb; do
  dpkg-deb -x "$package" "$TMP/root"
done
KDIR="$TMP/root/usr/src/$PKG"
if [[ ! -d "$KDIR" ]]; then
  echo "Extracted kernel build directory is missing: $KDIR" >&2
  exit 1
fi
LOG="$TMP/build.log"
if ! { make -C "$KDIR" M="$ROOT" clean && make -C "$KDIR" M="$ROOT" CC="${CC:-gcc}" modules; } > "$LOG" 2>&1; then
  tail -60 "$LOG" >&2
  exit 1
fi
printf 'PASS Ubuntu %s Linux %s (%s)\n' "$SUITE" "$FAMILY" "$PKG"
