# Live TV! 0.8.0-alpha.27

Windows resource version: 0.8.0.27. Driver and Debug Desk versions remain 0.7.0.

## Playback panel

- Restored a dedicated Receiver Panel icon and an Audio button drawn by the DAC renderer. Audio tracks and modes use the same cream text and dark native popup treatment.
- Transport order is previous frame, next frame, solid pause/play, solid stop. Channel and volume pairs retain equal dimensions. The round recording lens retains its thick metal ring.
- Fixed the persistent gray system rim after moving/activating the viewer. The timer uses a rounded recess over the panel background. Live uses the DAC icon, materials, green dot and green hover border/text. The recording/seek hint was removed.
- Frame stepping pauses on individual decoded frames. A history of existing NV12 image leases is capped at 32 frames and an estimated 64 MiB of image data, without additional per-frame GPU copies or CPU readback. Older recorded positions decode a preceding section again; frame-only preroll omits audio output so it need not wait for audio playback. Decode cost and keyframe spacing can still make older backward steps slower than cached steps. Frame stepping requires the Vulkan renderer and a seekable recording/file.

## Picture settings

The new Picture tab offers Neutral, Cold and Vivid presets, plus saturation (0–200%), brightness (-100 to +100), and contrast (0–200%). Manual adjustments select Custom. Neutral restores the identity transform. Settings persist and apply in the existing GPU color pass, before the monitor ICC transform; a held frame is refreshed when settings change. Windows recovery rendering remains neutral for these adjustments.

Windows HDR is queried for the monitor containing the player and changed only when its button is clicked. This controls Windows display HDR, not native HDR decoding or a new HDR swapchain. The display is resolved again at click time. Unsupported/policy-limited displays are disabled. HDR-specific APIs distinguish HDR from wide color gamut; the legacy advanced-color API is used only on Windows 10.

## Parental controls

Based on the installed AverTV 3D help: password-protected rating settings and channel locks, optional blocking of unrated programs, and temporary unlock until app exit. Passwords use a random salt and PBKDF2-HMAC-SHA256 (210,000 iterations), with no plaintext password in settings. A password must be set before blocking can be enabled. Editing and temporary unlock require the password afterwards.

The player reads the EIT parental-rating descriptor. Brazilian ratings use the low nibble: Livre, 10, 12, 14, 16 and 18; Japanese age codes are also recognized. Unsupported/unknown ratings follow Block unrated. Channel locks identify frequency and service ID. The policy is checked before playback and recording and during broadcast playback; a newly blocked program stops playback/recording. Opening a recording while controls are enabled requires temporary unlock because its rating may be unavailable.

Reference: installed AVerTV 3D GUIDE.chm, “Parental Control” and “Using Channel Lock”. Rating encoding: [ISDB-T harmonization, Basic Information SI, page 30](https://www.dibeg.org/wp/wp-content/uploads/techp/aribstd/harmonization/2009_09_186_BasicInformation-SI_ABNT_ARIB_SBTVD_JD.pdf). Windows API definitions: installed Windows SDK 10.0.26100.0 wingdi.h.

## Validation and limits

- Player suite: 65 passed, 2 previously ignored GPU execution tests. WGSL parses and validates with Naga. New coverage includes picture persistence/defaults, adjacent-frame selection, backward timestamp boundaries, bounded history/flush, password hashing and rating/channel policy.
- EPG tests: 2 passed, including Brazilian age/content-bit decoding.
- Isolated native UI checks passed for transport geometry, equal rockers, receiver/audio actions, all three DAC materials, Live and DAC Rec/CC hover states, popup text, captions, maximize/restore and fullscreen.
- New settings UI checks passed for preset values, manual slider persistence, Custom/Neutral switching, password setup, incorrect-password rejection and temporary unlock. The display query reported Windows HDR Off; testing did not change Windows HDR.
- Native move/activation regression: zero gray rim pixels before and after; timer background RGB (30,20,16) unchanged across the synchronous update.
- Tests launched only isolated --ui-preview processes. Actual tuner reception, GPU frame-step smoothness, real broadcast rating transitions, recording gates and HDR mode changes have not been exercised on hardware. No performance/FPS claim is made from the UI and CPU-only checks.
