# A865R TV alpha 10 changes

## Alpha 10: RBI startup, aspect ratio, and on-screen display

RBI HD transmits non-IDR intra pictures. The old startup gate waited indefinitely for an IDR even when the parser supplied an independent picture. Startup now also accepts a first intra picture with an empty reference list, excluding complementary second fields. The bitstream IDR flag, Vulkan session handling, and missing-reference checks are preserved. The CPU parser diagnostic verifies reference continuity from either independent start. Decoder reports include parsed and skipped startup pictures.

Settings now offers Auto, 4:3, and 16:9. Selection applies immediately, including a paused picture, and is saved between launches. Auto uses broadcast sample-aspect metadata when present; the selected display ratio also constrains native window edge resizing. Processing resolution remains a separate Native/2K/4K choice.

Changing channel shows its number and name over the video for three seconds; the label is shown again when the first picture arrives. Changing volume shows a percentage and horizontal bar for three seconds. These are newly drawn windows-rs child controls, so they appear while tuning, paused, and fullscreen without copying decoded GPU frames. They do not make the player globally topmost. Channel buttons can restart watching after a failed service.

The installed AVerTV executable was inspected read-only for its OSD implementation: AVEROSD/BCOSDWnd, OsdAlign, OSD_DURATION, OSD_TEXT/EDGE/BACK, OSD_ITEM_DTV, and volume OSD entry points. Its Skin/OSD directory supplies transport icons but no channel/volume layout. Alignment, gold styling and the three-second timeout in this implementation are our own choices; original program binaries and artwork are not bundled.

The CPU-only RBI capture contained 657 parsed pictures, 329 display events, and no IDRs. Five entry offsets passed with zero missing references or timestamp regressions. See the release validation JSON for hardware measurements.

Vulkan reference resources are supplied only for decoded pictures, consistent with https://docs.vulkan.org/spec/latest/chapters/videocoding.html .

Hardware verification on RX 7900 XTX: RBI first frame 3.2 seconds in a 36-second baseline. A second bounded run switched RBI → TV Tribuna → RBI, with first frames in 3.4–3.8 seconds, active audio output, and clean shutdown. That run verified Auto, 4:3, paused 16:9 changes, 3840×2160 fullscreen presentation, pause/resume, and timed overlay visibility. 23 player tests and 2 parser-helper tests passed; the two previously disabled GPU unit tests remain excluded.
