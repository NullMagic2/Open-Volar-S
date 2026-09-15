# Development history

Archived from the README shipped with **0.8.0-alpha.43**. The material below
records earlier development, validation, and setup notes. Some entries describe
superseded behavior; they are not the current installation instructions.

For the project introduction, see [README.md](README.md). For current build and
packaging instructions, see [BUILD-SOURCE.md](BUILD-SOURCE.md).

---

## Alpha.43 — receiver title color

The receiver panel title uses the stream panel cream text color with plain lettering. This is a one-line drawing change; title size and position are retained.

## Alpha.42 — window resizing and additional aspect ratios

Settings > Video > Aspect ratio now includes 16:10 and 5:4 alongside Auto, 4:3 and 16:9. Auto follows broadcast display-aspect metadata, including 16:10 and 5:4, and falls back to frame dimensions when metadata is missing. EVR now refreshes its layout when that metadata changes during playback. Both manual ratios save by their ratio name and use the existing shared geometry in Vulkan and EVR presentation. The selector retains its Orbit styling and generates its choices from the same list used for selection and persistence.

The stream window's title-bar Maximize button uses Windows' native maximize/restore command and shows the restore symbol when maximized. Stream resizing updates only the stream layout, commits its child positions together, and skips minimized layouts. The existing chassis artwork is converted once and drawn in three bands by GDI instead of CPU-resampling a full-window bitmap on each resize. Video-covered pixels are excluded from chassis painting.

## Alpha.41 — playback backend switching and DirectX presentation

Changing decoder or shader selection creates a fresh video child window after the previous playback worker stops. EVR now receives repaint requests instead of the owner-drawn surface painting black over video.

Microsoft decoding with DirectX 12 or DirectX 11 effects uses the existing paced Vulkan presenter when Vulkan graphics is available. The selected DirectX API applies picture effects and ICC once per source frame, with a three-frame cache for repeated refreshes and interlaced fields. Without Vulkan graphics, Microsoft decoding retains the EVR path. Caption timing follows the actual presentation route.

Vulkan decoding retains its shared Vulkan frame resources and Vulkan shaders. A Vulkan-to-DirectX transfer experiment was rejected after measuring approximately 5–6 ms per source frame; it is not included. No decoder fallback or recovery-policy change was added.

The installer and source are versioned together as 0.8.0-alpha.41. See [alpha.41 validation](docs/ALPHA41-VALIDATION.md) for the test scope and limitations.

## Alpha.40 — decoder and shader selection, synchronized captions

Settings > Video separates Decoder from Shader acceleration. Microsoft decoding can use Vulkan graphics, DirectX 12 or DirectX 11 picture effects. Automatic prefers them in that order; Off (CPU) disables custom picture effects. Vulkan decoding remains the default and uses its shared Vulkan renderer. Explicit graphics-device loss permits five fresh attempts with the same decoder, then an error, without automatic Microsoft fallback.

The Video page puts Color profile and Choose ICC on one top row, with a gap before the remaining settings. AverTV signal is on Video, its confirmation clears after five seconds, and Themes replaces Appearance. The shader selector uses the existing Orbit combobox styling and four-language catalog.

Captions follow presented Vulkan frame timestamps, or the EVR shared clock bounded by delivered video, including DirectX and CPU-only playback. Future captions wait for their timestamps; Latin caption text is approximately 10% larger with safe-area wrapping retained. See [alpha.40 validation](docs/ALPHA40-VALIDATION.md).

The per-user installer runs without elevation. Only a necessary protected adapter update requests elevation; installing or running Live TV does not. An existing machine-wide installation can remain alongside the new per-user copy.

## Alpha.39 — simplified Vulkan recovery

Vulkan remains the default. A reported device loss releases the old playback attempt and recreates Vulkan once; failure of that retry stops playback with an error. There is no automatic Microsoft-codec fallback and no process-wide Vulkan lockout. Submission tracking now uses a bounded count and the graphics API completion wait instead of per-frame stopwatch deadlines. Independent intra-picture startup is restored. See [alpha.39 validation](docs/ALPHA39-VALIDATION.md).

## Historical alpha.38 interim Vulkan build

