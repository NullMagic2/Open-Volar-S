# Alpha 21 validation

## Implemented

- AAC ADTS / LATM multichannel PCM negotiation through the Windows decoder. Audio → 5.1 surround preserves FL, FR, center, LFE and both surrounds, with both rear and side 5.1 Windows channel masks accepted. No speaker-fill upmix. Broadcast channel count comes from the input codec properties, not the initially negotiated output format. Selected output mode is saved.
- Stereo/mono/left/right modes downmix six-channel sources with center-dialogue and surround contributions, headroom and no LFE injection. Dynamic mono/stereo/5.1 PCM format changes are forwarded with the audio samples through the userspace filter.
- ISDB closed-caption PIDs discovered from PMT data-component descriptors. Bounded PES queue, native timestamps, libaribcaption 1.1.2 decoder/DirectWrite renderer, per-pixel-alpha caption overlay. Captions default on, support CC toggle, follow pause and recorded playback, resize with the video and clear on channel changes. Caption rendering does not read back video frames.
- Demux flushes now invalidate old video frames while preserving the user's running/paused state. Previously, mapping a caption stream flushed the video input and left it paused, eventually blocking audio.
- Windows Known Folder resolution for the current user's Documents\A865R\recordings, saved folder chooser in Settings, dated channel filenames, Library opens that folder. Changing the location during capture affects the next recording.

## Checks

- CPU/unit/protocol regression suite: 100 checks pass; two GPU execution tests remain intentionally ignored because of earlier reported GPU-driver instability.
- Six-channel AAC fixture decoded through the actual Windows decoder and userspace PCM filter. Speaker tones expected at 220, 330, 440, 55, 660 and 880 Hz measured at 220.0, 330.1, 439.9, 54.9, 659.9 and 879.8 Hz in FL/FR/FC/LFE/BL/BR order. 123,904 decoded six-channel frames. Stereo fixture changes correctly from the initial six-channel negotiation to two-channel PCM at the receiving pin. Windows DirectSound output also exercised with recorded RBI LATM audio, without GPU execution.
- The locally installed FFmpeg was used only to generate synthetic test fixtures; it is not required or invoked by the player.
- Portuguese caption regression covers management/statement decoding, timed appearance, transparent alpha, resize, cache invalidation on re-enable, flush clearing and truncated input. PMT tests cover component replacement, malformed descriptors and unsupported component types.
- Caption replay: 57 decoded statements, zero caption errors; overlay window visible; CC off/on and pause/resume observed in playback diagnostics. Audio meter had nonzero output in 18 of 21 one-second reports, including after resume.
- Final bounded live RBI recording + return-to-live test exited cleanly. Audio modes changed without restart. Video advanced to 499 received pictures in recording playback and 372 after return to live; all final reports show video unpaused. Audible output in 17/19 and 14/15 reports respectively, including the last report. The recording was saved as a dated RBI_HD.ts in an isolated custom folder (27,465,860 bytes), whose full path was read back from Settings. Folder, CC preference and audio mode persist in settings.json.
- No kernel binding, firmware flashing or system speaker configuration changes. No GPU stress tests.

## Limits

Physical six-speaker output is not verified: the channel separation test validates decoded PCM. A 5.1 broadcast, six-channel-capable output device and Windows speaker configuration are required for actual surround; Windows handles the endpoint routing. Unsupported compressed audio codecs remain unavailable.

RBI advertised captions but transmitted no caption statements in the sampled live session. Caption display was validated using a synthetic Portuguese caption stream multiplexed into an existing recording; live caption availability and broadcaster formatting vary. Recordings retain the original caption data instead of burning captions into video.

The previously reported sustained fullscreen slowdown remains unresolved. These bounded tests do not establish that earlier system-wide graphics-driver crashes are permanently fixed.

## Reproduction and provenance

CPU-only `player/examples/audio_probe.rs` decodes AAC TS input to a counted PCM sink, optionally saves PCM, and can use `--speakers` / `--latm`. `crates/a865r-bda/examples/caption_probe.rs` verifies PES payload delivery and timestamps without video decoding. The source archive contains the caption library, ISC license and pinned provenance in `third-party/libaribcaption/A865R-UPSTREAM.txt`.

Windows API references: [audio output negotiation](https://learn.microsoft.com/en-us/windows/win32/codecapi/avdeccommonoutputformat-property), [speaker channel configuration](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ksmedia/ns-ksmedia-ksaudio_channel_config). Caption library: [libaribcaption](https://github.com/xqq/libaribcaption).
