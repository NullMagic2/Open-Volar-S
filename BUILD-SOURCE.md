# Building Open Volar S

These instructions describe the **0.9.5** source package. Commands are
run from the extracted workspace root in PowerShell on Windows.

## Requirements

- Windows 10/11 x64, Rust with the **x86_64-pc-windows-msvc** toolchain, Visual
  Studio C++ Build Tools, and a Windows SDK.
- Internet access for the first Cargo dependency download. Keep `Cargo.lock`
  and use `--locked` so the build uses the supplied dependency versions.
- For the complete installer: the Rust **i686-pc-windows-msvc** target,
  **Inno Setup 6**, and the matching x64 Microsoft C++ redistributable DLLs
  expected by `windows/installer/build.ps1`.

The locally patched components are under `third-party/`; the complete Cargo
registry is not vendored. Vulkan headers and the broadcast parser are included,
so a separate Vulkan SDK is not required for this build. mpv and FFmpeg are not
build dependencies.

## Build and run Live TV!

```powershell
cargo build -p a865r-tv --release --locked
.\target\release\live-tv.exe
```

The executable is `target/release/live-tv.exe`. The internal Cargo package
name remains `a865r-tv`. Interface textures, icons, and shaders are embedded;
the application does not use a browser runtime.

Normal startup can open the tuner and resume the saved channel. To inspect only
the interface without accessing the receiver, use a separate preview profile:

```powershell
.\target\release\live-tv.exe --ui-preview --profile-dir .\target\ui-preview-profile
```

Runtime playback requires the appropriate Windows media components and graphics
driver. Vulkan Video is the default decoder; Microsoft decoding can be selected
in Settings. This is an alpha release: a successful build does not validate GPU
playback or receiver behavior on a particular computer.

## Build the workspace and run tests

```powershell
cargo build --workspace --exclude open-volar-s-wsl-video-host --release --locked
cargo test -p a865r-tv -p a865r-bda -p liba865r --lib --bins --locked
```

Hardware tests and live playback checks require a supported receiver and GPU;
CPU-only tests are not a substitute for those checks. Test results are not
asserted by these instructions.

The main outputs are:

| Output | Purpose |
| --- | --- |
| `target/release/live-tv.exe` | Live TV! |
| `target/release/a865r-debug.exe` | Debug Desk |
| `target/release/a865rctl.exe` | Command-line receiver tools |
| `target/release/a865r_bda.dll` | x64 BDA compatibility adapter |

To build the additional 32-bit adapter:

```powershell
rustup target add i686-pc-windows-msvc
cargo build -p a865r-bda --release --locked --target i686-pc-windows-msvc
```

Its output is `target/i686-pc-windows-msvc/release/a865r_bda.dll`. Building an
adapter does not register it or change the USB device binding.

## Build the installer

Install Inno Setup 6 and the x86 Rust target above. Locate the x64
`Microsoft.VC145.CRT` redistributable directory supplied by the matching Visual
Studio installation. The helper expects `msvcp140.dll`, `vcruntime140.dll`, and
`vcruntime140_1.dll` there. Replace the example paths with their actual locations:

```powershell
.\windows\installer\build.ps1 `
  -Compiler 'C:\path\to\Inno Setup 6\ISCC.exe' `
  -RuntimeDirectory 'C:\path\to\x64\Microsoft.VC145.CRT'
```

The helper builds the workspace and x86 adapter, stages both adapter
architectures, updates their SHA-256 record, copies the required runtime DLLs,
and invokes Inno Setup. It expects Cargo's default `target/` directory; do not
redirect `CARGO_TARGET_DIR` for this script without also updating its paths.

The installer output is
`Open-Volar-S-Setup-0.9.5-x64.exe`, in the `dist/` directory.
The current installer uses the native diagnostic player and does **not**
require the old mpv runtime files. Ordinary application installation is per-user;
only an optional protected adapter update may request elevation. Setup does not
establish a new WinUSB binding or a fresh BDA registration.

Native live decoding does not use an external FFmpeg process. External FFmpeg
is needed by recording validation/repair and optional processed AVerTV adapter
modes. Those tools are runtime dependencies for those features, not requirements
for compiling Live TV!.

## Source package

The archive contains project source, tests, shaders, the lockfile, installer
scripts, documentation, interface artwork and previews, firmware files, and
the `LICENSES.md` file. It excludes compiled applications/DLLs, Cargo build output,
local broadcast recordings, captures, and personal profiles.

`SOURCE-MANIFEST.json` records every other packaged file's byte count and
SHA-256 hash. It does not hash itself. This combined Windows/Linux source package keeps the application version at **0.9.5**. Platform-specific code lives under `windows/` and `linux/`; `crates/` and `debug/` are shared.

## Licensing

The combined build retains **GPL-3.0-only** licensing because it includes the
adapted receiver implementation. **MIT** is additionally offered for eligible
original project material; its scope and full text are in [LICENSES.md](LICENSES.md).
All project and third-party terms are in [LICENSES.md](LICENSES.md).

## Linux build

See [linux/README.md](linux/README.md) for native Ubuntu and WSL build instructions, package creation, and the Rust graphical installer. Linux Live TV! uses the Rust receiver and mpv presentation window. WSL now launches a Windows hardware helper using the shared Vulkan H.264 decoder, with D3D11VA and DXVA2 hardware fallbacks. Uncompressed NV12 frames cross to WSL; mpv presents them and plays the original audio. Compressed broadcast packets remain in the rewind cache, with no re-encoding. Native Linux retains its existing player path. WSLg presents through Mesa D3D12/OpenGL when Vulkan presentation is unavailable. Build and install the optional Windows helper separately as described in [windows/wsl-video-host/README.md](windows/wsl-video-host/README.md); the ordinary Windows installer build does not require its FFmpeg SDK.

## Cross-build Windows from Linux

Install Rust via rustup, add `x86_64-pc-windows-gnu` (and `i686-pc-windows-gnu`
for the x86 BDA adapter), and install the corresponding MinGW-w64 C/C++ compilers
and windres tools. Then run:

```sh
bash windows/cross_build.sh
bash windows/cross_build.sh i686-pc-windows-gnu
```

The GNU builds use windres for icons and the application manifest and preserve
Windows font/layout settings. Ship any MinGW runtime DLLs listed by objdump next
to the executable/adapter. This builds the native player, Debug Desk, CLI and
BDA adapter; the separate WSL video host needs a Windows FFmpeg SDK and is not
included by this script. Inno Setup installer generation and production driver
signing still need their respective tools. See `docs/TV-COMPATIBILITY.md`.

## Updated deliverables and validation

See [docs/BUILD-AND-TEST-REPORT.md](docs/BUILD-AND-TEST-REPORT.md) for the tested host, actual VLC results, Windows cross-build limitations and artifact list. After building all native and Windows targets, `python3 tools/package_deliverables.py` creates source/binary archives and checksums in `dist/`.

### Windows installer from Linux

The installer can now be compiled under Wine using `windows/installer/build_linux.sh`. See [windows/installer/BUILD-LINUX.md](windows/installer/BUILD-LINUX.md). This produces the same per-user installation layout and registers both BDA adapters. The Linux cross-build bundles GNU runtime DLLs; the Windows MSVC build retains its Microsoft runtime files.
