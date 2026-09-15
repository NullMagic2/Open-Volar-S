# Alpha.40 update

Video settings now separate Decoder and Shader acceleration. The automatic graphics order is Vulkan, DirectX 12, DirectX 11; Off (CPU) disables custom picture effects. Microsoft decoding can use Vulkan graphics without Vulkan Video support. The Vulkan decoder retains its shared Vulkan renderer. See ../docs/ALPHA40-VALIDATION.md for recovery, caption synchronization, installer behavior and measured shader costs.

Current UI update: [alpha.28 changes and validation](../docs/ALPHA28-VALIDATION.md). Four interface languages are available under Settings > General.

# Live TV!

The television presentation application of **Open Volar S**. The current interface uses Orbit; see [Orbit implementation notes](../LIVE-TV-ORBIT.md).

# Live TV! 0.8.0-alpha.31 — Vulkan Video preview

Alpha 27 fixes persistent gray system borders after moving or activating the viewing window. Native resize behavior is retained while the application owns its frame painting. The slider timer uses a rounded recess over the continuous panel background, including synchronous playback-time updates. Live now uses the DAC button renderer and icon directly. Settings popup text is larger and scales using the owning window's DPI. See docs/ALPHA27-VALIDATION.md.

Alpha 26 implements the physical-TV viewing console with a recessed screen frame, no decorative corner screws, and the same Metal/Glass/Plastic button rendering as the DAC panel. Playback order is back, forward, Play/Pause, Stop. Channel and Volume rockers have equal dimensions; the recording lamp is concentric glass and metal. The contextual slider counter shows live elapsed time, recording duration near Live, or the current rewound position. Includes the DAC corner correction, thicker REC/CC hover borders, green Live treatment, cream legends, and adjusted minimize position. Existing tuner and decoding architecture is retained. See docs/ALPHA26-VALIDATION.md.

Alpha 24 reduces CPU overhead by caching GPU texture views, avoiding duplicate GPU completion polling during active submission, skipping hidden fullscreen chrome painting, and using native GDI gradient fills for the interface. GPU decoding, synchronization and the alpha 23 fullscreen pacing remain in place. Bounded fullscreen checks retained about 60 presentations per second; measured CPU savings were modest. See [alpha 24 validation](../docs/ALPHA24-VALIDATION.md) for evidence and limits.


Alpha 23 adds three reusable GPU picture slots and separate preparation/presentation workers. Frames stay on the shared Vulkan GPU device; only handles and timing metadata pass between workers. The bounded handoff selects a clock-appropriate picture after image acquisition. Requested snapshots use a separate bounded worker. Fullscreen now uses a one-pixel black inset with display-vblank pacing; three sustained checks stayed near 60 fps on the tested RX 7900 XTX. The pointer hides after three seconds and returns on movement. See ../docs/ALPHA23-VALIDATION.md for measurements and hardware limits.

- **Audio → 5.1 surround** preserves all six channels of a 5.1 AAC broadcast. The Audio menu shows the broadcast channel count and available audio tracks. Configure your playback device as 5.1 in Windows for six-speaker output; stereo broadcasts remain stereo. Stereo, mono, left, and right output modes remain available and the selected mode is saved.
- **CC On / CC Off** controls broadcast closed captions, enabled by default. Captions appear only when the selected service transmits supported ISDB caption data, including Brazilian Portuguese. Their timing follows playback, including recorded playback. Broadcast formatting is preserved in a transparent overlay.
- **Settings → Recording folder → Choose folder…** selects where new recordings are stored. The default uses the current user’s Windows Documents folder plus `A865R\recordings`, including redirected Documents locations. This selection is saved in `settings.json`. Each recording gets a dated channel filename. Library opens this folder. Changing it during recording applies to the next recording.


## Alpha 20: audio modes and recording controls

The Audio button opens Stereo, Mono, Left channel and Right channel output modes, followed by the selected station's broadcast audio tracks. Output mode changes apply immediately and are saved. AAC ADTS and AAC LATM tracks can be selected; language names are shown when the broadcaster supplies language descriptors, otherwise tracks are numbered. Mono mixes left/right evenly; Left and Right send the selected channel to both speakers. Mono broadcasts remain mono. Unsupported broadcast codecs are listed as unavailable.

