# Linux port

This is the Linux side of one shared Cargo workspace. Windows-only applications, drivers, and installer sources are in [windows/](../windows/); the Rust tuner library, CLI, and Debug Desk are shared.

The Linux port provides a USB character driver (`open_volar_s_usb`), the shared Rust receiver library, `a865rctl`, and a graphical Rust package installer. The kernel module exposes the A865R bulk endpoints; firmware upload, tuner control, and MPEG transport-stream capture remain in Rust userspace. It also exposes a standard ISDB-T DVB frontend, demux and DVR through the automatically started Rust DVB broker. See [standard TV application access](../docs/TV-COMPATIBILITY.md) for VLC and other clients. The Rust Debug Desk runs on Linux for probing, scanning, direct transport-stream recording, and saved-file playback through mpv. The GTK Live TV! window in `GUI/Linux/` uses the original Windows artwork, opens the Orbit DAC receiver panel automatically, and embeds mpv video in its viewer while Rust streams the broadcast MPEG transport stream. The same receiver path serves WSL and native distributions. The Windows DirectX/Vulkan frontend and BDA adapter remain Windows-only; the Linux renderer does not yet implement all Windows picture controls.

The driver targets x86_64 Ubuntu 22.04 through 26.04, including the interim releases. It uses standard Linux USB APIs and DKMS to rebuild for the running kernel. Compile checks passed for one published generic kernel header set from each release:

| Ubuntu | Kernel family | Header package checked |
| --- | --- | --- |
| 22.04 | 5.15 | `linux-headers-5.15.0-194-generic` |
| 22.10 | 5.19 | `linux-headers-5.19.0-21-generic` |
| 23.04 | 6.2 | `linux-headers-6.2.0-20-generic` |
| 23.10 | 6.5 | `linux-headers-6.5.0-9-generic` |
| 24.04 | 6.8 | `linux-headers-6.8.0-142-generic` |
| 24.10 | 6.11 | `linux-headers-6.11.0-8-generic` |
| 25.04 | 6.14 | `linux-headers-6.14.0-15-generic` |
| 25.10 | 6.17 | `linux-headers-6.17.0-5-generic` |
| 26.04.1 | 7.0 | `linux-headers-7.0.0-34-generic` |

A compile check is not a hardware validation or a guarantee for every subsequent kernel update. `bash linux/check_kernel_matrix.sh <Ubuntu codename> <kernel family>` repeats a published-header build check and needs `curl`, `dpkg-deb`, `make`, `gcc`, and root access.

## Build from source

On native Ubuntu, install Rust/Cargo, `build-essential`, GTK 3 development libraries, and **headers matching `uname -r`**. Then run from the workspace root:

```bash
bash build.sh
# or, to build only the userspace tools:
bash build.sh --userspace-only
bash clean_build.sh
```

`KDIR=/path/to/prepared/kernel/build bash build.sh --kernel-only` builds against another kernel. `build.sh` automatically installs and applies device permissions after a successful build, using sudo (or polkit) for that final step. For CI, cross-compilation, or artifact-only builds, add `--no-permissions`. You can repair permissions independently with `make -C linux install-permissions`. The module can be loaded with `sudo modprobe open_volar_s_usb` after DKMS installation, or load `dvb_core` first and then run `sudo insmod linux/open_volar_s_usb.ko` for a local test. Only one process can open `/dev/open-volar-sN` at a time. The `70-open-volar-s.rules` rule grants the `video` group and active local desktop session read/write access. It runs before systemd's seat ACL rule. Package installation reloads the rule before loading the module and reapplies it to existing devices, so no unplug/replug is required. Reboots and reconnects use the same rule. Headless/SSH users need membership in the `video` group (log in again after changing group membership). Direct Cargo/Make compilation does not modify system permissions; the top-level build helper applies them automatically.

On native Ubuntu, activating either the viewer or DAC raises both visible windows while keyboard input remains in the activated window. Hidden or minimized windows remain hidden/minimized. The control captions are slightly larger, with translated button widths measured inside the existing panel size. Greek captions use the desktop Sans font for glyph coverage. Saved channels and scan results sort by broadcast channel number, then frequency and program ID, retaining the selected station; services without broadcast numbers follow numbered services.

## Graphical installer and packages

Install Cargo, GTK 3 development libraries, and the package build tools (`dpkg-deb` for DEB and `rpmbuild` for RPM). Launch the Rust GUI with the helper script, which builds it on first use. The GUI can build either format or both, and **Build and install here** installs the native format with administrator privileges:

```bash
bash linux/installer.sh
```

