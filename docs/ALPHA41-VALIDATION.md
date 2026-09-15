# Alpha.41 validation — 2026-09-13

## Report and diagnosis

The user reported audio without video after changing to Microsoft decoding with DirectX 11/12, and confirmed that closing and reopening Live TV restored video. Alpha.40 validation used recordings and did not establish that interactive live backend transitions remained visible. Early five-second EVR snapshot failures were not proof of the persistent failure: later snapshots contained video and EVR counted drawn frames.

Two presentation defects were addressed: the same Win32 child HWND was reused across graphics APIs, and its owner-draw handler unconditionally painted black without forwarding EVR repaint requests. The new HWND is created only after the previous playback worker has finished; its caption child is replaced with it and its sibling order stays behind the controls. Existing decoder loss/retry policy is unchanged.

## Retained processing route

Microsoft-decoded NV12 can use DirectX 11/12 picture processing on the preparation worker and the paced Vulkan presenter when Vulkan graphics is available. A maximum of three processed frames is cached by epoch and frame ID. Changing picture settings invalidates that cache, including on a paused frame. ICC and picture effects are disabled in the following Vulkan color pass to prevent applying them twice. This moves work off the decoder/audio-delivery thread and retains the existing field scheduling and displayed-frame caption clock.

Without Vulkan graphics, Microsoft decoding uses the EVR route with DirectX processing. Its paint/resize events now request EVR repaint. CPU selection remains supported and disables custom picture shaders.

Vulkan decoding continues to use Vulkan shaders and shared GPU textures. A working experimental cross-API download route measured approximately 5.1 ms (DX11) and 6.1 ms (DX12) per source frame, including transfer and effects. It was removed at the user's request to avoid costly transfers. No Vulkan download code is included in this release.

## Focused tests

31 passed: backend selection (2), native playback/recovery (15), ICC/DirectX effects (9), caption scheduling/layout (3), and the new processing cache/paused-frame effect tests (2). The DirectX hardware checks exercise both DX11 and DX12. The existing desktop caption-overlay test was excluded because the desktop automation helper could not initialize; no visual UI verification is claimed.

The retained Microsoft + DX11 pre-release HD recording test produced nonblack frames, zero dropped source frames, and a final presentation interval mean of 16.689 ms (p95 18.095 ms). These are display intervals, not shader processing time. Final release live and EVR results are recorded below after execution.

## Start menu inspection

The Start menu and desktop Live TV shortcuts pointed to LocalAppData/Programs/A865R/player/live-tv.exe. Its SHA256 matched the delivered alpha.40 binary. No separate alpha.39 Start menu shortcut or uninstall entry was found during inspection. The alpha.41 installer updates the same stable shortcut and AppUserModelID; no unrelated Start cache or registry entries were deleted.

## Limits

Playback diagnostics inspect frames returned by the renderer, playback counters, and timing; they do not substitute for observing the user's desktop during interactive switching. The known test machine uses an AMD Radeon RX 7900 XTX and a roughly 59.9 Hz display. Results are not a performance guarantee for other adapters or damaged broadcasts. No vendor driver, firmware, installed app, or Windows HDR setting was modified by this work.

## Final alpha.41 live regression

An 80-second RBI HD live run used the alpha.41 binary with isolated settings and hidden diagnostic windows. Saturation 115%, brightness +3, contrast 110%, HDR effect enabled, and the monitor ICC profile were active. The sequence used the normal Settings restart method. All four sessions returned success and stopped cleanly; four renderer snapshots contained nonblack image data. Surface HWNDs were distinct after each transition.

| Decoder / effects | Presentations | Dropped source frames | Final mean interval | Final p95 interval |
| --- | ---: | ---: | ---: | ---: |
| Vulkan / Vulkan | 854 | 0 | 16.709 ms | 18.196 ms |
| Microsoft / DX12 | 857 | 0 | 16.689 ms | 17.950 ms |
| Microsoft / DX11 | 826 | 0 | 16.699 ms | 18.124 ms |
| Microsoft / Vulkan | 852 | 0 | 16.703 ms | 18.750 ms |

DX12 shader time averaged 1.865 ms per processed source frame; preparation including copies and initialization amortization averaged 3.656 ms. DX11 averaged 2.065 ms shader time and 3.800 ms preparation. Each initialized its shader processor once. These are source-frame costs; cached frames serve the second field and repeated display refreshes. Presentation statistics above are the final rolling interval window, while presentation/drop counts cover the session.

## Final EVR fallback regression

Three separate 18-second runs forced the Windows renderer using the supplied TV GAZETA HD recording, with Microsoft decoding and DX12, DX11, then Off (CPU). All returned success, stopped cleanly, and produced nonblack EVR snapshots. This verifies the route that remains available when Vulkan presentation is unavailable. The native EVR counters and color-processing reports were retained in the diagnostic workspace.

## Reproduction commands

Use an isolated profile directory containing settings.json; the following modes do not install or change the normal user's settings:

- Live transitions: `live-tv.exe --verify-output <isolated-directory> --soak-seconds 80 --verify-hidden --verify-shader-switch` (requires the tuner to be free).
- EVR: `live-tv.exe --verify-output <isolated-directory> --soak-seconds 18 --verify-hidden --windows-renderer --play <recording.ts>`.

The soak snapshot is taken at 12 seconds, allowing live startup to finish before checking renderer output. `--verify-hidden` suppresses the initial player and receiver windows. The switch sequence begins with the profile's selected decoder/shader and then chooses Microsoft/DX12, Microsoft/DX11 and Microsoft/Vulkan at 20-second intervals.
