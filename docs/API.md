> Alpha.37 update: external BDA clients can now use persistent signal-processing presets. Original remains the default. See ALPHA37-VALIDATION.md; historical pass-through and installer limitations below describe earlier releases.

# Rust application API, version 2

Playback processing settings below apply to our standalone player. The AVerTV
BDA adapter delivers the original compressed broadcast stream and does not
apply these scaling, deinterlacing, or color-profile settings. AVerTV renders
at its normal resolution under its own control.

`liba865r` exposes the crate `a865r`; the `a865r-debug` package also exposes the playback library `a865r_media`. Both are built from this workspace. The native GUI uses the same driver, settings types and playback backend.

`Receiver::signal_status()` returns `SignalReport { mpeg_locked,
quality_percent }`; the percentage is optional when no valid measurement exists.
`stream_chunks_monitored` supplies periodic reports on the same receiver thread
during capture. BDA consumers can query the cached measured quality through
`IBDA_SignalStatistics` or its KS property without opening another USB client.

```rust
use a865r::api::{capabilities, PlaybackSettings, RenderBackend, Resolution, DRIVER_VERSION};

let supported = capabilities(None); // Static profile, no USB access.
println!("Driver {DRIVER_VERSION}: {supported:?}");
let mut settings = PlaybackSettings::default();
settings.resolution = Resolution::Uhd; // 3840 × 2160.
settings.upscaling_enabled = true;
settings.backend = RenderBackend::Gpu; // Prefer GPU, with CPU fallback.
settings.cpu_threads = 8;
settings.validate()?;
```

- `Device::capabilities()` also reports whether a probed board matches the supported IT9175/single tuner profile. `None` means no device was probed, not an incompatible device.
- `DRIVER_VERSION` and `API_VERSION` identify the build and API contract separately.
- `Resolution::{Native,Hd,Qhd,Uhd}` describes broadcast dimensions or the 1080p, 1440p and 4K processing canvas. Aspect ratio is preserved with padding.
- `upscaling_enabled = false` overrides the stored preset and preserves broadcast dimensions. Display-window resizing is separate from video processing.
- `RenderBackend::{Auto,Cpu,Gpu}` chooses automatic selection, CPU processing, or GPU preference. GPU requests fall back when device/filter execution fails. They are preferences, not evidence that a GPU is available.
- `cpu_threads` accepts 1–64 workers. GPU shaders and queues use the Vulkan driver's parallel execution model. CPU worker count does not pretend to configure shader hardware threads.
- `PlaybackStatus` holds requested settings, the active backend, measured source/output dimensions and fallback information for applications. Unmeasured fields start as `None`. This is an application state model; it is not automatically populated by a USB device query.

To execute those settings, pass `Options::from_settings(ffmpeg_path, &settings)` to `a865r_media::playback::play_file`, or use `LiveSink` with `Receiver::stream_chunks`. Clone a `Control` into your UI and call `stop()` to cancel and terminate only the owned media processes. `Control::message` contains live human-readable status. `play_file` returns structured JSON with the selected scaling path and session outcome. Decoder and GPU-probe logs distinguish actual accelerator selection from the requested mode. Run blocking playback/receiver operations on a worker thread, as the GUI does. Apply new settings on the next session; stop/restart to change a running pipeline.

The complete executable example is `debug/examples/player_api.rs`:

```text
cargo run -p a865r-debug --example player_api -- recording.ts gpu 4k
cargo run -p a865r-debug --example player_api -- recording.ts cpu native
```

`a865rctl capabilities` prints the hardware profile. The GUI's “Export capabilities & playback settings” action exports machine-readable JSON including the upscaling switch. Source resolution depends on the selected broadcast service, not the tuner model; an unmeasured source resolution remains null. A 4K processing capability does not mean the hardware receives a native 4K broadcast.

For tuning, use `channel_plan::custom_scan(first_khz,last_khz,step_khz)` or `brazil_uhf()`, then `Device::receiver()`, `initialize()`, `tune()`, and `record()`/`stream_chunks()`. Always call `stop()` after a tuning/capture attempt. The current board profile is 6 MHz ISDB-T UHF, 470000–697999 kHz. It is independent of city but is not a DVB/ATSC/worldwide standards implementation.

Open firmware 0.1.4.0 is accepted for normal reception after hardware validation. Cold startup uses the source-built image by default. `check_open_firmware_health()` is read-only; firmware upload still requires a cold tuner. Later unvalidated firmware versions remain restricted to the experimental path.