For a terminal workflow, run `bash linux/package.sh --format all --install auto`. DEB and RPM files are written to `dist/`. They contain the Rust receiver tools, Debug Desk, Open Volar S Live TV!, desktop launchers, the DKMS driver source, the udev rule, and Microsoft's SIL OFL-licensed Selawik fonts and license. Live viewing requires the packaged mpv runtime. Recording captures only the selected service in Rust, then uses the shared GPU export to apply the output resolution and picture settings selected at recording start. Export requires FFmpeg with Vulkan/libplacebo and a hardware encoder; WSL uses the Windows FFmpeg executable. See `../RECORDING.md` for dependencies, fallback diagnostics, validation and recovery. The bundled gpu-video Vulkan encoder is not used for this export. On native Linux, the package's install hook builds and installs the module when headers for the running kernel exist. The installer reports missing headers instead of installing a module built for a different kernel.

Linux playback requests Vulkan presentation first and falls back to OpenGL if no hardware Vulkan device is available. On WSLg, Mesa can use the Windows GPU for OpenGL presentation through D3D12. WSL hardware video decoding now runs in a Windows helper: the project's Vulkan decoder is preferred, followed by D3D11VA and DXVA2, with a console message for each fallback. There is no CPU video fallback in this WSL path. The helper transports uncompressed NV12 and original audio packets to Linux; mpv performs presentation/audio. The adapter keeps compressed TS on disk for rewind and feeds a bounded raw-frame queue after each seek. See [the WSL helper instructions](../windows/wsl-video-host/README.md) for building and installing its separate Windows runtime dependencies and current limitations. Native Linux retains its existing mpv decoder selection. The Live TV! window supports double-rate, single-rate, and off deinterlacing; this is not yet the custom adaptive Windows Vulkan shader.

The Linux installer and package scripts do not require Python. On Ubuntu, the RPM builder itself is in the `rpm` package. A native RPM distribution also needs `dkms` and matching kernel development headers for module installation.

## WSL 2

The viewer and DAC start as a centered, overlapping pair, at 90% of the previous
startup size. The DAC is initially in front; both remain independent top-level
windows, so clicking the viewer keeps focus there. There is no focus-triggered
DAC raising or persistent always-on-top flag. Placement is applied after mapping
to accommodate WSLg's initial positioning and invisible frame extents.

F11 uses the full viewer area for video, hides the DAC, and activates the viewer
for keyboard input. Toggling back restores the previous window sizes, positions,
and DAC visibility. The video overlay no longer promotes a fullscreen allocation
into the window's minimum size, so normal resizing and dragging remain available.

The WSL playback adapter sends decoded audio directly to Windows WASAPI by
default, bypassing WSLg's PulseAudio/RDP audio buffering. A private SDL callback
adapter follows the Windows endpoint clock and preserves volume, mute, pause,
channel mixing, and seeking. `OVS_WSL_AUDIO=sdl` restores the previous WSLg route;
startup failures log a fallback. This is PCM output, not compressed HDMI audio
bitstream passthrough. It requires the updated Windows helper bundle.

