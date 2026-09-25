# Open Volar S artwork

The approved application icon is the gentle-angle ruby USB tuner with a white etched open padlock. Only the marking receives the perspective transform; the casing remains unchanged.

- `open-volar-s.svg`: editable vector paths and gradients, 1024-unit canvas.
- `open-volar-s-1024.png`: exact approved transparent PNG; smaller PNG sizes are also included.
- `open-volar-s.ico`: Windows icon containing 16, 20, 24, 32, 40, 48, 64, 128 and 256 px images.
- `open-volar-s-steep.*`: alternate angle, with matching PNG sizes.
- `draw_volar_ruby_etched.py`: original Python drawing source (Pillow and NumPy).
- `export_svg.py`: reconstructs vector layers from that drawing, preserving the lock perspective. Fine satin brushing is approximated with sampled vertical gradient stops.

Live TV and Debug Desk embed the default icon; the installer uses it as well. Runtime assets in `GUI/Windows/assets` are byte-identical copies of the default PNG and ICO here. No raster images are embedded in the SVG files.

Alpha.34: enlarged by 8.5% using the opaque hardware bounds, with 12 px canvas padding at 1024 px. The original object proportions and lock perspective are preserved.
