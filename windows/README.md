# Windows sources

This directory holds the Windows-specific parts of the shared Open Volar S Rust workspace:

- `player/`: Live TV! application, Win32 media and Vulkan presentation.
- `a865r-bda/`: experimental DirectShow/BDA adapter.
- `transport/winusb.rs` and `winusb/`: WinUSB backend and driver INF files.
- `debug/native_player.rs` and `debug/app.rc`: Windows playback adapter and icon resource used by the shared Rust Debug Desk.
- `installer/` and `adapter-update/`: Inno Setup packaging and adapter update files.

Shared Rust code stays in `../crates/` and `../debug/`; Linux-specific code is in `../linux/`. Build from the workspace root with `cargo build -p a865r-tv -p a865r-debug -p a865rctl -p a865r-bda --release --locked`. See [BUILD-SOURCE.md](../BUILD-SOURCE.md) for toolchain requirements and the Windows installer command.
## Installer build on Linux

See [installer/BUILD-LINUX.md](installer/BUILD-LINUX.md) to compile the Windows installer with Inno Setup under Wine. Version 0.9.5 includes both BDA adapter architectures and the GNU runtime DLLs required by the cross-built application.
