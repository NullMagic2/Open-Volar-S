# Orbit artwork

These production PNG assets are exported from the approved Python/Pillow/NumPy interface study. They are embedded with `include_bytes!`; the program never downloads UI resources.

- `fascia.png`: approved coffee-brown brushed panel.
- `settings-fascia.png`: nine-slice adaptation of the same panel, preserving the proportions of its corner screws.
- `module.png`: recessed dark copper plate, burnished-clay scale, and stationary illuminated dial. The engraved scale has a very small reconstruction filter to smooth downsampled edges.
- `dial.png`: transparent silver disc. Illumination stays stationary in world space.
- `pointer.png`: separate recessed red ceramic inlay and slot, including its subtle glaze highlight, rendered at the 65% reference angle. Native rotation pivots exactly at its center.
- `metal.png`, `glass.png`, `plastic.png`: unlabelled material surfaces. Metal is cropped at a consistent grain scale and lit separately; the complete square is never squeezed into each button.
- Remaining PNGs: Python-drawn alpha icon masks. Native code colors them consistently with their text and state, resampling with Lanczos antialiasing.

The material textures and images remain reusable, independent of the button labels and dynamic receiver state. `player/src/orbit.rs` owns the native layout and lighting; `player/src/dial.rs` owns pointer interpolation and sampling.
