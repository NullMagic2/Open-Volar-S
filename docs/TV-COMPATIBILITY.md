# Standard TV application access

The AVerTV Volar S A865R is an ISDB-T / ISDB-Tb receiver. Its supported profile is
6 MHz UHF, 470000–697999 kHz. It does not become a DVB-T, DVB-T2, ATSC or cable
receiver by using the Linux DVB API or Windows BDA API.

## Linux (including VLC)

The USB module now registers a DVB adapter with `frontend0`, `demux0` and `dvr0`
under `/dev/dvb/adapterN/`. The frontend advertises `SYS_ISDBT` first, plus an optional `SYS_DVBT` compatibility
alias (enabled by default). Standard Linux
DVB applications can tune it and receive MPEG transport streams. The existing
`/dev/open-volar-sN` interface remains available to Live TV! and the diagnostic tools.

The `open-volar-s-dvb-bridge` service handles firmware and tuning through the
existing Rust receiver library. It starts automatically when udev discovers a
receiver, claims the raw USB interface only during a DVB tuning session, and
releases it after the DVB frontend closes. Only one application can control a
physical tuner at a time. Close Live TV! before opening it in VLC, and vice versa.

Install the DEB/RPM, then reconnect or reboot if installation reports that the
previous module is still in use. The package installs the bridge, systemd unit,
DKMS source and udev permissions. A native kernel must have `CONFIG_DVB_CORE`;
kernels without it retain the raw USB interface only. WSL must supply DVB core
in its custom kernel to use this additional interface.

For a source build, `bash build.sh` compiles and installs permissions and the
broker automatically. Load the compiled module using `sudo modprobe dvb_core`
followed by `sudo insmod linux/open_volar_s_usb.ko`, or install the package for
persistent DKMS integration. `--no-permissions` builds artifacts without system
installation. Administrator privileges are needed for installation, not playback.

Example using a locally known channel (choose a frequency/program valid in your area):

```sh
vlc 'isdb-t://frequency=641143000:bandwidth=6' --dvb-adapter=0 --program=17056
```

The DVB frequency is in **Hz**, and bandwidth is in **MHz** in VLC URLs.
In VLC 3's capture dialog, which has no ISDB-T button, the compatibility alias
allows **TV - digital → DVB-T**. Choose the adapter number, enter the channel's
frequency in **kHz** (for example **521143** for TV GAZETA HD on this test setup),
and set bandwidth to **6 MHz**. Click **Play** (Reproduzir), not Stream
(Transmissão). Select the desired service in Playback → Program if necessary.
The equivalent URL is `dvb-t://frequency=521143000:bandwidth=6`.

This is only an API alias: the physical tuner still receives ISDB-T/ISDB-Tb.
DVB-T modulation/FEC/guard settings are not used; the ISDB demodulator detects
those parameters automatically. Unsupported frequencies and nonzero bandwidths
other than 6 MHz remain rejected. Bandwidth Auto (0) resolves to 6 MHz.
No transcoding or extra packet processing is introduced; the alias adds only a
small conditional check when tuning. Native `isdb-t://` access remains preferred
when the application supports it.

For standards-only discovery, set `options open_volar_s_usb dvbt_compat=0` in
`/etc/modprobe.d/open-volar-s.conf` and reboot (or reload the idle module).
The module parameter is read-only while loaded so advertised capabilities remain
consistent for each connected adapter. Setting it back to 1 restores the alias.

VLC's incorrect Japanese titles on some Brazilian broadcasts can be worked around
by appending `:ts-standard=dvb` to **Edit Options**. This changes metadata parsing,
not RF tuning, and may affect ISDB-specific subtitles/EPG. VLC's program menu selects a service in the multiplex.
The standard adapter exposes RF tuning; it does not import Live TV!'s saved channel
list into third-party applications. `linux/tests/vlc_smoke.sh` captures a short
sample with VLC and records stream metadata with ffprobe.

The broker node `/dev/open-volar-dvbN` is root-only. Standard DVB device nodes and
the legacy raw USB node grant the active local desktop session access using udev
ACLs, plus the `video` group. Headless service accounts require video-group access.

## Windows (BDA)

The adapter registers discoverable tuner and receiver-component monikers in the
standard BDA categories. It forwards tuning and TS capture to the existing WinUSB
backend. Both x64 and x86 DLL registrations are needed for applications of both
bitnesses. This is a userspace BDA compatibility adapter, not a new signed kernel
AVStream driver. The underlying A865R must already have the project's WinUSB
binding; registration does not replace a vendor driver or sign/install an INF.

The native Inno installer registers both adapters for the current user. Under
Wine, registration automatically uses the prefix-local HKLM registry so Wine
can enumerate the adapters; this does not require Linux root. Third-party
playback under Wine still needs separate validation. For the
cross-built portable bundle, extract it to a permanent directory and run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\register_tuner.ps1
```

Restart VLC/other BDA clients afterward. Use `-Unregister` before deleting the
bundle. A 64-bit application uses `x64/a865r_bda.dll`; a 32-bit application uses
`x86/a865r_bda.dll`. The bundled GNU runtime DLLs must remain beside these files.
Windows runtime discovery and reception require testing on Windows; a Linux
cross-build is not proof of Windows playback compatibility. Applications that
require kernel-only KS interfaces may not support this userspace adapter.

## Interface references

- [Linux DVB frontend API](https://docs.kernel.org/driver-api/media/dtv-frontend.html)
- [Linux DVB demux API](https://docs.kernel.org/driver-api/media/dtv-demux.html)
- [Microsoft DirectShow filter categories](https://learn.microsoft.com/en-us/windows/win32/directshow/filter-categories)
- [Microsoft filter registration layout](https://learn.microsoft.com/en-us/windows/win32/directshow/layout-of-the-registry-keys)
