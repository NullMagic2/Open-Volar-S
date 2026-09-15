# Live TV! 0.8.0-alpha.28

Settings now has a General tab with English, Brazilian Portuguese, Spanish, and Greek, with country flags. Language changes apply immediately and persist after restart. Menus, native control labels, settings, audio modes, programme-guide chrome, and application messages share a translation catalog. Broadcast channel names, programme titles/descriptions, paths and recordings retain their original content. Windows-owned system dialogs and OS/driver error details follow Windows' language.

All seven settings pages use a wider aligned layout. Password textboxes have a light outline; Maximum age remains compact. Block unrated sits immediately beside its control. The Parental tab no longer has the curved right-hand join. Picture includes Reset defaults beside Apply, resetting only the picture preset, adjustments and app video-HDR preference. The two requested explanatory messages were removed.

The stream Live control now uses the DAC broadcast icon in a recessed square, with cream resting text and the DAC green hover accent. Dotted focus rectangles have been replaced by a solid keyboard-focus cue; mouse clicks hide that cue. Combo dropdowns and programme-guide scrollbars use the interface's dark track, warm metal thumb and cream arrows, preserving wheel, arrow and thumb interaction.

When the current broadcast programme is available, its title appears between the DAC channel name and signal quality and in the channel OSD.

## Video-only HDR

The app never changes Windows HDR. The switch controls only the Vulkan video surface. HDR mode selects a supported FP16/scRGB swapchain on the player's current HDR-enabled monitor, leaving the native interface in SDR. It follows monitor changes and falls back to SDR when HDR output is unavailable. SDR reference white follows the Windows setting. In this mode, Windows performs display color management; the app's SDR monitor ICC transform is bypassed to avoid applying it twice. Screenshots remain normal SDR PNG files.

Windows HDR must already be enabled by the user for that monitor. Current broadcast decoding is still 8-bit H.264 SDR: using an HDR-capable output does not create HDR detail from an SDR programme. Native HDR10/PQ/HLG source decoding is not added in this release. The Windows recovery renderer remains SDR.

Implementation follows Microsoft's [Advanced Color and SDR reference-white guidance](https://learn.microsoft.com/en-us/windows/win32/direct3darticles/high-dynamic-range). No display-setting write API remains in the player HDR module.

## Validation

- 71 player tests passed; 2 GPU execution tests remain explicitly ignored after earlier driver instability. This includes translation placeholder completeness, Greek text, dynamic labels, EPG selection and WGSL validation.
- Process-isolated native preview checks covered four languages, all settings pages, picture reset, presets/sliders, parental password save/unlock, preservation of unsaved input, localized native combo entries, and language persistence after restart.
- Native playback geometry, equal rockers, concentric Rec lamp, DAC hover colors/materials, captions, maximize/restore and fullscreen checks passed. Native move checks measured zero gray-rim pixels before and after movement; the timer background stayed RGB (30, 20, 16).
- Native scrollbar arrow, wheel and thumb-drag checks passed, including keeping the popup open. The rendered scrollbar and translated settings pages were visually inspected.
- No tuner playback or real HDR GPU rendering was used for this release's validation. Hardware HDR presentation remains unverified; shader/CPU tests do not establish driver correctness.

Driver and Debug Desk remain 0.7.0. Existing user preferences are preserved; older profiles default to English and video HDR off.
