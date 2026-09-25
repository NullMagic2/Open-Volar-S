# Recording on Windows, Linux and WSL

Both frontends use the same capture filter and export implementation.

- The selected service ID is mandatory. A recording cannot silently substitute a mobile feed or another channel on the multiplex.
- Capture includes that service's video, audio tracks and caption packets. The growing capture contains only that service, with a matching PAT and the original PMT.
- Capture starts with video sequence headers/keyframe and fresh AAC LATM configuration. Each PES is committed whole; stopping discards incomplete final packets.
- At recording start, the application snapshots the selected output resolution, deinterlacing mode and picture settings (saturation, brightness, contrast, temperature preset and SDR HDR effect).
- After Stop, GPU processing applies those settings and hardware encoding produces the final H.264 TS. Audio tracks are copied without resampling; ISDB caption metadata is restored. Seeking during recording does not change capture or the settings snapshot.
- Native size retains the source dimensions. Other sizes produce the selected dimensions. This is upscaling, not reconstruction of absent broadcast detail.
- Monitor ICC calibration is display-specific and is not baked into the recording. The HDR effect is the application's bounded SDR effect, not an HDR-format export.
- The final file is published only after a full video/all-audio decode check. That check uses CPU decoding for validation only; it does not process or encode the saved pictures.
- Failures preserve the selected-service `.broadcast.ts`, the settings and diagnostic logs. Existing output files are never overwritten. Concurrent exports in one application are serialized.
- After successful publication, the GUI removes its own temporary broadcast capture; the library keeps the rendered recording. The recovery command never removes its input.

## GPU dependencies

Windows and WSL use a Windows FFmpeg executable with D3D11VA, Vulkan, libplacebo (`custom_shader_bin`), `bwdif_vulkan` and an available hardware encoder. The current fallback order is AMD AMF, NVIDIA NVENC, Intel QSV; fallback attempts are printed to the console. Windows uses its configured FFmpeg path. WSL defaults to `/mnt/c/Program Files/FFmpeg/bin/ffmpeg.exe`; `OVS_RECORDING_FFMPEG` can override the path.

Native Linux requires a recent FFmpeg build with Vulkan/libplacebo and VAAPI, NVENC or QSV encoding. Distribution FFmpeg builds without libplacebo cannot produce the rendered export; capture and settings remain available and the application reports the failure. The VAAPI device defaults to `/dev/dri/renderD128`, overridable with `OVS_RECORDING_VAAPI_DEVICE`. Native Linux hardware export has not been exercised on physical Linux hardware in this WSL environment.

There is no software video encoder fallback in the new GUI recording path.

## Recovery tools

From the combined source workspace:

```
cargo run --locked -p a865r-debug --example record_selected_service -- INPUT.ts SELECTED.ts PROGRAM_ID
cargo run --locked -p a865r-debug --example export_recording -- SELECTED.ts OUTPUT.ts SETTINGS.json
```

Use a fresh output path for recovery. The first command preserves the selected broadcast without transcoding. The second applies the settings saved next to the capture. Export logs, settings and a success report are stored in the output's `.export` directory.

## Verification on 2026-09-24

- Shared Windows tests: 23 passed. Shared Linux tests: 27 passed, one unrelated ignored test.
- Three Linux recording/rewind tests passed; GPU picture/persistence regression passed.
- The user's HD multiplex was filtered to service 16960, excluding the 320×180 mobile service.
- Windows and WSL exports of that capture both completed using D3D11VA → Vulkan/libplacebo → AMD AMF on the RX 7900 XTX, at 2560×1440 with custom saturation, brightness, contrast, warm preset and HDR effect.
- Full video and all four audio tracks passed decode verification. ISDB caption descriptors were restored.
- A flat-color export changed the pixels in the expected direction and retained the requested output dimensions.

The original user's recording and saved channel list were not modified.
