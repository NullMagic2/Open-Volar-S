# Open Volar S 0.9.1 — updated builds and validation

Build date: 2026-09-24. Release version: 0.9.1.
Host: native Ubuntu x86_64, kernel 7.0.0-30-generic, glibc 2.43.
Native Rust 1.93; Windows GNU cross-build Rust 1.98.1 with MinGW-w64 GCC 13.

## Changes

- Linux installs `70-open-volar-s.rules` before the desktop ACL rule runs. Raw USB
  and DVB nodes receive video-group access and active-session read/write ACLs.
  Successful source builds run the permissions installer automatically;
  `--no-permissions` opts out. Packages also trigger rules for existing devices.
- The Linux USB driver registers a standard ISDB-T DVB frontend, demux and DVR.
  A root-only broker service reuses the existing Rust firmware/tuning implementation.
  The service starts automatically on device discovery and releases the raw receiver
  after the DVB client closes. DKMS installation persists across kernel updates.
- Windows BDA registration publishes standard tuner and capture category entries.
  The native installer registers both bitnesses; the portable bundle includes a
  current-user PowerShell registration/unregistration script.
- On native Linux, focusing either visible Live TV!/DAC window raises both,
  preserves keyboard focus, and leaves hidden/minimized windows hidden.
  The application uses X11/XWayland. WSL behavior is unchanged.
- Linux DAC/player fonts are approximately 16% larger, with compact adaptive button
  geometry and a Greek-capable fallback. Native Windows font sizes are unchanged.
- Under Wine, text and initial windows are approximately 10% larger; window
  placement remains bounded by the desktop work area.
- Numeric channel ordering is shared by Linux and Windows; startup, scan results
  and live channel updates maintain ordering and selection.
- Added MinGW resource compilation, cross-build scripts, and deliverable packaging.

## Version 0.9.1 build and installer validation

All nine project Cargo packages, Windows executable version resources, Linux DEB/RPM,
DKMS metadata, installer and source archive are version 0.9.1. Third-party dependency
versions remain unchanged. Native Linux, Windows x64, Windows x86 BDA and Windows
BDA diagnostic probes were rebuilt successfully.

`Open-Volar-S-Setup-0.9.1-x64.exe` was compiled on Linux with the official Inno Setup
7.1.0 x64 compiler under Wine 10.0. It is an unsigned, per-user x64 installer.
The shared Inno script supports MSVC runtimes on Windows and GNU runtimes in the
Linux cross-build. Build using `windows/installer/build_linux.sh`; see
`windows/installer/BUILD-LINUX.md` for prerequisites.

The installer and uninstaller both completed successfully in an isolated Wine
prefix. Both x64 and x86 regsvr32 registration/unregistration commands returned
success. All five installed application/BDA binaries matched the build outputs
byte for byte; adapter hashes and three GNU runtime DLLs were verified.

The initial Wine BDA enumeration failure was traced to registry visibility:
registration wrote HKCU\Software\Classes, while Wine 10's device enumerator read
HKCR and did not see those keys in the test prefix. Registering the identical DLL
in prefix-local HKLM made both tuner and receiver-component monikers appear, and
both bound successfully to IBaseFilter. The adapter now selects that scope
automatically under Wine, removes old per-user entries, and unregisters both
locations. Native Windows retains per-user registration. This is a user-owned
Wine registry change, not a Linux system-wide installation or root requirement.

Wine filter discovery/binding is verified; complete third-party BDA playback and
actual Windows discovery/playback remain unverified.

The final 0.9.1 DEB was installed on this host. DKMS built and installed the module,
normal-user read/write ACLs were verified on the DVB frontend/DVR, and native VLC
successfully captured 1920×1080 H.264 with AAC from program 17056. The installed
native helper matched the final build byte for byte.

Package metadata, all project versions, Windows embedded version resources,
permissions/build-workflow tests and archive checksums were checked for 0.9.1.

