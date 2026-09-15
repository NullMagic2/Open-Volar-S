# Alpha 22 validation

## Changes

- Vulkan GPU hardware linear texture filtering replaces the 16-tap manual bicubic luma scaler. Progressive video samples the NV12 luma plane directly; interlaced video reconstructs luma once at native source resolution into a filterable R16Float texture. Reconstruction precision exceeds that of the 8-bit broadcast. Scaling may look slightly softer than bicubic.
- Original motion-adaptive spatial/temporal deinterlacing remains. Chroma samples stay within the selected field and visible image boundaries, excluding decoder padding.
- Repeated presentations of the same frame and field reuse the processed image. Frame/epoch/output-size changes invalidate the cached result. All frames remain on the Vulkan device.
- One available-frame lookahead adds no wait and rejects different stream epochs, format changes, reordered timestamps and temporal gaps.
- No decoder, GPU resource ownership, swapchain mode, queue depth or synchronization changes are shipped.

## Verification

- `cargo test --workspace --offline`: 101 CPU/unit/protocol tests passed; two GPU execution tests remain intentionally ignored after earlier reported driver instability. Includes shader validation, lookahead discontinuities, flush/pause recovery and aspect-ratio geometry.
- Normal bounded RBI HD recorded-playback sessions exercised 1920x1080 interlaced input, smooth field-rate deinterlacing and 3840x2160 output. Vulkan H.264 decoding remained active, with no CPU NV12 bridge or per-frame readback. Sessions exited cleanly with no renderer error file. Player-generated snapshot inspected: visible picture, no padding fringe or corruption.
- In comparable background sessions, source-resolution deinterlacing with manual bicubic scaling averaged about 3.5% on the process 3D GPU engine; hardware filtering averaged about 2.7%. These sampled counters are indicative and are not a controlled estimate of total GPU cost or a guarantee for every channel. Decode-engine work is separate.
- A valid foreground hardware-filtering test still averaged about 41.74 ms per presentation (approximately 24 fps) over seconds 10–20. CPU submission averaged about 1.16 ms; acquisition about 40.40 ms. Thus reduced shader cost did not resolve the fullscreen presentation stall.
- An AMD-specific acquisition-wait workaround suggested by upstream issue 9559 was tested in a separate bounded session, including 70 confirmed foreground samples over seconds 15–50. It remained at 41.74 ms. The ineffective workaround was removed; the release uses unmodified crates.io wgpu-hal 29.0.4. Its source is not included in the release tree.

## Limitations

Sustained foreground fullscreen slowdown remains unresolved. Do not describe this release as fixing fullscreen smoothness or end-to-end latency. Background tests that lost foreground focus are excluded from such a claim. No lookahead delay, interpolated synthetic frames or deeper buffering was added. Earlier system-wide graphics-driver instability is not established as permanently resolved. Tests used normal recorded playback, without stress tests or driver/system configuration changes.

## References and evidence

[Khronos texture filtering](https://docs.vulkan.org/spec/latest/chapters/textures.html) documents linear sampling. [wgpu issue 9559](https://github.com/gfx-rs/wgpu/issues/9559) describes an acquisition regression on other AMD hardware; the attempted workaround did not solve this local case.

Local evidence under `work/latency-alpha22`: `optimized-foreground`, `hardware-foreground`, `hardware-focus-foreground` and `amd-userfocus-foreground`, with timing samples and per-session presentation traces. The latter workaround experiment is not the shipped configuration.
