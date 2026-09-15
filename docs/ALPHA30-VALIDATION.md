# Live TV! 0.8.0-alpha.30

Settings > Picture now includes **HDR effect**, with English, Brazilian Portuguese, Spanish and Greek translations. It is off by default. Turn it on to apply subtle contrast and highlight shaping to the video without enabling Windows HDR. Preset changes retain the effect setting, which is saved across restarts; Reset defaults clears it.

The effect uses a fixed monotonic luma curve in the existing Vulkan picture shader, before the monitor ICC transform. Black, midpoint gray and white are preserved. Maximum luma-signal displacement is approximately 1.54 percentage points. Chroma direction is retained, with compression toward gray only when needed to stay inside SDR gamut. There is no extra render pass, texture lookup, readback, frame history or adaptive exposure analysis. The effect is a restrained SDR enhancement; it does not reconstruct missing HDR information or extend display brightness. The separate Video HDR output setting remains unchanged.

Validation:

- 74 player tests passed, including WGSL parsing/validation, backward-compatible settings loading, effect persistence, monotonic curve bounds, and paused-frame repaint without queue flushing. Two existing hardware GPU tests remain intentionally ignored.
- Native preview confirmed toggle on/off, independence from Video HDR output, preset preservation, Reset defaults, restart persistence, and layout in all four languages. English and Greek screenshots were visually reviewed.
- A CPU reference evaluated 2,146,689 RGB samples: finite SDR-gamut results, preserved black/white, no chroma amplification, maximum luma-signal shift 0.015396. This numerical check is not a GPU rendering benchmark.
- Release installer and source archive were built; delivery packaging verifies ZIP CRC, source manifest hashes, installer copy and staged player identity.

Hardware playback appearance and GPU performance have not been measured for this release. Windows recovery output remains neutral, as with the other picture adjustments. The effect does not modify recordings; it is applied to the displayed picture and the processed snapshot path.

Includes the alpha.29 DAC Live-button removal and previous interface improvements. Driver and Debug Desk remain 0.7.0.