Press REC to save the original transport stream and enable time shift. Capture continues on its own worker while playback is paused, moved with the timeline, or stepped backward/forward one decoded frame at a time. Live resumes close to the recording's growing end with a small decoding margin. REC again finishes the recording and returns to live reception; Stop ends playback and recording. Recordings continue until stopped or an I/O error and remain in the chosen recording folder. Unrecorded live TV has no rewind buffer. Changing audio tracks rebuilds playback briefly without stopping the recording writer.

## Alpha 12: supplied animated volume dial

The user-provided Dial.png is embedded unchanged in the player and included in the source archive. The gold scale remains fixed while the silver knob and red pointer rotate from the minus mark at 0% to the plus mark at 100%. The exterior white background is removed once at load time using an edge-connected mask, preserving enclosed silver highlights and the actual gold silhouette. Alpha interpolation prevents a white fringe around the rim.

Volume changes animate for 220 ms with an ease-out curve. A new change begins at the currently displayed angle. The native control repaints only its own area at approximately 60 updates per second during a transition; its animation timer stops when settled or the panel is hidden. Decoding, interpolation, and the cached bitmap live in one small UI component and do not change Vulkan video processing.

## Alpha 11: transparent green OSD

The channel and volume OSD now use transparent layered child windows instead of bronze panels, with 75% foreground opacity. Text uses the original AVerTV defaults recovered from the installed executable: RGB #00FF00 foreground, #646464 outline, Microsoft Sans Serif, weight 700, non-italic. The volume indicator uses green segments with transparent gaps; it has no opaque trough. Existing three-second expiry, fullscreen positioning and first-picture redisplay remain. The fullscreen exit button hides after two seconds without pointer movement and reappears on movement; F11 and Escape remain available.

Static evidence from AVerTV.exe (image base 0x400000): the OSD_TEXT configuration call at 0x48DC6C is preceded by `push 0x00FF00`; OSD_EDGE at 0x48DC8F uses `push 0x646464`; font weight at 0x48DD66 uses `push 700`. Font-size setting defaults to 10 in AVerTV's own sizing units; this implementation scales its OSD font with Windows DPI. The original application was not patched and no proprietary graphics are bundled.

Windows color-key composition uses WS_EX_LAYERED, LWA_COLORKEY and LWA_ALPHA. A Windows 8/10 compatibility manifest is embedded because layered child windows require that declaration. Text uses crisp GDI glyphs to prevent colored antialias fringes around transparent pixels. Only UI surfaces are composed; Vulkan decoded frames are not read back or copied.

API reference: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setlayeredwindowattributes

## Alpha 10: RBI startup, aspect ratio, and on-screen display

RBI HD transmits non-IDR intra pictures. The old startup gate waited indefinitely for an IDR even when the parser supplied an independent picture. Startup now also accepts a first intra picture with an empty reference list, excluding complementary second fields. The bitstream IDR flag, Vulkan session handling, and missing-reference checks are preserved. The CPU parser diagnostic verifies reference continuity from either independent start. Decoder reports include parsed and skipped startup pictures.

Settings now offers Auto, 4:3, and 16:9. Selection applies immediately, including a paused picture, and is saved between launches. Auto uses broadcast sample-aspect metadata when present; the selected display ratio also constrains native window edge resizing. Processing resolution remains a separate Native/2K/4K choice.

Changing channel shows its number and name over the video for three seconds; the label is shown again when the first picture arrives. Changing volume shows a percentage and horizontal bar for three seconds. These are newly drawn windows-rs child controls, so they appear while tuning, paused, and fullscreen without copying decoded GPU frames. They do not make the player globally topmost. Channel buttons can restart watching after a failed service.

