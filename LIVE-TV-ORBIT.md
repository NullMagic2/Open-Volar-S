Current update: [alpha.31 settings corrections and audio validation](docs/ALPHA31-VALIDATION.md).

Current update: [alpha.30 HDR effect and validation](docs/ALPHA30-VALIDATION.md).

Current update: [alpha.29 DAC row adjustment and validation](docs/ALPHA29-VALIDATION.md).

Current UI update: [alpha.28 changes and validation](docs/ALPHA28-VALIDATION.md). Four interface languages are available under Settings > General.

# Live TV! — Orbit

Current release: **0.8.0-alpha.31**.

Native Windows implementation based on the supplied A865R 0.8.0-alpha.24 source archive.

## Interface

- The separate control panel uses the approved Orbit artwork: coffee-brown metal, a recessed copper mounting plate, burnished-clay dial scale, flush cross-head screws, and an independently rotating recessed ceramic pointer.
- The display frame and channel selector have slightly rounder corners, with the glass reflection clipped to the inner curve.
- The redundant Digital TV subtitle is removed from the channel display.
- The channel selector remains a native keyboard-accessible combobox, with a direct channel list, signal quality, and a shaded arrow. Its signal value comes from the receiver; it is not a demonstration percentage.
- Options uses its intended 610×580 logical window size. The background scales in nine sections so corner screws retain their circular proportions; tab names have no extra underline.
- Options comboboxes share one rounded frame; the editable country field is inset clear of its border and arrow.
- Settings opens centered over the panel, using its monitor scaling and constrained to the monitor work area. Its independent window activates without raising the video window, with a borderless rounded fascia and the same close screw as the panel. Drag its header or exposed metal frame to move it. Choose ICC is centered beneath the combobox column. Video, Channels, Storage, and Appearance share one connected tab/page frame; the selected tab opens directly into the page through connected rounded shoulders, with a straight outer left edge on Video. Existing video processing, aspect ratio, ICC, scanning, country profiles, and recording-folder controls remain available.
- Appearance saves `theme: "orbit"` and `button_material: "metal" | "glass" | "plastic"` independently. Material changes apply immediately. Missing or unknown material values fall back to Metal. Existing tuner and playback preferences remain in `%LOCALAPPDATA%/A865R/TV` for compatibility.
- REC keeps its small red icon illuminated and Live uses green; CC uses a yellow icon while captions are enabled. All button materials use softer corners and support hover reflection and press feedback. REC turns red, Live turns green, and CC turns yellow on hover, consistently across all three materials. Button labels and borders return to neutral when the pointer leaves, including during recording or with captions enabled. REC/REC ON and CC OFF/CC ON still report the actual state. Keyboard focus uses the outer edge without an extra inner rectangle. Accent state follows actual pointer entry/leave rather than the residual reflection animation. Metal reflects a stronger moving light band and adjacent warm shade. Glass reflections fade smoothly rather than forming a hard stripe.
- The Live icon/label group sits beneath the clock in the display frame’s right section, with the icon’s left edge aligned exactly to the clock box. Both are shifted right together to balance the space between the channel card and the display’s right inset. The larger local `hh:mm:ss` clock uses the regular interface font in its own small recessed box. The redundant Live television label is removed. The Digital TV receiver header label is removed.
- The panel starts centered in its monitor’s work area. Live TV! is shifted five design pixels right. Its cached 4× glyph mask adds a dark recessed cut and lower rim while retaining the original warm letter-face color. The panel has no system title bar. Two slightly enlarged screw-style minimize/close controls with recessed symbols sit slightly lower in the upper-right header, aligned with the mounting plate’s right edge. Their flat palette matches the other screws; the pair is nudged two design pixels left. Drag the header to move it; edge resizing preserves the circular dial. Closing the panel leaves the video window running. Supersampled artwork and Lanczos scaling smooth the fascia corners; the window silhouette clips exterior fill. The default panel is 1088×354 logical pixels (15% smaller than 1280×417), and the dial remains circular. The mounting plate now spans the display-frame top to the lower button-row bottom; the plate and dial shrink together by about 5% from their previous geometry.
- The audio status line shows only the selected audio mode; caption state is shown on the CC button.
- The two 10-second seek buttons sit together and use matching double-chevron icons. Their accessible labels retain the seek duration.
- The channel arrow uses supersampled masks with a heavier rounded chevron. Seek controls defer reentrant painting rather than falling back to the legacy renderer.
- Channel dropdown selection commits after the list closes; unchanged lists are preserved, and background metadata updates do not reset an open dropdown. Escape cancels selection. In the stream window, Up/Down select next/previous channel and Left/Right lower/raise volume, including with a playback control focused.
- Type a channel number from either player window, including leading zeros (`04`). Enter tunes immediately; a 1.2-second pause also commits. Backspace edits and Escape cancels. The overlay shows the entered channel without an Enter-to-tune prompt. Numbers match the displayed channel number, not the list position. Recording and scanning must be stopped before number entry can retune.
- The video overlay places live signal quality below the channel name. Unavailable readings show a dash, never an invented percentage.
- Only the video window has the playback slider. Its position follows the existing recorded-stream or saved-media timeline; the DAC retains its contextual elapsed/recording display.
- The video window now uses the approved physical-TV console, cream lettering, circular glass recording lamp, matching rockers and framed screen. See LIVE-TV-VIEWER.md for the final layout and validation. The tuner, decoder, recorder, audio pipeline, and Vulkan video renderer retain the original architecture.

