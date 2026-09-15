# Alpha.35 validation

## Stability investigation
Windows Error Reporting recorded LiveKernelEvent 141 and AMD watchdog reports at 2026-09-13 00:17:32 during the previous alpha.34 recording investigation. The diagnostic playback graph reported 0x80040204 and received/presented zero video frames. Its --software-decoder option still used Vulkan presentation; this was not a GPU-independent comparison. The kernel dump could not be opened because Windows denied file access. The exact driver hang cause remains unconfirmed.

Changes:
- Recording cancellation and joining now use ownership guards, including panic cleanup. A writer panic clears the growing-file flag so readers cannot wait indefinitely. Recording startup has a 30-second data-readiness limit.
- Frame history sums the actual sizes of retained frames across resolution changes, capped at 64 MiB and 32 frames. Stopped/stale delivery cannot repopulate history; flushing releases the previous step frame.
- GPU preparation and presentation each retain at most three outstanding submission leases. Missing GPU completion for two seconds stops the pipeline. Completion callbacks are drained after the presentation worker joins, with a bounded wait, releasing retained image leases on normal shutdown.
- GPU timeout or out-of-memory failure suppresses immediate automatic renderer recovery. Vulkan remains disabled for the process after those failures. Existing recovery for a reported device loss remains available.
- Added --capture-only FREQUENCY_KHZ SECONDS OUTPUT_DIRECTORY. This explicit diagnostic path accepts 1–30 seconds, creates no application window/decoder/renderer, respects parental restrictions, and refuses to overwrite an existing recording.ts. It does not contend for a tuner unless explicitly invoked.

These are verified cleanup and resource-budget safeguards, not proof that the original AMD driver hang has been eliminated. No further GPU playback, hardware recording, driver reset or installation was performed for alpha.35 validation. In-process guards cannot interrupt a graphics driver call that is already blocked inside the driver.

## Broadcast and picture
The saved 4,071,652-byte TS sample contains approximately 3.27 seconds of the Globo multiplex, including TV GAZETA MOVEL (program 16984, H.264 Constrained Baseline, 320x180) and TV GAZETA HD (16960, H.264 High, 1920x1080, limited-range BT.709). CPU-only FFmpeg decoding also shows the mobile service's blockiness; this short sample does not establish a GPU-only decoding defect. It is a mobile service, not conventional full-resolution SD. Initial missing-PPS warnings occur before the captured HD stream's headers/keyframe become available. No speculative SD decoder change was made.

Cold previously multiplied blue by 1.06, clipping the upper blue highlights. Cold and new Warm use a monotonic tonal tint that preserves black, white and near-white distinctions. Neutral, explicit brightness/contrast controls, and the HDR effect remain present. Warm is translated as Quente, Cálido and Θερμό for Portuguese, Spanish and Greek.

## Settings and captions
- Removed the global Apply button. Video options, ICC selection, Windows receiver startup, storage folders, and parental edits now save from their control events. Invalid storage paths or startup errors do not replace the last valid setting. Password confirmation and editing authorization remain required.
- Shared scrollbar handling covers client and nonclient arrow clicks, repeat, thumb dragging and partial mouse-wheel deltas without selecting a channel or releasing dropdown capture. Parent channel-wheel shortcuts defer to open channel/country dropdowns.
- Latin captions are centered within the video image, use tighter half-width glyph spacing and safe-area wrapping. Caption timing, colors, clear events and non-Latin/DRCS layouts are preserved.
- The approved ruby USB icon and previous alpha.34 startup layout changes are retained.

## Validation
- Release-mode player regression suite: 92 passed, zero failed, two real-GPU tests intentionally ignored.
- New tests cover recording cleanup on playback/writer panics, mixed-resolution memory budgeting, stopped frame delivery, bounded GPU completion ownership/timeouts, suppressed retry after timeout/OOM, bounded capture arguments, native dropdown scrolling, caption rendering at three sizes, and Warm/highlight behavior.
- Shader parsed and validated with Naga; captions rendered with the native caption library and visually inspected.
- Workspace release build, installer resource/icon comparison, source archive CRC and SHA-256 checks completed for the packaged release.
- Actual playback behavior and GPU stability of this build still require hardware validation. Windows sign-in startup remains untested.
