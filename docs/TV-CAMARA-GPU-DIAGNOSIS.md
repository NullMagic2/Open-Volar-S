# TV Câmara GPU crash diagnosis — 2026-09-12

## Confirmed evidence

The Windows WATCHDOG dumps from 12:50 and 12:57 local time both report **VIDEO_ENGINE_TIMEOUT_DETECTED (0x141)** with failure bucket `LKD_0x141_IMAGE_amdkmdag.sys`. The AMD display driver version is **32.0.31036.15** on the Radeon RX 7900 XTX. Symbolized stacks show the Windows graphics scheduler checking GPU progress and attempting engine recovery.

The earlier player session reported `Vulkan presentation worker panicked`. During the later diagnostic, the computer hard-crashed and surviving application logs were incomplete. The dumps establish a GPU engine timeout; they do not identify the submitted GPU command or establish whether the initiating fault is in driver behavior, application commands, or synchronization between decoding and presentation. GPU instability has not been fixed.

Analysis used local copies of WATCHDOG-20260912-1250.dmp and WATCHDOG-20260912-1257.dmp with matching Microsoft public symbols. The AMD auxiliary dump supplies vendor diagnostic collection context, not a proven root cause. Crash dumps were not uploaded.

## Safe repeat

A 12-second capture-only tuner run wrote 15,768,500 bytes without opening the Vulkan player. The receiver reported 83,806 transport packets, 27 sync losses, 26 transport errors, and no continuity errors. Offline analysis places the observed transport errors near capture startup; this is not proof of flawless reception.

FFmpeg with hardware acceleration disabled decoded 328 frames and exited successfully. It also reported startup/reference and truncated-tail warnings. The application's CPU-only H.264 parser reported 496 pictures, 330 fields, 331 displayed pictures, no missing references after initialization, and no timestamp regressions. Mixed frame/field coding is a candidate for further decoder investigation, not an established cause of the hang.

## Changes and limitations

Presentation and decoder panic handlers now retain available panic text instead of discarding it. The presentation error file is flushed to disk to improve evidence after a failure. These are diagnostic improvements, not a GPU crash fix.

The existing `--software-decoder` option bypasses Vulkan Video hardware decoding but still uses Vulkan presentation. It could isolate the decode path, but it has not been run after the crash and cannot be described as guaranteed safe. No additional Vulkan playback tests were performed. UI verification used `--ui-preview`, which does not initialize tuner or Vulkan playback. Hardware GPU tests remain ignored.


## RBI SD device loss and recovery — 14:07 follow-up

The application session 1789232192687 ran RBI SD (channel 05) for about 636 seconds. Its final decoder report contained 37,636 decoded field pictures and 18,812 received frames. The decoder returned a Vulkan logical-device-lost error. The presentation error file separately reports a panic while releasing a swapchain acquisition semaphore still referenced by a surface texture. Windows Error Reporting at 14:07:15 recorded another LiveKernelEvent 141 and named WATCHDOG-20260912-1407.dmp, alongside AMD diagnostic event a2000002. This establishes another GPU timeout; it does not establish a USB disconnection or the initiating command.

Code review found a shutdown-order defect: the presenter was declared after the playback graph guard, allowing error unwinding to release the presenter before stopping the graph. The presenter now outlives the graph and decoder filters. The graph guard first cancels renderer input and wakes blocked delivery, then stops the graph. Existing USB capture shutdown joins the capture and delivery workers; device opening is exclusive. A playback ownership mutex serializes complete graph lifetimes, including recovery.

The application now preserves the first worker error, flushes decoder diagnostics, and rejects further known-failed-device submissions. Device-loss detection immediately disables Vulkan for the rest of this process, including if a channel change cancels the failing session before its result reaches the UI. Native playback panic containment reports an error after unwinding instead of silently disconnecting the UI result channel.

Recovery makes one cancellable attempt through the existing Windows DirectShow/EVR path, after the old graph and workers have exited. It preserves file/time-shift position, pause state, volume, audio mode and selected aspect ratio. Live TV retunes the same service. The recording writer lives outside the playback recovery function and stays running during this retry. A second error stops playback and saves the recording through the existing writer cleanup; it does not loop or reuse the lost device. Original diagnostics remain in the session folder; fallback diagnostics use its recovery-windows child folder.

The fallback still uses Windows graphics services and is not guaranteed to survive a system-wide GPU failure. [Khronos device-loss requirements](https://docs.vulkan.org/spec/latest/chapters/devsandqueues.html) distinguish logical device recovery from physical/system-level failures. No driver binary, timeout registry value, or system installation was changed. The original GPU hang remains unproven and requires controlled hardware validation after these software fixes.

Validation: 52 CPU tests passed, two hardware GPU tests remain ignored. New fault-injection cases cover one bounded retry, subsequent channel selection bypassing Vulkan, cancellation before/during recovery, no retry for ordinary corrupt-stream errors, and blocked frame delivery being released without overwriting the primary error. No Vulkan or tuner playback was run for this revision.
