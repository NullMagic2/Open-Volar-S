# Third-party code and assets

The [LICENSES.md](LICENSES.md) lists components included in the Windows or Linux release whose licenses require notice or attribution text. Where a dependency offers alternative licenses, the notice file names the option used. Rust crate names and versions in those files identify their sources on [crates.io](https://crates.io/).

- The receiver adapts algorithms and register tables from [recfsusb2i](https://github.com/jeeb/recfsusb2i), copyright 2015–2016 trinity19683, at commit `a2eb8fd43fc51195b1053027616b6bfb491c42f0`. The Rust changes add board validation, state and calibration bounds, USB ownership, cancellation, streaming, and recording. GPLv3 applies to the combined receiver; its text is in [LICENSES.md](LICENSES.md). No proprietary ITE firmware header was copied.
- The Vulkan player incorporates vendored `gpu-video` (MIT), `h264-reader` (MIT option), and `broadcast-parser` (Apache-2.0). Their source and patch notes are under `third-party/`.
- The Windows and Linux interfaces compile the vendored `libaribcaption` decoder (ISC). Its source is under `third-party/libaribcaption`.
- The Linux release includes Selawik fonts. Their OFL text is included in [LICENSES.md](LICENSES.md).
- The Windows installer is built with Inno Setup 6. FFmpeg, ffprobe, and mpv are external runtime tools selected by the user; their binaries are not included in this source tree or the release packages described here.

Original Open Volar S material has the additional MIT offer described in [LICENSES.md](LICENSES.md). Third-party code keeps its own terms.
