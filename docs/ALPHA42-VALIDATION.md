# Alpha.42 validation

The maximize button posts the native system maximize/restore command after releasing the application state borrow. Windows therefore receives its sizing messages and preserves its normal placement. The button draws a restore symbol while maximized; the Minimize button retains taskbar minimization.

Stream resize messages now update only stream layout. Child positions are committed together, minimized windows skip layout, and GDI draws the fixed chassis artwork into one size-specific cache shared by controls. Hidden video pixels are excluded. No decoder, shader selection, GPU recovery policy, or adapter changes were made in this release.

## Focused checks

- Four aspect tests: persistence, manual geometry, Auto metadata and dimension fallback, existing SD aperture behavior.
- Two resize tests, including 16:10 and 5:4 across 100%, 125%, 150%, and 200% DPI.
- Two window/artwork tests: repeated native maximize/restore at two placements, minimize from maximized then restore, and pixel comparisons spanning every artwork band. The native state test creates and destroys its own temporary window.

## Exact packaged-player playback checks

The staged release player was tested with a 1080i H.264 TS fixture and muted audio. Each run performed 180 back-and-forth window resizes during playback, saved a nonblack video snapshot, and stopped successfully. DirectX runs used contrast 110% and HDR effect enabled to exercise actual shaders. Vulkan used the 5:4 override; Microsoft/DirectX 12 used 16:10; EVR/DirectX 11 used Auto.

| Path | Resizes | Mean layout + chrome (ms) | p95 (ms) | Maximum (ms) | Renderer drops |
|---|---:|---:|---:|---:|---:|
| vulkan | 180 | 1.84 | 2.20 | 23.18 | 0 |
| dx12 | 180 | 1.93 | 2.33 | 21.69 | 0 |
| evr-dx11 | 180 | 1.99 | 2.39 | 22.04 | 0 |

Timings include actual stream layout, child positioning, and GDI chassis painting in an app-owned hidden-window diagnostic. They are not a manual desktop dragging or end-to-end frame-latency measurement. Occasional maximum outliers are retained above. All saved video images had substantial pixel variation; Vulkan and EVR renderer counters reported no dropped frames. The Vulkan output rectangles were 1450x1160 (5:4) and 1856x1160 (16:10).

No live broadcast carrying 16:10 or 5:4 metadata was available. Auto detection was covered with synthetic metadata, while ordinary 16:9 file metadata exercised the EVR refresh path. Actual mid-stream broadcast aspect switches were not reproduced.

Build: installer/build.ps1, locked release workspace build, 32-bit adapter build, and Inno Setup 6. No installation or tuner/driver changes were performed.
