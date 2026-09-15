# Alpha 26 — Native console validation

Release: **0.8.0-alpha.26**. Windows file/product version: **0.8.0.26**.
The existing userspace driver and Debug Desk remain **0.7.0**; open firmware remains **0.1.4.0**.

Alpha 26 implements the physical-TV viewing console with a recessed screen frame, no decorative corner screws, and the same Metal/Glass/Plastic button rendering as the DAC panel. Playback order is back, forward, Play/Pause, Stop. Channel and Volume rockers have equal dimensions; the recording lamp is concentric glass and metal. The contextual slider counter shows live elapsed time, recording duration near Live, or the current rewound position. Includes the DAC corner correction, thicker REC/CC hover borders, green Live treatment, cream legends, and adjusted minimize position. Existing tuner and decoding architecture is retained. See docs/ALPHA26-VALIDATION.md.

## Completed checks

- Player regression suite: 57 passed; two pre-existing GPU execution tests remain ignored.
- Native process-isolated checks: correct transport order and framing at 800, 1120 and 1600 pixels; equal Channel/Volume rockers; centered circular recording lens.
- Slider time readout is visible without overlapping Live. Formatter tests cover recording time, rewind, paused position and returning to the live edge.
- Actual Appearance changes confirm that playback, utility and Live buttons all follow Metal, Glass and Plastic. The same surface renderer supplies the DAC and viewing panel.
- Native green Live hover border and thicker DAC REC/CC hover borders were inspected.
- Settings popup, Preferences, receiver panel, closed-caption persistence, maximize/restore and fullscreen restoration pass. Fullscreen retains the one-physical-pixel Vulkan inset.

Live tuner reception, GPU playback and actual recording were not exercised for this interface release. These UI and CPU checks do not establish new hardware playback guarantees.

## Release packaging

The Inno Setup recipe keeps the existing application identity, upgrade checks, Just for me/All users choices and shortcut conventions. Setup does not launch playback. The package includes the native player, Debug Desk, command-line tools, required existing runtime DLLs, firmware, documentation and license notices.

The matching source archive includes build sources, embedded artwork, tests, Cargo.lock and a SHA-256 source manifest. It excludes compiled applications/DLLs, build output, user profiles and recordings. Rebuilding the full installer requires the runtime binaries documented in BUILD-SOURCE.md.
