# Broadcast metadata parser provenance

Source: https://github.com/KhronosGroup/Vulkan-Video-Samples
Commit: 39cbc957a04424421b90ec753c576284182a75bc
License: Apache-2.0; upstream copyright/license headers retained.

Only the CPU H.264 parser, base classes, CPU start-code scanning implementations, and required headers are built. The factory was trimmed to H.264. No upstream GPU decoder or renderer is compiled here. parser.cpp supplies bounded host-memory buffers and synchronous callbacks; src/lib.rs wraps them in lifetime-bound views. The GPU calls reside in the separate gpu-video library.

Vulkan C/StdVideo headers: Vulkan SDK 1.4.341.1, headers/vulkan and headers/vk_video. Original SPDX notices retained; SDK license collection is included as HEADER-LICENSES.txt.

examples/inspect.rs checks an Annex-B file entirely on the CPU. It has no GPU creation, submission, or dynamic graphics-library loading calls.