## Wine playback and automatic setup (0.9.1)

The Linux-built Windows installer includes the native Linux helper. Setup records
its location and starts it as the desktop user. Normal Windows app/CLI launches
under Wine restart it automatically. Each prefix receives a random 256-bit token
in a mode-0600 file, without manual configuration; the listener is loopback-only.
The idle helper exits and removes its configuration after two minutes.

- The Windows CLI under Wine captured an actual tuner transport stream through
  the authenticated Linux helper (641143 kHz, locked, quality 100).
- The Windows GUI under Wine displayed actual live 1920×1080 television from
  program 17056. An earlier run reported an estimated 434 frames over 14.46 seconds and stopped
  successfully. The embedded video and DAC were visually inspected; this counter
  estimates playback progress and is not an independently measured decoded-frame count.
- File playback, seek and picture-shader output were also visually checked.
- Final live playback passed with both broadcast-size output and the 4K upscale
  shader. Rendered video/window captures contained real picture, and pause/resume,
  volume, selected PIDs, helper restart and install/uninstall checks passed.
- The private token file was verified as mode 0600, owned by desktop user ubuntu.
- Fresh installation and normal launch required no token/environment configuration.
  Install/uninstall from a path containing spaces passed. The installed binaries
  matched the build outputs. Pause/resume and volume were checked through mpv.
- Deliberately stopping the helper followed by an ordinary Windows CLI launch
  restarted it automatically with a new token. Closing the video window did not
  crash the helper. Selected live video/audio PIDs were verified as 301/302.
- Authentication rejection, bounded USB requests and authenticated transport
  roundtrip/disconnect tests passed.

Wine uses Linux mpv gpu-next with software decoding in an embedded X11 window.
The legacy gpu renderer produced black video on this host; gpu-next rendered the
same recording and actual live 1080p TV correctly. The enlarged Wine controls
were visually inspected at 1920×1200. Install the native
Linux package first for driver permissions and mpv. XWayland is supported; Wine's
native Wayland driver is not. These tests do not establish native Windows BDA
compatibility or provide a DVB bridge for arbitrary Windows TV apps under Wine.
The native helper shares the glibc 2.43 requirement of these Linux builds.
See WINE.md for usage. Validation screenshots/diagnostics are in dist/validation.

## Earlier hardware and installation tests (0.9.0 development build)

The following hardware and GUI tests were performed before the version bump.
These checks cover the native driver and Linux UI changes. The Wine playback path
was added and tested separately for 0.9.1.

The attached AVerTV Volar S A865R (USB 07ca:b865) was tested on this host.
The final DEB was installed successfully, including DKMS compilation/installation,
udev reload and automatic broker startup. `/dev/dvb/adapter0/frontend0`, `demux0`
and `dvr0` are present. `getfacl` confirms user `ubuntu` has read/write access.

VLC 3.0.23 ran as the normal desktop user, without sudo:

- 641143000 Hz / program 17056: captured a transport stream and decoded H.264
  1920×1080 video plus AAC 48 kHz stereo. A frame was visually inspected.
- 521143000 Hz / program 16960: captured a second multiplex/channel; ffprobe found
  H.264 1920×1080 and the lower-resolution service.
- A further VLC capture succeeded after installation of the final DEB, verifying
  the packaged driver/service/permissions rather than only the development module.
- The raw diagnostic probe succeeded after VLC closed, confirming the broker
  releases the receiver for Live TV! and diagnostic use.

VLC logs contain nonfatal PSI-section and startup timestamp/clock messages.
Reception tests do not establish perfect playback on every channel or RF condition.
Only one application can control the physical tuner at a time.

## Automated checks

- All 76 earlier native checks now have passing results: 75 passed initially, and
  the remaining `settings_change_the_player_and_persist` check passed after fixing
  its test setup. It now uses the production gpu-next renderer and requests 8-bit
  screenshots, matching its Cairo RGB24 pixel assertions. mpv otherwise produced
  16-bit PNGs; interpreting those surface bytes as RGB24 gave incorrect colors.
  The full rendered-pixel assertions, including warm/cold, saturation, brightness,
  contrast, HDR effect and OSD alpha, remain enabled and pass.
