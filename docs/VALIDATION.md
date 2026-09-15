# Release validation, 0.5.0

Windows 11 x64, original A865R IT9175 9175:8301, AMD Radeon RX 7900 XTX. Rust 1.92 / MSVC, locked dependencies, Inno Setup 6.7.3.

| Check | Result |
|---|---|
| Rust workspace | 41 tests passed: API bounds/defaults, custom scan validation, firmware layout/dispatch, protocol errors/chunking, TS reassembly/CRC, service selection and GUI export behavior |
| Python parser | 9 tests passed |
| Rust playback API example | Compiled successfully |
| Native GUI / CLI | Release build passed; GUI controls visually inspected |
| Reference reception | RF22 locked; 10-second TS capture independently identified and decoded as TV GAZETA HD 1920×1080 |
| Local UHF scan | 18 frequencies reported MPEG lock; six-second timeout used in released GUI |
| Open LINK + original OFDM | Both processors responded; RF22 locked and five seconds recorded |
| Fully open 0.1.2.0 | Both cores responded, forwarded reads and scheduler progress passed; calibration passed; RF22 did not lock |
| TS diagnostics | Real recording's previously missing multi-packet HD PMT is now parsed, with PCR and elementary streams |
| Vulkan progressive 60 fps | Eight-second synthetic 1080p60 source: all 480 frames processed at 3840×2160/60, session finished normally; no native 60 fps RF broadcast tested |
| Vulkan double-rate deinterlacing | Actual interlaced recording: 553 progressive output frames at 3840×2160/59.94; isolated processing approximately 134 fps / 2.48x real time |
| Live Vulkan | Original HD service at 3840×2160/59.94: 36,124,200 TS bytes, zero playback-queue drops and zero TEI packets in the final 30-second session; six continuity discontinuities remain recorded |
| CPU fallback | Full HD interlaced recording completed at 4K/59.94 with 554 output frames using multithreaded software processing; CPU performance remains host-dependent |
| Installer | Per-user installation, binary hash, HKCU registration, user-local exports, rendered GUI, uninstall and export preservation all passed. All-users option configured, not executed |

These are development tests, not a guarantee of loss-free long-duration reception, suspend/resume reliability, every GPU/codec combination, worldwide standards or full open-firmware compatibility. Initial Vulkan decoding with many host threads stalled; the final Vulkan pipeline uses one host decode-submission worker while GPU shaders/queues remain parallel. A limited live queue counts dropped bytes rather than allocating unbounded memory. Initial small-buffer tests lost startup data; the final queue is bounded to about 14.7 MB. Startup service probing selects the highest-resolution program and its audio independently of stream arrival order.

The native player window uses Vulkan in GPU mode. Source metadata, chosen service, pipeline settings, hardware probes, decoder errors, queue drops and session outcomes are exported. A successful process exit alone is not evidence of a 60 Hz physical display refresh rate. Saved broadcast recordings are not upscaled or re-encoded.

No original-driver USBPcap capture was available. Synthetic USBPcap fixtures are format tests, not measured tuner traffic. No EEPROM or global USB power policy was changed. Ordinary software USB port cycling did not clear tuner firmware RAM; physical disconnect/reconnect was used for cold-image tests, and reference firmware was restored afterward.