`a865r_media::playback::media_properties(export_folder)` returns measured source width, height, frame-rate rational, field order and selected program ID after a probe/playback session. Live startup samples two seconds of the multiplex, identifies the highest-resolution service and maps its audio/video by program ID, so stream arrival order does not accidentally choose the one-seg service. `Options::program_id` also permits an explicit service override for application callers.

Progressive source frame rates, including 60 fps, are preserved; there is no fixed 30 fps cap. `PlaybackSettings::deinterlacing` selects `DeinterlaceMode::{DoubleRate,SingleRate,Off}`. DoubleRate is the default and produces one progressive frame per interlaced field: 59.94 fields/s becomes 59.94 progressive frames/s. Progressive inputs bypass deinterlacing and retain their timing. Motion interpolation is not enabled. USB capability alone cannot guarantee a host's real-time 4K60 rendering performance.

Double-rate deinterlacing uses `bwdif_vulkan` on Vulkan frames, followed by libplacebo scaling without a host-memory round trip between decoding/deinterlacing/scaling. CPU fallback uses multithreaded `bwdif`. The GUI exposes Smooth motion / Standard / Off under playback settings. FFmpeg uses passthrough frame timing; it does not synthesize 60 frames from a progressive 30 fps source.

## Version 2 queries and color profiles

Driver version is `0.7.0`, reported independently from API version `2` and the LINK/OFDM firmware versions. `a865r::api::capabilities(Some(&device_info))` includes chipset ID, revision, prechip revision, tuner ID, firmware state, processing limits and supported color-profile modes. Without a probe, measured fields remain absent. Native broadcast dimensions are measured during playback, never guessed from the chipset.

```rust
use a865r::api::{ColorProfile, PlaybackSettings};
let mut settings = PlaybackSettings::default();
settings.color_profile = ColorProfile::Monitor; // Follow Windows ICC assignment.
// settings.color_profile = ColorProfile::File("display.icm".into());
// settings.color_profile = ColorProfile::Disabled;
settings.validate()?;
let options = a865r_media::playback::Options::from_settings(ffmpeg_path, &settings)?;
let control = a865r_media::playback::Control::default();
// Give a clone to the worker running play_file / LiveSink.
let current = control.snapshot(); // Cached JSON; no USB access.
```

Command-line queries (GUI executable: redirect standard output when needed):

```text
debug/a865r-debug.exe --capabilities
debug/a865r-debug.exe --query-device device.json
debug/a865r-debug.exe --query-status path/to/playback-session
debug/a865r-debug.exe --play recording.ts --color-profile monitor
debug/a865r-debug.exe --play recording.ts --cpu --color-profile "C:/profiles/display.icm"
debug/a865r-debug.exe --play recording.ts --color-profile off
```

`--query-device` reads the idle device and writes JSON to the supplied file. During playback use `Control::snapshot()`, the GUI status export or the session's `status.json`; these do not open a second USB client. Status updates about every 500 ms. It includes `driver_version`, `api_version`, cached `device` capabilities, `source`, requested and measured `output_resolution`, `upscaling_enabled`, actual `processing_backend`, CPU worker count, presenter hardware decoder, display, frame rate, window dimensions, and `running`. Values not yet measured are null/absent. The source's 29.97 interlaced frame rate and the presenter's 59.94 progressive rate describe different stages.

`color_profile` includes requested mode/path, state, applied flag, active path, filename and SHA-256. Applied requires the mpv/libplacebo ICC-open confirmation; a detected Windows profile path alone leaves it pending. Disabled is explicit. A renderer ICC error reports failed. Pending can also mean no usable monitor profile has been confirmed; do not treat it as success. File profiles are validated before spawning media processes. Monitor changes invalidate the old confirmation until the renderer opens the new profile. WCS and non-RGB profiles are unsupported. Changes to settings apply to the next playback session.

The renderer uses mpv gpu-next with relative colorimetric intent. CPU mode means CPU decoding/deinterlacing/scaling; mpv still uses a graphics API to present the window and apply the profile. TS recordings preserve broadcast data. See the runtime provenance in `debug/runtime/PROVENANCE.md` and the [mpv ICC options](https://mpv.io/manual/master/#options-icc-profile-auto).

Verified locally: live RF22 with Vulkan processing, 3840x2160 at 59.94 fps and the assigned ASUS ICC profile; file playback with CPU processing and an explicit sRGB ICC profile. Actual monitor switching and arbitrary profiles have not been hardware-tested.
