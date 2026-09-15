# Local development release 0.7.0

Includes the userspace driver/GUI, API 2, standalone open firmware 0.1.4.0, source, tools, x86/x64 experimental BDA adapters, and mpv presentation runtime. Installer supports current-user or all-users scope and creates removable shortcuts without launching Debug Desk.

Validation: 49 x64 workspace tests and 41 x86 driver/BDA tests passed. Rebuilt firmware SHA-256 is 7700d9678b51c19ea11a42373249ce62575162c624c7052b72450eefb1803ccc, identical to the tested image. Firmware differential checks: 6,883. Hardware: 19 UHF centers locked and recorded; short idle and active Windows S3 cycles recovered. Automatic timer wake, prolonged sleep and hibernation remain unverified.

Color: live RF22 Vulkan playback applied the assigned ASUS ICC profile and reported 3840x2160 at 59.94006 fps; CPU file playback applied the explicit sRGB ICC profile. Off reported disabled with no active profile. Invalid ICC input returned an error. The live color test delivered 19,208,900 bytes, zero transport-error packets, five continuity errors and zero playback queue drops; reception is not error-free. Monitor switching remains untested.

See docs/API.md for driver version, chipset, resolution, rendering/upscaling and color-profile queries. FFmpeg/ffprobe are external. The original USB binding remains WinUSB.

This is the baseline before further AVerTV compatibility work. AVerTV graph construction succeeded in the earlier investigation, but native AVerTV picture/sound are not established. Compatibility registration is separate from the GUI installer. This package excludes TV recordings, original firmware and driver executables, and personal ICC payloads. Its evidence archive contains local device/profile identifiers.
