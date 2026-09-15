# Original AVerTV fullscreen comparison

The original AVerTV was measured using process-filtered Intel PresentMon 2.5.1 and read-only module/window inspection. Tests used bounded ordinary playback and retained all GPU synchronization.

## Confirmed findings
- AVerTV loads EVR, Direct3D 9 and DXVA2. Its fullscreen trace contains 2,397 presentation events over 40 seconds using Composed: Copy with GPU GDI. Three rotating presentation addresses combine to approximately 59.94 presentations per second. Foreground was confirmed in 76/80 samples.
- Our alpha22 fullscreen can settle at approximately 23.96 presentations per second (41.74 ms), with Windows using Hardware Composed: Independent Flip. Earlier display-mode queries remained at 3840x2160, 60000/1001 Hz.
- A 200-frame fullscreen sample of instrumented Vulkan acquisition measured: previous rendering submission wait 0.014 ms; vkAcquireNextImageKHR 0.036 ms; acquired-image host fence completion/reset 40.126 ms.
- Decoded NV12 images remain on the shared Vulkan device. This wait is not CPU pixel copying or CPU decoding.

## Repeat-test results
- A diagnostic parent-window opacity of 254/255 forced Composed: Flip and sustained approximately 16.697 ms over the measured part of a 40-second test, with all foreground samples confirmed. It changes image opacity and is not a release fix.
- Fully opaque parent layering did not fix the sustained stall.
- Fully opaque child layering and explicit vblank pacing each appeared successful in a short test, then failed in 60-second repeats: approximately 41.74 ms with 88/88 foreground samples confirmed between seconds 10–54. Do not treat the short traces as validated fixes.
- Applying opaque child layering before fullscreen also failed.
- Correctly enabled Vulkan fullscreen-exclusive DISALLOWED, process-local fullscreen compatibility behavior, and both RGBA8/BGRA8 SDR surface formats did not resolve the sustained failure.
- An older fullscreen-policy experiment had not enabled its dependent extension. Correcting that omission still did not fix the problem.
- A previous semaphore-only/no-host-acquire-wait experiment likewise did not fix it. Moving a wait from the CPU to the GPU does not, by itself, make the presentation engine release a display image earlier.

## Status at the end of the alpha 22 investigation
The precise blocking stage has been localized, but no reliable fix is validated. No alpha23 was published. Experimental source changes, Cargo.lock changes and the build executable were restored and compared byte-for-byte with the saved pre-investigation copies. The existing published source ZIP was no longer present at its earlier location, so no archive comparison is claimed.

No persistent GPU/compatibility settings, display modes or drivers were changed. The remaining investigation is Vulkan/AMD/Windows display-buffer availability and presentation scheduling. Additional lookahead, assembly or frame interpolation does not address the measured wait.

## Evidence
work/avertv-comparison/comparison-summary.json contains interval/focus summaries. Original-player evidence is original-runtime.json, original-fullscreen.csv and original-window-state.json. Each *-foreground directory contains process-filtered ETW presentation records, focus samples, and player timing. bgra-acquire-trace-foreground also contains acquire-steps.csv.

References:
- https://github.com/GameTechDev/PresentMon
- https://docs.vulkan.org/refpages/latest/refpages/source/vkAcquireNextImageKHR.html
- https://docs.vulkan.org/refpages/latest/refpages/source/vkQueuePresentKHR.html
- https://devblogs.microsoft.com/directx/dxgi-flip-model/

Follow-up: alpha 23 implements the separate GPU preparation/presentation design. See ALPHA23-VALIDATION.md; the historical results above describe alpha 22 and the discarded experiments.
