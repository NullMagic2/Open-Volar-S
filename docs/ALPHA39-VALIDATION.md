# Alpha.39 validation

## Changes

Vulkan remains the default decoder and renderer. A reported device loss releases the previous playback attempt and recreates Vulkan once. Failure of the retry stops playback with an error. There is no automatic Microsoft-codec fallback and no process-wide Vulkan lockout; a later Play command can try Vulkan again. Explicit software diagnostic flags remain opt-in.

The custom per-frame stopwatch is removed. A maximum of three retained submissions bounds ownership; a full queue uses the graphics API completion wait with a two-second timeout. Memory errors, wait timeouts and unrelated validation errors are no longer classified as device loss. Independent intra-picture startup is restored. Parsed IDR metadata, zeroed bitstream padding, reference validation and lost-device cleanup remain.

## Regression tests

- Player release suite: 97 passed, zero failed, two existing GPU fixture tests ignored.
- Broadcast parser suite: two passed.
- Coverage includes pending GPU ownership, completion polling before a stall decision, one Vulkan retry after device loss, failure reporting without codec fallback or process lockout, cancellation, explicit diagnostic flags, and independent-picture startup.

## Hardware playback after AMD driver reinstall

Tests ran sequentially, with isolated settings, on the same local machine. No UI regression tests ran concurrently with these playback comparisons.

| Test | Result |
| --- | --- |
| Preserved alpha.36, 15-second HD file test | 1920 x 1080; Vulkan H.264; 439 received, 809 presented, zero reported dropped frames; clean shutdown |
| Alpha.39, 20-second same HD file test | 1920 x 1080; Vulkan H.264; 605 received, 1,110 presented, zero reported dropped frames; clean shutdown |
| Alpha.39 pause / seek / resume | Paused state confirmed; position moved backward while paused; playback resumed and advanced; all five verification stages produced; clean shutdown |
| Alpha.39, 120-second live TV GAZETA HD test | 1920 x 1080; Vulkan H.264; 3,453 received, 6,894 presented, zero reported dropped frames; clean shutdown |

Final live presentation timing sample: mean 16.69 ms, p95 18.13 ms across the last 60 intervals. The live decoder reported a shared Vulkan device, no CPU NV12 bridge, and no per-frame GPU readback. No error files or graphics recovery were reported in these successful runs. Startup time is included in test duration; these counters are application diagnostics, not a claim that every broadcast packet was lossless.

The preserved alpha.36 binary SHA-256 was 950f9da458ca34c477fecb5d1ae9990cb0aa7e86c765354ef958f858cc04492d. Its successful playback after the driver reinstall supports the earlier driver-state diagnosis. This does not prove that every previous artifact was caused solely by the driver.

A real GPU reset was not deliberately induced. Retry and failure behavior were verified using controlled regression-test errors, rather than destabilizing the restored AMD driver. Caption mappings are unchanged by this release.

## Release verification

The final release binary and installer were built successfully. Packaging verifies ZIP CRC, every source-manifest hash, copied installer identity, and equality of the staged and release player binaries. SHA-256 checksums accompany the source ZIP and installer. This release is packaged, not installed automatically.
