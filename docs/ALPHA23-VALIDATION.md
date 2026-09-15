# Alpha 23 validation — 2026-09-11

Alpha 23 adds three application-owned GPU picture slots, separates preparation from presentation, and retains Vulkan Video decoding and GPU rendering on the same device. Only metadata crosses the worker handoff. Requested snapshots use a separate worker; ordinary playback has no CPU frame readback.

The fullscreen video surface now has a one-physical-pixel black inset. The outer window remains fullscreen, taskbar handling is retained, picture opacity is unchanged, and the source aspect ratio is fitted inside the inset. The pointer hides after three seconds without movement and returns on movement, focus changes, or fullscreen exit.

## Measured fullscreen results

Windows 11, AMD Radeon RX 7900 XTX, 3840×2160 display near 59.94 Hz. The same recorded RBI HD 1920×1080 interlaced broadcast and 4K processing selection were used. Each run lasted 70 seconds, with fullscreen from approximately 5 to 65 seconds. Statistics below use seconds 15–60, after startup and past the reported 10–20-second slowdown period.

| Configuration | Prepared fields/s | Presentations/s | ETW mean interval | Mean acquisition wait |
|---|---:|---:|---:|---:|
| Split workers, full monitor-sized surface, run 1 | 59.93 | 23.96 | 41.74 ms | 40.58 ms |
| Split workers, full monitor-sized surface, run 2 | 59.93 | 23.96 | 41.74 ms | 40.70 ms |
| One-pixel inset experiment, run 1 | 59.93 | 59.89 | 16.70 ms | 0.55 ms |
| One-pixel inset experiment, run 2 | 59.93 | 59.91 | 16.70 ms | 0.55 ms |
| Integrated inset and cursor auto-hide | 59.93 | 59.80 | 16.72 ms | 0.60 ms |

Each analyzed interval had 90/90 foreground samples and 90/90 samples with five video points unoccluded. The integrated run had a p95 presentation interval of 18.05 ms and average prepared-picture age of 23.74 ms. Preparation submission averaged 0.37 ms. No renderer errors were recorded. This measures presentation cadence, not physical photon latency.

ETW still reported Hardware Composed: Independent Flip for most presentations. Therefore this is not evidence that the inset forces ordinary windowed composition. The tested combination is a slightly smaller video surface plus the existing display-vblank pacing path. The monitor-sized path uses FIFO pacing alone. No display refresh setting, system compatibility setting, or driver was changed.

## Buffer synchronization

Three reusable GPU textures are protected by CPU leases and completion callbacks. Queued, displayed and GPU-in-flight references prevent reuse. The ready handoff has at most two descriptors, discards stale unclaimed pictures, and chooses a suitable field using the audio clock after acquisition completes. Preparation lead is bounded to one source-frame interval, rather than seconds of extra live-TV delay.

The workspace-local wgpu-hal 29.0.4 patch yields between short acquisition checks because wgpu-core otherwise holds a shared submission lock during the display wait. It retains the same pending image, acquisition fence and semaphore until readiness is established. It does not remove synchronization. See ../third-party/wgpu-hal/A865R-PATCH.md and its exact patch.

During seek/channel cancellation, an already acquired surface image receives a clear submission and presentation so its acquisition semaphore is consumed. Resource cleanup still honors GPU and external-acquisition completion and can wait on the graphics driver.

## Functional checks

- 106 CPU, protocol and shader tests passed; the two previously disabled hardware tests remained ignored.
- New pool tests cover in-flight ownership, resize generations, timestamp selection, bounded replacement and stale-channel invalidation.
- Release build succeeded.
- Recorded-HD checks exercised pause/resume, minimize/restore, resizing, seeking to 40% and 90%, natural end-of-recording, snapshot capture and normal shutdown.
- The end-of-recording check reached the recording duration of 89.1667 seconds and returned to Ready.
- A 55-second live-tuner check exercised pause/resume, channel up, channel down, Stop and Play. Four playback sessions produced frames and exited without renderer-error files.
- The built-in snapshot was inspected and showed a valid picture.
- Cursor instrumentation confirmed a visible cursor after movement, a null/hidden cursor after three seconds, reappearance after further movement, and restoration on fullscreen exit.
- The user reported that the final picture looked good and requested packaging.

## Scope and remaining limits

These are bounded tests on one GPU/driver/display setup, not a claim of universal driver stability. The original system-wide driver-crash reports are not established as universally resolved. No stress fixtures were run. Surround audio, ICC, recording and caption features are retained but were not all independently requalified in this release.

Evidence is under work/buffer-alpha23: alpha23-fullscreen-1/2, alpha23-inset-1/2, alpha23-final-fullscreen, alpha23-lifecycle and alpha23-live. Fullscreen directories contain process-filtered PresentMon records, preparation/presentation traces, focus/visibility samples and summaries. Raw broadcast recordings and private settings are excluded from the distributed source archive.
