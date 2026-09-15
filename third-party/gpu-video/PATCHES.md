# Local changes to gpu-video 0.4.0 (MIT)

Upstream: Software Mansion, https://github.com/software-mansion/smelter (gpu-video 0.4.0 crate).

Added a broadcast-parser adapter supplying native H.264 SPS/PPS, slice offsets, DPB references, timestamps and field metadata. The adapter forbids unsafe Rust. Added new-session initialization, actual-profile capability queries, field-reference accounting, complementary-field handling, IDR/missing-reference guards and queue-family compatibility checks. NV12 output remains on the same Vulkan device as wgpu 29 rendering.

The experimental-broadcast-gpu feature is enabled by the A865R player dependency. It remains off by default for standalone library consumers. The original standalone GPU probe remains disabled outside the shipped source tree. Player hardware tests are ignored by default pending validation after reported driver instability.

Existing Rust parser patches retain L1 weighted-prediction parsing and avoid emitting empty access units. Broadcast playback uses the Khronos-derived parser instead. See ../broadcast-parser/PROVENANCE.md.