The installed AVerTV executable was inspected read-only for its OSD implementation: AVEROSD/BCOSDWnd, OsdAlign, OSD_DURATION, OSD_TEXT/EDGE/BACK, OSD_ITEM_DTV, and volume OSD entry points. Its Skin/OSD directory supplies transport icons but no channel/volume layout. Alignment, gold styling and the three-second timeout in this implementation are our own choices; original program binaries and artwork are not bundled.

Alpha 5 fixes the missing Win32 instance handle that prevented Vulkan surface creation in alpha 4.

Run `live-tv.exe`. It uses the existing A865R WinUSB setup and embedded open firmware 0.1.4.0 from the 0.7.0 driver. Close other tuner applications before watching or scanning. No FFmpeg, mpv, codec pack, or BDA registration is needed by this player.

The interface uses Rust and windows-rs, native Windows window borders, and newly drawn gold controls inspired by the original AVerTV Classic Tube layout. It omits radio, the radio meter, and 3D features. Snapshot uses a camera icon. The separate control panel follows normal window ordering; the player has a fullscreen button, with F11 and Escape shortcuts.

## Missing service clock recovery in alpha 9

The playback source now recovers an absent advertised PCR clock from validated video DTS (or PTS when DTS is absent), after observing one second of service timestamps without a real clock. This lets the Windows demultiplexer release mobile-service audio/video instead of waiting forever for missing PCR packets. Compressed payloads and PES timestamps are preserved. Valid-clock streams pass through byte-for-byte; recovery stops if their real clock arrives. Raw broadcast recordings are unchanged. The helper is safe Rust and adds no GPU API calls.

Live verification on RX 7900 XTX restored TV GAZETA MOVEL video and audio-output activity, with the first decoded frame at about 3.1 seconds and clean shutdown.

A CPU-only `a865r-bda` example, `demux_probe`, reproduces the failure using a saved TS file. On the problematic sample: original mobile output was 0 samples; recovered output was 52 samples / 40,436 bytes after the one-second observation period. HD output was identical with and without recovery (159 samples / 2,169,748 bytes, identical endpoint timestamps).

## Startup and paused-state fixes in alpha 8

Pausing before the first decoded frame now retains one still preview instead of discarding every picture and leaving the video area blank. The playback button shows its current action (Play or Pause), and an unbuffered paused stream explicitly says Paused. A live service with no video packets fails after 15 seconds instead of waiting indefinitely (the existing one-time cached-PID discovery retry can occur first); a stream delivering packets but no decoded picture fails after 30 seconds. GPU safety checks remain enabled.

## Streaming reliability and window resizing

Live pause holds the displayed picture and mutes audio while reception, the audio clock and H.264 decoding continue. Decoded frames are discarded during this live hold, so queues do not stall the graph; resume returns to the current live position without a decoder reset. Record first to pause and resume from a buffered position. File and recording playback retain their existing clock-based pause and seek behavior.

Dragging the main window edges preserves the current video dimensions’ aspect ratio, accounting for native window borders, DPI, the seek strip and either toolbar row count. Before dimensions arrive, it uses 16:9. Maximized and fullscreen windows continue to fit video without stretching.

## Classic toolbar grouping

The player toolbar separates playback, capture/EPG, channel/volume and application controls into four sunken bronze wells. Groups have wider spacing than individual buttons; the channel pair stays adjacent, the camera control is compact, and Settings retains its wider button. At narrow widths the groups wrap into two centered rows, moving the seek strip and video boundary together to preserve readable labels. Geometry and bevel painting share the same DPI-scaled layout.

## Faster first picture in alpha 7

Saved TV services with matching frequency/program and complete supported PID metadata skip the former three-second discovery recording and its extra tuner session. The actual live tune still occurs once. Unknown/incomplete services use discovery on a cancellable worker; its completion cannot cancel playback. The latest pending channel selection replaces earlier requests, and a worker-completion message starts it without waiting for the 300-ms status timer.