- The final receiver-library run passed 28 tests, including the three Wine bridge
  tests. Ten shared picture/export tests and both packaging workflow tests passed.
- Numeric channel ordering/selection and all four translated button layouts passed.
- Actual X11 stacking test passed for activation from both windows, keyboard focus
  preservation and keeping the unmapped DAC hidden.
- Installation-order and build-workflow Python tests passed. udev rule validation
  and shell syntax checks passed.

Detailed local logs and VLC samples are in `dist/validation/` of the working tree.
Broadcast recordings are intentionally excluded from the distribution archives.

## Build outputs

| Artifact | Contents / validation |
| --- | --- |
| `open-volar-s_0.9.1_amd64.deb` | Native application, diagnostics, broker, udev, systemd and DKMS; installed and hardware-tested as 0.9.1 |
| `open-volar-s-0.9.1-1.x86_64.rpm` | Equivalent RPM; built and inspected, not installed on an RPM distribution |
| `Open-Volar-S-Linux-Binaries-0.9.1.tar.gz` | GUI, CLI, debugger, package-builder GUI, Linux WSL helper and DVB broker |
| `open_volar_s_usb-7.0.0-30-generic.ko` | Module for this exact kernel; use DKMS source on other kernels |
| `Open-Volar-S-Setup-0.9.1-x64.exe` | Windows installer built with Inno Setup under Wine; install/uninstall smoke-tested |
| `Open-Volar-S-Windows-0.9.1.zip` | x64 Live TV!, debugger, CLI, BDA adapter and diagnostic probes; x86 BDA adapter; GNU runtime DLLs and registration script |
| `Open-Volar-S-Source-0.9.1-updated.zip` | Updated source, tests, documentation, build scripts and SHA-256 source manifest |
| `SHA256SUMS` | Checksums of the downloadable artifacts |

The Linux binaries target the host's glibc 2.43. The DEB declares this requirement;
rebuild from source on older Ubuntu versions. RPM installation on another distro
is untested. The standalone graphical package builder requires the extracted
source directory (current directory or `OPEN_VOLAR_S_SOURCE`).

Windows x64 and x86 builds succeeded. PE imports and COM registration exports were
inspected, and required GNU runtime DLLs are included. **Windows discovery and
playback have not been runtime-tested**, because this host runs Linux. BDA support
is a userspace compatibility adapter over the existing WinUSB binding, not a newly
signed kernel AVStream driver. Kernel-only clients may not support it. The package
must be extracted permanently before running `register_tuner.ps1`.

The Windows installer was produced here. A signed Windows driver/catalog and code-signing signature were not produced.
The separate Windows WSL video host requires a Windows FFmpeg development SDK and
was not cross-built. Its source remains included. The normal native Windows Live
TV! application was cross-built successfully.

## Reproduce

Native build: `bash build.sh` (requests administrator rights for permissions and
service installation). Package: `bash linux/package.sh --format deb` or `--format rpm`.
For permanent installation use `sudo apt install ./dist/open-volar-s_0.9.1_amd64.deb`.

Windows: install the MinGW x64/x86 compilers and corresponding Rust GNU targets,
then run `bash windows/cross_build.sh` and
`bash windows/cross_build.sh i686-pc-windows-gnu`.
Optional probes: `cargo build --release --target x86_64-pc-windows-gnu -p a865r-bda --examples`.
Installer: `ISCC=/path/to/ISCC.exe bash windows/installer/build_linux.sh --no-build`.
After building all targets, `python3 tools/package_deliverables.py` regenerates
archives, source manifest and checksums.

See `TV-COMPATIBILITY.md` for standard DVB/BDA use and VLC tuning instructions.
