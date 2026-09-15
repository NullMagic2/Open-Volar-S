# Open firmware 0.1.4.0 — reception and standby results

On 2026-09-10, the source-built firmware received RF22 (521143 kHz, 6 MHz ISDB-Tb) on the user's original A865R in Vila Velha, Brazil. Both processors ran open 0.1.4.0. WinUSB remained the Windows function driver. No vendor executable firmware core was used in the open-image tests.

## Changes

The missing ROM replacements are implemented and registered: `7353 -> signed-search` (reference `4B1B`), `30F6 -> layer-parameters` (`4AF1`), and `DC0F -> quality` (`4954`). Earlier descriptions that swapped the signed-search and layer-parameter roles were incorrect. All six startup patch-map entries are now populated. The original nine reconstructed services and complete downloaded Timer-2/serial callbacks remain implemented.

The complete 230-byte ROM ABI/startup data region at `4700..47E5` is now generated from numeric coefficients, mode tables, interrupt addresses and quality curves. Its bytes match the user-supplied reference. The provenance is reverse engineering, not a clean-room claim. The source contains reconstructed numeric data and source-authored machine instructions; it bundles no vendor executable payload or immutable ROM dump. Unidentified coefficient units are not guessed.

Conditional-branch relaxation, shared correction logic, constant-store reuse and compact threshold-table lookup keep the image inside the established RAM window. The scatter image is **5,640 bytes, 109 records**. Builds need no proprietary file.

The receiver now accepts this exact open version through its normal API. The Rust app uses built-in open firmware when the firmware path is empty, and also accepts a matching selected reference image. The BDA backend has the same default for cold devices; this change alone does not establish AVerTV application compatibility.

Explicit `Receiver::suspend()` / `resume()` implement the recfsusb2i RF/demodulator standby sequence and complete reinitialization/calibration. Suspend waits for acknowledgement before switching tuner power off, and fails closed on timeout. This is device standby, **not Windows system sleep or USB D3**. It retains RAM and the LINK command processor. `stop()` remains a stream stop; it preserves suspended/faulted state.

`Receiver::signal_quality()` reads the public `800049` register, validates its 0–100 range and checks MPEG lock before and after reading. `80446B` is internal working state and was an incorrect host-facing source. The observed score started at zero during settling and reached 100 after five seconds. The score reproduces the reference firmware's relative measure; independent RF-level calibration is not established.

## Hardware evidence

| Test | Result |
|---|---|
| Original firmware control | RF22 locked in 1,548 ms; 6,995,480 bytes in five seconds. A warm repeat also received. |
| Reference standby control | Three captures with two acknowledged standby/resume cycles; each capture 4,415,692 bytes. |
| Open cold boot | LINK and OFDM 0.1.4.0; scheduler 100 → 121. |
| Open initial tune | Lock in 3,779 ms; 4,357,840 bytes in three seconds. |
| Open standby/resume | Two acknowledged cycles; subsequent locks in 1,568 and 1,537 ms, with 4,415,692 and 4,358,352 bytes. |
| Open sustained recording | 30 seconds, 43,349,552 bytes; 229,164 recognized TS packets. |
| Normal reception API | Lock in 1,582 ms; five-second recording; public quality score 100 after capture. |
| Updated app live playback | Successful Vulkan path and 4K processing; 14,392,340 streamed bytes, zero application queue drops, zero transport-error packets, three continuity errors. |

FFprobe identified the full service as 1920×1080 top-field-first H.264, 30000/1001 frames/s, with AAC LATM audio. FFmpeg decoded the sustained capture. Its logs contain startup/PES/decode warnings; successful exit does not mean a pristine stream. The sustained capture had six synchronization losses, 34 continuity errors and four transport-error packets. Reference captures also contained errors. The live application's service-probe stage preceded its cleaner measured stream interval; those intervals are not directly equivalent.

Raw logs and JSON are in `debug/exports/firmware-rf-integration/`. The `.ts` recordings are retained locally and excluded from the distributable package. `open-0.1.4.0-live/result.json` reports the app run; GPU settings/logs record Vulkan decoding, double-rate deinterlacing and 3840×2160 output configuration. No visual inspection or frame-pacing benchmark is implied by these logs.

## Offline checks

The differential executor passed **6,883** cases against the specified reference hash, comparing XDATA, ordered MMIO writes, declared ROM call boundaries and selected control bits. It covers all decoded instructions in the earlier services and new quality/layer patches, and 417 of 418 in signed search. The uncovered instruction is the final not-ready path at `4D8E`, which leads into a wait for asynchronous completion; asynchronous interrupt behavior is not emulated.

Separate candidate-only cases verify bounded exit, cleanup and stack balance for stuck acquisition readiness and all three signed-search waits. Ten executor self-tests cover arithmetic, memory separation, call/stack behavior, code lookup, multiplication and carry/borrow. Rust tests check image layout, normal API capabilities, suspend timeout handling and public quality-register selection. The unavailable mask ROM is represented by declared deterministic oracles, not reconstructed or simulated in full.

## Reproduction

```text
a865rctl build-open-fw-probe open-0.1.4.0.fw
a865rctl probe-open-fw
a865rctl receive 521143 5 new-recording.ts
a865rctl test-lifecycle 521143 new-lifecycle-prefix
a865r-debug --watch 521143 --play-seconds 15 --play-export new-playback-folder
```

Only upload requires a physically cold tuner. There is still no verified software replacement for unplug/reconnect when changing RAM firmware. No EEPROM writes, global USB power-policy changes or vendor driver restoration are part of this work.

Reception is now demonstrated on this board/channel. Other frequencies, weak-signal behavior, long-duration operation, Windows suspend/hibernate and end-to-end AVerTV playback remain separate validation work. The firmware is an open RAM patch/startup layer using the chip's existing immutable ROM, not a replacement for that ROM.

## Expanded UHF and Windows S3 tests

The subsequent RF14�RF51 scan covered all 38 supported Brazilian UHF centers. MPEG lock and a three-second transport recording succeeded on **19 channels**: RF16, 18, 19, 20, 22, 25, 29, 30, 31, 33, 35, 38, 40, 41, 42, 43, 45, 49 and 50. This spans 485143�689143 kHz. The per-channel byte counts, transport errors and settled quality scores are exported in `channel-coverage.json`. Unlocked frequencies may be empty, weak, or otherwise unavailable locally; the scan is not a comparison against a nationwide station inventory.

Windows reported S3 support. Two real S3 transitions were verified with Kernel-Power events 42 and 107: an idle-tuner test and a live Vulkan playback test. Each sleep interval between those events was about four seconds. The helper's calls returned after 13.6 and 16.2 seconds including transition overhead. Both wake records identify the fixed power button, before the requested timer due time. Temporary wake timers were successfully armed and then cleaned up; timer-driven wake was **not** demonstrated. No persistent power settings were changed.

After idle S3, both open processors retained 0.1.4.0 and the scheduler advanced. All 19 channel recordings followed that wake. During active S3, the existing playback process continued: decoded-frame log count increased from 1189 before sleep to 1375 after wake. Its run reported 63,418,040 transport bytes, no application queue drops, no transport-error packets and four continuity errors. This used the pre-color-management FFplay/Vulkan presenter. The later ICC-aware presenter requires its own S3 validation if claimed. Hibernate, longer S3 intervals and other machines remain untested.

The helper follows Microsoft's [SetSuspendState](https://learn.microsoft.com/en-us/windows/win32/api/powrprof/nf-powrprof-setsuspendstate) and [SetWaitableTimer](https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-setwaitabletimer) contracts. It enables only the current process's sleep privilege and restores it on exit.