Reception now starts on the existing bounded receiver worker before the renderer is constructed, retaining broadcast data that arrives during GPU setup. If a saved service produces no compressed video packets for eight seconds without a decoder error, playback releases the old graph and tries fresh discovery once. Incoming H.264 packets keep the existing decoder alive while it waits for an IDR; a delayed first picture alone no longer triggers a retune. An absent requested program reports a rescan error rather than playing another channel. GPU errors do not trigger this retry. Decoding and presentation remain on their existing separate threads; Vulkan frame pacing and GPU submission code are unchanged.

Each playback session records `native-startup.json`: service source, service-ready time, receiver-ready time, renderer-ready time, graph-running time, first compressed video packet time and first decoded frame time. These are startup stages, not measured click-to-screen latency. Existing alpha 6 logs showed first-frame times around 2.9–3.4 seconds on cached services and 12.9 seconds on a probe path; retry timings in those old logs overwrite the initial attempt and are not a complete channel-change duration. Live switching latency has not been measured in alpha 7. Broadcast keyframe spacing and RF acquisition still limit startup; IDR/reference safety checks remain enabled. Previous smooth playback was reported by the user on alpha 5; short alpha 7 live and pause/resume tests subsequently completed; see the validation report for alpha 8 results.

## Watching and recording

The last selected channel starts automatically. Channel buttons and the mouse wheel cycle saved TV services, including multiple services on one frequency. Labels use the broadcast channel number and service name; an em dash means the number has not yet been received. Older saved scans are recovered automatically. Rescanning obtains missing broadcast metadata.

The gear button opens Settings. Channel discovery defaults to Brazil and includes editable ISDB-T profiles for Argentina, Bolivia, Chile, Costa Rica, Ecuador, El Salvador, Guatemala, Honduras, Nicaragua, Paraguay, Peru, Uruguay, and Venezuela. Save country adds a custom profile. These presets search within this receiver's supported 470–697.999 MHz range with 6 MHz channels; they do not add support for other broadcast standards. The US is excluded.

REC saves the original broadcast transport stream and opens playback of the growing recording. Pause and the stream position slider allow movement within the recorded portion. Ordinary live reception has no seek buffer and disables the slider. Open plays a saved `.ts` recording. Snapshots are BMP images in the playback session folder; Library opens the session storage.

Settings and sessions are under `%LOCALAPPDATA%\A865R\TV`.

## Program guide

The guide opens through a deferred UI message, outside the player's active window update. Closing during a guide update hides the window safely; reopening validates its handle and recreates it when necessary. Native window lifecycle regression tests cover opening, filtering, resizing, closing during an update, and reopening.

EPG opens a native program guide with a channel filter, program times and titles, and a description pane. Present/following and schedule EIT tables are collected from the stream already being watched, including while playing a recording. Programs received from previous channels are cached locally. Tune to a channel to refresh its guide; opening EPG does not interrupt playback or reserve a second tuner handle. Availability and schedule depth depend on what the broadcaster transmits. Times are displayed as broadcast, with no assumed PC timezone conversion. This version does not schedule future recordings from the guide.

## Picture and color

Native, 2K (2560 × 1440), and 4K (3840 × 2160) select the video processing resolution. A separate GPU presentation pass fits the result to the whole viewing area in every mode, preserving its aspect ratio. Native and 2K therefore also fill a 4K fullscreen display. The dedicated renderer uses Vulkan on the selected GPU, with source-resolution motion-adaptive field-rate deinterlacing and GPU hardware linear scaling. It follows the DirectSound audio reference clock and presents through a synchronized FIFO swapchain. Per-sample field-order and progressive/weave flags override the stream defaults. Stationary detail is preserved using neighboring frames rather than interpreting sharp scanlines as motion. Vulkan Video decodes H.264 on the same device used by the renderer. NV12 planes are sampled directly by the renderer; a GPU image copy keeps display frames independent from reusable decoder reference surfaces. Decoded pixels are never staged through CPU memory. Windows supplies AAC audio and the playback clock.

Choose ICC selects an RGB `.icc` or `.icm` display profile. Select that profile in Color profile and press Apply. Windows' color engine generates a 33 × 33 × 33 lookup table using relative colorimetric intent; the Vulkan shader applies it after NV12-to-RGB conversion and scaling, directly to the display output. This avoids the previous RGB-to-NV12 round trip and per-frame GPU readback. A Vulkan-capable GPU driver is required; initialization failures are reported. Off bypasses the transform. Monitor ICC resolves the current display's assigned profile at playback startup; apply again after moving between displays.

