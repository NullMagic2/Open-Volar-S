# Alpha 20 validation

Tested on the original A865R Volar S and AMD Radeon RX 7900 XTX under Windows. Sessions used isolated settings and bounded normal playback; no GPU stress tests or driver changes were made.

- 74 CPU/unit checks passed across player, BDA, core and debug packages. Two existing GPU execution tests remained ignored following prior driver instability. The Vulkan shader was validated without executing those tests.
- RBI's audio clock advanced at only 66–77% of wall time in the original 8-packet queue runs. With the bounded 64-packet / 8 MiB queue it tracked wall time at 99.86–99.99%, and repeated fields fell from 24–34% to 1.25–1.81% after initial acquisition. Tribuna remained smooth. These are short local measurements, not a universal performance guarantee.
- RBI → Tribuna → Gazeta → RBI, minimized channel switching, restore and pause/resume completed successfully. Gazeta still showed gaps between consecutive decoded timestamps despite no receiver queue drops; input/reception irregularities remain.
- Smooth, Standard and Off playback completed. Standard used only one field per picture. The settings combo exposes all three enabled choices.
- Recording kept growing while paused and after backward/forward seeks. Live resumed about 2.4 seconds behind capture. The test saved approximately 117 MB of original TS and shut down cleanly.
- Stereo, Mono, Left and Right were applied during recording. Both RBI AAC LATM PIDs (150 and 160) decoded stereo Float32 PCM. PID 160 was nearly silent during the test, and neither track supplied a language label. Both tracks were selectable in the native Audio menu.
- Switching tracks preserved capture and resumed video. Pressing REC again finalized recording and restarted live playback; recording seek controls then disabled.

Known issue: sustained foreground fullscreen video can still slow down on this GPU. Alpha 20 does not claim to fix that separate presentation problem.

The companion A865R-0.8.0-alpha.20-validation.json contains detailed session results. TV recordings and monitor ICC payloads are excluded from the installer and source archive.
