# Multithreaded GPU frame-buffer design

Status: implemented in alpha 23. The subsequent one-pixel fullscreen inset plus display-vblank pacing also passed sustained local checks. See ALPHA23-VALIDATION.md for measurements and hardware limits.

## Objective
Separate GPU image preparation from display-buffer acquisition. Reuse the existing decoder and renderer, add one presentation thread, and connect them with three application-owned GPU image slots.

The presenter can then wait for a display buffer while preparation continues. Windows still controls swapchain image availability. The measured 40 ms wait remains a separate acceptance criterion: faster preparation alone is not a fullscreen fix.

## Threads and ownership

~~~mermaid
flowchart LR
    D[Existing decode worker] -->|Vulkan NV12 images| R[Existing rendering worker]
    R --> B[Three GPU texture slots]
    B --> P[New presentation worker]
    P -->|GPU blit and FIFO present| W[Windows display]
    A[Existing audio clock] --> R
    A --> P
~~~

Keep the same Vulkan device and safe wgpu queue interface. Reuse existing audio and UI workers. No additional graphics backend, CPU pixel bridge, raw GPU queue manipulation, busy polling or general task scheduler is needed.

The rendering worker owns deinterlacing history, source views, ICC transform, processing uniforms and the preparation pipeline. The presentation worker exclusively owns the surface, swapchain configuration, acquisition, viewport blit and present calls. The UI supplies coalesced resize, visibility and fullscreen changes.

## Pool and synchronization
Use three reusable processed-image textures at the current processing resolution and SDR format. Their intended roles are one preparing, one available and one presenting. This is a concurrency limit, not a requirement to fill three frames before playback.

Each prepared descriptor contains a texture-slot lease/generation, stream epoch, frame ID, field index, media timestamp/duration, processing dimensions and settings generation.

A bounded mutex/condition-variable handoff is sufficient. Publish metadata after the preparation command buffer has been submitted. Published means GPU work is queued, not that the CPU has waited for completion. Existing GPU dependencies still protect image accesses.

Slot lifecycle: Free → Rendering → Ready → Presenting → Retiring → Free.
Retire a slot through completion notification from the final referencing GPU submission. A stale frame also retires its rendering work before reuse. Never overwrite a texture while rendering, a blit or a snapshot still references it. Never hold the pool mutex across acquisition, submission, GPU waits or worker joins.

Three RGBA8/BGRA8 images require about 23.7 MiB at 1920×1080 or 94.9 MiB at 3840×2160, in addition to existing decoder/history resources. Reuse allocations.

## Timing and bounded latency
The producer prepares at most one source-frame interval ahead of the media clock and sleeps when the pool is full or playback is paused. It must not process an arbitrarily distant decoded backlog.

After acquisition returns, the presenter reads the audio clock and chooses the newest prepared field suitable for the upcoming display opportunity. Discard stale unclaimed descriptors; repeat the last suitable image when necessary. Retain the current display slot until a replacement is available. If no slot is free, retire an obsolete ready frame rather than overwrite a presenting slot.

Use the existing small GPU viewport/upscale blit and vsync. Only handles and timestamps cross the CPU handoff.

Retain the existing acquired-image host fence. The workspace-local wgpu-hal patch yields between short readiness checks while retaining the same acquisition, so wgpu-core releases its shared submission lock between checks. See ../third-party/wgpu-hal/A865R-PATCH.md. The earlier semaphore-only experiment did not resolve the sustained stall. Any further synchronization experiment must be isolated and retain all Vulkan-required image and semaphore dependencies.

## Minimal source changes
The original combined Gpu::draw path has been split between player/src/vulkan.rs, player/src/vulkan_pipeline.rs and player/src/frame_pool.rs:

1. Extract preparation into a renderer accepting a leased pool target and returning timestamped PreparedFrame metadata after submission.
2. Separate processed texture/bind-group ownership from player/src/canvas.rs Canvas. Reuse its viewport shader and geometry in the presenter.
3. Move surface/configure/acquire/present ownership into the new thread. Share device/queue handles through existing safe abstractions.
4. Connect the pool to existing epoch, audio-clock, pause, error and shutdown handling. Preserve decoder resource ownership and broadcast/audio behavior.
5. Run explicitly requested snapshot readback from a retained frame lease without blocking the presentation worker. Normal playback continues without readback.

Reuse the current GPU processing pass. Do not add a duplicate processing pass.

## Lifecycle
Pause retains the last displayed image and stops preparation. Resume selects from the current clock. Channel changes, seek and flush invalidate descriptors by epoch without changing the existing clock-preservation rules. Old resources retire normally and stale prepared descriptors are rejected before submission. A submission already handed to the GPU cannot be recalled.

Only the presenter reconfigures the swapchain. Coalesce resize requests. If processing dimensions change, retire old pool leases safely; bound allocation to the old and replacement generations instead of allocating for every drag event.

Stop/error closes the handoff and wakes workers. Join both before destroying the HWND/device. Acquisition checks yield; final synchronization cleanup can still wait on the graphics driver.

## Release criteria
CPU tests cover slot ownership, stale-frame replacement, epoch invalidation, timestamp selection, pause/resume, resize generations and cancellation. A simulated in-flight GPU reference must prevent slot reuse.

Validate using ordinary recorded HD/interlaced playback, keeping the two previously ignored GPU tests ignored. Run two separate 60-second foreground-fullscreen comparisons on the same recording, recording focus/occlusion, actual ETW cadence, acquisition durations, prepared-frame age, audio drift and resource usage. Also check fullscreen exit, channel change, pause/resume, snapshot and close.

Only claim a fullscreen fix if sustained display cadence improves beyond the reported 10–20-second failure period without growing latency or resource errors. If acquisition still limits output to approximately 24 fps, report the threading benefit separately and continue the WSI investigation.

## References
- [Original-player investigation](AVERTV-FULLSCREEN-COMPARISON.md)
- [Vulkan image acquisition](https://docs.vulkan.org/refpages/latest/refpages/source/vkAcquireNextImageKHR.html)
- [Vulkan presentation and platform-controlled timing](https://docs.vulkan.org/refpages/latest/refpages/source/vkQueuePresentKHR.html)
