# Windows hardware decoder for WSL

WSL runs `open-volar-s-wsl-player`; that process launches this Windows helper
through WSL interop. It directly uses the project's `gpu-video::broadcast`
Vulkan H.264 decoder. If unavailable, Windows D3D11VA and then DXVA2 are tried.
No CPU video decoder is selected. Each hardware transition is logged as:

```text
[DEBUG]: Vulkan Video not available; trying D3D11VA
[DEBUG]: D3D11VA not available; trying DXVA2
```

The receiver writes original compressed TS to a private WSL cache. Seeking
restarts hardware decode with keyframe preroll and discards pictures before the
requested timestamp. A growing-file reader waits for incoming bytes instead
of flushing incomplete PES packets at temporary EOF. Uncompressed NV12 and
original audio packets travel over a per-session authenticated TCP connection
to Linux mpv for presentation/audio. mpv does not decode H.264 in this path.
The raw-frame queue is bounded (150 MiB); the compressed session cache is
limited to 16 GiB and removed on normal exit. Reaching that limit stops playback
with an error rather than silently deleting rewind history. Source recordings
are never modified. This first bridge transports video and audio; broadcast
subtitle/data streams are not yet transported.

### WSL playback timing

The Linux adapter defaults to **direct Windows WASAPI audio output** on WSL.
mpv decodes the selected audio track and applies its volume, mute, channel mix,
and filters. A private SDL2 callback adapter sends float PCM over a separate,
authenticated TCP connection to a Windows helper audio session. WASAPI pulls
one endpoint period at a time and maintains a two-period target queue. Windows
therefore paces the samples, without WSLg's PulseAudio/RDP audio queue. The tested
endpoint uses 480-frame periods at 48 kHz (10 ms). Mono, stereo, 5.1, and 7.1 use
the standard Windows channel order; the shared Windows mixer converts to the
default output's supported format. This is PCM output, not Dolby/DTS bitstream
passthrough. The Windows application and native Linux audio paths are unchanged.

Pause feeds silence without advancing the media clock. Seek/channel restarts
close the old endpoint and restore player controls; a track/layout change can
reopen the same authenticated connection. Closing the player also closes the
helper through its liveness pipe. A runtime endpoint/connection failure stops
playback with a diagnostic; restart playback after changing/removing a device.
No system audio device or global audio setting is changed.

If the helper or WASAPI cannot initialize, the console reports the fallback to
WSLg audio. `OVS_WSL_AUDIO=sdl` uses the previous SDL/Pulse path for diagnosis.
Explicit `--ao` arguments also bypass native audio. mpv builds without SDL fall
back to PulseAudio. The private callback adapter is tested with Ubuntu 22.04's
mpv 0.34.1 and SDL2.0.20; it is loaded only into this player's child process.

The flash/click calibration previously measured roughly 130–170 ms of sound
lag through WSLg. The initial WASAPI run measured a median around 36 ms (valid
pairs 19–68 ms), with zero video frame drops. These are screen-readback versus
Windows-loopback measurements, not physical display/speaker measurements; they
do not establish a universal correction for every output. No fixed sync offset
is applied. Rewind, return-to-live, pause, mute/volume persistence, 5.1 reinit,
and finite playback shutdown were exercised through the bridge.

The adapter also embeds `linux/debug/wsl_clock.c` as a small private compatibility
library and loads it only into its mpv child. Realtime condition waits of 15–47 ms
were observed taking about 4.76 seconds in WSL. The library converts those
deadlines to monotonic waits, preserving conditions already using a monotonic
clock. It does not change the system clock or replace system libraries. A C
compiler and glibc are required to build this compatibility library; GNU/Linux
builds embed it in the adapter, so users do not compile it at launch.

This fixes the measured playback timing failures while retaining double-rate
deinterlacing. It does not make the bridge GPU-only: NV12 still crosses to Linux,
and mpv still performs presentation, audio decoding and picture filtering.
Windows WASAPI performs the final WSL audio output.

## Build and install

Use an x64 Windows Rust/MSVC toolchain and a shared FFmpeg 9 SDK containing
`include/`, `lib/`, and `bin/`. Tested with Gyan's FFmpeg 9.0.2 full shared build:
<https://www.gyan.dev/ffmpeg/builds/>. The downloaded archive SHA-256 was
`4d2060a8b34a940aa47d785142055bb92a63053781e55f2ace4546edd519a8f5`.

```powershell
.\windows\wsl-video-host\build.ps1 -FfmpegSdk C:\path\to\ffmpeg-shared-sdk
```

In WSL, build the Linux package (`bash linux/package.sh --format deb`). It
includes the Linux adapter. Install the locally built Windows bundle alongside
it:

```bash
sudo install -d /usr/lib/open-volar-s/wsl-video-host
sudo cp -a dist/wsl-video-host/. /usr/lib/open-volar-s/wsl-video-host/
```

The PE helper must be executable. `OVS_WSL_VIDEO_HOST` can point to an alternate
helper executable; its dependent DLLs must be adjacent. WSL interop must be
enabled and host-to-WSL TCP connectivity allowed. Native Linux uses its existing
player path and never launches the Windows helper. Hardware compatibility
depends on the Windows adapter/driver's codec/profile capabilities, not just
the AMD brand. Validated on RX 7900 XTX with Vulkan and D3D11VA; other AMD cards
have not been validated.

FFmpeg libraries are runtime dependencies, licensed separately under GPLv3 in
the tested build. Keep their license and source/build information with any
redistribution; this source archive does not bundle FFmpeg DLLs or their SDK.
Ordinary Windows Live TV builds exclude this optional helper and need no SDK.
