# Live TV! 0.8.0-alpha.34

## Fixes

- Explicit large and small native icons for Live TV windows, a stable OpenVolarS.LiveTV application identity, and a dedicated live-tv-ruby.ico shortcut path. This removes reliance on the old executable-icon cache entry. Existing pinned legacy shortcuts may retain their old identity until replaced with the updated shortcut.
- Ruby icon enlarged by 8.5% using solid hardware bounds instead of the diffuse shadow. Casing, lock perspective and proportions are unchanged. PNG, SVG, ICO, Debug Desk and installer assets updated together.
- Restored the HDR effect button. Alpha.32 assigned control ID 348 to both the HDR button and the Windows startup dropdown. The later General-page layout hid the HDR button and dropdown operations could address the wrong control. Startup now uses dedicated IDs 361/362, with its owner-drawn menu wired into the combo renderer.
- Removed the Orbit description and immediate-apply explanatory text from Appearance.
- Joined the selected tab's right shoulder to the page rim, matching the left shoulder.
- Clear the country edit selection on opening Channels, preserving its value and normal editing behavior.

- Start centered with the control panel in front, overlapping the streaming window by 76 logical pixels at the nominal size (about 10% of its height). Both scale together to fit the work area.

## Validation

- Player regression tests: 80 passed, 0 failed, 2 existing GPU tests ignored.
- New regression creates hidden native controls, verifies unique settings IDs, verifies hiding startup does not hide HDR, and checks that removing the country selection preserves Brazil.
- Release workspace build and Inno installer compile passed. Embedded icon frame identities checked for Live TV, Debug Desk and installer.
- Updated icon inspected at 512 px. SVG remains composed of editable vector paths and gradients.

Receiver startup remains Off by default. Actual Windows sign-in and the user's existing taskbar cache were not exercised by these automated tests. Installation and reopening Live TV are needed for the running application to use this build.
