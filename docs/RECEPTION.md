# Reception and playback, 0.5.0

The Windows userspace Rust receiver has tuned and recorded the original A865R / IT9175 9175:8301, EEPROM tuner 0x70, single tuner, USB TS endpoint 0x84. The original LINK 3.0.3.0 / OFDM 3.0.4.5 reference firmware is used for ordinary reception. The GUI loads the user-selected, fingerprint-checked reference file into a cold device's RAM. It does not distribute that file or flash EEPROM.

The supported channel plan is 6 MHz ISDB-T UHF, 470000–697999 kHz. Brazil UHF channels 14–51 use centers 473143–695143 kHz. Manual frequencies and custom start/end/step scans contain no city-specific station list. Compatible broadcasts elsewhere in that range can be attempted; only Vila Velha, ES, Brazil has been tested. Other bands, bandwidths and standards remain unsupported. A country preset cannot add DVB-T, ATSC or other hardware capabilities.

## Hardware evidence

- Reference firmware, 521143 kHz: MPEG lock around 1.6 seconds; 10 seconds recorded, 14,621,700 bytes.
- Independent FFprobe: service 16960, TV GAZETA HD, H.264 1920 × 1080 with AAC audio. A decoded frame is in the development export.
- A 2.5-second-per-frequency exploratory scan found MPEG lock on RF 16, 18, 19, 20, 22, 25, 27, 29, 30, 33, 38, 40, 41, 42, 43, 45, 49 and 50. This is measured local evidence, not a guaranteed complete service lineup. The GUI uses up to six seconds per frequency and exports every outcome.
- Open LINK 0.1.1.0 + original OFDM 3.0.4.5: lock in 1.53 seconds, five-second capture of 7,339,520 bytes. This component test still uses proprietary OFDM firmware.
- Fully open 0.1.2.0: both command processors respond; OFDM scheduler advances; tuner calibration completes; RF22 did not lock in six seconds. Full open-firmware TV reception is not verified.

Transport diagnostics now reassemble PAT/PMT across packets, honor pointer fields, validate MPEG CRC, discard corrupted PSI and reset assembly on continuity gaps. This recovers the multi-packet HD PMT in the measured recording. Broadcast startup and damaged packets can still cause decoder warnings; captures are not claimed error-free.

## Playback

Select upscaling on/off, broadcast size/1080p/1440p/3840 × 2160, Auto/CPU/GPU preference, and 1–64 CPU workers. FFmpeg and FFplay must be installed together; they are not bundled. This PC's `C:\Program Files\FFmpeg\bin` installation is detected. Choose another FFmpeg executable in the GUI if needed.

The GPU path probes actual Vulkan/libplacebo filter execution and uses Vulkan hardware decoding, GPU scaling and Vulkan presentation. The initial Vulkan run with many host decoder threads stalled. A single host decode-submission worker completed all 277 frames at approximately 76 fps (2.79x real time) in the isolated 4K throughput test. GPU shaders and queue execution remain parallel. D3D11VA was tested as a comparison path, not chosen as the default. A short successful test does not establish long-term stability on every GPU. GPU scaling uses libplacebo's EWA Lanczos filter. CPU fallback uses multithreaded software decoding, Lanczos scaling and filters. Interlaced frames are deinterlaced; aspect ratio is preserved. The selected size is the processed frame size even when the player window is smaller.

The playback transport uses a separately probed AMF/NVENC/QSV H.264 encoder where available, otherwise multithreaded x264. This avoids transferring uncompressed 4K frames between processes. Playback therefore includes a high-quality lossy intermediate encode; requested `.ts` recordings remain the original broadcast bytes. FFplay presents a separate video window with audio. GPU frame scheduling is managed by the driver and libraries, not by a fabricated thread-count control.

USB reads, media processing and the interface run independently. A bounded live playback queue prevents unlimited memory growth. If processing falls behind, dropped bytes are counted in the export and picture glitches are possible. Stop terminates the owned media processes and stops TS submission without invoking the unverified firmware power-down command. Hardware calls have bounded timeouts; a scan can take its current tune timeout before stopping.

## Host algorithm provenance

The tuner initialization/calibration/frequency algorithm and register tables were adapted to Rust from trinity19683's `recfsusb2i`, commit `a2eb8fd43fc51195b1053027616b6bfb491c42f0`, [source](https://github.com/jeeb/recfsusb2i). The original author's GPLv3 notice is retained. The proprietary `it9175_fw.h` payload was not copied into this project. The host sequence was compared against the supplied AVerMedia x64 driver: 138/140 and 146/149 register/value signatures from the two initialization tables were present; the UHF clock table and 6 MHz coefficients also matched. Pattern presence alone is not proof of an active path; the successful hardware tuning/recording supplies the behavioral evidence.

Reference product specification: [AVerMedia A865R datasheet](https://storage.avermedia.com/web_release_www/A865R/A865R_Datasheet_EN%2020140214.pdf). Playback interfaces: [FFmpeg filters](https://ffmpeg.org/ffmpeg-filters.html#libplacebo), [FFplay](https://ffmpeg.org/ffplay.html).

Live startup saves a short `service-probe.ts` sample, reads its program metadata and chooses the highest-resolution service with audio from the same program. This prevents one-seg packets arriving first from selecting low-resolution video. FFprobe is required alongside FFmpeg/FFplay. The selected program and actual source dimensions/frame rate are exported.

Native progressive 60 fps is preserved. Double-rate deinterlacing is now the default: 59.94 fields/s interlaced input produces 59.94 progressive frames/s. Standard-rate and Off modes remain selectable. No motion interpolation is included.

The double-rate Vulkan test on the actual interlaced HD recording produced progressive 3840×2160 at 59.94 fps, 553 output frames, processing at approximately 134 fps / 2.48x real time in the isolated throughput test. Decoding, BWDIF deinterlacing and libplacebo scaling stayed in Vulkan memory until transfer to the playback encoder. [BWDIF Vulkan documentation](https://ffmpeg.org/ffmpeg-filters.html#bwdif_005fvulkan). These throughput results are from this PC, not guarantees for other GPUs.