Vsync uses FIFO presentation and a dedicated render thread paced by the current display's vertical blank. The audio clock is sampled after acquiring a presentation image. The renderer chooses the nearest source field for the upcoming refresh and requests three swapchain images when supported by the Vulkan surface limits. A high-resolution timer provides fallback pacing if the display wait is unavailable. Reception, decoding/audio, and presentation execute independently with a bounded decoded-frame queue; paused playback avoids continually redrawing unchanged frames. Vulkan Video decoding runs on its own bounded worker queue. Stop/seek invalidates queued pictures and recreates the parser/decoder for the new stream epoch.

`native-vulkan.json` reports the surface limits, requested image count, timing mode, and submission intervals. These intervals describe application frame submission, not measured physical scanout. Diagnostic `--trace-pacing` records individual submissions; `--legacy-pacing` enables the former polling loop for comparisons. `--verify-epg` with `--verify-output` repeatedly exercises guide opening, filtering and hiding during a bounded playback verification.

This is an SDR display-profile conversion using sRGB primaries and display encoding. Native HDR source decoding, WCS XML profiles, non-RGB profiles, and loading calibration curves into the system gamma ramp are not implemented. Alpha.28 adds a separate video-only scRGB output mode; that mode bypasses this SDR ICC conversion. Broadcast recordings remain unchanged. Each session's `native-color.json` records the selected profile's hash, actual frame count, backend; selecting a path alone is not reported as successful application.

## Preview validation and recovery

Vulkan Video is enabled in the normal build. This version has passed compilation, CPU-only parser/reference checks on the recorded broadcast, and player tests excluding hardware GPU tests. It has **not** been playback-tested after the earlier reported graphics-driver crashes. Those crashes are not established as fixed. Setup does not launch the player or modify GPU drivers.

This preview requires Vulkan H.264 8-bit 4:2:0 support for the stream's actual profile/layout and separate transfer/video queues from the presentation queue family. Unsupported profiles, reference limits, or missing references stop playback with an error. Format changes require an IDR picture. The C++ parser is vendored and bounded at its Rust interface; video decompression runs in Vulkan hardware. No third-party codec executable is used by the player.

For recovery, launch `live-tv.exe --windows-renderer` to bypass Vulkan Video and the Vulkan renderer. `--software-decoder` uses Windows H.264 decoding with Vulkan presentation for comparison. The latter still needs a Vulkan Video-capable adapter in this preview. Neither is selected silently during normal startup.

`native-decoder.json` reports the selected decode backend and counters from the Vulkan decoder worker. A successful build is not proof of correct GPU playback; new timing, stability and CPU-usage measurements are pending.

## Build

From the workspace root on Windows with Rust 1.92+ and Visual Studio C++ build tools. Vulkan headers and the broadcast parser are vendored; the Vulkan SDK is not required to build. Microsoft C++ runtime DLLs must be beside the executable or supplied by the installed redistributable:

```text
cargo build -p a865r-tv --release --locked
cargo test -p a865r-tv -p a865r-bda -p liba865r --lib --bins --locked
```

Windows 10/11 media components and a Vulkan-capable graphics driver are required. The diagnostic `--windows-renderer` switch retains the earlier EVR path for comparison; normal startup uses Vulkan. `native-vulkan.json` records the actual adapter and presentation counters. `native-health.jsonl` samples this player's audio-session output peak and video timing, without recording audio. Current playback supports H.264 TV with AAC audio. This is an alpha; long recordings, suspend/resume, and every country's broadcasts have not been exhaustively tested.

GPL-3.0-only. See the workspace LICENSE, THIRD_PARTY.md, and third-party-licenses directory.

### Alpha 13 dial controls

