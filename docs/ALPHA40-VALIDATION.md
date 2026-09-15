# Alpha.40 validation

## Changes

- Decoder and Shader acceleration are separate, persisted settings. Vulkan decoding remains the default and keeps the shared Vulkan renderer. Microsoft decoding offers Automatic, Vulkan, DirectX 12, DirectX 11, and Off (CPU), filtered by hardware availability. Automatic tries Vulkan graphics, then DirectX 12 and DirectX 11. Graphics-only Vulkan initialization does not require Vulkan Video codec support. Explicit selections report initialization failure rather than silently choosing a different decoder.
- DirectX 12/11 counterparts implement saturation, brightness, contrast, cold/warm/vivid presets, HDR effect and ICC lookup. The effect is bounded SDR processing, not reconstructed HDR. CPU-only playback disables custom effects and retains CPU ICC correction and saved picture values.
- A decoded NV12 frame is processed in reused system memory before copying once into the EVR allocator. Processing directly in EVR-owned mapped video memory was the measured cause of the initial approximately 23 ms effects path; the isolated shader benchmark had concealed that integration overhead.
- Device-loss recovery permits five fresh attempts, with cancellation between attempts and resource destruction before retry. The selected decoder is retained. Unrelated failures stop; no automatic Microsoft fallback or process lockout was added.
- Captions use presented frame timestamps with Vulkan. EVR paths (DirectX 12, DirectX 11 and CPU) use the same graph/audio clock as video scheduling, bounded by timestamped samples delivered to EVR. This avoids advancing captions past stalled video. Future packets remain queued until due. Paused frame steps can update captions without advancing a wall clock. Latin caption text is approximately 10% larger, retaining wrapping, original case and broadcast timing.
- Color profile and Choose ICC occupy the top Video row; other options begin after a gap. Shader acceleration uses the existing Orbit combobox painting, font, rim and focus behavior. AverTV signal is in Video, its status is dark and clears after five seconds. Themes replaces Appearance. New labels are translated in the four-language catalog.
- Tab pages extend to the window's bottom inset. Child status controls paint the tab background, fixing the wood rectangle under the Channels scan buttons. The same shared geometry applies to every tab and DPI.
- Setup and application use normal-user privileges. Optional adapter updates compare hashes before requesting elevation; absent or matching registrations do not prompt. Existing machine-wide installations can coexist with the per-user installation. The adapter updater preserves registrations and supports backup/rollback. The final page offers Run Live TV.

## Automated checks

Final focused player tests: 40 passed (2 backend, 15 native/recovery, 9 ICC/shader, 3 caption, 8 picture, 3 localization). Adapter tests: 18 passed. The isolated PowerShell update-plan tests passed for absent/equal adapters, changed machine/user DLLs, duplicate registrations, missing registered DLLs and damaged staged files. Both x64 and x86 adapter builds succeeded.

GPU tests executed DirectX 12 and DirectX 11 on the actual adapter and compared NV12 pixels with the independent CPU reference for BT.601 and BT.709, presets, individual controls, HDR effect, and repeated on/off toggles. Maximum allowed discrepancy is two code values. ICC parity and WGSL validation passed. The CPU-off regression verifies byte-exact passthrough despite saved non-neutral effects. Caption tests exercise cue start/clear boundaries, a far-ahead graph clock, pause, forward/reverse buffered steps, and safe-area wrapping.

## File playback measurements

These diagnostics used isolated settings and file input, without requesting the USB tuner. All completed and shut down cleanly; all snapshots contained image detail.

| Decoder / shaders | Fixture | Evidence |
|---|---|---|
| Microsoft / Vulkan (Automatic) | User-supplied TV GAZETA recording | 20 seconds, seven caption cues, zero caption decoder errors; caption clock follows presented-frame time (19,638 ms vs graph 19,631 ms in final sample). |
| Microsoft / DirectX 12 | Same recording | Seven cues, zero errors; caption and bounded EVR clock both 19,551 ms. GPU effects average 1.616 ms/frame; 546 frames transformed. |
| Microsoft / DirectX 11 | Same recording | Seven cues, zero errors; both clocks 19,594 ms. GPU effects average 1.748 ms/frame; 548 frames transformed. |
| Microsoft / Off (CPU) | Same recording | Seven cues, zero errors; both clocks 19,592 ms. Zero GPU initializations; effects disabled. |
| Vulkan Video / Vulkan | Preserved clean HD fixture | 593 hardware-decoded frames received, 1,091 field/repeat presentations, zero queue drops; presentation mean 16.691 ms and p95 17.949 ms. |

Additional clean-HD DirectX 12 validation transformed 597 frames in 20 seconds, with GPU effects averaging 1.852 ms/frame. Earlier corrected DirectX 11 integration averaged 1.950 ms/frame for 597 frames. Measurements are hardware-specific, not a promise for other GPUs.

Real pause/seek/resume diagnostics on the user's recording passed with Microsoft+Vulkan and Microsoft+DirectX 12. Both held the paused clock, restarted the seek epoch and resumed without caption decoding errors. Exact cue-boundary and buffered reverse-step behavior is covered by deterministic tests. A seek can still wait for the next broadcast caption management/cue packet; it does not invent missing caption content.

## Limits and references

This verifies application scheduling against media timestamps, not whether a broadcaster authored its captions early or late relative to speech. No fixed compensating delay or altered cue durations were introduced. The user's recording contains damaged opening H.264 packets, as noted in the earlier recording investigation; it was exercised through Microsoft software decoding. The native Vulkan decoder was checked against the preserved clean fixture.

No forced driver reset was injected. Five-attempt recovery is tested with deterministic simulated errors, not an induced GPU crash. The desktop automation helper could not initialize (sandbox ACL error), so final tab visuals could not be independently captured through it. Shared paint paths, layout coordinates, compilation and user-provided visual evidence were used for the gap correction. The final tab-background edit changes painting only, after the media-path tests above.

EVR scheduling uses media sample timestamps and its clock, as described in Microsoft's [EVR presenter documentation](https://learn.microsoft.com/en-us/windows/win32/medfound/how-to-write-an-evr-presenter). We did not add per-frame GetCurrentImage readbacks to inspect timestamps.

The vendor driver/firmware, active tuner session and installed adapter registrations were not changed. The final installer and source archive are built together; SOURCE-MANIFEST.json records every source file hash, with separate release checksums.
