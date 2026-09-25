# Running Live TV! under Wine

The Linux-built 0.9.5 Windows installer includes a native x86_64 Linux helper.
Install the native Linux DEB/RPM first for the tuner driver, automatic device
permissions and mpv. Then run `wine Open-Volar-S-Setup-0.9.5-x64.exe` as your normal
desktop user. Launch Live TV! from the installed shortcut. No token, port, service
command or environment variable needs to be configured.

Setup records the helper location inside that Wine prefix and starts it. Live TV!
and the Windows CLI restart it automatically when needed. The helper listens only
on localhost, generates a random 256-bit token and writes it with mode 0600 in
`drive_c/open-volar-s-bridge.txt`. It exits after two minutes without clients and
removes its configuration. It does not run as root. Uninstall removes the startup
metadata; the idle helper then exits. Each Wine prefix has its own configuration.
Do not share the token file or run Wine with sudo.

Under Wine the Windows controls use native Linux mpv with software decoding and
GPU rendering for compatibility, and the Linux USB driver for tuner access. Wine's X11 driver is required; XWayland is supported,
while Wine's native Wayland driver is not supported by the embedded player. This
path avoids reliance on Wine's Windows media APIs or Windows kernel USB drivers.
The bundled helper targets glibc 2.43, matching these Ubuntu builds. On older
systems rebuild the native helper and installer from source. Native Windows keeps
its existing playback and WinUSB path and does not install the Linux helper.

The optional `open-volar-s-wine` launcher is included with the Linux package and
binary archive. Installed Windows applications work directly through `wine`.
For a portable Windows ZIP, use:

```sh
open-volar-s-wine /path/to/Open-Volar-S-Windows-0.9.5/x64/live-tv.exe
```

Install Wine separately using your distribution's packages. Close other programs
using the receiver before starting live TV. The helper also supports diagnostics,
recording, pause, seek for files, volume, picture controls, audio selection and
snapshots. Wine support is for this application; standard Linux VLC tuner access
uses DVB directly. It does not install a Wine BDA-to-DVB layer for other Windows
TV applications. Native Windows BDA discovery requires testing on Windows.

See BUILD-AND-TEST-REPORT.md for the checks actually performed and remaining limits.

Wine uses approximately 10% larger text and requests 10% larger initial windows,
subject to the available desktop work area. Button labels retain their adaptive
fit behavior for translations. Native Windows sizing is unchanged.

BDA registration under Wine uses the prefix's HKLM registry so Wine's HKCR-based
System Device Enumerator can see both adapter categories. This registry belongs
to the desktop user's Wine prefix; no Linux administrator permission is needed.
Native Windows keeps per-user registration. Old per-user Wine registrations are
removed during migration/uninstall. Filter enumeration and IBaseFilter binding
can work independently of complete third-party BDA tuning/playback support.
