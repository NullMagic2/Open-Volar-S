# A865R wgpu-hal acquisition patch

Based on the crates.io wgpu-hal 29.0.4 package. Original crate SHA-256:
97ace1c17727311c22a46e4e3faf56ea6de81af99dcc839bdfb54857b94d448d

Upstream MIT/Apache-2.0 licenses remain in this directory. The only upstream source modification is src/vulkan/swapchain/native.rs; A865R-acquisition.patch records it. Cargo's patch entry is workspace-local; the registry installation is unchanged.

On Windows, each acquisition readiness check waits at most 1 ms in each Vulkan wait call. wgpu-core holds the shared device submission-fence read lock during acquisition, so returning Timeout lets the preparation worker submit between checks.

A successful vkAcquireNextImageKHR result remains pending until its original acquisition fence signals. Retries keep the same image, semaphore index and fence; they never reacquire it. The fence is reset and the semaphore index advanced only after successful completion. The original binary acquire/present semaphore dependencies remain unchanged. The preceding submission fence must also report completion before its acquire semaphore is reused.

Resize/close waits for a pending external acquisition before destroying its synchronization objects. This cleanup may still wait on the graphics driver, just as the upstream device-idle cleanup does. No GPU wait or resource-safety requirement has been removed.

The player sleeps between retries and skips a second vblank wait during the same acquisition. It records total elapsed acquisition time across retries, not just the final successful call. Acquired images invalidated by a channel change receive a clear submission and presentation, because upstream Vulkan discard_texture does not consume the acquisition semaphore.

This patch releases the shared submission lock while the image is unavailable. It does not promise that Windows/AMD will release display images at the desired rate.

References:
- https://github.com/gfx-rs/wgpu/issues/8310
- https://docs.vulkan.org/refpages/latest/refpages/source/vkAcquireNextImageKHR.html
