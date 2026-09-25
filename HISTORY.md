# Development history

This is a short map of the project's changes. [README.md](README.md) and [BUILD-SOURCE.md](BUILD-SOURCE.md) are the current usage and build instructions. The linked validation reports preserve measurements, test scope, and limitations from earlier alpha releases.

## Combined Windows and Linux source (0.8.0-alpha.43)

- One Cargo workspace and lockfile now build the shared Rust receiver, CLI, and Debug Desk for Windows or Linux. Platform code is under [`windows/`](windows/) and [`linux/`](linux/).
- Linux adds a USB character driver, Rust USB transport, direct MPEG transport-stream recording, a Rust graphical installer, and DEB/RPM packages with DKMS source. The driver compiled against published Ubuntu kernel headers from 22.04 through 26.04.1; The WSL kernel module loaded against matching 6.6.87.2 symbols; /dev/open-volar-s0 probed the A865R and captured a locked 8.2 MB MPEG-TS sample.
- Windows retains the Live TV! application, WinUSB backend, experimental BDA adapter, and Inno Setup installer. Linux now has a Rust Live TV! control window and mpv playback; the custom Windows Vulkan presentation path remains Windows-specific.
- Linux recording stores the broadcast stream without FFmpeg or reencoding. The bundled Vulkan encoder code is not connected to Linux recording; mpv is required for Linux live viewing and can also play saved files.

## Windows release milestones

| Alpha | Main change | Detailed evidence |
| --- | --- | --- |
| 43 | Receiver panel title uses the stream panel cream text color. | [Player notes](GUI/Windows/README.md) |
| 42 | Adds 16:10 and 5:4 aspect ratios, stream-window resize improvements, and native maximize/restore. | [Validation](docs/ALPHA42-VALIDATION.md) |
| 41 | Reworks playback backend switching and DirectX 11/12 picture effects with Vulkan presentation when available. | [Validation](docs/ALPHA41-VALIDATION.md) |
| 40 | Separates decoder and shader acceleration settings and synchronizes captions to presentation. | [Validation](docs/ALPHA40-VALIDATION.md) |
| 39 | Simplifies Vulkan device-loss recovery to one fresh attempt with the same decoder. | [Validation](docs/ALPHA39-VALIDATION.md) |
| 38 | Investigates Vulkan metadata, captions, and device-loss behavior; the interim comparison did not validate Vulkan playback. | [Investigation](docs/ALPHA38-INVESTIGATION.md), [validation](docs/ALPHA38-VALIDATION.md) |
| 37 | Combines the alpha.36 base with caption, recording-recovery, native-diagnostic, and AverTV signal-processing work. | [Validation](docs/ALPHA37-VALIDATION.md) |

Earlier work established the Orbit interface, background receiver startup, HDR effects, audio, captions, recording settings, Vulkan decoding and presentation, and open 0.1.4.0 firmware reception. The release-specific reports remain in [`docs/`](docs/), including [alpha.20 validation](docs/ALPHA20-VALIDATION.md), [alpha.32 validation](docs/ALPHA32-VALIDATION.md), and the [firmware reception report](docs/RF_INTEGRATION_0.1.4.md). Those reports describe the tested hardware and limits; they are not claims of validation on every receiver, kernel, GPU, or broadcast.

The Windows application and experimental adapter retain GPL-3.0-only obligations through the combined receiver implementation. See [LICENSES.md](LICENSES.md), [THIRD_PARTY.md](THIRD_PARTY.md), and [LICENSES.md](LICENSES.md).