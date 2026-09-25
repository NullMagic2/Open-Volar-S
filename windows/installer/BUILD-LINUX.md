# Build the Windows installer on Linux

Requires MinGW-w64 x64/x86 C++ compilers and windres, Rust's
`x86_64-pc-windows-gnu` and `i686-pc-windows-gnu` targets, Python 3, Wine and
Inno Setup, a native Linux Rust toolchain and libX11 development headers. Xvfb is useful on a headless build machine.

The compiler is available from the [official Inno Setup downloads](https://jrsoftware.org/isdl.php).
Inno Setup 7's x64 compiler works with 64-bit Wine; it can also emit a native x64
installer. The shared script remains compatible with Inno Setup 6 on Windows.

Install Inno Setup into a dedicated Wine prefix (substitute your downloaded file):

```sh
(
export WINEPREFIX="$PWD/.build-cache/inno-wine"
source windows/installer/wine_build_environment.sh
wine /path/to/innosetup-7.1.0-x64.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP- '/DIR=C:\InnoSetup'
export ISCC="$WINEPREFIX/drive_c/InnoSetup/ISCC.exe"
bash windows/installer/build_linux.sh
)
```

The subshell keeps these environment settings out of your normal desktop session.
Use the same environment helper before every installer smoke test, including
installation and uninstallation. It disables Wine's menu builder and places XDG
data/config/cache inside the test prefix. A separate WINEPREFIX alone still lets
Wine create Linux application-menu shortcuts pointing at temporary test files.
The build script applies this isolation automatically. Normal user installation
under Wine should retain its desktop integration and does not use this helper.

`WINE` may specify a custom Wine executable. `--no-build` reuses already compiled
Windows binaries. Without that flag, both architectures are built first. The
script uses Xvfb automatically when no DISPLAY is available.

Output: `dist/Open-Volar-S-Setup-0.9.5-x64.exe`.

The staging helper includes the actual GNU runtime dependencies, both BDA DLLs,
and freshly calculated adapter hashes. The same Inno script supports the original
MSVC Windows build through `build.ps1`. The installer preserves its per-user
installation, shortcuts, BDA registration/unregistration and optional AverTV update.
It requires the existing WinUSB binding; it does not install a signed kernel driver.

The installer is unsigned. Code signing requires the project's signing certificate
and is separate from compiling the installer. Wine installation tests do not prove
hardware reception or full Windows application compatibility.

The default build also compiles the native Linux a865rctl helper. With --no-build,
provide target/release/a865rctl alongside the Windows outputs. The cross-built
installer installs this helper only under Wine and configures automatic startup
and private authentication. See ../../docs/WINE.md.
