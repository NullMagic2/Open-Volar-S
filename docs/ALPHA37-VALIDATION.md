# Alpha.37 — unified release

This release combines the alpha.36 checkout from the second working window with this window's caption, recording, recovery, diagnostic-player and AverTV work. A file-by-file comparison found 1,309 identical baseline files and no missing baseline files before release documentation and version updates. Overlapping source changes were reviewed instead of replacing the newer files wholesale.

## Included alpha.36 work

Shared dropdown arrow, thumb, capture and wheel handling; centered Latin captions; Warm with Portuguese, Spanish and Greek translations; highlight-preserving Cold/Warm color adjustments; automatic settings saving with Apply removed; recording-worker cleanup; bounded frame history/GPU submissions; and the Microsoft decoder's supported same-peer media-type reconnect. The alpha.36 hardware report remains included and identifies its own tested binary. Those hardware results must not be attributed to this new binary.

## New fixes

The caption overlay used a child layered window with desktop coordinates. The native regression reproduced an extra parent-window offset, including when a window was moved. Captions now belong to the video surface and receive parent-relative coordinates; viewport changes invalidate the rendered overlay. A separate C++ regression reproduced stale DirectWrite font metrics after changing font families. The font caches are now reset when the family changes.

The observed `Parent device is lost` error now enters bounded recovery. Non-invasive stacks from the stalled installed application also showed a wait inside AMD Vulkan device destruction. Lost-device destruction is retired on a separate cleanup thread while retaining the Vulkan instance/loader; the playback owner does not wait for a stuck destructor. The existing once-per-session Vulkan failure latch remains. The blocked-cleanup regression passed. No new GPU reset was deliberately induced to validate that failure path.

## TS recording validation

The supplied TV GAZETA HD recording contained real transport/video/audio corruption. Finalization keeps the original `.broadcast.ts`, remuxes the selected service, and performs full CPU-only decoding. If that fails, software H.264/AAC repair is followed by another strict decode before publication as `.ts`. Healthy video is not re-encoded. Captions retain their PID and ISDB component descriptors; a remuxer's generic `bin_data` label no longer makes captions undiscoverable. The supplied file passed the software-repair path with four audio tracks and a recognized ARIB caption stream.

The external FFmpeg installation is required; it is not bundled. Missing tools, cancellation, existing output paths and failed verification must not publish an unverified normal recording. Conversion takes time and keeps both raw and finalized files. Re-encoding cannot reconstruct lost broadcast content, and software decoding success cannot guarantee that every third-party GPU driver is free of defects.

## AverTV signal presets

General settings persist `Original`, `Improved`, `2K improved (upscaled)` and `4K improved (upscaled)` in the current user's compatibility profile. Both BDA architectures read the same profile on the next tune. Original bypasses the processor. Processed modes launch a hidden CPU FFmpeg worker per TV service, owned by the adapter, not the player window. Independent service clock domains are recombined at the final transport boundary; they are not compared in one FFmpeg interleaver. Closing/releasing AverTV stops it; a Windows Job Object handles abrupt owner termination. The native player and raw recorder do not inherit this external-client preset.

Improved applies conservative deinterlacing, contrast/saturation and sharpening at source size. 2K is 2560×1440; 4K is 3840×2160. Aspect ratio is retained with padding. Upscaling cannot recover missing source detail. The stream keeps the original service IDs, elementary PIDs, audio timestamps and broadcast descriptors/guide data, with updated PCR references and valid table CRCs. Reserved mux table IDs avoid collisions with original elementary PIDs. A source-cadence filter removes repeated mobile-service timestamps while passthrough synchronization retains negative/unwrapped broadcast timestamps. Each announced video service has its own output watchdog. Queues are bounded; a stalled/overloaded processor reports an error instead of silently changing presets.

The roughly 51-second HD/mobile multiplex was processed without tuner or GPU access. Final CPU measurements were 14.226 seconds for 2K and 20.888 seconds for 4K; these are local sample measurements, not performance guarantees. Both outputs decoded 1,527 HD frames and 765 mobile frames without decoding errors, at the requested dimensions for both services. A separate 33-bit timestamp-wrap fixture decoded 1,528 HD frames and 765 mobile frames without errors. Original was verified byte-for-byte and remains the default.

**The new processed feed has not been rendered end-to-end in the separate AverTV application.** In particular, its legacy decoder's acceptance of 4K H.264 remains unverified. The existing unprocessed BDA/AverTV path was validated in earlier releases. The new installer includes x86/x64 adapters and an existing-installation updater; close AverTV first and use the installation's matching administrator scope. The updater verifies staged hashes, backs up old DLLs and registration values, pauses/resumes AverRemote when needed, and rolls back modified files/registrations on failure. It does not alter the USB binding or flash firmware. No installed adapters were replaced during these tests.

## Smaller diagnostic package

Debug Desk reuses the native Live TV executable with an isolated settings directory, a stop request and a bounded exit deadline. MPV is no longer a playback dependency or installer payload. Upgrade cleanup removes the old mpv executable, its legacy compiler DLL and the old duplicate player executable. FFmpeg remains an external probe/recording-processing dependency. Native diagnostic playback still uses a graphics renderer; CPU-only recording verification is a separate headless path.

## Validation and limitations

The release includes the player, BDA transport and diagnostic unit tests, the real C++ font regression, and the exact lost-device retirement regression. Two existing GPU fixture tests remain explicitly ignored. See the accompanying release verification JSON for current counts, hashes and artifact checks. The new graphics recovery path, native diagnostic child lifecycle and processed AverTV rendering have not received another live GPU stress test. Source/installer version resources are unified as 0.8.0-alpha.37 / 0.8.0.37.
