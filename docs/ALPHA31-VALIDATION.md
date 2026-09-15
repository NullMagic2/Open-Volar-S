# Live TV! 0.8.0-alpha.31

## Changes

- Removed Video HDR and its warning from Picture settings. Only HDR effect remains; a previously saved Video HDR output request is disabled at startup so no invisible output mode remains enabled.
- Increased HDR effect strength from 0.16 to 0.55. The fixed, monotonic luma curve now shifts the normalized video signal by up to 0.052924 (about 5.3 percentage points), versus 1.54 points previously. Black, midpoint gray and white remain unchanged. Chroma is not amplified. This is SDR contrast shaping, not recovered scene brightness. It adds no render pass or frame history. Runtime diagnostics now include picture settings, effect strength and picture revision.
- Apply now saves authorized parental changes, including password, TV blocking, unrated policy, maximum age and channel locks. Removed Save controls. Password and folder validation prevent invalid changes from being committed; locked controls remain protected. Draft parental edits can also be applied from another settings tab.
- Aligned TV blocking with the fields above it. Removed the extra inner outlines from folder fields and comboboxes and the inner focus rectangle on selected tabs. Native text editing, selection, keyboard navigation and the outer frame remain.
- Removed the channel name above the playback toolbar. Channel information elsewhere is unchanged.

## Audio investigation

The running alpha.30 session reported two-channel Float32 PCM from a stereo broadcast, with 5.1 surround selected. The sound mixer is connected between the Windows AAC decoder and DirectSound. Surround preserves source channels; it does not create discrete 5.1 from stereo. Left and Right select that source channel and send it to both front speakers. Mono averages the channels.

No mixer defect was reproduced. An audio-only DirectShow probe decoded generated AAC transport streams with distinct per-channel tones through the native Windows decoder and the production PCM mixer. All five modes were checked on mono, stereo and 5.1 inputs (15 combinations). Stereo/Mono/Left/Right outputs matched the expected sample routing; 5.1 retained all source speakers. Four additional tests switched modes while streaming: the PCM changed on the next buffer, with no graph restart. UI command checks also confirmed selection and persistence of all five modes.

The probe supports `--mode INDEX` and `--switch-mode INDEX`, with indices 0 Stereo, 1 Mono, 2 Left, 3 Right, 4 Surround. Build with `cargo build -p a865r-tv --release --locked --example audio_probe`; run `audio_probe INPUT.ts AUDIO_PID OUTPUT.pcm --mode 0 --switch-mode 2`. Output is decoded PCM, with its negotiated format printed in the log. No speakers, tuner or GPU are used unless the existing optional `--speakers` flag is explicitly supplied.

## Validation

- 74 player tests passed, including shader validation, picture-state updates, audio mixing and parental policy checks. Two pre-existing GPU hardware tests remain intentionally ignored.
- Native preview passed missing/mismatched-password rejection, invalid-folder rejection before parental commit, valid password/channel/rating/unrated save, disabled-control authorization checks, correct and incorrect unlock attempts, and applying parental changes from another tab.
- Native preview verified TV-blocking alignment, absence of Save controls and Video HDR, removal of folder edit borders, legacy HDR output disabled, and four-language layout. Focused tabs were checked after removing their inner outline. The revised settings pages were visually inspected.
- A CPU reference checked 2,146,689 RGB values: finite output within SDR gamut, preserved black/white and no chroma amplification. This validates the effect math; it is not a measurement of GPU-rendered appearance or playback performance.
- Installer/source delivery checks cover source manifest hashes, ZIP CRC, staged-player identity and installer-copy identity.

Actual speaker output and the stronger effect's appearance/performance on the user's live playback device were not measured. Controlled audio tests found no routing fault; identical source channels can still make modes sound alike. The Windows recovery video path continues to show neutral output instead of Vulkan picture effects.

Driver and Debug Desk remain 0.7.0. Earlier validation is retained in ALPHA30-VALIDATION.md and preceding release notes.
