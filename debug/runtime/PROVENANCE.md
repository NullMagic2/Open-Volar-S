# Presentation runtime

Previously tested unmodified mpv.exe and d3dcompiler_43.dll came from the mpv.io-listed shinchiro Windows build, release 20260903, asset `mpv-x86_64-20260903-git-69e63f425a.7z`.

Archive SHA-256: `418dbfb5feb851cbed33d6c05d8481ba71802621bfd6efe8974522b28d42ac97`.

Release: https://github.com/shinchiro/mpv-winbuild-cmake/releases/tag/20260903

Player source: https://github.com/mpv-player/mpv/tree/69e63f425a

Build recipes and linked dependency sources: https://github.com/shinchiro/mpv-winbuild-cmake

The mpv runtime is external and is not bundled here. This records its upstream build/source provenance. FFmpeg/ffprobe remain separately installed dependencies. `A865R_MPV` can select another compatible executable. No monitor ICC payload is bundled.

The tested player reports mpv v0.41.0-1023-g69e63f425 and libplacebo v7.371.0 with Little CMS and Vulkan support.