The program guide now shares Orbit's metal frame, borderless draggable/resizable header, close screw, rounded channel filter, warm program selection, and inset description pane. The window is titled Live TV! — Program guide. Its existing passive guide ingestion, channel filtering, keyboard selection, and broadcaster-clock handling remain intact.

Storage provides matching editable textboxes and folder pickers for recordings and snapshots. Apply validates and saves both absolute folder paths; existing recording-folder preferences remain compatible. Snapshots default to Documents/A865R/snapshots. The Snapshot action saves a PNG in that configured folder and reports completion or failure. Both existing video presenters share CPU PNG encoding; diagnostic commands requesting BMP retain BMP output. PNG files are published only after encoding completes. Live/GPU snapshot capture was not exercised in this update.

## SD picture proportions

RBI SD (channel 05) logs reported 720×480 with a 15:11 full-raster aspect. Auto now recognizes the exact conventional SD aperture ratios: the central 704 pixels are displayed at 4:3 (or 16:9 for 20:11 metadata), omitting eight nominal padding samples on each side. Other ratios, already-cropped pictures, and explicit user overrides retain their existing interpretation. This is based on the [documented SD display-aperture convention](https://learn.microsoft.com/en-us/windows/win32/medfound/picture-aspect-ratio); it cannot infer distortions baked into the broadcast. CPU geometry and shader validation passed; no hardware playback test was performed after the change.

## Build and run

Use the Windows MSVC toolchain and Windows SDK required by the original source:

```powershell
cargo build -p a865r-tv --release --locked
cargo test -p a865r-tv --bin live-tv --locked
```

The internal Cargo package remains `a865r-tv` to preserve workspace compatibility. The executable is `target/release/live-tv.exe`; its window titles, Windows product metadata, branding, and installer shortcuts say **Live TV!**.

All Orbit textures and icons are embedded. There is no browser, web runtime, or loose artwork dependency. The normal player still requires the original Windows media components and Vulkan-capable graphics driver.

For an isolated UI review without opening the tuner:

```powershell
target/release/live-tv.exe --ui-preview --profile-dir C:/path/to/review-profile
```

The profile path holds only that review's preferences. Without `--ui-preview`, startup follows the original tuning behavior.

## Validation

- Locked offline native build.
- Snapshot CPU tests: 2 passed, covering PNG decoding, colors/orientation, diagnostic BMP compatibility, and folder validation.
- Isolated Storage checks cover editable fields, invalid-path feedback, Apply persistence, reopening, and application restart.
- Guide lifecycle and broadcast-clock tests: 2 passed. Native fixture checks cover filtering, description selection, rounded borderless clipping and the close screw.
- Native checks confirm Settings has no video owner and opens centered over the panel within its monitor work area.
- Player tests: 52 previously passed, plus 2 focused idle/playback status tests passed; 2 pre-existing hardware GPU tests remain ignored.
- Idle settings clear stale Preparing playback text; real preparation and error messages remain visible.
- Native UI smoke checks also verify red REC/Live and yellow CC hover across all materials, borderless Settings clipping and its close screw, the borderless panel client area, resize/drag hit testing, minimize, panel close/reopen, and circular dial proportions. They use a separately launched process and isolated profile: keyboard navigation through all four tabs, immediate material switching and persistence after restart, channel dropdown, absence of a duplicate slider in Orbit, adjacent seek buttons, and direct channel entry (Enter, timeout, cancellation, editing, and unknown numbers).
- Run `python tools/test_orbit_ui.py` after the release build on Windows (requires Pillow). Screenshots and the isolated profile are reused under `target/orbit-ui-check`.
- Native screenshots inspected for the three materials and Settings pages at 200% Windows scaling.
- A limited TV Câmara capture and offline CPU decode/parser check completed. A prior Vulkan playback diagnostic caused a system crash; GPU stability remains unresolved. No further Vulkan playback was run. See [GPU diagnosis](docs/TV-CAMARA-GPU-DIAGNOSIS.md).

The installer includes the rebuilt player, Debug Desk, CLI, and runtime dependencies copied from the existing matching installation. Building from source still requires the external runtime files listed in `BUILD-SOURCE.md`; these binaries are excluded from the source archive. The installer was compiled but was not run against the user’s installation.

Latest Options corrections preserve circular corner screws, remove tab underlines, inset editable country text, and anchor Apply with clearance above the rim. The obsolete 793-pixel minimum height is replaced by the compact 610×580 window.

Graphics device-loss handling now stops and joins the old graph before one cancellable Windows-renderer recovery attempt. Vulkan remains disabled for the rest of the process after a detected loss, including subsequent channel changes. Recording capture continues during playback recovery. This is a software lifecycle/recovery fix, not a confirmed fix for the GPU engine timeout; see the diagnosis above.

Project naming: **Open Volar S** names the overall project and distribution. **Live TV!** names the television presentation application; **Orbit** names its main theme. Legacy A865R installation/profile paths and internal crate names are retained for compatibility.

- The live indicator sits below the clock, aligned to its left edge; recording or scan status retains its separate row above the live indicator. The Video tab joins the page with a straight left edge and retains its rounded top corners.

- Opening any channel, Settings, or guide dropdown dims only the visible widgets beneath it by a subtle 19% black overlay. The combo face and native popup retain their existing colors. The dimming layer is mouse-transparent, never activates, and disappears on close or cancellation. Native UI checks verify the region excludes the combo, covers lower controls, and hides on close in all three windows.
