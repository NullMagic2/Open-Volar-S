# Native diagnostics in alpha.37

Debug Desk now launches the native Live TV executable for live and recorded playback. It uses an isolated diagnostic profile and stops its owned child when the diagnostic session ends. The mpv pipeline and bundled mpv runtime have been removed. Older descriptions below refer to the earlier diagnostic implementation.

# A865R TV and debug interface

Run `a865r-debug.exe`. Leave the firmware path empty to use built-in open 0.1.4.0, or select your matching original firmware. Use Scan or a manual frequency, then Watch / Record TV. Playback supports Native / 1080p / 1440p / 4K, an upscaling switch, Vulkan GPU preference and multithreaded CPU fallback. Stop remains available while receiving. FFmpeg is an external runtime dependency; mpv is included for color-managed presentation.

Monitor colors selects the Windows monitor ICC profile, an explicit RGB ICC/ICM file, or Off. The footer and `status.json` report the profile actually confirmed by the renderer, including its path and SHA-256. Device capabilities & versions queries chipset, firmware and driver version; Export current playback status reads cached playback information without competing for USB access. See `../docs/API.md` for Rust and command-line queries.

Probe tuner and Power snapshot read identification/checkpoints. Inspect firmware and Compare with original .sys analyze the selected files. Inventory capture and Analyze capture process a saved USBPcap PCAP/PCAPNG file with Python. Export report saves notes and results; Open exports folder opens the session.

Installed exports: `%LOCALAPPDATA%/A865R/Debug/exports`. Portable exports: `debug/exports`. Scan JSON, playback settings, GPU probes, decoder logs and requested recordings remain available there. Raw proprietary firmware is not exported.

Public Rust playback API: `a865r_media::playback`; settings and device capabilities: `a865r::api`. See `../docs/API.md` and `examples/player_api.rs`. Open 0.1.4.0 now receives TV on the tested original A865R; see `../docs/RF_INTEGRATION_0.1.4.md` for hardware results and limits.

Infrared learning: expand Infrared remote through the tuner, name one button and choose Listen for 30 seconds. Codes and errors are exported automatically. Handset decoding and button actions remain unverified; see ../docs/REMOTE_CONTROL.md. Debug Desk does not start automatically after installation.
