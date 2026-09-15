# Alpha 24 validation — 2026-09-11

Alpha 24 reduces avoidable CPU overhead without changing the decoder algorithm, GPU synchronization, picture queue depth, display mode or fullscreen pacing.

## Changes

- Cache the canvas texture view and ICC LUT view instead of recreating them for each picture.
- Remove duplicate device polling during active GPU submission. Submission already processes completion callbacks; explicit polling remains when idle, paused or waiting for a reusable picture slot.
- Avoid allocating and painting a full-size interface bitmap for the fullscreen video window, and skip invalidation of its hidden status strip.
- Replace per-row interface gradient allocations and brush operations with native GDI GradientFill. Preserve clipping, parent-relative coordinates, buffered controls and a fallback.
- Include worker thread IDs in diagnostic reports to distinguish preparation, presentation and decoding CPU work.

## Measurements

Four bounded 70-second recorded RBI HD runs used 4K output, the same left-channel audio setting and 16 logical CPUs. Foreground and unobscured checks passed throughout their measurement windows.

| Build | Approximate CPU estimate | Mean present interval | 95th percentile |
| --- | ---: | ---: | ---: |
| Alpha 23 baseline 1 | 1.01% | 16.696 ms | 18.024 ms |
| Alpha 23 baseline 2 | 0.99% | 16.697 ms | 18.043 ms |
| Alpha 24 candidate 1 | 0.97% | 16.697 ms | 18.066 ms |
| Alpha 24 candidate 2 | 0.96% | 16.696 ms | 18.024 ms |

CPU estimates sum persistent thread CPU deltas, normalized across the machine; they can omit threads that exit between samples. These are preliminary measurements, not proof of a statistically significant process-wide reduction. The approximately 60-per-second presentation cadence was retained. Present intervals measure submission cadence, not every frame's visible display duration.

The final native-gradient change was added after those fullscreen runs. A bounded 45-second windowed playback check of the final alpha 24 executable completed with approximately 16.75 ms mean present intervals. Inconsistent foreground ownership makes its CPU numbers unsuitable for comparison, so they are excluded.

## Release checks

Final source regression suite: 106 passed, zero failures, two hardware tests ignored (cargo test --workspace --offline --locked). GPU hardware stress tests remain ignored. Release executable and installer are built without launching installation; source archive entries are checked against a SHA-256 manifest and ZIP CRCs.

## Limits and optional follow-up

The savings are modest, and no matched AVerTV comparison has established parity. Smooth playback is the priority; these changes are sufficient for this release. If further CPU reduction is useful, first measure both applications with the same channel, audio mode, output size and foreground state using process-wide CPU counters. Investigate only repeatable hot paths from that comparison.

Results apply to bounded checks on the local RX 7900 XTX/display/driver setup. Alpha 23's fullscreen inset, cursor timeout and pacing remain unchanged; its lifecycle and live-channel checks are historical evidence, not rerun claims for alpha 24.