A private monotonic-clock compatibility library also fixes multi-second
realtime timer stalls observed during 1080i/60 playback. Both libraries are
built with the adapter and embedded in the executable; neither replaces system
libraries or alters the system clock. Native Linux playback is unchanged.
See the [WSL helper notes](../windows/wsl-video-host/README.md#wsl-playback-timing).

WSL distributions share Microsoft's kernel even when their Ubuntu userlands differ. Ubuntu generic headers do not match a Microsoft WSL2 kernel. Install usbipd-win on Windows and build the Linux USB module as described below. When the receiver is missing from `/dev`, the Linux application now asks Windows usbipd to force-share the A865R (with a Windows administrator prompt when necessary), attaches it to WSL, loads `open_volar_s_usb` if needed, and retries the device node. Windows cannot use the receiver while it is attached to WSL. From the extracted source tree in Ubuntu, run:

    sudo bash linux/wsl_prepare_kernel.sh
    a865rctl probe
    open-volar-s-live-tv

The helper fetches the exact Microsoft kernel source tag for the running kernel, prepares its config and Module.symvers, builds the driver, and loads it. It caches source under /var/lib/open-volar-s/wsl-kernel/ because the first build can take several gigabytes and a long time. The packaged helper is also at /usr/src/open-volar-s-0.9.5/wsl_prepare_kernel.sh; run it with sudo bash after package installation. The device appears as /dev/open-volar-s0 once USB passthrough and the module are active. Manual usbipd commands remain useful for diagnosis if automatic attachment fails.

On a native Linux distribution, install matching distribution headers and the DEB or RPM. DKMS builds the same module; no WSL kernel source or usbipd is needed. Launch open-volar-s-live-tv from the desktop menu or terminal. Reception requires a connected antenna and an ISDB-T signal.

`a865rctl cycle-usb` remains unavailable on Linux: USB core reset is not a physical hub-port power cycle. Unplug and reattach the receiver, or detach and attach it through usbipd on WSL.

The GUI uses Selawik, Microsoft's freely redistributable Segoe-like font. The font files and their complete SIL Open Font License are in `GUI/Linux/fonts/` and included in DEB/RPM packages.

### Recording timeline and picture controls

An active recording follows the live edge visually until the viewer pauses or
seeks backward. The thumb stays at the end despite normal decoder buffering;
actual playback timestamps remain available for relative seeking. Clicking Live
or seeking to the end while playing restores live following without repeatedly
restarting the decoder as capture grows.

Picture controls use the same SDR math as Windows, applied in a GPU shader on
either Vulkan or OpenGL. Output resizing also runs in that shader, and monitor
or custom ICC profiles use the renderer's color management. Slider movements
are coalesced for 150 ms before applying and saving the last value. Picture-only
changes leave the decoder and deinterlacing filter intact. The existing bwdif
deinterlacer remains a separate CPU filter; this change removes CPU color-LUT
processing and scaling, not every CPU operation in the playback pipeline.

Settings opens centered over the streaming window, inside the monitor work
area. Initial placement stops once settled and is cancelled as soon as the user
starts dragging, so the window remains freely movable afterward. The optional GUI
login-startup control has been removed. Driver loading is independent: native
Linux uses the module's USB device alias, while the WSL setup enables its cached
module loader service. Installation and USB attachment may require elevation;
normal playback uses the device's video-group/uaccess permissions.

### Native Linux captions and programme guide (0.9.5)

CC renders ISDB caption packets with the same bundled libaribcaption decoder and
layout as Windows, including Portuguese/Spanish Latin characters, DRCS and bitmap
captions. It uses the displayed mpv timestamp, preserves captions while paused,
and removes/restores the overlay when CC is toggled. For a multi-service TS file,
captions follow mpv's selected video service. Grossly unrelated caption encoder
clocks are anchored to the video; valid caption timestamps are preserved.
Live reception and original
188-byte transport-stream recordings are supported. Caption history is bounded;
backward seeks in files reparse in the background and can take longer in large
recordings. Converted recordings need an embedded subtitle track; converting the
video does not automatically preserve the original ISDB caption packets.

The guide retains up to 8192 events in `live-tv.epg.json` beside the configuration.
It refreshes while receiving, filters by channel, shows start/end times, current
and next events, age ratings and full descriptions. Double-click an event to tune
its saved channel. Only information actually broadcast and received is available;
the application cannot retrieve schedules from multiplexes it has not visited.

Native playback uses mpv gpu-next with Vulkan or OpenGL and a separate bwdif
filter: Double rate outputs both fields; Single rate outputs one frame; Off
preserves the source. Only frames marked interlaced are deinterlaced. The existing
native Double rate default and Windows font sizes are unchanged.

Building the caption renderer requires a C++17 compiler, pkg-config, fontconfig
and FreeType development headers (`libfontconfig1-dev libfreetype-dev` on Ubuntu).
The Linux packages install the runtime libraries and DejaVu caption fonts.

### VLC on native Linux

The VLC 3 capture dialog does not offer an ISDB-T radio button. Use **Media → Open
Network Stream** (Portuguese: **Mídia/Arquivo → Abrir fluxo de rede**), then paste
`isdb-t://frequency=641143000:bandwidth=6` and play. This example is the locally
tested multiplex, not a universal channel frequency. Adapter 0 is the default.
Choose **Playback → Program → 17056** for the tested HD service. Close Live TV!
before using VLC because both applications need exclusive tuner access.

If programme titles look Japanese, enable **Show more options** and append
` :ts-standard=dvb` in **Edit options**. Alternatively run:

```
vlc --ts-standard=dvb 'isdb-t://frequency=641143000:bandwidth=6' --dvb-adapter=0 --program=17056
```

Keep `isdb-t://`: `ts-standard=dvb` changes transport-stream interpretation, not
the RF delivery standard. This is a metadata workaround and can affect ISDB
subtitle/EPG interpretation; it is not a complete Brazilian-ISDB patch for VLC.
On the saved test multiplex it changed Japanese-looking programme names to
readable Portuguese, but the service name became the generic programme number.

The same recording produced eight libdvbpsi `PSI section too long` warnings.
A diagnostic copy excluding nine empty EIT stuffing packets produced none.
This isolates that sample's warnings to VLC's stuffing handling, not to decoded
video. The driver continues to pass the broadcast unchanged. Open Volar S's EPG
parser already ignores section stuffing. Other transport errors still need their
own diagnosis; this test does not prove every future PSI warning harmless.