Vulkan remains the default decoder, as requested. This interim package contains decoder metadata/padding corrections and exhaustive caption case regression coverage. The GPU device-loss issue is still unresolved and requires the old/new comparison after a Windows restart. The two-minute Microsoft-decoder comparison does not validate Vulkan playback. See [investigation and limits](docs/ALPHA38-INVESTIGATION.md) and [validation](docs/ALPHA38-VALIDATION.md).

## Unified release 0.8.0-alpha.37

Combines the complete alpha.36 source from the second working window with the caption, recording-recovery, native-diagnostics and AverTV signal-processing changes. See [alpha.37 validation](docs/ALPHA37-VALIDATION.md) and the installer notes for dependencies and validation limits.

Current update: [alpha.32 background receiver startup](docs/ALPHA32-VALIDATION.md).

Current update: [alpha.31 settings corrections and audio validation](docs/ALPHA31-VALIDATION.md).

Current update: [alpha.30 HDR effect and validation](docs/ALPHA30-VALIDATION.md).

Current update: [alpha.29 DAC row adjustment and validation](docs/ALPHA29-VALIDATION.md).

Current UI update: [alpha.28 changes and validation](docs/ALPHA28-VALIDATION.md). Four interface languages are available under Settings > General.

## Open Volar S

**Open Volar S** is the overall project: the userspace driver, firmware, diagnostic tools, and **Live TV!** television application.

Live TV! now uses the Orbit front panel and tabbed Settings. See [LIVE-TV-ORBIT.md](LIVE-TV-ORBIT.md) for build, appearance options, and validation. The original driver documentation follows.

### Release 0.8.0-alpha.32 / userspace driver 0.7.0

Alpha 27 fixes persistent gray system borders after moving or activating the viewing window. Native resize behavior is retained while the application owns its frame painting. The slider timer uses a rounded recess over the continuous panel background, including synchronous playback-time updates. Live now uses the DAC button renderer and icon directly. Settings popup text is larger and scales using the owning window's DPI. See docs/ALPHA27-VALIDATION.md.

Alpha 26 implements the physical-TV viewing console with a recessed screen frame, no decorative corner screws, and the same Metal/Glass/Plastic button rendering as the DAC panel. Playback order is back, forward, Play/Pause, Stop. Channel and Volume rockers have equal dimensions; the recording lamp is concentric glass and metal. The contextual slider counter shows live elapsed time, recording duration near Live, or the current rewound position. Includes the DAC corner correction, thicker REC/CC hover borders, green Live treatment, cream legends, and adjusted minimize position. Existing tuner and decoding architecture is retained. See docs/ALPHA26-VALIDATION.md.

Alpha 24 reduces CPU overhead by caching GPU texture views, avoiding duplicate GPU completion polling during active submission, skipping hidden fullscreen chrome painting, and using native GDI gradient fills for the interface. GPU decoding, synchronization and the alpha 23 fullscreen pacing remain in place. Bounded fullscreen checks retained about 60 presentations per second; measured CPU savings were modest. See [alpha 24 validation](docs/ALPHA24-VALIDATION.md) for evidence and limits.


Alpha 23 adds three reusable GPU picture slots and separate preparation/presentation workers. Frames stay on the shared Vulkan GPU device; only handles and timing metadata pass between workers. The bounded handoff selects a clock-appropriate picture after image acquisition. Requested snapshots use a separate bounded worker. Fullscreen now uses a one-pixel black inset with display-vblank pacing; three sustained checks stayed near 60 fps on the tested RX 7900 XTX. The pointer hides after three seconds and returns on movement. See docs/ALPHA23-VALIDATION.md for measurements and hardware limits.

TV alpha 22 adds GPU hardware-filtered upscaling, source-resolution deinterlacing, and processed-frame reuse. It retains alpha 21 surround audio, captions and recording settings. The fullscreen slowdown observed in alpha 22 was addressed locally in alpha 23; see its validation report. See [validation and measured limits](docs/ALPHA22-VALIDATION.md).

**Phase 2 player:** run `player/live-tv.exe`. The new Rust/windows-rs interface uses Vulkan Video H.264 decoding and Vulkan presentation on one GPU device, with Windows AAC audio, applies custom ICC profiles, and includes a broadcast EPG. See [player instructions and current limits](player/README.md). This executable does not use FFmpeg or mpv. The new installer includes this player alongside driver/debug tools 0.7.0.

