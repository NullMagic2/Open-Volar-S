# LiveTV 0.9.5: XFCE window activation fix

On XFCE with X11, entering LiveTV from another application brings both the viewer
and the DAC forward. This includes taskbar activation of covered windows and
restoring minimized windows. Once LiveTV is active, clicks between its two
windows leave their relative stacking to the window manager. Maximizing the
viewer is not the condition for grouping activation.

Desktop selection uses the first nonempty value of `XDG_CURRENT_DESKTOP`,
`XDG_SESSION_DESKTOP`, and `DESKTOP_SESSION`. XFCE, xfce4, and Xubuntu names
select the new behavior. Ubuntu GNOME and unrecognized desktops retain the
original `raise_visible_pair` focus-in handlers, copied unchanged from 0.9.5.
This also prevents a nested Ubuntu session from inheriting a stale XFCE login
session name. WSLg retains its original activation exception.

The original code raised both windows on every focus-in event. The earlier
maximized-only patch missed ordinary taskbar activation of covered windows.
This revision observes the WM's `_NET_ACTIVE_WINDOW` property and handles the
transition from another application to LiveTV once. The companion is placed
immediately beneath the activated window with a single sibling-restacking
request; repeated focus events inside LiveTV do not restack the pair.

Automatically restoring a minimized companion temporarily disables its focus
hint through XFWM's map processing (100 ms after the unminimize event). The hint
is then restored, allowing subsequent clicks to focus either window normally.
A deliberately hidden DAC stays hidden. Both windows remain independent
ordinary top-level windows, without a transient parent or always-on-top hint.

Build:

    cargo build --release --locked -p open-volar-s-live-tv

Regression checks (isolated desktop; no tuner access):

    python3 linux/tests/ui_window_stacking.py target/release/open-volar-s-live-tv

Requires Xvfb, XFWM, xfce4-panel, xdotool, xmessage, dbus-run-session, and Python
GI with Atspi 2.0. The test locates actual XFCE task buttons through accessibility
and clicks both entries, checking restoration of minimized and covered windows.
It also checks independent activation through real client-area clicks and
EWMH requests, stable stacking, unchanged positions, and the same behavior with
a maximized viewer and after returning to windowed mode.

Existing fullscreen/restore regression:

    xvfb-run -a cargo test --locked -p open-volar-s-live-tv independent_windows_restore_and_shrink_after_fullscreen -- --test-threads=1

The supplied archive's kernel driver and native video player are unchanged.

Desktop detection regression:

    cargo test --locked -p open-volar-s-live-tv desktop_activation_tests
