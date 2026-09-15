# Native player inspection and validation

Inspected September 10, 2026. Baseline userspace driver 0.7.0, API 2, open LINK/OFDM firmware 0.1.4.0. The native player links the existing tuner implementation; no replacement USB binding, EEPROM write, or COM registration was performed.

## Original application

Inspected the installed `AVerTV.exe` in `C:\Program Files (x86)\AVerMedia\AVerTV 3D`. File version 6.9.1.18, product version 6.5.2.0; SHA-256 `6BA448498EE80A7BA89166C58D400EBA37CB7F67D0C571DFB2AE1DF54BA216DC`.

The original Classic Tube layout is described by `Skin\Classic Tube\Main\SKIN\skin.xml`, `Main\control\control.xml`, `VideoFrame\skin\skin.xml`, and `VideoFrame\control\control.xml`. The design separates a large video window from a compact control panel, with a gold information display, tools, transport controls, stream position, and volume. User screenshots supplied additional layout references. The new application draws its own controls in Rust/GDI; original proprietary skin bitmaps and executables are not bundled.

Radio, its meter, and 3D/see3D controls were excluded as requested. Native window borders, a normal gear button, adjacent channel controls, camera snapshot icon, fullscreen, country profiles, and a separate EPG are implemented.

## Validation evidence

- Tests cover transport synchronization after USB acquisition noise, CRC and fragmented sections, broadcast channel numbers, bounded country scans, PCR indexing and wraparound, GPU color interpolation against scalar reference, NV12 pitch expansion, and EIT time/text/bounds handling.
- Native playback of an existing H.264/AAC recording passed pause, seek, resume, and actual EVR snapshot capture. The original unsupported-service error was caused by synchronization discarding valid transport tables after acquisition noise.
- Existing interrupted scans were recovered; the user's saved database contained 48 services after merging. Broadcast metadata correctly identified `4 – TV GAZETA HD`, rather than substituting RF22 for its channel number.
- A custom RGB monitor ICC profile was applied using the Windows CMM and Direct3D 11 LUT. Actual video snapshots were inspected after seeking. GPU/reference test pixel differences were at most one code value for the test transform. An EVR 2048-byte pitch versus decoder 1920-byte pitch mismatch was fixed and covered by a regression test.
- In a bounded live recording, capture continued while playback was paused: the file grew from 17,202,512 to 20,356,212 bytes while the timeline remained paused. Seeking moved playback back to approximately 0.517 seconds; resume and snapshot succeeded, and recording stopped cleanly.
- A subsequent live recording exercised ICC, pause/seek, and EPG collection together. Eleven real program events were received from TV GAZETA HD/mobile services, including Portuguese accents, titles, times, and descriptions.

The GPU ICC test averaged approximately 24 ms per decoded 1080-line frame on this machine, including synchronization/readback and initialization. This is not a claim of zero-copy processing or universal 60 fps performance. CPU fallback is slower. Native output size was verified to prevent enlargement above 1920 × 1080 for the test recording.

## Remaining limits

The current renderer is DirectShow/EVR with Direct3D 11 ICC processing. Vulkan, an independently controlled smooth deinterlacer, HDR color management, scheduled EPG recording, and exhaustive long-duration/suspend testing remain outside this alpha. Country profiles do not expand the hardware's ISDB-T UHF capabilities. See `player/README.md` for usage and precise limits.

EPG field layout and time-reference handling were checked against the [ISDB harmonization document](https://www.dibeg.org/wp/wp-content/uploads/techp/aribstd/harmonization/2009_09_186_BasicInformation-SI_ABNT_ARIB_SBTVD_JD.pdf). The guide displays station clock values explicitly as broadcast times.
