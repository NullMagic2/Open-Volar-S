# Vulkan Video preview — 0.8.0-alpha.24

Alpha 24 reduces CPU overhead by caching GPU texture views, avoiding duplicate GPU completion polling during active submission, skipping hidden fullscreen chrome painting, and using native GDI gradient fills for the interface. GPU decoding, synchronization and the alpha 23 fullscreen pacing remain in place. Bounded fullscreen checks retained about 60 presentations per second; measured CPU savings were modest. See [alpha 24 validation](ALPHA24-VALIDATION.md) for evidence and limits.


Alpha 23 adds three reusable GPU picture slots and separate preparation/presentation workers. Frames stay on the shared Vulkan GPU device; only handles and timing metadata pass between workers. The bounded handoff selects a clock-appropriate picture after image acquisition. Requested snapshots use a separate bounded worker. Fullscreen now uses a one-pixel black inset with display-vblank pacing; three sustained checks stayed near 60 fps on the tested RX 7900 XTX. The pointer hides after three seconds and returns on movement. See ALPHA23-VALIDATION.md for measurements and hardware limits.

## Alpha 22: lower-cost GPU scaling

GPU hardware linear filtering replaces the manual bicubic luma taps. Motion-adaptive deinterlacing runs once per source pixel, and repeated refreshes reuse the processed frame. Lookahead uses only a contiguous frame already decoded in the same stream epoch, without adding a wait. Native, 2K, 4K, aspect ratio and ICC behavior remain available. Linear filtering can look slightly softer. The sustained fullscreen presentation stall remains unresolved; see [measurements and limits](ALPHA22-VALIDATION.md).

## Alpha 21: surround, captions, recording location

Audio → 5.1 surround preserves native six-channel AAC, with correct format-change forwarding to the Windows audio output. Stereo sources stay stereo. CC On/Off enables timestamped ISDB / Brazilian Portuguese closed captions. Recording folder defaults to the current user’s Documents\A865R\recordings and can be selected and saved in Settings. Stream reconfiguration flushes preserve playback state.

See [alpha 21 validation](ALPHA21-VALIDATION.md) for the tested paths and limitations.

## Alpha 20: audio modes and recording controls

The Audio button opens Stereo, Mono, Left channel and Right channel output modes, followed by the selected station's broadcast audio tracks. Output mode changes apply immediately and are saved. AAC ADTS and AAC LATM tracks can be selected; language names are shown when the broadcaster supplies language descriptors, otherwise tracks are numbered. Mono mixes left/right evenly; Left and Right send the selected channel to both speakers. Mono broadcasts remain mono. Unsupported broadcast codecs are listed as unavailable.

Press REC to save the original transport stream and enable time shift. Capture continues on its own worker while playback is paused, moved with the timeline, or stepped backward/forward by ten seconds. Live resumes close to the recording's growing end with a small decoding margin. REC again finishes the recording and returns to live reception; Stop ends playback and recording. Recordings continue until stopped or an I/O error and remain in the chosen recording folder. Unrecorded live TV has no rewind buffer. Changing audio tracks rebuilds playback briefly without stopping the recording writer.

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

Alpha 5 fixes startup surface creation: the Win32 raw window handle now includes the video HWND’s actual GWLP_HINSTANCE. A message-only window regression checks valid, null, and destroyed handles without creating a GPU device.

Alpha 6 removes the fixed three-second discovery recording for valid saved channels, forwards cancellation to fallback discovery, wakes the UI immediately when playback releases the tuner, and preserves the existing threaded Vulkan decode/render pipeline. Native startup timings are written to native-startup.json. Alpha 7 begins reception before renderer setup using the existing bounded receiver queue. A stale cache gets one fresh-discovery retry after eight seconds without compressed video packets, excluding decoder errors; a keyframe wait with arriving video no longer restarts discovery. First-packet, receiver-ready and renderer-ready timing stages are also recorded. IDR safety gates and Vulkan submission/presentation timing remain unchanged. Same-frequency services retain the requested program ID.

Alpha 7 also keeps the live graph running during pause, discarding paused display frames and muting audio so bounded queues cannot block resume. A CPU-only regression fills the queue and checks that live pause unblocks delivery and resumes without resetting the decoder epoch or clock. Native edge resizing preserves video proportions across DPI changes and toolbar wrapping. The classic toolbar uses spaced sunken control groups.

Alpha 8 retains a first still picture when live pause is requested during tuning, labels the Play/Pause action explicitly, and reports inactive-service or missing-picture timeouts instead of waiting indefinitely.

Normal startup selects Vulkan H.264 decode and Vulkan presentation on the same device. The superseded D3D11/NV12 staging bridge was removed. AAC decoding, demultiplexing and audio clock remain native Windows services. The CPU still parses compressed H.264 headers and manages queues; GPU acceleration does not mean zero CPU use.