Drag the volume dial clockwise to increase volume or counterclockwise to decrease it. The silver disc animates while the gold scale stays fixed. Volume is saved on release; the redundant label below the dial is removed. Nine subpixel samples per output pixel soften the artwork at small sizes, with rendering cached between changes.

### Alpha 14

Dial animation avoids unbuffered background erasure and keeps the last frame during reentrant updates. The supplied TV icon is embedded in the executable and installer. Building requires the Windows SDK resource compiler.

### Alpha 15

Volume controls use the supplied silver-and-gold artwork, with separate pressed sprites for mouse and keyboard activation. Exterior background and a one-source-pixel matte border are removed during loading. Downscaled images are cached and painted with the panel background in one buffered frame.

### Alpha 16

The control panel uses a centered draggable volume knob without the two buttons beneath it.

### Alpha 17

Centered information-display text and clock. Restored the main player’s rectangular VOL − / VOL + controls; the separate panel keeps its centered knob without buttons below it.

### Alpha 18 fullscreen pacing

Fullscreen uses the Vulkan FIFO swapchain to pace rendering, avoiding an extra display-vblank wait. Windowed playback retains its existing vblank clock. VSync, triple buffering, Vulkan Video decoding, and GPU-only image processing remain enabled. The original timing comparison did not verify foreground fullscreen; see the correction and known issue below.

### Alpha 19 fullscreen taskbar handling

Entering fullscreen activates the player and explicitly marks its window as fullscreen with the Windows Shell. Leaving fullscreen removes that marking and restores the previous window geometry. The player is not made globally always-on-top.

Known issue: sustained foreground fullscreen playback still slows to approximately 24 presentations per second on the tested RX 7900 XTX, compared with approximately 60 windowed. This release does not claim to fix that slowdown. Measurements place most of the extra time in swapchain acquisition; GPU completion was approximately 2.3 ms in the diagnostic run. Alternative buffering and presentation experiments did not solve it and are not included. Vulkan GPU decoding/rendering, FIFO VSync, and supported triple buffering remain enabled.

Correction to the alpha 18 result: its faster fullscreen comparison did not verify foreground activation and taskbar coverage, so it does not establish a foreground fullscreen performance fix. Alpha 19 validation checks foreground activation and the bottom-edge window at physical display coordinates.

### Alpha 20 — RBI pacing and GPU deinterlacing

RBI HD sends separately coded H.264 fields. The old eight-packet compressed-video queue could block shared demultiplexer delivery while future decoded pictures waited for the audio clock, starving audio and repeatedly holding the picture. The queue now accommodates 64 packets with an aggregate 8 MiB compressed-data limit. Cancellation releases reservations. The decoded-frame queue remains bounded at twelve pictures, and decoding/presentation stay on Vulkan.

The GPU deinterlacer reconstructs moving luma edges along the best neighboring direction and samples interlaced NV12 color within the selected field, avoiding color mixing between capture times. Stationary luma detail still uses temporal weaving; progressive pictures bypass deinterlacing. Smooth, Standard, and Off settings now reach the Vulkan path for live, file, and recorded playback. GPU image views and bindings are reused for repeated fields instead of recreated at every display refresh.

Starting or changing a channel while minimized now configures the swapchain before acquiring its first image. A new transport loss marks one discontinuity rather than making every later sample discontinuous. Detailed renderer failures are saved in native-renderer-error.txt.

The separate sustained foreground-fullscreen presentation slowdown documented in alpha 19 remains unresolved. Broadcast signal loss or missing input pictures can still cause visible interruptions; this release does not synthesize absent broadcast frames.

Alpha 22 reconstructs each source pixel once, uses hardware texture filtering for scaling, and reuses the processed image on repeated refreshes. Linear scaling can look slightly softer than the previous bicubic filter. Lookahead uses only an already available, contiguous frame from the same stream epoch; it adds no waiting. Sustained foreground fullscreen slowdown remains unresolved; see [validation](../docs/ALPHA22-VALIDATION.md).

Picture and Parental settings, the restored Receiver/Audio controls, and alpha.27 validation are documented in [the release notes](../docs/ALPHA27-VALIDATION.md).
