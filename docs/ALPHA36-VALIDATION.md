# Alpha.36 — hardware-tested recording stability

Alpha.36 preserves alpha.35 and fixes a decoder error reproduced during hardware validation.

## Reproduced issue and fix
The Microsoft decoder recording path in alpha.35 returned 0x80040204 (VFW_E_ALREADY_CONNECTED) when the decoder renegotiated its output media type. Alpha.35 shut that failed playback down cleanly, stopped the recording writer, and released its large graphics allocations. No GPU fault was observed during or after this reproduction.

The terminal input now accepts a supported dynamic media type from its existing peer. Different-peer reconnects and reconnects to passthrough transforms remain rejected, before they can mutate the active format. The owned media type is validated before configuration. A native COM regression exercises accepted format growth and verifies that rejected requests preserve the previous connection and format.

This follows Microsoft's [ReceiveConnection format-change mechanism](https://learn.microsoft.com/en-us/windows/win32/directshow/receiveconnection). Fixing this reproduced application error does not establish that it was the sole cause of the original AMD watchdog event.

## Hardware results on AMD Radeon RX 7900 XTX
Tests launched the packaged player build from an isolated settings directory with muted audio. The existing installed application and user settings were not replaced. All runs had independent memory/driver-event monitoring and bounded shutdown deadlines.

| Build | Workload | Result |
|---|---|---|
| alpha.35 | Mobile file playback, snapshot, 15 seconds | Passed |
| alpha.35 | HD file playback, snapshot, 20 seconds | Passed |
| alpha.35 | Live mobile recording with Vulkan decoding | Passed |
| alpha.35 | Original Microsoft decoder recording path | Reproduced connection error; clean shutdown, no GPU fault |
| alpha.36 | Same original recording path, including snapshot | Passed; video decoded and displayed |
| alpha.36 | Same recording path, 120-second soak plus startup | Passed; steady memory, file kept growing, clean shutdown |
| alpha.36 | HD pause, seek, resume, snapshot and shutdown | Passed |
| alpha.36 | 10-second capture-only recording | Passed; no decoder or renderer, no GPU allocation observed |

The two-minute run held private memory around 549 MiB after warmup and dedicated GPU memory around 640 MiB. The short capture-only run used approximately 2.6 MiB peak private memory. Test-process GPU memory-counter instances disappeared after exit. No new LiveKernelEvent or display-driver reset was observed, including delayed checks after the recording tests. No test required forced termination.

## Automated and artifact validation
- 103 active tests passed: 92 player tests and 11 BDA adapter tests. Two dedicated GPU image-fixture tests remain ignored; the actual application hardware runs above were executed separately.
- Workspace release build passed.
- Installer embeds the approved nine icon sizes. Source archive CRC and SHA-256 checks passed.
- The release executable SHA-256 recorded in the hardware report identifies the exact tested binary.

See ALPHA36-HARDWARE.json for per-run durations, frame counts, memory peaks, recording sizes and executable hashes. These tests cover short operation and a two-minute soak, not indefinite stability. The original kernel dump was inaccessible to the debugger, so the precise AMD driver hang cause remains unproven.