Corrections include initialization of every video session before decode, actual stream-profile capability queries, IDR gating, interlaced field-pair flags and active-reference accounting, missing-reference rejection, distinct queue-family checks, native parser buffer/marker bounds, synchronous callback lifetime checks and stream-epoch invalidation on stop/seek. The GPU adapter retains a GPU image copy so presentation does not sample a reference surface while it is being overwritten.

Validation: 21 player tests passed; two GPU tests were ignored. The isolated CPU-only parser inspector accepted the recording with 891 coded pictures, 18 field pictures, 882 display frames, 2 IDRs, zero missing references after IDR, and zero timestamp regressions. This validates parser output, not hardware decode output.

Short hardware verification results are recorded in the release validation JSON. These checks are bounded and do not establish long-term stability or comprehensively measure CPU utilization. Earlier green/red-screen system crashes are not confirmed resolved. This is a preview, not a stability-certified release. The installer does not start the application. Recovery: a865r-tv.exe --windows-renderer.

The installer includes driver/debug version 0.7.0 and embedded open firmware 0.1.4.0, unchanged. It does not install or update graphics drivers, flash firmware, or change WinUSB bindings.

Hardware debugging for alpha 8: the requested bounded live test on RX 7900 XTX resumed video and audible-output meter activity after startup pause, then exited normally. The first keyframe arrived after the scheduled resume, so the new paused-preview behavior itself has only CPU regression coverage. Earlier alpha 7 baseline and pause/resume runs also completed. TV GAZETA MOVEL remains a separate timing/demultiplexing failure: its captured sample contains video PID 4097 but no packets for advertised PCR PID 4098. The player now reports a startup error rather than waiting indefinitely; that was the remaining alpha 8 limitation.

Alpha 9 adds playback-only recovery for an absent selected-service PCR clock, derived from validated existing DTS/PTS after a one-second observation period. CPU-only demultiplexer comparison confirms restored mobile video delivery and identical HD sample counts, bytes and timestamps. Nine BDA tests and 21 player tests pass; two GPU unit tests remain excluded. Live alpha 9 results are recorded in the release validation JSON. The Windows demultiplexer clock requirement is documented at https://learn.microsoft.com/en-us/windows/win32/directshow/demux-clock-behavior and https://learn.microsoft.com/en-us/windows/win32/directshow/mpeg-2-demultiplexer.

Alpha 9 live result: TV GAZETA MOVEL now decodes and presents on RX 7900 XTX with nonzero audio-output meter activity. The first session reached its first decoded frame at 3,126 ms, then recorded 301 received frames and 587 presentations; a second session recorded 85 frames and 167 presentations. Both ended normally. These bounded checks do not establish long-term driver stability.

### Alpha 19 fullscreen taskbar handling

Entering fullscreen activates the player and explicitly marks its window as fullscreen with the Windows Shell. Leaving fullscreen removes that marking and restores the previous window geometry. The player is not made globally always-on-top.

Known issue: sustained foreground fullscreen playback still slows to approximately 24 presentations per second on the tested RX 7900 XTX, compared with approximately 60 windowed. This release does not claim to fix that slowdown. Measurements place most of the extra time in swapchain acquisition; GPU completion was approximately 2.3 ms in the diagnostic run. Alternative buffering and presentation experiments did not solve it and are not included. Vulkan GPU decoding/rendering, FIFO VSync, and supported triple buffering remain enabled.

Correction to the alpha 18 result: its faster fullscreen comparison did not verify foreground activation and taskbar coverage, so it does not establish a foreground fullscreen performance fix. Alpha 19 validation checks foreground activation and the bottom-edge window at physical display coordinates.

### Alpha 20 — RBI pacing and GPU deinterlacing

RBI HD sends separately coded H.264 fields. The old eight-packet compressed-video queue could block shared demultiplexer delivery while future decoded pictures waited for the audio clock, starving audio and repeatedly holding the picture. The queue now accommodates 64 packets with an aggregate 8 MiB compressed-data limit. Cancellation releases reservations. The decoded-frame queue remains bounded at twelve pictures, and decoding/presentation stay on Vulkan.

The GPU deinterlacer reconstructs moving luma edges along the best neighboring direction and samples interlaced NV12 color within the selected field, avoiding color mixing between capture times. Stationary luma detail still uses temporal weaving; progressive pictures bypass deinterlacing. Smooth, Standard, and Off settings now reach the Vulkan path for live, file, and recorded playback. GPU image views and bindings are reused for repeated fields instead of recreated at every display refresh.

Starting or changing a channel while minimized now configures the swapchain before acquiring its first image. A new transport loss marks one discontinuity rather than making every later sample discontinuous. Detailed renderer failures are saved in native-renderer-error.txt.

The separate sustained foreground-fullscreen presentation slowdown documented in alpha 19 remains unresolved. Broadcast signal loss or missing input pictures can still cause visible interruptions; this release does not synthesize absent broadcast frames.