Windows userspace driver, TV/debug GUI and experimental x86/x64 BDA adapters for the original AVerTV Volar S A865R (07CA:B865), IT9175 revision 1, tuner ID 0x70. API version 2; source-built open LINK/OFDM firmware 0.1.4.0.

This is the **0.8.0-alpha.32 Vulkan Video preview**. Alpha 20 fixes RBI audio starvation, improves GPU deinterlacing, and fixes minimized channel switching. The subsequent fullscreen correction and its measured limits are documented in alpha 23. Short live playback and pause/resume checks have been run after the user requested hardware debugging; earlier graphics-driver crashes are not established as permanently fixed. See [validation and recovery](docs/VULKAN-VIDEO-PREVIEW.md).

### Install and use

The matching installer is `Open-Volar-S-Setup-0.8.0-alpha.32-x64.exe`. Choose Just for me or All users. It creates **Start Menu > Open Volar S > Live TV!** and **A865R Debug Desk**; a desktop shortcut is optional. Setup does not launch either application. Uninstall removes owned shortcuts and preserves diagnostic exports. Portable GUI: `debug/a865r-debug.exe`.

The existing WinUSB binding is required. The GUI installer does not register or remove the separate experimental BDA adapters. See [BDA compatibility](docs/BDA_COMPATIBILITY.md) and [the Start Menu repair](docs/START_MENU_REPAIR.md). User and system adapter versions must stay aligned; the user's AVerTV Start Menu launch worked after the outdated system adapters were updated. `tools/repair-installed-bda.ps1` repairs the validated existing installation with backups and administrator rights; it is not a fresh-install registration tool.

Leave the firmware field empty for ordinary cold startup using open 0.1.4.0. The image is embedded and also supplied in `firmware/`; firmware is loaded into RAM, never EEPROM. Do not replace firmware on an already running tuner. Scan for channels, select a found frequency, and Watch or Record TV. RF22 at 521143 kHz was tested in Vila Velha, Brazil; frequencies depend on the transmitter.

AVerTV receives the original compressed stream by default. General settings can select software improvement and 2K/4K upscaling through the updated BDA adapter; this requires external FFmpeg and does not require a Live TV window. Debug Desk now reuses the native player. Only one TV/diagnostic client should own the tuner.

### API and supported hardware

The public Rust API reports driver/API versions, chipset and firmware details, capabilities, source/output dimensions, rendering/upscaling state and confirmed color profile. See [API.md](docs/API.md) and `debug/examples/player_api.rs`. The receiver supports 6 MHz ISDB-T UHF, 470000–697999 kHz, with a Brazil preset and custom scans. Other standards/bands are not implemented.

Open firmware received 19 UHF centers and recovered from short Windows S3 tests. AVerTV live picture, channel switching and measured quality were verified. Long sleep, hibernation, generic Network Provider playback, shared ownership and arbitrary IR remotes remain incomplete or untested. Read the [firmware report](docs/RF_INTEGRATION_0.1.4.md) and [AVerTV update](docs/AVERTV_UPDATE_0.7.0.md) for evidence and precise limits.

### Debugging and building

Portable exports go to `debug/exports`; installed exports go to `%LOCALAPPDATA%/A865R/Debug/exports`. The GUI exports diagnostic data and analyzes USBPcap PCAP/PCAPNG files through `tools/usb_trace.py`; Python 3.10+ is needed for these offline scripts. Source archives omit local captures and personal profiles.

```text
cargo build --workspace --release --locked
cargo build -p a865r-bda --release --locked --target i686-pc-windows-msvc
cargo test --workspace --locked
```

Use Rust/MSVC with the x86 target installed. Copy the release TV player to `player/`, debug GUI to `debug/`, CLI to `bin/`, and each BDA DLL to its matching `compatibility/x86` or `x64` folder. Compile `installer/a865r.iss` with Inno Setup (6.7.3 was used), using its default output filename. `installer/build.ps1` builds the TV player/GUI/CLI installer from a default Cargo target directory; it does not build/register the separate x86 adapter.

GPL-3.0-only; see `LICENSE`, `THIRD_PARTY.md`, and `third-party-licenses`. Firmware is reconstructed from reference behavior, not a clean-room claim. No original AVerMedia installer, kernel driver, or proprietary firmware is bundled.
