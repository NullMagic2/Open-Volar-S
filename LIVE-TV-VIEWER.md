Current update: [alpha.31 settings corrections and audio validation](docs/ALPHA31-VALIDATION.md).

Current update: [alpha.30 HDR effect and validation](docs/ALPHA30-VALIDATION.md).

Current update: [alpha.29 DAC row adjustment and validation](docs/ALPHA29-VALIDATION.md).

Current UI update: [alpha.28 changes and validation](docs/ALPHA28-VALIDATION.md). Four interface languages are available under Settings > General.

# Live TV! native viewing console

The viewing window now implements the approved physical-TV design using the DAC's coffee-metal frame and shared cream ink (`#E9DFCE`). The screen has a continuous recessed bezel on all four sides. Artwork is embedded; the application does not need Python or a browser at runtime.

The transport order is double-left (previous frame), double-right (next frame), Play/Pause, Stop. Pause uses two solid bars and Stop uses a solid square. Glyphs are centered and the former shared box, Playback label and pause underline are gone. Play appears when playback is stopped or paused, preserving the existing toggle behavior.

Channel and Volume rockers have equal dimensions. REC is centered between transport and Channel, with a circular glass lens and thicker concentric metal ring. Its lens is dim when off and red while recording. The Live dot is green; its label and border become green on pointer hover. Labels, title and utility-button text use the DAC's shared cream ink. Volume has no numeric legend.

Settings opens a native popup for audio tracks/mode, closed captions, opening media, the receiver panel and Preferences. Preferences retains the existing Video, Channels, Storage and Appearance pages. Snapshot, EPG, fullscreen, keyboard shortcuts and tuner/decoder commands remain connected to their existing handlers.

The viewing window has no decorative corner screws. All its standard button faces now call the DAC renderer directly and follow the same Metal, Glass or Plastic setting, with shared hover reflections, rounded edges, press depth and ink. The circular recording lens retains its approved glass and metal construction. The DAC minimize button is shifted four design pixels to the right.

The slider has one visible contextual time counter: elapsed live viewing time, recording duration near Live, or the current playback position when rewound. Dragging previews that position; returning to Live shows the elapsed recording duration again. Live viewing publishes a pause-aware elapsed clock even without a seek index.

The DAC retains its corrected corner artwork and recessed title treatment. Its Live indicator uses green; REC and CC have stronger colored hover borders, drawn inward at 3 logical pixels and scaled for monitor DPI.

## Validation

- Release build and all 57 enabled player tests passed. Two pre-existing tests requiring GPU execution remain ignored.
- Real Win32 UI checks passed at widths of 800, 1120 and 1600 pixels: transport order, rocker equality, lamp centering/circularity and complete video frame.
- Native Settings popup, Preferences and receiver panel opening, caption preference persistence, maximize/restore, and fullscreen restoration were exercised in an isolated `--ui-preview` profile.
- Fullscreen retains the existing one-physical-pixel Vulkan surface inset. Actual DAC hover screenshots confirm the thicker red and yellow borders.
- Live tuner reception, GPU playback and actual recording were not exercised in this UI preview.

## Files

- `GUI/Windows/src/viewer.rs`: viewing console geometry, native painting, popup and hit testing.
- `GUI/Windows/src/main.rs`: native window/command integration, contextual time and aspect sizing.
- `GUI/Windows/src/native.rs`: live elapsed clock publication.
- `GUI/Windows/src/orbit.rs`: shared cream ink and DAC hover-border thickness.
- `GUI/Windows/assets/viewer`: embedded faces, indicators and filled glyphs.

Build with `cargo build -p a865r-tv --release --locked`; test with `cargo test -p a865r-tv --bin live-tv --locked`. Run a hardware-free preview with `live-tv.exe --ui-preview --profile-dir <temporary-profile-folder>`.

## Alpha 27 corrections

Alpha 27 fixes persistent gray system borders after moving or activating the viewing window. Native resize behavior is retained while the application owns its frame painting. The slider timer uses a rounded recess over the continuous panel background, including synchronous playback-time updates. Live now uses the DAC button renderer and icon directly. Settings popup text is larger and scales using the owning window's DPI. See docs/ALPHA27-VALIDATION.md.

Alpha.27 restores dedicated Receiver Panel and DAC Audio buttons. The Picture and Parental settings and validation limits are described in [ALPHA27-VALIDATION.md](docs/ALPHA27-VALIDATION.md).
