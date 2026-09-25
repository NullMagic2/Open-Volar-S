# Open Volar S 0.9.5 — native Linux playback update

The GTK Live TV! application and native Debug Desk now launch the included
`open-volar-s-player` helper for native Linux playback. This playback path does
not invoke mpv, FFmpeg or GStreamer. The DEB/RPM includes the helper and declares
its system-library dependencies. Earlier 0.9.3 packages do not contain this port;
the refreshed deliverables are in `dist/release-0.9.5`.

## Picture and sound

- Vulkan Video decodes broadcast H.264 directly to GPU textures.
- Linux and Windows use `GUI/shared/video.wgsl`: motion-adaptive deinterlacing
  with previous/current/next frames, diagonal reconstruction and field-aware
  chroma. Double-rate mode displays both broadcast fields.
- Shared aspect-ratio and viewport rules preserve broadcast display proportions
  independently of the window size and Native/QHD/UHD processing limits. Auto
  respects non-square pixels and Windows' standard SD active-aperture rule;
  other formats are not cropped. Explicit aspect overrides remain available.
- Picture controls, monitor ICC conversion, caption/OSD bitmaps and PNG snapshots
  are composed by the native renderer. Resizing also redraws paused pictures.
- FAAD2 decodes AAC (ADTS and LOAS/LATM). Linux and Windows share PCM sound modes;
  Linux sends audio through PulseAudio/PipeWire and uses measured output timing
  to synchronize video. Pause corks the audio stream instead of discarding PCM.
- TS-file seeking and a bounded 2 GiB live cache support playback controls.

## Requirements and limits

These Linux binaries were built on Ubuntu with **glibc 2.43**. Rebuild the source
on older distributions. Runtime libraries include GTK3, FAAD2 (`libfaad.so.2`),
PulseAudio (`libpulse.so.0`), Vulkan, X11 and Little CMS. Wayland desktops use
XWayland for the GTK video embedding.

This backend requires Vulkan Video H.264 decoding with the queue capabilities
used by the project's broadcast decoder. It was tested on an AMD Radeon RX 7900
XTX with Mesa 26.0.8; other GPU/driver combinations are not validated. There is no
silent software-codec fallback. Saved-file input currently supports MPEG-TS,
not arbitrary MP4/MKV/media files. The live cache can expire old positions after
long pauses, and seeking relies on broadcast reference frames becoming available.
Multi-monitor ICC profile changes are not tracked automatically.

**Scope:** this update replaces native Linux playback. Processed recording export
and the existing Wine/WSL playback bridges still use their previous backends.
They have not been ported to the new engine. In particular, FFmpeg remains in
recording export (including the existing Windows export helper); mpv remains a
package dependency for the legacy bridges. Do not interpret this release as a
complete removal of FFmpeg from every project function.

## Verification

All 32 native unit tests passed, including recovery of the control socket after
forced channel-change shutdowns. A captured broadcast passed three consecutive
forced playback restarts on the same socket, automatically presenting video on
each start. GTK's regression also verifies that channel selection queues playback
and that old service updates cannot overwrite the newly selected channel.

A fresh TV Gazeta HD tuner capture from the original native-player validation passed
native LOAS/LATM AAC playback and control/EOF checks (570 presented fields, no
dropped fields). Actual 5.1 speaker output remains unverified.

The user confirmed audible playback and lip-sync of the captured stereo sample.
The shared shader rendered 1080i video at QHD, with both fields presented. Native
control tests cover pause/resume, sound modes, deinterlacing, seek while paused,
audio-track switching and end of file. The GTK integration test uses its actual
embedded video window, checks displayed-frame seeking, settings persistence,
snapshots, explicit 4:3 and automatic 16:9 proportions, UHD processing, and
resizing a paused window. Shared unit tests additionally cover 4:3, 16:9, 16:10,
5:4, portrait windows, SD pixel aspect metadata, transport resynchronization,
AAC framing, PCM mixing, cache wrap/cancellation and malformed ICC profiles.

Windows applications and BDA adapters are cross-built on Linux; native Windows
playback is not re-tested here. See BUILD-AND-TEST-REPORT.md for the final package
validation and the distinction between current and historical checks.

## Installation

From the folder containing the new DEB:

```sh
sudo apt install --reinstall ./open-volar-s_0.9.5_amd64.deb
```

APT resolves the added library dependencies. Close the old Live TV! instance
before launching the newly installed application. In video settings choose
**Auto** aspect ratio to preserve the broadcast's proportions. QHD/UHD controls
processing resolution, not the aspect ratio or application window size.

## Development

```sh
cargo build --release --locked -p open-volar-s-player -p open-volar-s-live-tv
cargo test -p open-volar-s-player
python3 linux/player/tests/control_smoke.py target/release/open-volar-s-player capture.ts
python3 linux/player/tests/seek_smoke.py target/release/open-volar-s-player capture.ts
A865R_NATIVE_PLAYER="$PWD/target/release/open-volar-s-player" \
  A865R_TEST_TS=/absolute/path/to/capture.ts \
  cargo test -p open-volar-s-live-tv native_embedded_playback_controls -- --ignored --nocapture --test-threads=1
```

Hardware integration tests need a desktop/audio session and supported Vulkan
Video GPU; they use a saved capture and do not open the tuner. Their audio is
muted. Set `A865R_TEST_PROGRAM` for a particular service in a multiplex when
running `control_smoke.py`. `CARGO_TARGET_DIR` is supported by the Linux package
builder; `OPEN_VOLAR_S_DIST` selects an alternate artifact directory.
