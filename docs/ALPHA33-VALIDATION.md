# Live TV! 0.8.0-alpha.33

Merged from the completed alpha.32 source with its General -> Start receiver with Windows setting and receiver-only startup path intact. It remains Off by default. Original alpha.32 validation is retained in ALPHA32-VALIDATION.md; actual Windows sign-in is still untested.

## Changes

- Approved ruby USB tuner icon embedded in Live TV, Debug Desk and Windows installer. Debug Desk also sets the same window icon.
- Root images directory contains transparent PNGs, editable SVGs, multi-resolution ICOs and Python drawing/export sources for both approved angles. The installer includes this directory.
- Live TV starts with the streaming window above the DAC-style control panel. Both share a horizontal center, and their combined bounds are centered within the monitor work area before either is shown. Initial sizes scale down together to fit the available work area, preserving proportions.
- Debug Desk requests centered startup placement.

## Validation

- Workspace release build passed.
- Player regression suite: 79 passed, zero failed, 2 pre-existing GPU tests ignored. This includes the alpha.32 startup checks.
- New placement checks cover 1366x768 and 1080p-class work areas, 4K, high DPI, portrait displays, taskbar offsets and monitors with negative desktop coordinates. They check containment, center alignment, separation and preserved aspect ratios.
- SVG rendered and visually compared with the exact approved PNG. Installed PNG/ICO copies verified by SHA-256. Resource verification checks actual embedded icon frames in both applications and the installer.

No Windows sign-in, reboot, automatic-start opt-in or driver installation was performed for this merge. Window placement was checked with geometry tests; a live multi-monitor desktop session was not exercised.
