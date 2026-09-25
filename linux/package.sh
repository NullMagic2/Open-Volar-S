#!/usr/bin/env bash
# Build and optionally install Open Volar S DEB/RPM packages without Python.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LINUX="$ROOT/linux"
DIST="${OPEN_VOLAR_S_DIST:-$ROOT/dist}"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
[[ "$TARGET_DIR" = /* ]] || TARGET_DIR="$ROOT/$TARGET_DIR"
DKMS_VERSION="0.9.5"
DEB_VERSION="0.9.5"
RPM_VERSION="0.9.5"
RPM_RELEASE="1"
GLIBC_VERSION="$(getconf GNU_LIBC_VERSION)"
GLIBC_VERSION="${GLIBC_VERSION#glibc }"
FORMAT="native"
INSTALL=""
while (($#)); do
  case "$1" in
    --format) FORMAT="${2:?Missing format}"; shift 2 ;;
    --install) INSTALL="${2:?Missing install format}"; shift 2 ;;
    *) echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
done
case "$FORMAT" in native|deb|rpm|all) ;; *) echo "Invalid format: $FORMAT" >&2; exit 2 ;; esac
case "$INSTALL" in ''|auto|deb|rpm) ;; *) echo "Invalid install format: $INSTALL" >&2; exit 2 ;; esac
if [[ "$(uname -m)" != x86_64 ]]; then
  echo 'Linux packages currently target x86_64 only.' >&2; exit 1
fi
. /etc/os-release
case " ${ID:-} ${ID_LIKE:-} " in
  *ubuntu*|*debian*|*mint*|*pop*) NATIVE=deb ;;
  *fedora*|*rhel*|*centos*|*suse*|*rocky*|*alma*) NATIVE=rpm ;;
  *) echo 'Cannot identify DEB or RPM distribution from /etc/os-release' >&2; exit 1 ;;
esac
TMP="$(mktemp -d /tmp/open-volar-s-package-XXXXXXXX)"
trap 'case "$TMP" in /tmp/open-volar-s-package-*) rm -rf -- "$TMP" ;; esac' EXIT
mkdir -p "$DIST" "$TMP/open-volar-s-files/bin" "$TMP/open-volar-s-files/src"
PAYLOAD="$TMP/open-volar-s-files"
printf '+ cargo build --release --locked -p a865rctl -p a865r-debug -p open-volar-s-live-tv -p open-volar-s-dvb-bridge -p open-volar-s-player\n'
(cd "$ROOT" && cargo build --release --locked -p a865rctl -p a865r-debug -p open-volar-s-live-tv -p open-volar-s-dvb-bridge -p open-volar-s-player)
install -m755 "$TARGET_DIR/release/open-volar-s-dvb-bridge" "$PAYLOAD/bin/open-volar-s-dvb-bridge"
install -m644 "$LINUX/open-volar-s-dvb@.service" "$PAYLOAD/open-volar-s-dvb@.service"
install -m755 "$LINUX/open-volar-s-wine" "$PAYLOAD/bin/open-volar-s-wine"
install -m755 "$TARGET_DIR/release/a865rctl" "$PAYLOAD/bin/a865rctl"
install -m755 "$TARGET_DIR/release/a865r-debug" "$PAYLOAD/bin/a865r-debug"
install -m755 "$TARGET_DIR/release/open-volar-s-player" "$PAYLOAD/bin/open-volar-s-player"
install -m755 "$TARGET_DIR/release/open-volar-s-live-tv" "$PAYLOAD/bin/open-volar-s-live-tv"
install -m755 "$TARGET_DIR/release/open-volar-s-wsl-player" "$PAYLOAD/bin/open-volar-s-wsl-player"
for name in Makefile open_volar_s_usb.c open_volar_s_usb.h open_volar_s_dvb.c open_volar_s_dvb.h dkms.conf wsl_prepare_kernel.sh install_permissions.sh 70-open-volar-s.rules; do
  install -m644 "$LINUX/$name" "$PAYLOAD/src/$name"
done
install -m644 "$LINUX/70-open-volar-s.rules" "$PAYLOAD/70-open-volar-s.rules"
install -m644 "$LINUX/open-volar-s-debug.desktop" "$PAYLOAD/open-volar-s-debug.desktop"
install -m644 "$LINUX/open-volar-s-live-tv.desktop" "$PAYLOAD/open-volar-s-live-tv.desktop"
install -m644 "$ROOT/images/open-volar-s-256.png" "$PAYLOAD/open-volar-s.png"
install -m644 "$ROOT/LICENSES.md" "$PAYLOAD/LICENSES.md"
install -m644 "$ROOT/GUI/Linux/fonts/selawk.ttf" "$PAYLOAD/selawk.ttf"
install -m644 "$ROOT/GUI/Linux/fonts/selawksb.ttf" "$PAYLOAD/selawksb.ttf"

for name in COMPATIBILITY.md THIRD_PARTY.md; do
  install -m644 "$ROOT/$name" "$PAYLOAD/$name"
done
install -m644 "$ROOT/docs/NATIVE-LINUX-PLAYER.md" "$PAYLOAD/NATIVE-LINUX-PLAYER.md"

build_deb() {
  command -v dpkg-deb >/dev/null || { echo 'dpkg-deb is required.' >&2; exit 1; }
  local stage="$TMP/deb-stage" source_dir="$TMP/deb-stage/usr/src/open-volar-s-$DKMS_VERSION"
  install -Dm755 "$PAYLOAD/bin/open-volar-s-dvb-bridge" "$stage/usr/bin/open-volar-s-dvb-bridge"
  install -Dm644 "$PAYLOAD/open-volar-s-dvb@.service" "$stage/usr/lib/systemd/system/open-volar-s-dvb@.service"
  install -Dm755 "$PAYLOAD/bin/open-volar-s-wine" "$stage/usr/bin/open-volar-s-wine"
  install -Dm755 "$PAYLOAD/bin/a865rctl" "$stage/usr/bin/a865rctl"
  install -Dm755 "$PAYLOAD/bin/a865r-debug" "$stage/usr/bin/a865r-debug"
  install -Dm755 "$PAYLOAD/bin/open-volar-s-player" "$stage/usr/bin/open-volar-s-player"
  install -Dm755 "$PAYLOAD/bin/open-volar-s-live-tv" "$stage/usr/bin/open-volar-s-live-tv"
  install -Dm755 "$PAYLOAD/bin/open-volar-s-wsl-player" "$stage/usr/bin/open-volar-s-wsl-player"
  install -Dm644 "$PAYLOAD/open-volar-s-debug.desktop" "$stage/usr/share/applications/open-volar-s-debug.desktop"
  install -Dm644 "$PAYLOAD/open-volar-s-live-tv.desktop" "$stage/usr/share/applications/open-volar-s-live-tv.desktop"
  install -Dm644 "$PAYLOAD/open-volar-s.png" "$stage/usr/share/icons/hicolor/256x256/apps/open-volar-s.png"
  install -Dm644 "$PAYLOAD/70-open-volar-s.rules" "$stage/etc/udev/rules.d/70-open-volar-s.rules"
  install -Dm644 "$PAYLOAD/LICENSES.md" "$stage/usr/share/doc/open-volar-s/LICENSES.md"
  ln -s LICENSES.md "$stage/usr/share/doc/open-volar-s/copyright"
    install -Dm644 "$PAYLOAD/selawk.ttf" "$stage/usr/share/fonts/truetype/open-volar-s/selawk.ttf"
  install -Dm644 "$PAYLOAD/selawksb.ttf" "$stage/usr/share/fonts/truetype/open-volar-s/selawksb.ttf"
  for name in COMPATIBILITY.md THIRD_PARTY.md NATIVE-LINUX-PLAYER.md; do
    install -Dm644 "$PAYLOAD/$name" "$stage/usr/share/doc/open-volar-s/$name"
  done
  mkdir -p "$source_dir" "$stage/DEBIAN"
  for name in Makefile open_volar_s_usb.c open_volar_s_usb.h open_volar_s_dvb.c open_volar_s_dvb.h dkms.conf wsl_prepare_kernel.sh install_permissions.sh 70-open-volar-s.rules; do
    install -m644 "$PAYLOAD/src/$name" "$source_dir/$name"
  done
  install -m755 "$LINUX/postinst.sh" "$stage/DEBIAN/postinst"
  install -m755 "$LINUX/prerm.sh" "$stage/DEBIAN/prerm"
  cat > "$stage/DEBIAN/control" <<CONTROL
Package: open-volar-s
Version: $DEB_VERSION
Architecture: amd64
Maintainer: Open Volar S contributors
Section: video
Priority: optional
Depends: libc6 (>= $GLIBC_VERSION), dkms, mpv, libfaad2, libpulse0, libvulkan1, libx11-6, liblcms2-2, libfontconfig1, libfreetype6, fonts-dejavu-core, x11-utils, libgtk-3-0 | libgtk-3-0t64, libgl1
Description: AVerTV Volar S A865R userspace tuner tools and Linux USB driver
 Rust receiver tools, graphical Debug Desk, and DKMS source for the USB driver.
CONTROL
  local package="$DIST/open-volar-s_${DEB_VERSION}_amd64.deb"
  dpkg-deb --build --root-owner-group "$stage" "$package"
  printf 'DEB package: %s\n' "$package"
}

build_rpm() {
  command -v rpmbuild >/dev/null || { echo 'rpmbuild is required (on Ubuntu: sudo apt install rpm).' >&2; exit 1; }
  local top="$TMP/rpmbuild" source_dir="/usr/src/open-volar-s-$DKMS_VERSION"
  mkdir -p "$top"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS,rpmdb}
  tar -C "$TMP" -czf "$top/SOURCES/open-volar-s-files.tar.gz" open-volar-s-files
  cat > "$top/SPECS/open-volar-s.spec" <<SPEC
%global debug_package %{nil}
Name:           open-volar-s
Version:        $RPM_VERSION
Release:        $RPM_RELEASE%{?dist}
Summary:        AVerTV Volar S A865R receiver tools, Debug Desk, and Linux USB driver
License:        GPL-3.0-only
BuildArch:      x86_64
Requires:       dkms
Requires:       mpv
Requires:       libfaad.so.2()(64bit)
Requires:       libpulse.so.0()(64bit)
Requires:       libvulkan.so.1()(64bit)
Requires:       libX11.so.6()(64bit)
Requires:       lcms2
Requires:       fontconfig
Requires:       freetype
Requires:       dejavu-sans-mono-fonts
Requires:       xprop
Requires:       gtk3
Requires:       libGL.so.1()(64bit)
Source0:        open-volar-s-files.tar.gz

%description
Rust receiver tools, graphical Debug Desk, and DKMS source for the USB driver.

%prep
%setup -q -n open-volar-s-files

%build

%install
install -Dm755 bin/open-volar-s-dvb-bridge %{buildroot}/usr/bin/open-volar-s-dvb-bridge
install -Dm644 open-volar-s-dvb@.service %{buildroot}/usr/lib/systemd/system/open-volar-s-dvb@.service
install -Dm644 src/open_volar_s_dvb.c %{buildroot}$source_dir/open_volar_s_dvb.c
install -Dm644 src/open_volar_s_dvb.h %{buildroot}$source_dir/open_volar_s_dvb.h
install -Dm755 bin/open-volar-s-wine %{buildroot}/usr/bin/open-volar-s-wine
install -Dm755 bin/a865rctl %{buildroot}/usr/bin/a865rctl
install -Dm755 bin/a865r-debug %{buildroot}/usr/bin/a865r-debug
install -Dm755 bin/open-volar-s-player %{buildroot}/usr/bin/open-volar-s-player
install -Dm755 bin/open-volar-s-live-tv %{buildroot}/usr/bin/open-volar-s-live-tv
install -Dm755 bin/open-volar-s-wsl-player %{buildroot}/usr/bin/open-volar-s-wsl-player
install -Dm644 open-volar-s-debug.desktop %{buildroot}/usr/share/applications/open-volar-s-debug.desktop
install -Dm644 open-volar-s-live-tv.desktop %{buildroot}/usr/share/applications/open-volar-s-live-tv.desktop
install -Dm644 open-volar-s.png %{buildroot}/usr/share/icons/hicolor/256x256/apps/open-volar-s.png
install -Dm644 src/Makefile %{buildroot}$source_dir/Makefile
install -Dm644 src/open_volar_s_usb.c %{buildroot}$source_dir/open_volar_s_usb.c
install -Dm644 src/open_volar_s_usb.h %{buildroot}$source_dir/open_volar_s_usb.h
install -Dm644 src/dkms.conf %{buildroot}$source_dir/dkms.conf
install -Dm755 src/wsl_prepare_kernel.sh %{buildroot}$source_dir/wsl_prepare_kernel.sh
install -Dm644 src/install_permissions.sh %{buildroot}$source_dir/install_permissions.sh
install -Dm644 src/70-open-volar-s.rules %{buildroot}$source_dir/70-open-volar-s.rules
install -Dm644 70-open-volar-s.rules %{buildroot}/etc/udev/rules.d/70-open-volar-s.rules
install -Dm644 LICENSES.md %{buildroot}/usr/share/doc/open-volar-s/LICENSES.md
install -Dm644 selawk.ttf %{buildroot}/usr/share/fonts/truetype/open-volar-s/selawk.ttf
install -Dm644 selawksb.ttf %{buildroot}/usr/share/fonts/truetype/open-volar-s/selawksb.ttf

for name in COMPATIBILITY.md THIRD_PARTY.md NATIVE-LINUX-PLAYER.md; do
  install -Dm644 "\$name" "%{buildroot}/usr/share/doc/open-volar-s/\$name"
done

%post
SPEC
  tail -n +2 "$LINUX/postinst.sh" >> "$top/SPECS/open-volar-s.spec"
  printf '\n' >> "$top/SPECS/open-volar-s.spec"
  cat >> "$top/SPECS/open-volar-s.spec" <<'SPEC'
%preun
if [ "$1" -eq 0 ]; then
SPEC
  tail -n +2 "$LINUX/prerm.sh" >> "$top/SPECS/open-volar-s.spec"
  printf '\n' >> "$top/SPECS/open-volar-s.spec"
  cat >> "$top/SPECS/open-volar-s.spec" <<SPEC
fi

%files
/usr/bin/open-volar-s-dvb-bridge
/usr/lib/systemd/system/open-volar-s-dvb@.service
$source_dir/open_volar_s_dvb.c
$source_dir/open_volar_s_dvb.h
/usr/bin/a865rctl
/usr/bin/open-volar-s-wine
/usr/bin/a865r-debug
/usr/bin/open-volar-s-live-tv
/usr/bin/open-volar-s-player
/usr/bin/open-volar-s-wsl-player
/usr/share/applications/open-volar-s-debug.desktop
/usr/share/applications/open-volar-s-live-tv.desktop
/usr/share/icons/hicolor/256x256/apps/open-volar-s.png
$source_dir/Makefile
$source_dir/open_volar_s_usb.c
$source_dir/open_volar_s_usb.h
$source_dir/dkms.conf
$source_dir/wsl_prepare_kernel.sh
$source_dir/install_permissions.sh
$source_dir/70-open-volar-s.rules
/etc/udev/rules.d/70-open-volar-s.rules
/usr/share/doc/open-volar-s/COMPATIBILITY.md
/usr/share/doc/open-volar-s/THIRD_PARTY.md
/usr/share/doc/open-volar-s/NATIVE-LINUX-PLAYER.md
/usr/share/doc/open-volar-s/LICENSES.md
/usr/share/fonts/truetype/open-volar-s/selawk.ttf
/usr/share/fonts/truetype/open-volar-s/selawksb.ttf
SPEC
  rpmbuild -bb --nodeps --define "_topdir $top" --define "_dbpath $top/rpmdb" --define "_tmppath $TMP" "$top/SPECS/open-volar-s.spec"
  local package
  package="$(find "$top/RPMS" -name '*.rpm' -type f -print -quit)"
  [[ -n "$package" ]] || { echo 'RPM build produced no package.' >&2; exit 1; }
  cp -- "$package" "$DIST/$(basename "$package")"
  printf 'RPM package: %s\n' "$DIST/$(basename "$package")"
}

case "$FORMAT" in
  native) "build_$NATIVE" ;;
  all) build_deb; build_rpm ;;
  deb|rpm) "build_$FORMAT" ;;
esac
if [[ -n "$INSTALL" ]]; then
  [[ "$INSTALL" == auto ]] && INSTALL="$NATIVE"
  if [[ "$INSTALL" != "$NATIVE" ]]; then
    echo "Cannot install $INSTALL on this $NATIVE distribution." >&2; exit 1
  fi
  if [[ "$INSTALL" == deb ]]; then
    PACKAGE="$DIST/open-volar-s_${DEB_VERSION}_amd64.deb"
    COMMAND=(apt-get install --reinstall -y "$PACKAGE")
  else
    PACKAGE="$(find "$DIST" -maxdepth 1 -name "open-volar-s-${RPM_VERSION}-${RPM_RELEASE}*.rpm" -type f -print -quit)"
    if command -v dnf >/dev/null; then COMMAND=(dnf install -y "$PACKAGE")
    elif command -v zypper >/dev/null; then COMMAND=(zypper --non-interactive install "$PACKAGE")
    else COMMAND=(rpm -Uvh --replacepkgs "$PACKAGE"); fi
  fi
  if (( EUID == 0 )); then "${COMMAND[@]}"
  elif command -v sudo >/dev/null && sudo -n true 2>/dev/null; then sudo "${COMMAND[@]}"
  elif [[ -n "${WSL_DISTRO_NAME:-}" ]] && command -v wsl.exe >/dev/null; then wsl.exe -d "$WSL_DISTRO_NAME" -u root -- "${COMMAND[@]}"
  elif command -v pkexec >/dev/null; then pkexec "${COMMAND[@]}"
  else echo 'Installation requires root; configure sudo or polkit.' >&2; exit 1; fi
  printf 'Installed %s\n' "$PACKAGE"
fi
