# Open Volar S — Project Tree

This tree records the original Windows source-package snapshot and retains its historical paths, including the former license directory now covered by `LICENSES.md`. The combined workspace now places platform code under `windows/` and `linux/`; see [windows/README.md](windows/README.md) and [linux/README.md](linux/README.md). This file intentionally uses **one uninterrupted tree**. Every visible node is boxed, and its description is written inside the same box. Parallel implementations, fallbacks, compatibility code, diagnostics, vendored source, generated/support material, and possible dead/orphan code are left where they actually live instead of being reorganized into a cleaner conceptual architecture.

```text
┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ open-volar-s/: Repository root for the complete Open Volar S userspace tuner, Live TV, compatibility, │
│ diagnostics, firmware, tooling, vendored dependencies, and documentation source package.              │
└───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│
├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │ .gitignore: Git ignore rules for build products, generated files, and local development artifacts. │
│  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │ Cargo.toml: Root Rust workspace definition; declares first-party members and applies the local │
│  │ wgpu-hal crates.io override.                                                                   │
│  └────────────────────────────────────────────────────────────────────────────────────────────────┘
├──┌─────────────────────────────────────────────────────────────────────────────┐
│  │ Cargo.lock: Locked Rust dependency graph for reproducible workspace builds. │
│  └─────────────────────────────────────────────────────────────────────────────┘
├──┌───────────────────────────────────────────────────────────────────────────────┐
│  │ README.md: Main project overview, usage, build, and repository documentation. │
│  └───────────────────────────────────────────────────────────────────────────────┘
├──┌──────────────────────────────────────────────┐
│  │ HISTORY.md: Version and development history. │
│  └──────────────────────────────────────────────┘
├──┌─────────────────────────────────────────────────────────────────────┐
│  │ BUILD-SOURCE.md: Instructions for building the project from source. │
│  └─────────────────────────────────────────────────────────────────────┘
├──┌──────────────────────────────────────────────────────────────────┐
│  │ LIVE-TV-ORBIT.md: Documentation for the Orbit Live TV interface. │
│  └──────────────────────────────────────────────────────────────────┘
├──┌──────────────────────────────────────────────────────────────────────────────┐
│  │ LIVE-TV-VIEWER.md: Documentation for the physical-TV-style Viewer interface. │
│  └──────────────────────────────────────────────────────────────────────────────┘
├──┌──────────────────────────────────────────────────────────────────────────────────┐
│  │ THIRD_PARTY.md: Third-party source, dependency, licensing, and provenance notes. │
│  └──────────────────────────────────────────────────────────────────────────────────┘
├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │ SOURCE-MANIFEST.json: Machine-readable manifest describing the contents/provenance of the source │
│  │ package.                                                                                         │
│  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
├──┌─────────────────────────────────────────────────────┐
│  │ LICENSES.md: Project and third-party license texts. │
│  └─────────────────────────────────────────────────────┘
├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │ PROJECT_TREE.md: This uninterrupted boxed architecture/filesystem tree with inline descriptions. │
│  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
├──┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  │ crates/: First-party Rust crates for hardware control, command-line diagnostics, and the │
│  │ DirectShow/BDA compatibility layer.                                                      │
│  └──────────────────────────────────────────────────────────────────────────────────────────┘
│  │
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ a865r-bda/: Userspace DirectShow/BDA compatibility adapter, built both as a Rust library and a │
│  │  │ registerable COM DLL.                                                                          │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ Cargo.toml: Rust package manifest for a865r-bda; declares package metadata, features, and │
│  │  │  │ dependencies.                                                                             │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ examples/: Standalone BDA/DirectShow interoperability, graph, demux, and caption probes. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ caption_probe.rs: CPU-only regression harness for the caption/stream-facing DirectShow transform path; │
│  │  │  │  │ opens no tuner, GPU, decoder, or renderer.                                                             │
│  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌─────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ demux_probe.rs: CPU-only H.264 media-type/demux regression harness. │
│  │  │  │  └─────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ discover.rs: Enumerates relevant DirectShow/BDA categories and attempts to bind discovered filters. │
│  │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ graph_capture.rs: Builds an independent DirectShow graph, tunes through COM TV interfaces, and writes │
│  │  │  │  │ a bounded capture without directly driving `Device`/`Receiver`.                                       │
│  │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ network_provider.rs: Builds a graph using the real Windows DVB-T Network Provider to verify BDA │
│  │  │  │  │ antenna/tuning interoperability.                                                                │
│  │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │ process_signal.rs: Minimal CLI wrapper around `a865r_bda::signal` processing for an input/output TS │
│  │  │     │ pair and selected processing preset.                                                                │
│  │  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ src/: BDA/DirectShow implementation: receiver session, COM controls, filters/pins, registration, clock │
│  │     │ recovery, and external-client processing.                                                              │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │
│  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ backend.rs: Owns tune/status/session state and the worker that opens `liba865r`, prepares the      │
│  │     │  │ receiver, receives TS bytes, and publishes status/data to DirectShow filters. Also supports replay │
│  │     │  │ sessions.                                                                                          │
│  │     │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ clock_recovery.rs: Playback-only recovery for services that advertise a PCR PID but fail to provide │
│  │     │  │ usable PCR. Observes timestamps and injects a recovered clock without rewriting PES                 │
│  │     │  │ payload/timestamps; raw recordings bypass it.                                                       │
│  │     │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ controls.rs: Implements BDA COM control/property interfaces such as device-control change        │
│  │     │  │ transactions, frequency/bandwidth controls, signal statistics, and selected AVerMedia-compatible │
│  │     │  │ property behavior expected by clients.                                                           │
│  │     │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ filter.rs: The largest BDA module. Implements source/capture/video filters, pins, enumerators,         │
│  │     │  │ media-type negotiation, delivery workers, NV12 frame preparation, replay/clocked/observed sources, and │
│  │     │  │ helper constructors used by both external clients and `player/native.rs`.                              │
│  │     │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ lib.rs: Declares the BDA/DirectShow modules, tracks COM object lifetime, defines common HRESULT   │
│  │     │  │ helpers/tracing, and exports `DllGetClassObject`, unload, register, unregister, and install entry │
│  │     │  │ points.                                                                                           │
│  │     │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ pin_control.rs: Implements pin-control identity/category information required by DirectShow/BDA pin │
│  │     │  │ interfaces.                                                                                         │
│  │     │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ registration.rs: Registers/unregisters only this project's CLSIDs and DirectShow categories in the │
│  │     │  │ requested registry scope.                                                                          │
│  │     │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │        │ signal.rs: Stores processing configuration for external BDA clients, filters/processes TS service      │
│  │        │ data, manages packet/section metadata, can invoke processing jobs, and restores caption metadata where │
│  │        │ required. This path is separate from the Live TV window.                                               │
│  │        └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ a865rctl/: Command-line frontend exposing low-level probing, firmware, register/I2C, capture, and │
│  │  │ transport-stream diagnostics.                                                                     │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ Cargo.toml: Rust package manifest for a865rctl; declares package metadata, features, and dependencies. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────┐
│  │     │ src/: Source for the a865rctl command-line executable. │
│  │     └────────────────────────────────────────────────────────┘
│  │     │
│  │     └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │        │ main.rs: Single-file command dispatcher for hardware probing, firmware extraction/build/probe, cold │
│  │        │ reset/USB cycling, initialization and diagnosis, register and I²C access, EEPROM dump, raw capture, │
│  │        │ and MPEG-TS analysis. It exposes low-level research functions without putting them into the GUI.    │
│  │        └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  └──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│     │ liba865r/: Core userspace A865R hardware library: WinUSB transport, device/protocol control, │
│     │ tuner/OFDM logic, firmware, MPEG-TS, EPG, and IR handling.                                   │
│     └──────────────────────────────────────────────────────────────────────────────────────────────┘
│     │
│     ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │ Cargo.toml: Rust package manifest for liba865r; declares package metadata, features, and dependencies. │
│     │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     ├──┌─────────────────────────────────────────────────────────────┐
│     │  │ src/: Implementation modules for the core hardware library. │
│     │  └─────────────────────────────────────────────────────────────┘
│     │  │
│     │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │  │ api.rs: Defines dependency-free application-facing enums and status/configuration structures: render │
│     │  │  │ backend, resolution, deinterlacing, color-profile selection/status, playback settings, device        │
│     │  │  │ capabilities, and playback status. Keeps GUI/policy concepts out of the low-level protocol modules.  │
│     │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │  │ channel_plan.rs: Provides supported 6 MHz ISDB-T UHF frequency validation and scan lists, including │
│     │  │  │ the Brazil UHF plan and custom scan validation.                                                     │
│     │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │  │ device.rs: Discovers and safely identifies the A865R, probes EEPROM/chip/firmware state, loads   │
│     │  │  │ firmware, exposes receiver/IR access, and performs bounded raw MPEG-TS capture. This is the main │
│     │  │  │ high-level hardware object above `Transport` and `Protocol`.                                     │
│     │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │  │ eeprom.rs: Reads documented AF9035/IT9135 EEPROM windows and interprets board layout, tuner/IF/IR   │
│     │  │  │ fields without inventing unknown meanings. Includes logic for layouts where EEPROM presence must be │
│     │  │  │ established before interpreting bytes.                                                              │
│     │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │  │ epg.rs: Parses CRC-checked ISDB EIT event information into program-guide events, including time/text │
│     │  │  │ and minimum-age/rating information.                                                                  │
│     │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │  │ error.rs: Defines the library-wide `Error` enum and `Result` alias used by transport, protocol, │
│     │  │  │ firmware, receiver, and tools.                                                                  │
│     │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │  │ firmware.rs: Parses IT9175 scatter firmware, validates region bounds/descriptors, builds open images, │
│     │  │  │ and can extract the user's reference firmware payload from the supported original driver without      │
│     │  │  │ redistributing it in source.                                                                          │
│     │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │  │ lib.rs: Exposes the hardware library, USB VID/PID constants, public modules, transport traits, and the │
│     │  │  │ core `Device`/firmware/receiver API surface. `receiver_tables.rs` remains private behind this root.    │
│     │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │  │ open_firmware/: Reconstructed/open IT9175 firmware service code and startup/calibration data used by │
│     │  │  │ the firmware generator.                                                                              │
│     │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │  │  │
│     │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │  │  │ services.rs: Implements reconstructed downloaded OFDM-side service routines and their ABI-visible │
│     │  │  │  │ register/data interactions: acquisition, measurement, correction, tracking, quality, callbacks,   │
│     │  │  │  │ timer/layer handling, and related service dispatch.                                               │
│     │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │     │ startup_data.rs: Returns numeric startup/calibration/lookup tables reconstructed from the reference │
│     │  │     │ board download. It intentionally stores data separately from executable firmware logic and avoids   │
│     │  │     │ guessing unknown units.                                                                             │
│     │  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │  │ open_firmware.rs: Generates independently authored coordinated-boot/probe firmware using an internal │
│     │  │  │ 8051 assembler and explicitly reconstructed ROM/RAM contracts. Owns link/OFDM entry points,          │
│     │  │  │ vector/callback setup, and generated scatter regions.                                                │
│     │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │  │ protocol.rs: Encodes/decodes the framed ITE/AF9035-family command protocol: register I/O, I²C │
│     │  │  │ transactions, firmware scatter transfer, IR polling, response validation, checksums, sequence │
│     │  │  │ handling, and bounded transfer sizes.                                                         │
│     │  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│     │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │  │ receiver.rs: Implements the original-board UHF ISDB-T receiver state machine: initialization,     │
│     │  │  │ calibration, frequency tuning, lock/status reporting, signal/quality reads, and oscillator/tuning │
│     │  │  │ calculations.                                                                                     │
│     │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │  │ receiver_tables.rs: Contains large initialization/register tables consumed by `receiver.rs`. It is │
│     │  │  │ private implementation data rather than an independent public API.                                 │
│     │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │  │ remote.rs: Polls the device firmware for raw four-byte infrared codes, preserves full raw key identity │
│     │  │  │ for learning, and exposes only cautious NEC interpretation candidates.                                 │
│     │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │  │ transport/: Platform transport abstraction and the Windows WinUSB implementation beneath the device │
│     │  │  │ protocol.                                                                                           │
│     │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │  │  │
│     │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │  │  │ mod.rs: Defines the platform-neutral `Transport` trait and bulk-endpoint description. Selects WinUSB │
│     │  │  │  │ on Windows and an explanatory unsupported transport elsewhere, keeping protocol code independent of  │
│     │  │  │  │ the Windows API.                                                                                     │
│     │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │     │ winusb.rs: Implements device enumeration, interface opening, pipe discovery, overlapped WinUSB I/O, │
│     │  │     │ command exchange, bulk reads, and physical USB-port cycling using direct Windows SDK FFI.           │
│     │  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │     │ ts.rs: Incrementally resynchronizes 188-byte transport packets, checks continuity/transport errors, │
│     │     │ assembles PSI sections, validates CRCs, parses PAT/PMT/service metadata/audio languages/caption     │
│     │     │ profiles, and feeds EPG event parsing.                                                              │
│     │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     └──┌────────────────────────────────────────────────────────────────────┐
│        │ tests/: Regression tests for low-level hardware/protocol behavior. │
│        └────────────────────────────────────────────────────────────────────┘
│        │
│        └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│           │ protocol.rs: Regression tests for framed protocol behavior using test/fake transport state rather than │
│           │ real hardware.                                                                                         │
│           └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │ player/: The Live TV application, containing the Win32 UI, playback orchestration, Vulkan/Microsoft │
│  │ decode paths, audio, captions, recording, color management, and two UI families.                    │
│  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ app.manifest: Embedded process manifest controlling Windows application behavior/capabilities. │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────┐
│  │  │ app.rc: Resource script for the application icon/version/resource payload. │
│  │  └────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────┐
│  │  │ assets/: Runtime artwork and UI resources for the Live TV application. │
│  │  └────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────┐
│  │  │  │ app-icon.png: Artwork/UI/image asset: app-icon.png. │
│  │  │  └─────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────┐
│  │  │  │ app.ico: Artwork/UI/image asset: app.ico. │
│  │  │  └───────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────┐
│  │  │  │ dial.png: Artwork/UI/image asset: dial.png. │
│  │  │  └─────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────┐
│  │  │  │ live-tv-ruby.ico: Artwork/UI/image asset: live-tv-ruby.ico. │
│  │  │  └─────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ orbit/: Complete Orbit skin: fascia, controls, status indicators, materials, and action icons. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │
│  │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ audio.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                             │
│  │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ caption-screw.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, │
│  │  │  │  │ materials, controls, indicators, and action icons.                                               │
│  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ captions.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                                │
│  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ channel-chevron.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, │
│  │  │  │  │ materials, controls, indicators, and action icons.                                                 │
│  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ channel-face.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, │
│  │  │  │  │ materials, controls, indicators, and action icons.                                              │
│  │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ channel-frame.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, │
│  │  │  │  │ materials, controls, indicators, and action icons.                                               │
│  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ close.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                             │
│  │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ dial.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                            │
│  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ fascia.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                              │
│  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ fullscreen.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, │
│  │  │  │  │ materials, controls, indicators, and action icons.                                            │
│  │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ glass.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                             │
│  │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ guide.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                             │
│  │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ library.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                               │
│  │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ live.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                            │
│  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ metal.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                             │
│  │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ minimize.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                                │
│  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ module.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                              │
│  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ mono.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                            │
│  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ open.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                            │
│  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ pause.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                             │
│  │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ plastic.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                               │
│  │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ play.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                            │
│  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ pointer.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                               │
│  │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ README.md: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                             │
│  │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ record.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                              │
│  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ seek-back.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, │
│  │  │  │  │ materials, controls, indicators, and action icons.                                           │
│  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ seek-forward.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, │
│  │  │  │  │ materials, controls, indicators, and action icons.                                              │
│  │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ settings-fascia.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, │
│  │  │  │  │ materials, controls, indicators, and action icons.                                                 │
│  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ settings.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                                │
│  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ signal.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                              │
│  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ snapshot.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                                │
│  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ stereo.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                              │
│  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ stop.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │  │  │ controls, indicators, and action icons.                                                            │
│  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │ surround.png: Image resources embedded/loaded by `orbit.rs` and `dial.rs`; includes fascia, materials, │
│  │  │     │ controls, indicators, and action icons.                                                                │
│  │  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────┐
│  │  │  │ README.md: Documentation: Application icon. │
│  │  │  └─────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ viewer/: Complete physical-TV Viewer skin: chassis, keys, status lights, playback controls, and │
│  │  │  │ related artwork.                                                                                │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │
│  │  │  ├──┌──────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ chassis.png: Image resources for `viewer.rs`; chassis, keys, indicators, │
│  │  │  │  │ playback/settings/guide/snapshot controls, and related visual state.     │
│  │  │  │  └──────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ dark.png: Image resources for `viewer.rs`; chassis, keys, indicators, playback/settings/guide/snapshot │
│  │  │  │  │ controls, and related visual state.                                                                    │
│  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ fast-forward.png: Image resources for `viewer.rs`; chassis, keys, indicators, │
│  │  │  │  │ playback/settings/guide/snapshot controls, and related visual state.          │
│  │  │  │  └───────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌─────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ fullscreen.png: Image resources for `viewer.rs`; chassis, keys, indicators, │
│  │  │  │  │ playback/settings/guide/snapshot controls, and related visual state.        │
│  │  │  │  └─────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ guide.png: Image resources for `viewer.rs`; chassis, keys, indicators, │
│  │  │  │  │ playback/settings/guide/snapshot controls, and related visual state.   │
│  │  │  │  └────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ key.png: Image resources for `viewer.rs`; chassis, keys, indicators, playback/settings/guide/snapshot │
│  │  │  │  │ controls, and related visual state.                                                                   │
│  │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ live-green-off.png: Image resources for `viewer.rs`; chassis, keys, indicators, │
│  │  │  │  │ playback/settings/guide/snapshot controls, and related visual state.            │
│  │  │  │  └─────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ live-green-on.png: Image resources for `viewer.rs`; chassis, keys, indicators, │
│  │  │  │  │ playback/settings/guide/snapshot controls, and related visual state.           │
│  │  │  │  └────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ mounted-indicator-off.png: Image resources for `viewer.rs`; chassis, keys, indicators, │
│  │  │  │  │ playback/settings/guide/snapshot controls, and related visual state.                   │
│  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ mounted-indicator-on.png: Image resources for `viewer.rs`; chassis, keys, indicators, │
│  │  │  │  │ playback/settings/guide/snapshot controls, and related visual state.                  │
│  │  │  │  └───────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ pause.png: Image resources for `viewer.rs`; chassis, keys, indicators, │
│  │  │  │  │ playback/settings/guide/snapshot controls, and related visual state.   │
│  │  │  │  └────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ play.png: Image resources for `viewer.rs`; chassis, keys, indicators, playback/settings/guide/snapshot │
│  │  │  │  │ controls, and related visual state.                                                                    │
│  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌─────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ rewind.png: Image resources for `viewer.rs`; chassis, keys, indicators, │
│  │  │  │  │ playback/settings/guide/snapshot controls, and related visual state.    │
│  │  │  │  └─────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌─────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ rocker.png: Image resources for `viewer.rs`; chassis, keys, indicators, │
│  │  │  │  │ playback/settings/guide/snapshot controls, and related visual state.    │
│  │  │  │  └─────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌───────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ settings.png: Image resources for `viewer.rs`; chassis, keys, indicators, │
│  │  │  │  │ playback/settings/guide/snapshot controls, and related visual state.      │
│  │  │  │  └───────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌───────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ snapshot.png: Image resources for `viewer.rs`; chassis, keys, indicators, │
│  │  │  │  │ playback/settings/guide/snapshot controls, and related visual state.      │
│  │  │  │  └───────────────────────────────────────────────────────────────────────────┘
│  │  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │ stop.png: Image resources for `viewer.rs`; chassis, keys, indicators, playback/settings/guide/snapshot │
│  │  │     │ controls, and related visual state.                                                                    │
│  │  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ volume-buttons-pressed.png: Artwork/UI/image asset: volume-buttons-pressed.png. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────┐
│  │     │ volume-buttons.png: Artwork/UI/image asset: volume-buttons.png. │
│  │     └─────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ build.rs: Compiles libaribcaption + `caption_bridge.cpp`, emits its generated configuration header, │
│  │  │ links DirectWrite/Direct2D/WIC dependencies, compiles the Windows `.rc`, and embeds the application │
│  │  │ manifest.                                                                                           │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ caption_bridge.cpp: Exception-contained C ABI wrapper around libaribcaption. Owns decoder/renderer  │
│  │  │ lifetime, decodes caption packets, renders merged images, selects DirectWrite fonts, flushes state, │
│  │  │ and exposes image buffers to Rust.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ caption_layout.hpp: Re-centers and safely rewraps Latin Portuguese/Spanish/English caption text while │
│  │  │ preserving timing/colors/identity and leaving non-Latin/ruby/broadcast-specific graphics untouched.   │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ Cargo.toml: Rust package manifest for player; declares package metadata, features, and dependencies. │
│  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────┐
│  │  │ examples/: Standalone player capability/diagnostic probes. │
│  │  └────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ audio_probe.rs: Small standalone probe for audio-related capabilities/behavior. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ video_capabilities.rs: Reports video/graphics capability information used when diagnosing supported │
│  │     │ presentation/decode paths.                                                                          │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────┐
│  │  │ README.md: Documentation: Alpha.40 update. │
│  │  └────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ src/: Live TV application modules. The architecture intentionally contains parallel decode, │
│  │  │ presentation, picture-processing, UI, and fallback paths.                                   │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ app_icon.rs: Manages native application/window icon resources and icon assignment for the Win32 │
│  │  │  │ interface.                                                                                      │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ aspect.rs: Represents Auto/4:3/16:9/16:10/5:4 display choices, handles conventional 704-in-720 SD │
│  │  │  │ active-aperture metadata, and returns display ratios without changing processing resolution.      │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ audio.rs: Defines decoded PCM formats/output modes and applies stereo/surround mixing/downmix policy │
│  │  │  │ after broadcast audio decoding. Compressed broadcast audio and recordings remain untouched.          │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ audio_health.rs: Measures the application's own output-session health/behavior without │
│  │  │  │ loopback-recording system audio.                                                       │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ backend.rs: Defines user-visible playback backend and picture-shader choices, filters them by measured │
│  │  │  │ Vulkan/DX12/DX11 availability, loads persisted IDs, and prevents unsupported combinations. This is     │
│  │  │  │ distinct from `a865r-bda/src/backend.rs`.                                                              │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ canvas.rs: Calculates viewport/processing geometry independently of window size and contains the small │
│  │  │  │ GPU canvas pipeline/vertices used to place processed video into the target surface.                    │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ captions.rs: Accepts timestamped ISDB caption packets, maintains caption clock/state, calls the C++ │
│  │  │  │ ARIB bridge, converts rendered caption images, and supplies them to presentation without            │
│  │  │  │ copying/decoding video frames.                                                                      │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ capture_only.rs: Implements explicit no-window diagnostic capture arguments/path. It records without │
│  │  │  │ constructing a decoder or graphics device.                                                           │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ countries.rs: Maps country/region selection to default frequency and scan lists used by the player. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ dial.rs: Native dial control using supplied artwork; handles drag-to-volume mapping and animated │
│  │  │  │ interpolation of dial/pointer state.                                                             │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ display_hdr.rs: Reads HDR/output capabilities and related white-level scaling from Windows; it is │
│  │  │  │ explicitly read-only and never changes the system HDR setting.                                    │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ epg_source.rs: Observes TS data already available to playback, derives EPG/current-title/audio/caption │
│  │  │  │ track data, and updates player control state without opening another receiver path.                    │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ frame_pool.rs: Bounded lease pool that prevents a CPU-visible frame slot from being reused while │
│  │  │  │ commands using its GPU image are still in flight.                                                │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ gpu_work.rs: Bounds retained GPU submissions and associated leases until the graphics API reports │
│  │  │  │ completion, avoiding unbounded in-flight resource retention.                                      │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ guide.rs: Native guide window: ingests program events, groups stations/dates/ranges, paints and shapes │
│  │  │  │ rows/details, handles selection/filtering, and reacts to language changes.                             │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ i18n.rs: Loads the embedded translation catalogue, tracks language selection, translates runtime UI │
│  │  │  │ strings, refreshes controls/tabs, and preserves broadcast metadata/filenames/settings values        │
│  │  │  │ unchanged.                                                                                          │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ icc.rs: Loads/validates monitor or selected ICC profiles, uses the Windows color-management system to │
│  │  │  │ generate the color transform/3D LUT, owns transform state/statistics, and exposes data for GPU or     │
│  │  │  │ CPU/Windows processing without modifying recorded TS bytes.                                           │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ icc_dx12.rs: DirectX 12/wgpu compute counterpart for picture and ICC processing when the Microsoft │
│  │  │  │ decode path selects DX12 processing.                                                               │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ icc_gpu.rs: Applies the Windows-CMM-generated 3D LUT using a Direct3D 11 compute shader; also provides │
│  │  │  │ availability probing and a processor abstraction.                                                      │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ main.rs: Native Win32 application entry point and central state machine. Creates/coordinates windows │
│  │  │  │ and controls, persists settings, starts/stops playback, handles                                      │
│  │  │  │ scan/channel/audio/captions/recording/snapshot/guide/settings commands, switches Orbit/Viewer        │
│  │  │  │ behavior, reports status, and dispatches work to the media modules.                                  │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ native.rs: Builds and owns the actual Windows media graph. Selects a service, creates the Rust/BDA TS  │
│  │  │  │ source and Microsoft MPEG-2 demux, selects Vulkan or Microsoft H.264 decode, selects Vulkan            │
│  │  │  │ presentation or EVR, configures audio/captions/ICC, handles seeks/commands/recording, reports pipeline │
│  │  │  │ state, and performs bounded graphics-device recovery. Despite its name, it is not only the Microsoft   │
│  │  │  │ fallback; it coordinates all current playback combinations.                                            │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ orbit.rs: Large owner-drawn Win32 implementation of the Orbit interface: fascia/material artwork,    │
│  │  │  │ tabs, fields, buttons, animation, hover/focus, settings layout, native accessibility/input behavior, │
│  │  │  │ and icon/texture rendering.                                                                          │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ osd.rs: Implements the transparent Classic Tube on-screen display using recovered AVerTV-like │
│  │  │  │ colors/font defaults for channel/volume/status overlays.                                      │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ pacing.rs: Uses the target display's vblank timing rather than polling sleeps to pace Vulkan │
│  │  │  │ presentation and tracks interval/margin behavior.                                            │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ parental.rs: Implements password-protected channel/rating restrictions, password     │
│  │  │  │ derivation/verification, persisted policy, and process-local temporary unlock state. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ picture.rs: Holds brightness/contrast/saturation/effect/preset state and UI identifiers. Picture │
│  │  │  │ controls are conceptually applied before monitor ICC conversion.                                 │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ picture_compute.wgsl: Shared WGSL compute program for picture adjustments and ICC 3D-LUT application, │
│  │  │  │ used by the DX12 path and validated from the ICC module.                                              │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ recording_finalize.rs: Finalizes a raw TS recording into the selected-service output, verifies it on │
│  │  │  │ CPU, invokes the software repair route only when necessary, and publishes the finished file.         │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ recordings.rs: Resolves/defaults recording folders, validates directories, creates filesystem-safe │
│  │  │  │ names, generates new output paths, and presents folder selection.                                  │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ scrollbars.rs: Skins native combo-list scrollbars while retaining the underlying Windows list │
│  │  │  │ selection, keyboard, wheel, and capture behavior.                                             │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ snapshots.rs: Resolves snapshot folders and provides CPU BMP encoding shared by both presenter routes. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ startup.rs: Optional per-user startup configuration. Reads/writes the HKCU Run entry and supports a │
│  │  │  │ background preparation mode without leaving a resident process or tuning the device.                │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ timeline.rs: Builds/updates an incremental transport-stream clock index whose byte offsets refer to │
│  │  │  │ the original unmodified recording; translates between live/relative positions and seek offsets.     │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ translations.json: Embedded multilingual UI string catalogue consumed by `i18n.rs`. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ video.wgsl: Main Vulkan/wgpu video rendering shader used by `vulkan.rs` for │
│  │  │  │ sampling/conversion/composition into the output surface.                    │
│  │  │  └─────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ viewer.rs: Separate viewing-console UI based on the approved physical-TV face. Owns frame geometry, │
│  │  │  │ background/skin artwork, controls, hit testing, maximize/options behavior, menu drawing, and viewer │
│  │  │  │ chrome.                                                                                             │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ vkdecode.rs: Receives compressed H.264 elementary-stream bytes from the DirectShow graph, keeps        │
│  │  │  │ parser/decoder work on one worker, manages bounded input reservations, and hands owned GPU textures to │
│  │  │  │ the Vulkan presenter while audio retains the DirectShow reference clock.                               │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ vulkan.rs: Owns the Vulkan/wgpu presentation device, decoded-frame queue, swapchain, frame timing,    │
│  │  │  │ shader resources, captions/picture/ICC integration, snapshots, and asynchronous presentation. Imports │
│  │  │  │ `vulkan_picture.rs` as `picture_stage` and `vulkan_pipeline.rs` as `pipeline`.                        │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ vulkan_picture.rs: Nested as vulkan::picture_stage. Applies/caches GPU picture-effect processing for │
│  │  │  │ Microsoft-decoded NV12 entering the Vulkan presentation path; it is active hybrid-path code, not an  │
│  │  │  │ orphan.                                                                                              │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ vulkan_pipeline.rs: Nested as vulkan::pipeline. Owns prepared GPU images, worker/pause/handoff │
│  │  │  │ lifetime, surface-stage preparation, snapshots, presentation, and diagnostics.                 │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ window_placement.rs: Computes centered physical-pixel placement for the related player windows while │
│  │     │ preserving their own proportions.                                                                    │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  └──┌────────────────────────────────────────────────────────────────────────────────┐
│     │ tests/: Native regression tests for caption behavior and font/layout handling. │
│     └────────────────────────────────────────────────────────────────────────────────┘
│     │
│     ├──┌────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │ caption_case_regression.cpp: C++ regression coverage for caption case/layout behavior. │
│     │  └────────────────────────────────────────────────────────────────────────────────────────┘
│     └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │ caption_font_regression.cpp: C++ regression coverage for caption font selection/rendering behavior. │
│        └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │ debug/: Debug Desk application plus reusable diagnostic library for hardware, playback, ICC, IR, and │
│  │ evidence-collection workflows.                                                                       │
│  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ app.rc: Windows resource script for debug; embeds icons/version/resources into the executable. │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────┐
│  │  │ build.ps1: PowerShell build/package helper for the Debug Desk. │
│  │  └────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ build.rs: Compiles/links the Windows icon resource when the `desk` feature is active. │
│  │  └───────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ Cargo.toml: Rust package manifest for debug; declares package metadata, features, and dependencies. │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────┐
│  │  │ examples/: Examples for the exported diagnostic/player-facing API. │
│  │  └────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ player_api.rs: Demonstrates use of the exported diagnostic/player-facing library API. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────┐
│  │  │ README.md: Documentation: Native diagnostics in alpha.37. │
│  │  └───────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────┐
│  │  │ runtime/: Runtime provenance material used by the diagnostic package. │
│  │  └───────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ PROVENANCE.md: Documents runtime/tool provenance relevant to packaged diagnostic behavior. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────┘
│  └──┌──────────────────────────────────────────────────┐
│     │ src/: Debug Desk and diagnostic-library modules. │
│     └──────────────────────────────────────────────────┘
│     │
│     ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │ color.rs: Validates ICC file structure/bounds, hashes profiles, and interprets renderer-generated │
│     │  │ color/pipeline evidence while preserving the rule that original TS recordings are never color     │
│     │  │ transformed.                                                                                      │
│     │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│     ├──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │ lib.rs: Exposes color, playback, remote, television, and native-player helpers as a reusable │
│     │  │ diagnostic library.                                                                          │
│     │  └──────────────────────────────────────────────────────────────────────────────────────────────┘
│     ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │ main.rs: eframe/egui diagnostic application for probing device/firmware/hardware state, invoking test │
│     │  │ actions, collecting JSON evidence, launching playback, and exposing research/validation workflows.    │
│     │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │ native_player.rs: Finds and launches `live-tv.exe` with an isolated profile/settings folder, places it │
│     │  │ in an owned Windows Job Object, maps diagnostic options to player settings, and enforces bounded       │
│     │  │ shutdown/lifetime. It reuses the application as a child rather than embedding its UI.                  │
│     │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │ playback.rs: Defines diagnostic playback options/control state, optional FFmpeg probe handling,  │
│     │  │ bounded child-process/lifetime management, and result reporting. Native Live TV is the preferred │
│     │  │ diagnostic playback path; FFmpeg remains an optional probe.                                      │
│     │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│     ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │ presenter_status.lua: [possible orphan/dead-code candidate] Lua presenter-status script present in the │
│     │  │ repository, but no active source include, build-script reference, or runtime loader was found in this  │
│     │  │ package.                                                                                               │
│     │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │ remote.rs: Runs bounded IR-learning sessions against `liba865r`, records raw firmware codes and │
│     │  │ cautious NEC candidates to evidence JSON, and performs no EEPROM/RF/system-key writes.          │
│     │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│     └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │ television.rs: Discovers TV services from captured TS, filters video services, scans frequencies,  │
│        │ records/watches services, and wraps device/receiver actions for diagnostics. As noted above, it is │
│        │ compiled in both library and binary module contexts.                                               │
│        └────────────────────────────────────────────────────────────────────────────────────────────────────┘
├──┌──────────────────────────────────────────────────────────────────────────────┐
│  │ firmware/: Built open firmware artifact and firmware-specific documentation. │
│  └──────────────────────────────────────────────────────────────────────────────┘
│  │
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ a865r-open-0.1.4.0.fw: Prebuilt open firmware image produced by the project's independent firmware │
│  │  │ generator/reconstruction work.                                                                     │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  └──┌────────────────────────────────────────────────────────────────────────────┐
│     │ README.md: Explains the firmware artifact and its intended use/provenance. │
│     └────────────────────────────────────────────────────────────────────────────┘
├──┌─────────────────────────────────────────────────────────────────────┐
│  │ winusb/: Alternative Windows WinUSB driver-binding INF definitions. │
│  └─────────────────────────────────────────────────────────────────────┘
│  │
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ A865R-WinUSB-WholeDevice.inf: Alternate whole-device WinUSB binding strategy retained in parallel with │
│  │  │ the normal interface binding.                                                                          │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │ A865R-WinUSB.inf: Normal WinUSB installation/binding definition used by the userspace A865R transport. │
│     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │ installer/: Inno Setup packaging, installer build automation, and installed-mode support files. │
│  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │
│  ├──┌─────────────────────────────────────────────────────────────┐
│  │  │ a865r.iss: Inno Setup definition for packaged installation. │
│  │  └─────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────┐
│  │  │ build.ps1: Drives installer/package creation. │
│  │  └───────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────┐
│  │  │ INSTALL-NOTES.txt: Installation/package notes. │
│  │  └────────────────────────────────────────────────┘
│  └──┌────────────────────────────────────────────────────────────────────────────────┐
│     │ installed-mode.txt: Marker/configuration material for installed-mode behavior. │
│     └────────────────────────────────────────────────────────────────────────────────┘
├──┌──────────────────────────────────────────────────────────────────────────┐
│  │ adapter-update/: Integrity metadata used by the adapter/update workflow. │
│  └──────────────────────────────────────────────────────────────────────────┘
│  │
│  └──┌───────────────────────────────────────────────────────────────┐
│     │ SHA256.json: Integrity hashes used by adapter-update tooling. │
│     └───────────────────────────────────────────────────────────────┘
├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │ tools/: Developer, reverse-engineering, validation, migration, USB-trace, update, and UI/power │
│  │ regression tools.                                                                              │
│  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ Adapter-UpdatePlan.ps1: Plans adapter/update actions before mutation, providing the update workflow │
│  │  │ with an explicit plan rather than embedding all decisions in the installer.                         │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ compare_firmware.py: Compares firmware images/regions to support reverse-engineering and independent │
│  │  │ open-firmware validation.                                                                            │
│  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ it9175_boot_contract.py: Models/parses the IT9175 scatter/boot contract and exposes memory-map/decode │
│  │  │ helpers used by firmware validation.                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ mcs51_service_cpu.py: Small MCS-51 execution model/emulator used to exercise reconstructed firmware │
│  │  │ service routines offline.                                                                           │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ migrate_avertv_profile.py: Migrates/imports relevant AVerTV profile/settings data into the project's │
│  │  │ format.                                                                                              │
│  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ repair-installed-bda.ps1: Repairs an installed BDA compatibility setup. This source package does not │
│  │  │ contain the compatibility/ binary layout the script expects, so it is not self-contained here.       │
│  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────┐
│  │  │ test_adapter_update.ps1: Regression coverage for adapter-update behavior. │
│  │  └───────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────┐
│  │  │ test_mcs51_service_cpu.py: Tests the offline MCS-51 service CPU model. │
│  │  └────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ test_orbit_ui.py: Automated isolated-profile UI regression/screenshot/input checks for the Orbit │
│  │  │ interface.                                                                                       │
│  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────┐
│  │  │ test_usb_trace.py: Regression tests for the offline USB trace parser. │
│  │  └───────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────┐
│  │  │ test_viewer_ui.py: Automated isolated-profile UI checks for the Viewer interface. │
│  │  └───────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ test_window_move.py: Exercises window movement/geometry with an isolated settings profile and native │
│  │  │ Win32 capture/inspection.                                                                            │
│  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ test_windows_sleep.ps1: Explicit power-management test helper using a wake timer and optional suspend; │
│  │  │ makes no persistent power-policy change.                                                               │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ Update-AverTV.ps1: Applies the supported AVerTV/Open Volar S adapter update workflow. │
│  │  └───────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ usb_trace.py: Offline USBPcap/pcapng analyzer for A865R/AF9035-family traffic. It parses captures only │
│  │  │ and explicitly does not open/write USB devices.                                                        │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │ validate_ofdm_services.py: Compares open OFDM service behavior with a user-supplied reference using │
│     │ reconstructed memory/service contracts and the MCS-51 model.                                        │
│     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │ third-party/: Vendored source dependencies retained in-tree, including Vulkan Video, H.264 parsing, │
│  │ ARIB caption rendering, and a locally patched wgpu HAL.                                             │
│  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ broadcast-parser/: Rust/C++ bridge to NVIDIA Vulkan Video parser sources used to turn broadcast H.264 │
│  │  │ bitstreams into Vulkan decode instructions.                                                           │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ build.rs: Compiles the C++ bridge and selected upstream parser sources/headers. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────┐
│  │  │  │ Cargo.lock: Locked Rust dependencies retained for broadcast-parser. │
│  │  │  └─────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ Cargo.toml: Rust package manifest for broadcast-parser; declares package metadata, features, and │
│  │  │  │ dependencies.                                                                                    │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────┐
│  │  │  │ examples/: Example/probe programs for broadcast-parser. │
│  │  │  └─────────────────────────────────────────────────────────┘
│  │  │  │
│  │  │  └──┌───────────────────────────────────────────────────┐
│  │  │     │ inspect.rs: Standalone parser-inspection example. │
│  │  │     └───────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────┐
│  │  │  │ HEADER-LICENSES.txt: Documentation/support text for HEADER LICENSES. │
│  │  │  └──────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ headers/: Bundled Vulkan and Vulkan Video headers needed by the native parser bridge. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ vk_video/: Standard Vulkan Video codec structures for H.264/H.265/AV1/VP9. │
│  │  │  │  └────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  │
│  │  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ vulkan_video_codec_av1std.h: Standard Vulkan Video codec structures for H.264/H.265/AV1/VP9. │
│  │  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ vulkan_video_codec_av1std_decode.h: Standard Vulkan Video codec structures for H.264/H.265/AV1/VP9. │
│  │  │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ vulkan_video_codec_av1std_encode.h: Standard Vulkan Video codec structures for H.264/H.265/AV1/VP9. │
│  │  │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ vulkan_video_codec_h264std.h: Standard Vulkan Video codec structures for H.264/H.265/AV1/VP9. │
│  │  │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ vulkan_video_codec_h264std_decode.h: Standard Vulkan Video codec structures for H.264/H.265/AV1/VP9. │
│  │  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ vulkan_video_codec_h264std_encode.h: Standard Vulkan Video codec structures for H.264/H.265/AV1/VP9. │
│  │  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ vulkan_video_codec_h265std.h: Standard Vulkan Video codec structures for H.264/H.265/AV1/VP9. │
│  │  │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ vulkan_video_codec_h265std_decode.h: Standard Vulkan Video codec structures for H.264/H.265/AV1/VP9. │
│  │  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ vulkan_video_codec_h265std_encode.h: Standard Vulkan Video codec structures for H.264/H.265/AV1/VP9. │
│  │  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ vulkan_video_codec_vp9std.h: Standard Vulkan Video codec structures for H.264/H.265/AV1/VP9. │
│  │  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ vulkan_video_codec_vp9std_decode.h: Standard Vulkan Video codec structures for H.264/H.265/AV1/VP9. │
│  │  │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │     │ vulkan_video_codecs_common.h: Standard Vulkan Video codec structures for H.264/H.265/AV1/VP9. │
│  │  │  │     └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  └──┌─────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │ vulkan/: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     └─────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     │
│  │  │     ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vk_enum_string_helper.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vk_icd.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vk_layer.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌───────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vk_platform.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └───────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vulkan.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vulkan_android.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌───────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vulkan_beta.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └───────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌───────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vulkan_core.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └───────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vulkan_directfb.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vulkan_fuchsia.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vulkan_ggp.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vulkan_ios.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vulkan_macos.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vulkan_metal.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌───────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vulkan_ohos.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └───────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vulkan_screen.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └─────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌─────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vulkan_vi.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └─────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vulkan_wayland.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vulkan_win32.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vulkan_xcb.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌───────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ vulkan_xlib.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │     │  └───────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │        │ vulkan_xlib_xrandr.h: Vendored Vulkan SDK/API headers required to build the parser reproducibly. │
│  │  │        └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE-2.0: Support file for LICENSE APACHE 2 within broadcast-parser. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ parser.cpp: C++ bridge/adaptation layer connecting the Rust crate to upstream Vulkan video parser │
│  │  │  │ objects.                                                                                          │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────┐
│  │  │  │ PROVENANCE.md: Documentation: Broadcast metadata parser provenance. │
│  │  │  └─────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────┐
│  │  │  │ src/: Rust-facing wrapper for the native broadcast/Vulkan parser bridge. │
│  │  │  └──────────────────────────────────────────────────────────────────────────┘
│  │  │  │
│  │  │  └──┌────────────────────────────────────────────────────────────────────────────┐
│  │  │     │ lib.rs: Rust-facing parser bridge/API and ownership around the C++ parser. │
│  │  │     └────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────┐
│  │     │ upstream/: Vendored upstream NVIDIA Vulkan Video parser and utility sources. │
│  │     └──────────────────────────────────────────────────────────────────────────────┘
│  │     │
│  │     ├──┌──────────────────────────────────────────────────────────────────────────┐
│  │     │  │ common/: Project directory for common and its subordinate files/modules. │
│  │     │  └──────────────────────────────────────────────────────────────────────────┘
│  │     │  │
│  │     │  ├──┌────────────────────────────────────┐
│  │     │  │  │ include/: Include tree for common. │
│  │     │  │  └────────────────────────────────────┘
│  │     │  │  │
│  │     │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │  │ crcgenerator.h: Vendored broadcast-parser header declaring crcgenerator interfaces/data. │
│  │     │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │  │ nvidia_utils/: Project directory for nvidia utils and its subordinate files/modules. │
│  │     │  │  │  └──────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │  │  │
│  │     │  │  │  └──┌──────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │     │ vulkan/: Project directory for vulkan and its subordinate files/modules. │
│  │     │  │  │     └──────────────────────────────────────────────────────────────────────────┘
│  │     │  │  │     │
│  │     │  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │     │  │ ycbcr_utils.h: Vendored broadcast-parser header declaring ycbcr utils interfaces/data. │
│  │     │  │  │     │  └────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │     │  │ ycbcrinfotbl.h: Vendored broadcast-parser header declaring ycbcrinfotbl interfaces/data. │
│  │     │  │  │     │  └──────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │  │     └──┌────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │        │ ycbcrvkinfo.h: Vendored broadcast-parser header declaring ycbcrvkinfo interfaces/data. │
│  │     │  │  │        └────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │  ├──┌───────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │  │ VkVideoCore/: Common upstream decode-frame/profile/capability interfaces. │
│  │     │  │  │  └───────────────────────────────────────────────────────────────────────────┘
│  │     │  │  │  │
│  │     │  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │  │  │ DecodeFrameBufferIf.h: Common upstream decode-frame/profile/capability interfaces. │
│  │     │  │  │  │  └────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │  │  │ VkVideoCoreProfile.h: Common upstream decode-frame/profile/capability interfaces. │
│  │     │  │  │  │  └───────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │  │  └──┌────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │     │ VulkanVideoCapabilities.h: Common upstream decode-frame/profile/capability interfaces. │
│  │     │  │  │     └────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │  │ VkVSCommon.h: Vendored broadcast-parser header declaring VkVSCommon interfaces/data. │
│  │     │  │  │  └──────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │     │ vulkan_interfaces.h: Vendored broadcast-parser header declaring vulkan interfaces interfaces/data. │
│  │     │  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  └──┌──────────────────────────────────────────────────────────────────────┐
│  │     │     │ libs/: Project directory for libs and its subordinate files/modules. │
│  │     │     └──────────────────────────────────────────────────────────────────────┘
│  │     │     │
│  │     │     └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │ VkCodecUtils/: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers used │
│  │     │        │ by Vulkan Video sample/parser infrastructure.                                                         │
│  │     │        └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        │
│  │     │        ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ DecoderConfig.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers │
│  │     │        │  │ used by Vulkan Video sample/parser infrastructure.                                                 │
│  │     │        │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ FrameProcessor.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers │
│  │     │        │  │ used by Vulkan Video sample/parser infrastructure.                                                  │
│  │     │        │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ Helpers.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers used by │
│  │     │        │  │ Vulkan Video sample/parser infrastructure.                                                           │
│  │     │        │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ HelpersDispatchTable.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                        │
│  │     │        │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ pattern.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers used by │
│  │     │        │  │ Vulkan Video sample/parser infrastructure.                                                           │
│  │     │        │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VkBufferResource.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers │
│  │     │        │  │ used by Vulkan Video sample/parser infrastructure.                                                    │
│  │     │        │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VkEncoderRenderFrame.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                        │
│  │     │        │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VkImageResource.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers │
│  │     │        │  │ used by Vulkan Video sample/parser infrastructure.                                                   │
│  │     │        │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VkThreadPool.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers used │
│  │     │        │  │ by Vulkan Video sample/parser infrastructure.                                                          │
│  │     │        │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VkThreadSafeQueue.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers │
│  │     │        │  │ used by Vulkan Video sample/parser infrastructure.                                                     │
│  │     │        │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VkVideoFrameOutput.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                      │
│  │     │        │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VkVideoQueue.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers used │
│  │     │        │  │ by Vulkan Video sample/parser infrastructure.                                                          │
│  │     │        │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VkVideoRefCountBase.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                       │
│  │     │        │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanBistreamBufferImpl.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                            │
│  │     │        │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanBitstreamBuffer.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                         │
│  │     │        │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanBufferPool.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers │
│  │     │        │  │ used by Vulkan Video sample/parser infrastructure.                                                    │
│  │     │        │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanCommandBufferPool.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                           │
│  │     │        │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanCommandBuffersSet.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                           │
│  │     │        │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanComputePipeline.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                         │
│  │     │        │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanDecodedFrame.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                      │
│  │     │        │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanDecoderFrameProcessor.h: Upstream                                                       │
│  │     │        │  │ resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers used by Vulkan Video │
│  │     │        │  │ sample/parser infrastructure.                                                                 │
│  │     │        │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanDescriptorSetLayout.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                             │
│  │     │        │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanDeviceContext.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                       │
│  │     │        │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanDeviceMemoryHostAccess.h: Upstream                                                      │
│  │     │        │  │ resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers used by Vulkan Video │
│  │     │        │  │ sample/parser infrastructure.                                                                 │
│  │     │        │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanDeviceMemoryImpl.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                          │
│  │     │        │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanDisplayFrame.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                      │
│  │     │        │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanEncoderFrameProcessor.h: Upstream                                                       │
│  │     │        │  │ resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers used by Vulkan Video │
│  │     │        │  │ sample/parser infrastructure.                                                                 │
│  │     │        │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanEncoderInputFrame.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                           │
│  │     │        │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanFenceSet.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers │
│  │     │        │  │ used by Vulkan Video sample/parser infrastructure.                                                  │
│  │     │        │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanFilter.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers used │
│  │     │        │  │ by Vulkan Video sample/parser infrastructure.                                                          │
│  │     │        │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanFilterYuvCompute.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                          │
│  │     │        │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanFrame.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers used │
│  │     │        │  │ by Vulkan Video sample/parser infrastructure.                                                         │
│  │     │        │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanQueryPoolSet.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                      │
│  │     │        │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanSamplerYcbcrConversion.h: Upstream                                                      │
│  │     │        │  │ resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers used by Vulkan Video │
│  │     │        │  │ sample/parser infrastructure.                                                                 │
│  │     │        │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanSemaphoreDump.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                       │
│  │     │        │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanSemaphoreSet.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                      │
│  │     │        │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanShaderCompiler.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                        │
│  │     │        │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanVideoDisplayQueue.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                           │
│  │     │        │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanVideoEncodeDisplayQueue.h: Upstream                                                     │
│  │     │        │  │ resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers used by Vulkan Video │
│  │     │        │  │ sample/parser infrastructure.                                                                 │
│  │     │        │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanVideoImagePool.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                        │
│  │     │        │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanVideoProcessor.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                        │
│  │     │        │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanVideoReferenceCountedPool.h: Upstream                                                   │
│  │     │        │  │ resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers used by Vulkan Video │
│  │     │        │  │ sample/parser infrastructure.                                                                 │
│  │     │        │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanVideoSession.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter │
│  │     │        │  │ helpers used by Vulkan Video sample/parser infrastructure.                                      │
│  │     │        │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanVideoSessionParameters.h: Upstream                                                      │
│  │     │        │  │ resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers used by Vulkan Video │
│  │     │        │  │ sample/parser infrastructure.                                                                 │
│  │     │        │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │  │ VulkanVideoUtils.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers │
│  │     │        │  │ used by Vulkan Video sample/parser infrastructure.                                                    │
│  │     │        │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │        └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │           │ YCbCrConvUtilsCpu.h: Upstream resource/session/buffer/frame/queue/descriptor/sync/image/filter helpers │
│  │     │           │ used by Vulkan Video sample/parser infrastructure.                                                     │
│  │     │           └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     └──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │        │ vk_video_decoder/: Project directory for vk video decoder and its subordinate files/modules. │
│  │        └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │        │
│  │        ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │        │  │ include/: Public/internal parser interfaces, picture-parameter containers, and byte-stream decoder │
│  │        │  │ declarations.                                                                                      │
│  │        │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │        │  │
│  │        │  ├──┌────────────────────────────────────────────────────────────────────────────────────────┐
│  │        │  │  │ NvVideoParser/: Project directory for NvVideoParser and its subordinate files/modules. │
│  │        │  │  └────────────────────────────────────────────────────────────────────────────────────────┘
│  │        │  │  │
│  │        │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│  │        │  │  │  │ nvVulkanVideoParser.h: Public/internal parser interfaces, picture-parameter containers, and │
│  │        │  │  │  │ byte-stream decoder declarations.                                                           │
│  │        │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────┘
│  │        │  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │        │  │     │ nvVulkanVideoUtils.h: Public/internal parser interfaces, picture-parameter containers, and byte-stream │
│  │        │  │     │ decoder declarations.                                                                                  │
│  │        │  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │        │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  │        │  │  │ vkvideo_parser/: Project directory for vkvideo parser and its subordinate files/modules. │
│  │        │  │  └──────────────────────────────────────────────────────────────────────────────────────────┘
│  │        │  │  │
│  │        │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │        │  │  │  │ PictureBufferBase.h: Public/internal parser interfaces, picture-parameter containers, and byte-stream │
│  │        │  │  │  │ decoder declarations.                                                                                 │
│  │        │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │        │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │        │  │  │  │ StdVideoPictureParametersSet.h: Public/internal parser interfaces, picture-parameter containers, and │
│  │        │  │  │  │ byte-stream decoder declarations.                                                                    │
│  │        │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │        │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │        │  │  │  │ VulkanVideoParser.h: Public/internal parser interfaces, picture-parameter containers, and byte-stream │
│  │        │  │  │  │ decoder declarations.                                                                                 │
│  │        │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │        │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│  │        │  │  │  │ VulkanVideoParserIf.h: Public/internal parser interfaces, picture-parameter containers, and │
│  │        │  │  │  │ byte-stream decoder declarations.                                                           │
│  │        │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────┘
│  │        │  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │        │  │     │ VulkanVideoParserParams.h: Public/internal parser interfaces, picture-parameter containers, and │
│  │        │  │     │ byte-stream decoder declarations.                                                               │
│  │        │  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │        │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │        │     │ vulkan_video_decoder.h: Public/internal parser interfaces, picture-parameter containers, and │
│  │        │     │ byte-stream decoder declarations.                                                            │
│  │        │     └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │        └──┌──────────────────────────────────────────────────────────────────────┐
│  │           │ libs/: Project directory for libs and its subordinate files/modules. │
│  │           └──────────────────────────────────────────────────────────────────────┘
│  │           │
│  │           └──┌────────────────────────────────────────────────────────────────────────────────────────┐
│  │              │ NvVideoParser/: Project directory for NvVideoParser and its subordinate files/modules. │
│  │              └────────────────────────────────────────────────────────────────────────────────────────┘
│  │              │
│  │              ├──┌───────────────────────────────────────────┐
│  │              │  │ include/: Include tree for NvVideoParser. │
│  │              │  └───────────────────────────────────────────┘
│  │              │  │
│  │              │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │              │  │  │ ByteStreamParser.h: Vendored broadcast-parser header declaring ByteStreamParser interfaces/data. │
│  │              │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │              │  ├──┌────────────────────────────────────────────────────────────────────────────────────┐
│  │              │  │  │ cpudetect.h: Vendored broadcast-parser header declaring cpudetect interfaces/data. │
│  │              │  │  └────────────────────────────────────────────────────────────────────────────────────┘
│  │              │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │              │  │  │ nvVulkanh264ScalingList.h: Vendored broadcast-parser header declaring nvVulkanh264ScalingList │
│  │              │  │  │ interfaces/data.                                                                              │
│  │              │  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │              │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │              │  │  │ nvVulkanh265ScalingList.h: Vendored broadcast-parser header declaring nvVulkanh265ScalingList │
│  │              │  │  │ interfaces/data.                                                                              │
│  │              │  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │              │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │              │  │  │ VulkanAV1Decoder.h: Vendored broadcast-parser header declaring VulkanAV1Decoder interfaces/data. │
│  │              │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │              │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │              │  │  │ VulkanH264Decoder.h: Vendored broadcast-parser header declaring VulkanH264Decoder interfaces/data. │
│  │              │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │              │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │              │  │  │ VulkanH265Decoder.h: Vendored broadcast-parser header declaring VulkanH265Decoder interfaces/data. │
│  │              │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │              │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │              │  │  │ VulkanH26xDecoder.h: Vendored broadcast-parser header declaring VulkanH26xDecoder interfaces/data. │
│  │              │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │              │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │              │  │  │ VulkanVideoDecoder.h: Vendored broadcast-parser header declaring VulkanVideoDecoder interfaces/data. │
│  │              │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │              │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │              │     │ VulkanVP9Decoder.h: Vendored broadcast-parser header declaring VulkanVP9Decoder interfaces/data. │
│  │              │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │              └──┌───────────────────────────────────┐
│  │                 │ src/: Src tree for NvVideoParser. │
│  │                 └───────────────────────────────────┘
│  │                 │
│  │                 ├──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │                 │  │ cpudetect.cpp: Vendored broadcast-parser native source implementing cpudetect functionality. │
│  │                 │  └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │                 ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │                 │  │ NextStartCodeAVX2.cpp: Vendored broadcast-parser native source implementing NextStartCodeAVX2 │
│  │                 │  │ functionality.                                                                                │
│  │                 │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │                 ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │                 │  │ NextStartCodeAVX512.cpp: Vendored broadcast-parser native source implementing NextStartCodeAVX512 │
│  │                 │  │ functionality.                                                                                    │
│  │                 │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │                 ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │                 │  │ NextStartCodeC.cpp: Vendored broadcast-parser native source implementing NextStartCodeC functionality. │
│  │                 │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │                 ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │                 │  │ NextStartCodeSSSE3.cpp: Vendored broadcast-parser native source implementing NextStartCodeSSSE3 │
│  │                 │  │ functionality.                                                                                  │
│  │                 │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │                 ├──┌───────────────────────────────────────────────────────────────────────────────────┐
│  │                 │  │ nvVulkanh264ScalingList.cpp: Vendored broadcast-parser native source implementing │
│  │                 │  │ nvVulkanh264ScalingList functionality.                                            │
│  │                 │  └───────────────────────────────────────────────────────────────────────────────────┘
│  │                 ├──┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│  │                 │  │ VulkanH264Parser.cpp: Vendored broadcast-parser native source implementing VulkanH264Parser │
│  │                 │  │ functionality.                                                                              │
│  │                 │  └─────────────────────────────────────────────────────────────────────────────────────────────┘
│  │                 └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │                    │ VulkanVideoDecoder.cpp: Vendored broadcast-parser native source implementing VulkanVideoDecoder │
│  │                    │ functionality.                                                                                  │
│  │                    └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ gpu-video/: Vendored/forked Vulkan Video stack providing GPU video device, parser, decode, encode, │
│  │  │ transcode, and wgpu integration used by the Vulkan playback route.                                 │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────┐
│  │  │  │ .cargo-ok: Support file for .cargo ok within gpu-video. │
│  │  │  └─────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ .cargo_vcs_info.json: JSON configuration, evidence, manifest, or generated data for .cargo vcs info. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────┐
│  │  │  │ .gitignore: Support file for .gitignore within gpu-video. │
│  │  │  └───────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ build.rs: Rust build script for gpu-video; prepares native/generated resources or linker/build │
│  │  │  │ configuration before compilation.                                                              │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────┐
│  │  │  │ Cargo.lock: Locked Rust dependencies retained for gpu-video. │
│  │  │  └──────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ Cargo.toml: Rust package manifest for gpu-video; declares package metadata, features, and │
│  │  │  │ dependencies.                                                                             │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────┐
│  │  │  │ Cargo.toml.orig: Support file for Cargo.toml within gpu-video. │
│  │  │  └────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────┐
│  │  │  │ CHANGELOG.md: Documentation: Changelog. │
│  │  │  └─────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ examples/: Upstream/vendor examples for decode, encode, transcode, hardware capability reporting, and │
│  │  │  │ player rendering.                                                                                     │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │
│  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ decode.rs: Upstream/general decode, encode, transcode, capability, wgpu, and sample-player programs; │
│  │  │  │  │ not the Open Volar S UI.                                                                             │
│  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ decode_wgpu.rs: Upstream/general decode, encode, transcode, capability, wgpu, and sample-player │
│  │  │  │  │ programs; not the Open Volar S UI.                                                              │
│  │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ encode.rs: Upstream/general decode, encode, transcode, capability, wgpu, and sample-player programs; │
│  │  │  │  │ not the Open Volar S UI.                                                                             │
│  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ encode_wgpu.rs: Upstream/general decode, encode, transcode, capability, wgpu, and sample-player │
│  │  │  │  │ programs; not the Open Volar S UI.                                                              │
│  │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ encode_wgpu.wgsl: Upstream/general decode, encode, transcode, capability, wgpu, and sample-player │
│  │  │  │  │ programs; not the Open Volar S UI.                                                                │
│  │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌──────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ player/: Project directory for player and its subordinate files/modules. │
│  │  │  │  └──────────────────────────────────────────────────────────────────────────┘
│  │  │  │  │
│  │  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ main.rs: Upstream/general decode, encode, transcode, capability, wgpu, and sample-player programs; not │
│  │  │  │  │  │ the Open Volar S UI.                                                                                   │
│  │  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌──────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ player/: Project directory for player and its subordinate files/modules. │
│  │  │  │  │  └──────────────────────────────────────────────────────────────────────────┘
│  │  │  │  │  │
│  │  │  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │  │ decoder.rs: Upstream/general decode, encode, transcode, capability, wgpu, and sample-player programs; │
│  │  │  │  │  │  │ not the Open Volar S UI.                                                                              │
│  │  │  │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │  │ renderer.rs: Upstream/general decode, encode, transcode, capability, wgpu, and sample-player programs; │
│  │  │  │  │  │  │ not the Open Volar S UI.                                                                               │
│  │  │  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │     │ shader.wgsl: Upstream/general decode, encode, transcode, capability, wgpu, and sample-player programs; │
│  │  │  │  │     │ not the Open Volar S UI.                                                                               │
│  │  │  │  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │     │ player.rs: Upstream/general decode, encode, transcode, capability, wgpu, and sample-player programs; │
│  │  │  │     │ not the Open Volar S UI.                                                                             │
│  │  │  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ print_hw_capabilities.rs: Upstream/general decode, encode, transcode, capability, wgpu, and │
│  │  │  │  │ sample-player programs; not the Open Volar S UI.                                            │
│  │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │ transcode.rs: Upstream/general decode, encode, transcode, capability, wgpu, and sample-player │
│  │  │     │ programs; not the Open Volar S UI.                                                            │
│  │  │     └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────┐
│  │  │  │ LICENSE: Support file for LICENSE within gpu-video. │
│  │  │  └─────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────┐
│  │  │  │ PATCHES.md: Documentation: Local changes to gpu-video 0.4.0 (MIT). │
│  │  │  └────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────┐
│  │  │  │ README.md: Documentation: gpu-video. │
│  │  │  └──────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────┐
│  │  │  │ RELEASE.md: Documentation: gpu-video release guide. │
│  │  │  └─────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────┐
│  │     │ src/: Core source modules of the vendored GPU video stack. │
│  │     └────────────────────────────────────────────────────────────┘
│  │     │
│  │     ├──┌────────────────────────────────────────────────────────────┐
│  │     │  │ adapter.rs: Selects/query adapters and video capabilities. │
│  │     │  └────────────────────────────────────────────────────────────┘
│  │     ├──┌────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ broadcast.rs: Broadcast-oriented glue used by the experimental broadcast GPU path. │
│  │     │  └────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌───────────────────────────────────────────────────────────────────┐
│  │     │  │ codec/: Codec-neutral and codec-specific Vulkan Video structures. │
│  │     │  └───────────────────────────────────────────────────────────────────┘
│  │     │  │
│  │     │  ├──┌───────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │ h264/: H.264 Vulkan Video session, picture-parameter, and encode support. │
│  │     │  │  └───────────────────────────────────────────────────────────────────────────┘
│  │     │  │  │
│  │     │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │  │ encode.rs: H.264 encode-side support retained in the vendored crate even though Live TV primarily │
│  │     │  │  │  │ consumes decode.                                                                                  │
│  │     │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │  └──┌─────────────────────────────────────────────────────────────────────┐
│  │     │  │     │ parameters.rs: H.264 Vulkan session/picture parameter construction. │
│  │     │  │     └─────────────────────────────────────────────────────────────────────┘
│  │     │  ├──┌────────────────────────────────────────────┐
│  │     │  │  │ h264.rs: H.264 codec/profile/session data. │
│  │     │  │  └────────────────────────────────────────────┘
│  │     │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │ h265/: H.265/HEVC support retained in the vendored stack; not the primary Open Volar S live-TV decode │
│  │     │  │  │ path.                                                                                                 │
│  │     │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │  │
│  │     │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │  │ encode.rs: Vendored gpu-video Rust module implementing encode support; retained from or adapted from │
│  │     │  │  │  │ the upstream dependency.                                                                             │
│  │     │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │     │ parameters.rs: Vendored gpu-video Rust module implementing parameters support; retained from or │
│  │     │  │     │ adapted from the upstream dependency.                                                           │
│  │     │  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │     │ h265.rs: Vendored gpu-video Rust module implementing h265 support; retained from or adapted from the │
│  │     │     │ upstream dependency.                                                                                 │
│  │     │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌─────────────────────────────────────────┐
│  │     │  │ codec.rs: Codec-neutral types/dispatch. │
│  │     │  └─────────────────────────────────────────┘
│  │     ├──┌─────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ device/: GPU-video device capability, queue-selection, and wgpu-bridge modules. │
│  │     │  └─────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │
│  │     │  ├──┌──────────────────────────────────────────────────────────┐
│  │     │  │  │ caps.rs: Video/device capability structures and queries. │
│  │     │  │  └──────────────────────────────────────────────────────────┘
│  │     │  ├──┌──────────────────────────────────────────────────────────────────────┐
│  │     │  │  │ queues.rs: Queue-family/queue selection for video and graphics work. │
│  │     │  │  └──────────────────────────────────────────────────────────────────────┘
│  │     │  └──┌────────────────────────────────────────────────────────────────────────────────┐
│  │     │     │ wgpu_api.rs: Bridges device state to the wgpu-facing API used by Open Volar S. │
│  │     │     └────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌────────────────────────────────────────────┐
│  │     │  │ device.rs: Core device creation/ownership. │
│  │     │  └────────────────────────────────────────────┘
│  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ instance.rs: Creates/owns Vulkan/wgpu-facing instance state and extension discovery. │
│  │     │  └──────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌─────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ lib.rs: Public crate surface for GPU video decode/encode/transcode support. │
│  │     │  └─────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ parser/: Compressed-stream parsing, access-unit splitting, reference management, and │
│  │     │  │ decoder-instruction generation.                                                      │
│  │     │  └──────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │
│  │     │  ├──┌───────────────────────────────────────────────────────────────────┐
│  │     │  │  │ au_splitter.rs: Splits compressed byte streams into access units. │
│  │     │  │  └───────────────────────────────────────────────────────────────────┘
│  │     │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │ decoder_instructions.rs: Converts parser output into decoder work/instruction structures. │
│  │     │  │  └───────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  ├──┌──────────────────────────────────────────────────┐
│  │     │  │  │ nalu_parser.rs: Parses NAL-unit content/headers. │
│  │     │  │  └──────────────────────────────────────────────────┘
│  │     │  ├──┌───────────────────────────────────────────┐
│  │     │  │  │ nalu_splitter.rs: Finds/splits NAL units. │
│  │     │  │  └───────────────────────────────────────────┘
│  │     │  └──┌─────────────────────────────────────────────────────────────────────────────┐
│  │     │     │ reference_manager.rs: Tracks decoded reference-picture relationships/state. │
│  │     │     └─────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌────────────────────────────────────────────────────────────┐
│  │     │  │ parser.rs: Parser facade used before video-session decode. │
│  │     │  └────────────────────────────────────────────────────────────┘
│  │     ├──┌────────────────────────────────────────────────────────────────────┐
│  │     │  │ shaders/: GPU conversion shaders used by the vendored video stack. │
│  │     │  └────────────────────────────────────────────────────────────────────┘
│  │     │  │
│  │     │  ├──┌──────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │ nv12_to_rgba.wgsl: WGSL GPU shader implementing nv12 to rgba processing. │
│  │     │  │  └──────────────────────────────────────────────────────────────────────────┘
│  │     │  └──┌──────────────────────────────────────────────────────────────────────────┐
│  │     │     │ rgba_to_nv12.wgsl: WGSL GPU shader implementing rgba to nv12 processing. │
│  │     │     └──────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ vulkan_decoder/: Vulkan Video decoder implementation and session/frame-resource management. │
│  │     │  └─────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │
│  │     │  ├──┌─────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │ frame_sorter.rs: Reorders/dispatches decoded frames according to display order. │
│  │     │  │  └─────────────────────────────────────────────────────────────────────────────────┘
│  │     │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │ session_resources/: Decoder image and parameter resources tied to Vulkan Video sessions. │
│  │     │  │  └──────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │  │
│  │     │  │  ├──┌───────────────────────────────────────────────────┐
│  │     │  │  │  │ images.rs: Decoder image/DPB resource management. │
│  │     │  │  │  └───────────────────────────────────────────────────┘
│  │     │  │  └──┌────────────────────────────────────────────────────────────────┐
│  │     │  │     │ parameters.rs: Session-parameter resource/lifetime management. │
│  │     │  │     └────────────────────────────────────────────────────────────────┘
│  │     │  └──┌───────────────────────────────────────────────────────────┐
│  │     │     │ session_resources.rs: Owns per-session decoder resources. │
│  │     │     └───────────────────────────────────────────────────────────┘
│  │     ├──┌────────────────────────────────────────────────────────┐
│  │     │  │ vulkan_decoder.rs: Main Vulkan decoder implementation. │
│  │     │  └────────────────────────────────────────────────────────┘
│  │     ├──┌─────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ vulkan_encoder.rs: General-purpose Vulkan video encoder retained by the dependency. │
│  │     │  └─────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ vulkan_transcoder/: Vendored Vulkan transcoder path retained upstream; not central to normal Live TV │
│  │     │  │ playback.                                                                                            │
│  │     │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │
│  │     │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │ pipeline.rs: Vendored gpu-video Rust module implementing pipeline support; retained from or adapted │
│  │     │  │  │ from the upstream dependency.                                                                       │
│  │     │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  └──┌──────────────────────────────────────────────────────────────┐
│  │     │     │ shader.wgsl: WGSL GPU shader implementing shader processing. │
│  │     │     └──────────────────────────────────────────────────────────────┘
│  │     ├──┌──────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ vulkan_transcoder.rs: General-purpose transcode path retained by the dependency. │
│  │     │  └──────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ vulkan_video/: Project directory for vulkan video and its subordinate files/modules. │
│  │     │  └──────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │
│  │     │  └──┌────────────────────────────────────────────────────────────┐
│  │     │     │ wgpu_api.rs: wgpu integration around Vulkan Video objects. │
│  │     │     └────────────────────────────────────────────────────────────┘
│  │     ├──┌────────────────────────────────────────────────────────┐
│  │     │  │ vulkan_video.rs: Vulkan Video API/session abstraction. │
│  │     │  └────────────────────────────────────────────────────────┘
│  │     ├──┌────────────────────────────────────────────────────────────────────────┐
│  │     │  │ wgpu_helpers/: wgpu helper conversions between NV12 and RGBA surfaces. │
│  │     │  └────────────────────────────────────────────────────────────────────────┘
│  │     │  │
│  │     │  ├──┌──────────────────────────────────────────────────────┐
│  │     │  │  │ nv12_to_rgba.rs: NV12-to-RGBA GPU conversion helper. │
│  │     │  │  └──────────────────────────────────────────────────────┘
│  │     │  └──┌──────────────────────────────────────────────────────┐
│  │     │     │ rgba_to_nv12.rs: RGBA-to-NV12 GPU conversion helper. │
│  │     │     └──────────────────────────────────────────────────────┘
│  │     ├──┌────────────────────────────────────────────────────────┐
│  │     │  │ wgpu_helpers.rs: Shared wgpu conversion helper facade. │
│  │     │  └────────────────────────────────────────────────────────┘
│  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ wrappers/: Low-level Vulkan command, memory, sync, pipeline, video, debug, and extension wrappers. │
│  │     │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │
│  │     │  ├──┌───────────────────────────────────────────┐
│  │     │  │  │ command.rs: Command-buffer/pool wrappers. │
│  │     │  │  └───────────────────────────────────────────┘
│  │     │  ├──┌─────────────────────────────────────────────┐
│  │     │  │  │ debug.rs: Vulkan debug/diagnostic wrappers. │
│  │     │  │  └─────────────────────────────────────────────┘
│  │     │  ├──┌────────────────────────────────────────────────────┐
│  │     │  │  │ mem.rs: Vulkan memory/resource allocation helpers. │
│  │     │  │  └────────────────────────────────────────────────────┘
│  │     │  ├──┌───────────────────────────────────────────────┐
│  │     │  │  │ pipeline.rs: Pipeline/layout wrapper helpers. │
│  │     │  │  └───────────────────────────────────────────────┘
│  │     │  ├──┌────────────────────────────────────────────────────┐
│  │     │  │  │ sync.rs: Fence/semaphore/synchronization wrappers. │
│  │     │  │  └────────────────────────────────────────────────────┘
│  │     │  ├──┌──────────────────────────────────────────────────┐
│  │     │  │  │ video.rs: Video-session/video-resource wrappers. │
│  │     │  │  └──────────────────────────────────────────────────┘
│  │     │  └──┌──────────────────────────────────────────────────────┐
│  │     │     │ vk_extensions.rs: Extension-function/feature access. │
│  │     │     └──────────────────────────────────────────────────────┘
│  │     └──┌─────────────────────────────────────┐
│  │        │ wrappers.rs: Vulkan wrapper facade. │
│  │        └─────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ h264-reader/: Vendored Rust parser for H.264 Annex-B/AVCC streams, RBSP syntax, NAL units, SPS/PPS, │
│  │  │ slices, and SEI messages.                                                                           │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────┐
│  │  │  │ .cargo-ok: Support file for .cargo ok within h264-reader. │
│  │  │  └───────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ .cargo_vcs_info.json: JSON configuration, evidence, manifest, or generated data for .cargo vcs info. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────┐
│  │  │  │ .gitattributes: Support file for .gitattributes within h264-reader. │
│  │  │  └─────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ .github/: Upstream repository automation/configuration retained with the vendored source. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │
│  │  │  ├──┌─────────────────────────────────────────────────────────────┐
│  │  │  │  │ dependabot.yml: Configuration/metadata file for dependabot. │
│  │  │  │  └─────────────────────────────────────────────────────────────┘
│  │  │  └──┌──────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │ workflows/: CI workflow definitions retained from the vendored upstream project. │
│  │  │     └──────────────────────────────────────────────────────────────────────────────────┘
│  │  │     │
│  │  │     ├──┌───────────────────────────────────────────────────────────┐
│  │  │     │  │ benchmark.yml: Configuration/metadata file for benchmark. │
│  │  │     │  └───────────────────────────────────────────────────────────┘
│  │  │     ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ pr-benchmark-upload-from-main.yml: Configuration/metadata file for pr benchmark upload from main. │
│  │  │     │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌─────────────────────────────────────────────────────────────────┐
│  │  │     │  │ pr-benchmark.yml: Configuration/metadata file for pr benchmark. │
│  │  │     │  └─────────────────────────────────────────────────────────────────┘
│  │  │     └──┌─────────────────────────────────────────────────┐
│  │  │        │ rust.yml: Configuration/metadata file for rust. │
│  │  │        └─────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────┐
│  │  │  │ .gitignore: Support file for .gitignore within h264-reader. │
│  │  │  └─────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────┐
│  │  │  │ benches/: Upstream parser benchmarks. │
│  │  │  └───────────────────────────────────────┘
│  │  │  │
│  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ bench.rs: Benchmark on a large video file. Expects a copy of [Big Buck                           │
│  │  │  │  │ Bunny](https://peach.blender.org/download/): ```text $ curl -OL                                  │
│  │  │  │  │ https://download.blender.org/peach/bigbuckbunny_movies/big_buck_bunny_1080p_h264.mov $ ffmpeg -i │
│  │  │  │  │ big_buck_bunny_1080p_h264.mov -c copy big_buck_bunny_1080p.h264 ```                              │
│  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ ci_bench.rs: Vendored h264-reader Rust module implementing ci bench support; retained from or adapted │
│  │  │  │  │ from the upstream dependency.                                                                         │
│  │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  └──┌───────────────────────────────────────┐
│  │  │     │ README.md: Documentation: Benchmarks. │
│  │  │     └───────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────┐
│  │  │  │ Cargo.lock: Locked Rust dependencies retained for h264-reader. │
│  │  │  └────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ Cargo.toml: Rust package manifest for h264-reader; declares package metadata, features, and │
│  │  │  │ dependencies.                                                                               │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────┐
│  │  │  │ Cargo.toml.orig: Support file for Cargo.toml within h264-reader. │
│  │  │  └──────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────┐
│  │  │  │ CHANGELOG.md: Documentation: Change Log. │
│  │  │  └──────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────┐
│  │  │  │ examples/: Upstream parser examples. │
│  │  │  └──────────────────────────────────────┘
│  │  │  │
│  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ decode_avcc.rs: Creates a context from an encoded [`h264_reader::avc::AVCDecoderConfigurationRecord`] │
│  │  │  │  │ and prints it.                                                                                        │
│  │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  └──┌─────────────────────────────────────────────────────────────┐
│  │  │     │ dump.rs: Example/probe program demonstrating dump behavior. │
│  │  │     └─────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: Support file for LICENSE APACHE within h264-reader. │
│  │  │  └─────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: Support file for LICENSE MIT within h264-reader. │
│  │  │  └───────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────┐
│  │  │  │ README.md: Documentation: Supported syntax. │
│  │  │  └─────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────┐
│  │  │  │ release.toml: Configuration/metadata file for release. │
│  │  │  └────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────┐
│  │     │ src/: Implementation of the vendored H.264 bitstream parser. │
│  │     └──────────────────────────────────────────────────────────────┘
│  │     │
│  │     ├──┌───────────────────────────────────────────────┐
│  │     │  │ annexb.rs: Annex-B start-code stream parsing. │
│  │     │  └───────────────────────────────────────────────┘
│  │     ├──┌────────────────────────────────────────────────────────────┐
│  │     │  │ avcc.rs: AVC configuration/length-prefixed stream parsing. │
│  │     │  └────────────────────────────────────────────────────────────┘
│  │     ├──┌─────────────────────────────────────────────────────┐
│  │     │  │ lib.rs: Crate root/common H.264 parsing interfaces. │
│  │     │  └─────────────────────────────────────────────────────┘
│  │     ├──┌──────────────────────────────────────────────────┐
│  │     │  │ nal/: NAL-unit parsers and codec syntax modules. │
│  │     │  └──────────────────────────────────────────────────┘
│  │     │  │
│  │     │  ├──┌────────────────────────────────────────────────────┐
│  │     │  │  │ mod.rs: NAL type/header/common parser definitions. │
│  │     │  │  └────────────────────────────────────────────────────┘
│  │     │  ├──┌───────────────────────────────────────┐
│  │     │  │  │ pps.rs: Picture Parameter Set parser. │
│  │     │  │  └───────────────────────────────────────┘
│  │     │  ├──┌─────────────────────────────────────────────────────┐
│  │     │  │  │ sei/: Supplemental Enhancement Information parsers. │
│  │     │  │  └─────────────────────────────────────────────────────┘
│  │     │  │  │
│  │     │  │  ├──┌────────────────────────────────────────────────────┐
│  │     │  │  │  │ buffering_period.rs: Buffering-period SEI parsing. │
│  │     │  │  │  └────────────────────────────────────────────────────┘
│  │     │  │  ├──┌──────────────────────────────────────────────┐
│  │     │  │  │  │ mod.rs: SEI message dispatch/common parsing. │
│  │     │  │  │  └──────────────────────────────────────────────┘
│  │     │  │  ├──┌────────────────────────────────────────────┐
│  │     │  │  │  │ pic_timing.rs: Picture-timing SEI parsing. │
│  │     │  │  │  └────────────────────────────────────────────┘
│  │     │  │  └──┌─────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │     │ user_data_registered_itu_t_t35.rs: ITU-T T.35 registered user-data SEI parsing. │
│  │     │  │     └─────────────────────────────────────────────────────────────────────────────────┘
│  │     │  ├──┌─────────────────────────────────────┐
│  │     │  │  │ slice/: H.264 slice syntax parsing. │
│  │     │  │  └─────────────────────────────────────┘
│  │     │  │  │
│  │     │  │  └──┌───────────────────────────────┐
│  │     │  │     │ mod.rs: Slice-header parsing. │
│  │     │  │     └───────────────────────────────┘
│  │     │  └──┌────────────────────────────────────────┐
│  │     │     │ sps.rs: Sequence Parameter Set parser. │
│  │     │     └────────────────────────────────────────┘
│  │     ├──┌──────────────────────────────────────────┐
│  │     │  │ push/: Incremental/push parsing support. │
│  │     │  └──────────────────────────────────────────┘
│  │     │  │
│  │     │  └──┌────────────────────────────────────────────┐
│  │     │     │ mod.rs: Incremental/push-style parser API. │
│  │     │     └────────────────────────────────────────────┘
│  │     └──┌────────────────────────────────────┐
│  │        │ rbsp.rs: RBSP bit/escape handling. │
│  │        └────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ libaribcaption/: Vendored libaribcaption C++ library for ARIB/ISDB caption decoding, font handling, │
│  │  │ layout, DRCS, and rendering.                                                                        │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────┐
│  │  │  │ .clang-format: Support file for .clang format within libaribcaption. │
│  │  │  └──────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────┐
│  │  │  │ .gitignore: Support file for .gitignore within libaribcaption. │
│  │  │  └────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────┐
│  │  │  │ A865R-UPSTREAM.txt: Documentation/support text for A865R UPSTREAM. │
│  │  │  └────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────┐
│  │  │  │ cmake/: Project directory for cmake and its subordinate files/modules. │
│  │  │  └────────────────────────────────────────────────────────────────────────┘
│  │  │  │
│  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ aribcaption-config.cmake.in: Support file for aribcaption config.cmake within cmake. │
│  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────────────────┐
│  │  │  │  │ EnableCMP0048.cmake: CMake build definition/support for cmake. │
│  │  │  │  └────────────────────────────────────────────────────────────────┘
│  │  │  └──┌────────────────────────────────────────────────────────────────────┐
│  │  │     │ GeneratePkgConfig.cmake: CMake build definition/support for cmake. │
│  │  │     └────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ CMakeLists.txt: Documentation: Copyright (C) 2021 magicxqq <xqq@xqq.im>. All rights reserved.. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────┐
│  │  │  │ include/: Public libaribcaption headers. │
│  │  │  └──────────────────────────────────────────┘
│  │  │  │
│  │  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │ aribcaption/: Public decoder, renderer, context, image, color, and caption API headers. │
│  │  │     └─────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     │
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ aligned_alloc.hpp: Public C and C++ caption/context/decoder/image/renderer/color APIs. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ aribcaption.h: Public C and C++ caption/context/decoder/image/renderer/color APIs. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ aribcaption.hpp: Public C and C++ caption/context/decoder/image/renderer/color APIs. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌─────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ aribcc_config.h.in: Public C and C++ caption/context/decoder/image/renderer/color APIs. │
│  │  │     │  └─────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ aribcc_export.h: Public C and C++ caption/context/decoder/image/renderer/color APIs. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ caption.h: Public C and C++ caption/context/decoder/image/renderer/color APIs. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ caption.hpp: Public C and C++ caption/context/decoder/image/renderer/color APIs. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ color.h: Public C and C++ caption/context/decoder/image/renderer/color APIs. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ color.hpp: Public C and C++ caption/context/decoder/image/renderer/color APIs. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ context.h: Public C and C++ caption/context/decoder/image/renderer/color APIs. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ context.hpp: Public C and C++ caption/context/decoder/image/renderer/color APIs. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ decoder.h: Public C and C++ caption/context/decoder/image/renderer/color APIs. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ decoder.hpp: Public C and C++ caption/context/decoder/image/renderer/color APIs. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ image.h: Public C and C++ caption/context/decoder/image/renderer/color APIs. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ image.hpp: Public C and C++ caption/context/decoder/image/renderer/color APIs. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌─────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ renderer.h: Public C and C++ caption/context/decoder/image/renderer/color APIs. │
│  │  │     │  └─────────────────────────────────────────────────────────────────────────────────┘
│  │  │     └──┌───────────────────────────────────────────────────────────────────────────────────┐
│  │  │        │ renderer.hpp: Public C and C++ caption/context/decoder/image/renderer/color APIs. │
│  │  │        └───────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ libaribcaption.pc.in: Support file for libaribcaption.pc within libaribcaption. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE: Support file for LICENSE within libaribcaption. │
│  │  │  └──────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────┐
│  │  │  │ README.md: Documentation: Background. │
│  │  │  └───────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────┐
│  │  │  │ README_ja.md: Documentation: 背景. │
│  │  │  └──────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────┐
│  │  │  │ src/: libaribcaption implementation sources. │
│  │  │  └──────────────────────────────────────────────┘
│  │  │  │
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ base/: Portable allocation, logging, hashing/XML, string, and utility support. │
│  │  │  │  └────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  │
│  │  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ aligned_alloc.cpp: Common allocation, logging, hashing, Unicode/UTF/string/platform helpers and small │
│  │  │  │  │  │ utility types.                                                                                        │
│  │  │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ always_inline.hpp: Common allocation, logging, hashing, Unicode/UTF/string/platform helpers and small │
│  │  │  │  │  │ utility types.                                                                                        │
│  │  │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ cfstr_helper.hpp: Common allocation, logging, hashing, Unicode/UTF/string/platform helpers and small │
│  │  │  │  │  │ utility types.                                                                                       │
│  │  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ floating_helper.hpp: Common allocation, logging, hashing, Unicode/UTF/string/platform helpers and │
│  │  │  │  │  │ small utility types.                                                                              │
│  │  │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ language_code.hpp: Common allocation, logging, hashing, Unicode/UTF/string/platform helpers and small │
│  │  │  │  │  │ utility types.                                                                                        │
│  │  │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ logger.cpp: Common allocation, logging, hashing, Unicode/UTF/string/platform helpers and small utility │
│  │  │  │  │  │ types.                                                                                                 │
│  │  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ logger.hpp: Common allocation, logging, hashing, Unicode/UTF/string/platform helpers and small utility │
│  │  │  │  │  │ types.                                                                                                 │
│  │  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ md5.c: Common allocation, logging, hashing, Unicode/UTF/string/platform helpers and small utility │
│  │  │  │  │  │ types.                                                                                            │
│  │  │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ md5.h: Common allocation, logging, hashing, Unicode/UTF/string/platform helpers and small utility │
│  │  │  │  │  │ types.                                                                                            │
│  │  │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ md5_helper.hpp: Common allocation, logging, hashing, Unicode/UTF/string/platform helpers and small │
│  │  │  │  │  │ utility types.                                                                                     │
│  │  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ result.hpp: Common allocation, logging, hashing, Unicode/UTF/string/platform helpers and small utility │
│  │  │  │  │  │ types.                                                                                                 │
│  │  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ scoped_cfref.hpp: Common allocation, logging, hashing, Unicode/UTF/string/platform helpers and small │
│  │  │  │  │  │ utility types.                                                                                       │
│  │  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ scoped_com_initializer.hpp: Common allocation, logging, hashing, Unicode/UTF/string/platform helpers │
│  │  │  │  │  │ and small utility types.                                                                             │
│  │  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ scoped_holder.hpp: Common allocation, logging, hashing, Unicode/UTF/string/platform helpers and small │
│  │  │  │  │  │ utility types.                                                                                        │
│  │  │  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ tinyxml2.cpp: Common allocation, logging, hashing, Unicode/UTF/string/platform helpers and small │
│  │  │  │  │  │ utility types.                                                                                   │
│  │  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ tinyxml2.h: Common allocation, logging, hashing, Unicode/UTF/string/platform helpers and small utility │
│  │  │  │  │  │ types.                                                                                                 │
│  │  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ unicode_helper.hpp: Common allocation, logging, hashing, Unicode/UTF/string/platform helpers and small │
│  │  │  │  │  │ utility types.                                                                                         │
│  │  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ utf_helper.hpp: Common allocation, logging, hashing, Unicode/UTF/string/platform helpers and small │
│  │  │  │  │  │ utility types.                                                                                     │
│  │  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │     │ wchar_helper.hpp: Common allocation, logging, hashing, Unicode/UTF/string/platform helpers and small │
│  │  │  │     │ utility types.                                                                                       │
│  │  │  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────┐
│  │  │  │  │ common/: Context implementation and C API bridges. │
│  │  │  │  └────────────────────────────────────────────────────┘
│  │  │  │  │
│  │  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ caption_capi.cpp: Vendored libaribcaption native source implementing caption capi functionality. │
│  │  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ context.cpp: Vendored libaribcaption native source implementing context functionality. │
│  │  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │     │ context_capi.cpp: Vendored libaribcaption native source implementing context capi functionality. │
│  │  │  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │ decoder/: ARIB STD-B24 caption decoder, code sets, colors, control codes, conversion tables, DRCS, and │
│  │  │  │  │ gaiji handling.                                                                                        │
│  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  │
│  │  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ b24_codesets.cpp: Vendored libaribcaption native source implementing b24 codesets functionality. │
│  │  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ b24_codesets.hpp: Vendored libaribcaption header declaring b24 codesets interfaces/data. │
│  │  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ b24_colors.cpp: Vendored libaribcaption native source implementing b24 colors functionality. │
│  │  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ b24_colors.hpp: Vendored libaribcaption header declaring b24 colors interfaces/data. │
│  │  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌──────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ b24_controlsets.hpp: Control-code definitions/state support. │
│  │  │  │  │  └──────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌────────────────────────────────────────────────┐
│  │  │  │  │  │ b24_conv_tables.hpp: Conversion lookup tables. │
│  │  │  │  │  └────────────────────────────────────────────────┘
│  │  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ b24_drcs_conv.cpp: Vendored libaribcaption native source implementing b24 drcs conv functionality. │
│  │  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ b24_drcs_conv.hpp: Vendored libaribcaption header declaring b24 drcs conv interfaces/data. │
│  │  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌───────────────────────────────────────────────────────┐
│  │  │  │  │  │ b24_gaiji_table.hpp: Additional-symbol/gaiji mapping. │
│  │  │  │  │  └───────────────────────────────────────────────────────┘
│  │  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ b24_macros.hpp: Vendored libaribcaption header declaring b24 macros interfaces/data. │
│  │  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ decoder.cpp: Vendored libaribcaption native source implementing decoder functionality. │
│  │  │  │  │  └────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ decoder_capi.cpp: Vendored libaribcaption native source implementing decoder capi functionality. │
│  │  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │  │  │ decoder_impl.cpp: Vendored libaribcaption native source implementing decoder impl functionality. │
│  │  │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │     │ decoder_impl.hpp: Vendored libaribcaption header declaring decoder impl interfaces/data. │
│  │  │  │     └──────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │ renderer/: Caption renderer, regions, text backends, font providers, DRCS, canvas/bitmap, GSUB, and │
│  │  │     │ alpha blending.                                                                                     │
│  │  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     │
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ alphablend.hpp: Vendored libaribcaption header declaring alphablend interfaces/data. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ alphablend_generic.hpp: Vendored libaribcaption header declaring alphablend generic interfaces/data. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ alphablend_x86.hpp: Vendored libaribcaption header declaring alphablend x86 interfaces/data. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ bitmap.cpp: Vendored libaribcaption native source implementing bitmap functionality. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ bitmap.hpp: Vendored libaribcaption header declaring bitmap interfaces/data. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ canvas.cpp: Vendored libaribcaption native source implementing canvas functionality. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ canvas.hpp: Vendored libaribcaption header declaring canvas interfaces/data. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ drcs_renderer.cpp: Vendored libaribcaption native source implementing drcs renderer functionality. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ drcs_renderer.hpp: Vendored libaribcaption header declaring drcs renderer interfaces/data. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ font_provider.cpp: Vendored libaribcaption native source implementing font provider functionality. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ font_provider.hpp: Vendored libaribcaption header declaring font provider interfaces/data. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ font_provider_android.cpp: Vendored libaribcaption native source implementing font provider android │
│  │  │     │  │ functionality.                                                                                      │
│  │  │     │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌───────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ font_provider_android.hpp: Vendored libaribcaption header declaring font provider android │
│  │  │     │  │ interfaces/data.                                                                          │
│  │  │     │  └───────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ font_provider_coretext.cpp: Vendored libaribcaption native source implementing font provider coretext │
│  │  │     │  │ functionality.                                                                                        │
│  │  │     │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ font_provider_coretext.hpp: Vendored libaribcaption header declaring font provider coretext │
│  │  │     │  │ interfaces/data.                                                                            │
│  │  │     │  └─────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ font_provider_directwrite.cpp: Vendored libaribcaption native source implementing font provider │
│  │  │     │  │ directwrite functionality.                                                                      │
│  │  │     │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ font_provider_directwrite.hpp: Vendored libaribcaption header declaring font provider directwrite │
│  │  │     │  │ interfaces/data.                                                                                  │
│  │  │     │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ font_provider_fontconfig.cpp: Vendored libaribcaption native source implementing font provider │
│  │  │     │  │ fontconfig functionality.                                                                      │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ font_provider_fontconfig.hpp: Vendored libaribcaption header declaring font provider fontconfig │
│  │  │     │  │ interfaces/data.                                                                                │
│  │  │     │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ font_provider_gdi.cpp: Vendored libaribcaption native source implementing font provider gdi │
│  │  │     │  │ functionality.                                                                              │
│  │  │     │  └─────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ font_provider_gdi.hpp: Vendored libaribcaption header declaring font provider gdi interfaces/data. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ image_capi.cpp: Vendored libaribcaption native source implementing image capi functionality. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ open_type_gsub.cpp: Vendored libaribcaption native source implementing open type gsub functionality. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ open_type_gsub.hpp: Vendored libaribcaption header declaring open type gsub interfaces/data. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ rect.hpp: Vendored libaribcaption header declaring rect interfaces/data. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ region_renderer.cpp: Vendored libaribcaption native source implementing region renderer functionality. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ region_renderer.hpp: Vendored libaribcaption header declaring region renderer interfaces/data. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ renderer.cpp: Vendored libaribcaption native source implementing renderer functionality. │
│  │  │     │  └──────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ renderer_capi.cpp: Vendored libaribcaption native source implementing renderer capi functionality. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ renderer_impl.cpp: Vendored libaribcaption native source implementing renderer impl functionality. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ renderer_impl.hpp: Vendored libaribcaption header declaring renderer impl interfaces/data. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ text_renderer.cpp: Vendored libaribcaption native source implementing text renderer functionality. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ text_renderer.hpp: Vendored libaribcaption header declaring text renderer interfaces/data. │
│  │  │     │  └────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ text_renderer_coretext.cpp: Vendored libaribcaption native source implementing text renderer coretext │
│  │  │     │  │ functionality.                                                                                        │
│  │  │     │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ text_renderer_coretext.hpp: Vendored libaribcaption header declaring text renderer coretext │
│  │  │     │  │ interfaces/data.                                                                            │
│  │  │     │  └─────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ text_renderer_directwrite.cpp: Vendored libaribcaption native source implementing text renderer │
│  │  │     │  │ directwrite functionality.                                                                      │
│  │  │     │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ text_renderer_directwrite.hpp: Vendored libaribcaption header declaring text renderer directwrite │
│  │  │     │  │ interfaces/data.                                                                                  │
│  │  │     │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │     │  │ text_renderer_freetype.cpp: Vendored libaribcaption native source implementing text renderer freetype │
│  │  │     │  │ functionality.                                                                                        │
│  │  │     │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │     └──┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │        │ text_renderer_freetype.hpp: Vendored libaribcaption header declaring text renderer freetype │
│  │  │        │ interfaces/data.                                                                            │
│  │  │        └─────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────┐
│  │     │ test/: Vendored upstream caption decoder/renderer tests and helpers. │
│  │     └──────────────────────────────────────────────────────────────────────┘
│  │     │
│  │     ├──┌──────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ alphablend/: Project directory for alphablend and its subordinate files/modules. │
│  │     │  └──────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │
│  │     │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │ CMakeLists.txt: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper │
│  │     │  │  │ utilities; not application runtime code.                                                      │
│  │     │  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │     │ test.cpp: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper utilities; not │
│  │     │     │ application runtime code.                                                                              │
│  │     │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌──────────────────────────────────────────────────────────────────────┐
│  │     │  │ capi/: Project directory for capi and its subordinate files/modules. │
│  │     │  └──────────────────────────────────────────────────────────────────────┘
│  │     │  │
│  │     │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │ CMakeLists.txt: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper │
│  │     │  │  │ utilities; not application runtime code.                                                      │
│  │     │  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │     │ test.c: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper utilities; not │
│  │     │     │ application runtime code.                                                                            │
│  │     │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ caption2srt/: Project directory for caption2srt and its subordinate files/modules. │
│  │     │  └────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │
│  │     │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │ CMakeLists.txt: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper │
│  │     │  │  │ utilities; not application runtime code.                                                      │
│  │     │  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │     │ main.cpp: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper utilities; not │
│  │     │     │ application runtime code.                                                                              │
│  │     │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ CMakeLists.txt: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper │
│  │     │  │ utilities; not application runtime code.                                                      │
│  │     │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌──────────────────────────────────────────────────────────────────────────┐
│  │     │  │ decode/: Project directory for decode and its subordinate files/modules. │
│  │     │  └──────────────────────────────────────────────────────────────────────────┘
│  │     │  │
│  │     │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │ CMakeLists.txt: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper │
│  │     │  │  │ utilities; not application runtime code.                                                      │
│  │     │  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │     │ test.cpp: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper utilities; not │
│  │     │     │ application runtime code.                                                                              │
│  │     │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌──────────────────────────────────────────────────────────────────────┐
│  │     │  │ drcs/: Project directory for drcs and its subordinate files/modules. │
│  │     │  └──────────────────────────────────────────────────────────────────────┘
│  │     │  │
│  │     │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │ CMakeLists.txt: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper │
│  │     │  │  │ utilities; not application runtime code.                                                      │
│  │     │  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │     │ test.cpp: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper utilities; not │
│  │     │     │ application runtime code.                                                                              │
│  │     │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌──────────────────────────────────────────────────────────────────────────┐
│  │     │  │ ffmpeg/: Project directory for ffmpeg and its subordinate files/modules. │
│  │     │  └──────────────────────────────────────────────────────────────────────────┘
│  │     │  │
│  │     │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │ CMakeLists.txt: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper │
│  │     │  │  │ utilities; not application runtime code.                                                      │
│  │     │  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │     │ test.cpp: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper utilities; not │
│  │     │     │ application runtime code.                                                                              │
│  │     │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ fontconfig_freetype/: Project directory for fontconfig freetype and its subordinate files/modules. │
│  │     │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │
│  │     │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │ CMakeLists.txt: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper │
│  │     │  │  │ utilities; not application runtime code.                                                      │
│  │     │  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │     │ test.cpp: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper utilities; not │
│  │     │     │ application runtime code.                                                                              │
│  │     │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌──────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ png_writer/: Project directory for png writer and its subordinate files/modules. │
│  │     │  └──────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │
│  │     │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │ CMakeLists.txt: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper │
│  │     │  │  │ utilities; not application runtime code.                                                      │
│  │     │  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  ├──┌────────────────────────────────────────┐
│  │     │  │  │ include/: Include tree for png_writer. │
│  │     │  │  └────────────────────────────────────────┘
│  │     │  │  │
│  │     │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │  │ png_writer.h: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper utilities; │
│  │     │  │  │  │ not application runtime code.                                                                          │
│  │     │  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │     │ png_writer.hpp: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper │
│  │     │  │     │ utilities; not application runtime code.                                                      │
│  │     │  │     └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │     │ png_writer.cpp: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper │
│  │     │     │ utilities; not application runtime code.                                                      │
│  │     │     └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ sample_data/: Project directory for sample data and its subordinate files/modules. │
│  │     │  └────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  │
│  │     │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │  │ CMakeLists.txt: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper │
│  │     │  │  │ utilities; not application runtime code.                                                      │
│  │     │  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │  └──┌─────────────────────────────────────────┐
│  │     │     │ include/: Include tree for sample_data. │
│  │     │     └─────────────────────────────────────────┘
│  │     │     │
│  │     │     └──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │        │ sample_data.h: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper │
│  │     │        │ utilities; not application runtime code.                                                     │
│  │     │        └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     └──┌────────────────────────────────────────────────────────────────────────────────┐
│  │        │ stopwatch/: Project directory for stopwatch and its subordinate files/modules. │
│  │        └────────────────────────────────────────────────────────────────────────────────┘
│  │        │
│  │        ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │        │  │ CMakeLists.txt: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper │
│  │        │  │ utilities; not application runtime code.                                                      │
│  │        │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │        └──┌───────────────────────────────────────┐
│  │           │ include/: Include tree for stopwatch. │
│  │           └───────────────────────────────────────┘
│  │           │
│  │           └──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │              │ stopwatch.hpp: Upstream decoder/renderer/C-API/font/FFmpeg/alpha-blend/DRCS tests and helper │
│  │              │ utilities; not application runtime code.                                                     │
│  │              └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │ wgpu-hal/: Locally patched wgpu hardware-abstraction layer. It is excluded as a workspace member but │
│     │ globally replaces crates.io wgpu-hal through the root patch section.                                 │
│     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │
│     ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │ A865R-acquisition.patch: Patch payload containing the project-local wgpu HAL acquisition/presentation │
│     │  │ changes.                                                                                              │
│     │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │ A865R-PATCH.md: Documents the Open Volar S-specific changes carried by the locally patched wgpu HAL. │
│     │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │ build.rs: Rust build script for wgpu-hal; prepares native/generated resources or linker/build │
│     │  │ configuration before compilation.                                                             │
│     │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│     ├──┌─────────────────────────────────────────────────────────────┐
│     │  │ Cargo.lock: Locked Rust dependencies retained for wgpu-hal. │
│     │  └─────────────────────────────────────────────────────────────┘
│     ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │ Cargo.toml: Rust package manifest for wgpu-hal; declares package metadata, features, and dependencies. │
│     │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     ├──┌───────────────────────────────────────────────────────────────┐
│     │  │ Cargo.toml.orig: Support file for Cargo.toml within wgpu-hal. │
│     │  └───────────────────────────────────────────────────────────────┘
│     ├──┌──────────────────────────────────────────────────┐
│     │  │ examples/: Upstream HAL examples and benchmarks. │
│     │  └──────────────────────────────────────────────────┘
│     │  │
│     │  ├──┌────────────────────────────────────────────────────────────────────────────┐
│     │  │  │ halmark/: Project directory for halmark and its subordinate files/modules. │
│     │  │  └────────────────────────────────────────────────────────────────────────────┘
│     │  │  │
│     │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────┐
│     │  │  │  │ main.rs: Upstream HAL examples/benchmarks such as halmark, raw GLES, and ray tracing. │
│     │  │  │  └───────────────────────────────────────────────────────────────────────────────────────┘
│     │  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │     │ shader.wgsl: Upstream HAL examples/benchmarks such as halmark, raw GLES, and ray tracing. │
│     │  │     └───────────────────────────────────────────────────────────────────────────────────────────┘
│     │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │  │ raw-gles.em.html: Upstream HAL examples/benchmarks such as halmark, raw GLES, and ray tracing. │
│     │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │  │ raw-gles.rs: Upstream HAL examples/benchmarks such as halmark, raw GLES, and ray tracing. │
│     │  │  └───────────────────────────────────────────────────────────────────────────────────────────┘
│     │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │     │ ray-traced-triangle/: Project directory for ray traced triangle and its subordinate files/modules. │
│     │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │     │
│     │     ├──┌───────────────────────────────────────────────────────────────────────────────────────┐
│     │     │  │ main.rs: Upstream HAL examples/benchmarks such as halmark, raw GLES, and ray tracing. │
│     │     │  └───────────────────────────────────────────────────────────────────────────────────────┘
│     │     └──┌───────────────────────────────────────────────────────────────────────────────────────────┐
│     │        │ shader.wgsl: Upstream HAL examples/benchmarks such as halmark, raw GLES, and ray tracing. │
│     │        └───────────────────────────────────────────────────────────────────────────────────────────┘
│     ├──┌───────────────────────────────────────────────────────────┐
│     │  │ LICENSE.APACHE: Support file for LICENSE within wgpu-hal. │
│     │  └───────────────────────────────────────────────────────────┘
│     ├──┌────────────────────────────────────────────────────────┐
│     │  │ LICENSE.MIT: Support file for LICENSE within wgpu-hal. │
│     │  └────────────────────────────────────────────────────────┘
│     ├──┌──────────────────────────────────────────────────────────────────────────────────┐
│     │  │ README.md: Documentation: wgpuhal: a cross-platform unsafe graphics abstraction. │
│     │  └──────────────────────────────────────────────────────────────────────────────────┘
│     └──┌─────────────────────────────────────────────────────────┐
│        │ src/: Patched HAL implementation and backend selection. │
│        └─────────────────────────────────────────────────────────┘
│        │
│        ├──┌───────────────────────────────────────────────────────────────────────────────┐
│        │  │ auxil/: Shared backend helper code, including DXGI and RenderDoc integration. │
│        │  └───────────────────────────────────────────────────────────────────────────────┘
│        │  │
│        │  ├──┌──────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │ dxgi/: Shared DXGI helpers: factory, names, errors/results, conversions, timing. │
│        │  │  └──────────────────────────────────────────────────────────────────────────────────┘
│        │  │  │
│        │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │  │ conv.rs: Shared DXGI helpers: factory, names, errors/results, conversions, timing. │
│        │  │  │  └────────────────────────────────────────────────────────────────────────────────────┘
│        │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │  │ exception.rs: Shared DXGI helpers: factory, names, errors/results, conversions, timing. │
│        │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────┘
│        │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │  │ factory.rs: Shared DXGI helpers: factory, names, errors/results, conversions, timing. │
│        │  │  │  └───────────────────────────────────────────────────────────────────────────────────────┘
│        │  │  ├──┌───────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │  │ mod.rs: Shared DXGI helpers: factory, names, errors/results, conversions, timing. │
│        │  │  │  └───────────────────────────────────────────────────────────────────────────────────┘
│        │  │  ├──┌────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │  │ name.rs: Shared DXGI helpers: factory, names, errors/results, conversions, timing. │
│        │  │  │  └────────────────────────────────────────────────────────────────────────────────────┘
│        │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │  │ result.rs: Shared DXGI helpers: factory, names, errors/results, conversions, timing. │
│        │  │  │  └──────────────────────────────────────────────────────────────────────────────────────┘
│        │  │  └──┌────────────────────────────────────────────────────────────────────────────────────┐
│        │  │     │ time.rs: Shared DXGI helpers: factory, names, errors/results, conversions, timing. │
│        │  │     └────────────────────────────────────────────────────────────────────────────────────┘
│        │  ├──┌─────────────────────────┐
│        │  │  │ mod.rs: cbindgen:ignore │
│        │  │  └─────────────────────────┘
│        │  └──┌─────────────────────────────────────────────┐
│        │     │ renderdoc.rs: RenderDoc integration helper. │
│        │     └─────────────────────────────────────────────┘
│        ├──┌─────────────────────────────────────────────────────────────────┐
│        │  │ dx12/: Direct3D 12 HAL backend retained for Windows/wgpu paths. │
│        │  └─────────────────────────────────────────────────────────────────┘
│        │  │
│        │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │ adapter.rs: Vendored wgpu-hal Rust module implementing adapter support; retained from or adapted from │
│        │  │  │ the upstream dependency.                                                                              │
│        │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  ├──┌─────────────────────────────────────────────────────────┐
│        │  │  │ command.rs: D3D12 command recording/submission support. │
│        │  │  └─────────────────────────────────────────────────────────┘
│        │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │ conv.rs: Vendored wgpu-hal Rust module implementing conv support; retained from or adapted from the │
│        │  │  │ upstream dependency.                                                                                │
│        │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  ├──┌──────────────────────────────────────────┐
│        │  │  │ dcomp.rs: DirectComposition integration. │
│        │  │  └──────────────────────────────────────────┘
│        │  ├──┌─────────────────────────────────────────────────────────────────┐
│        │  │  │ descriptor.rs: How large the block allocated to this handle is. │
│        │  │  └─────────────────────────────────────────────────────────────────┘
│        │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │ device.rs: Vendored wgpu-hal Rust module implementing device support; retained from or adapted from │
│        │  │  │ the upstream dependency.                                                                            │
│        │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  ├──┌─────────────────────────────────────────────────────────────┐
│        │  │  │ device_creation.rs: Abstraction over D3D12 device creation. │
│        │  │  └─────────────────────────────────────────────────────────────┘
│        │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │ instance.rs: Vendored wgpu-hal Rust module implementing instance support; retained from or adapted │
│        │  │  │ from the upstream dependency.                                                                      │
│        │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  ├──┌───────────────────────────────────────┐
│        │  │  │ mod.rs: Direct3D 12 HAL backend root. │
│        │  │  └───────────────────────────────────────┘
│        │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │ pipeline_desc.rs: We try to use pipeline stream descriptors where possible, but this isn't allowed on  │
│        │  │  │ some older windows 10 versions. Therefore, we also must have some logic to convert such descriptors to │
│        │  │  │ the "traditional" equivalent, `D3D12_GRAPHICS_PIPELINE_STATE_DESC`. Stream descriptors allow extending │
│        │  │  │ the pipeline, enabling more advanced features, including mesh shaders and multiview/view instancing.   │
│        │  │  │ Using a stream descriptor is like using a vulkan descriptor                                            │
│        │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │ sampler.rs: Sampler management for DX12. Nearly identical to the Vulkan sampler cache, with added │
│        │  │  │ descriptor heap management.                                                                       │
│        │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │ shader_compilation.rs: Creates a [`CompilerContainer`] that delegates to the statically-linked version │
│        │  │  │ of DXC.                                                                                                │
│        │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  ├──┌────────────────────────────────────────────────────────┐
│        │  │  │ suballocation.rs: D3D12 resource-memory suballocation. │
│        │  │  └────────────────────────────────────────────────────────┘
│        │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │ types.rs: Vendored wgpu-hal Rust module implementing types support; retained from or adapted from the │
│        │  │  │ upstream dependency.                                                                                  │
│        │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │     │ view.rs: Vendored wgpu-hal Rust module implementing view support; retained from or adapted from the │
│        │     │ upstream dependency.                                                                                │
│        │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│        ├──┌─────────────────────────────────────────────────────────┐
│        │  │ dynamic/: Runtime-erased/dynamic HAL dispatch wrappers. │
│        │  └─────────────────────────────────────────────────────────┘
│        │  │
│        │  ├──┌───────────────────────────────────────────────────────────┐
│        │  │  │ adapter.rs: Runtime-erased/dynamic HAL dispatch wrappers. │
│        │  │  └───────────────────────────────────────────────────────────┘
│        │  ├──┌───────────────────────────────────────────────────────────┐
│        │  │  │ command.rs: Runtime-erased/dynamic HAL dispatch wrappers. │
│        │  │  └───────────────────────────────────────────────────────────┘
│        │  ├──┌──────────────────────────────────────────────────────────┐
│        │  │  │ device.rs: Runtime-erased/dynamic HAL dispatch wrappers. │
│        │  │  └──────────────────────────────────────────────────────────┘
│        │  ├──┌────────────────────────────────────────────────────────────┐
│        │  │  │ instance.rs: Runtime-erased/dynamic HAL dispatch wrappers. │
│        │  │  └────────────────────────────────────────────────────────────┘
│        │  ├──┌───────────────────────────────────────────────────────┐
│        │  │  │ mod.rs: Runtime-erased/dynamic HAL dispatch wrappers. │
│        │  │  └───────────────────────────────────────────────────────┘
│        │  ├──┌─────────────────────────────────────────────────────────┐
│        │  │  │ queue.rs: Runtime-erased/dynamic HAL dispatch wrappers. │
│        │  │  └─────────────────────────────────────────────────────────┘
│        │  └──┌───────────────────────────────────────────────────────────┐
│        │     │ surface.rs: Runtime-erased/dynamic HAL dispatch wrappers. │
│        │     └───────────────────────────────────────────────────────────┘
│        ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │ gles/: OpenGL ES/EGL/WGL/Web HAL backend retained from upstream; not the primary Live TV renderer. │
│        │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  │
│        │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │ adapter.rs: OpenGL ES/EGL/WGL/Web/Emscripten HAL backend retained from upstream; not the primary │
│        │  │  │ Windows Live TV path.                                                                            │
│        │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │ command.rs: OpenGL ES/EGL/WGL/Web/Emscripten HAL backend retained from upstream; not the primary │
│        │  │  │ Windows Live TV path.                                                                            │
│        │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │ conv.rs: OpenGL ES/EGL/WGL/Web/Emscripten HAL backend retained from upstream; not the primary Windows │
│        │  │  │ Live TV path.                                                                                         │
│        │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │ device.rs: OpenGL ES/EGL/WGL/Web/Emscripten HAL backend retained from upstream; not the primary │
│        │  │  │ Windows Live TV path.                                                                           │
│        │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │ egl.rs: OpenGL ES/EGL/WGL/Web/Emscripten HAL backend retained from upstream; not the primary Windows │
│        │  │  │ Live TV path.                                                                                        │
│        │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │ emscripten.rs: OpenGL ES/EGL/WGL/Web/Emscripten HAL backend retained from upstream; not the primary │
│        │  │  │ Windows Live TV path.                                                                               │
│        │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │ fence.rs: OpenGL ES/EGL/WGL/Web/Emscripten HAL backend retained from upstream; not the primary Windows │
│        │  │  │ Live TV path.                                                                                          │
│        │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │ mod.rs: OpenGL ES/EGL/WGL/Web/Emscripten HAL backend retained from upstream; not the primary Windows │
│        │  │  │ Live TV path.                                                                                        │
│        │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │ queue.rs: OpenGL ES/EGL/WGL/Web/Emscripten HAL backend retained from upstream; not the primary Windows │
│        │  │  │ Live TV path.                                                                                          │
│        │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  ├──┌────────────────────────────────────────────────────────────────────────────┐
│        │  │  │ shaders/: Project directory for shaders and its subordinate files/modules. │
│        │  │  └────────────────────────────────────────────────────────────────────────────┘
│        │  │  │
│        │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │  │ clear.frag: OpenGL ES/EGL/WGL/Web/Emscripten HAL backend retained from upstream; not the primary │
│        │  │  │  │ Windows Live TV path.                                                                            │
│        │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │  │ clear.vert: OpenGL ES/EGL/WGL/Web/Emscripten HAL backend retained from upstream; not the primary │
│        │  │  │  │ Windows Live TV path.                                                                            │
│        │  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │  │ srgb_present.frag: OpenGL ES/EGL/WGL/Web/Emscripten HAL backend retained from upstream; not the │
│        │  │  │  │ primary Windows Live TV path.                                                                   │
│        │  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │     │ srgb_present.vert: OpenGL ES/EGL/WGL/Web/Emscripten HAL backend retained from upstream; not the │
│        │  │     │ primary Windows Live TV path.                                                                   │
│        │  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │  │  │ web.rs: OpenGL ES/EGL/WGL/Web/Emscripten HAL backend retained from upstream; not the primary Windows │
│        │  │  │ Live TV path.                                                                                        │
│        │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│        │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │     │ wgl.rs: OpenGL ES/EGL/WGL/Web/Emscripten HAL backend retained from upstream; not the primary Windows │
│        │     │ Live TV path.                                                                                        │
│        │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│        ├──┌─────────────────────────────────────────────────────────────┐
│        │  │ lib.rs: HAL traits/types and conditional backend selection. │
│        │  └─────────────────────────────────────────────────────────────┘
│        ├──┌─────────────────────────────────────────────────────────┐
│        │  │ metal/: Apple Metal HAL backend retained from upstream. │
│        │  └─────────────────────────────────────────────────────────┘
│        │  │
│        │  ├──┌─────────────────────────────────────────────────────────────┐
│        │  │  │ adapter.rs: Apple Metal HAL backend retained from upstream. │
│        │  │  └─────────────────────────────────────────────────────────────┘
│        │  ├──┌─────────────────────────────────────────────────────────────┐
│        │  │  │ command.rs: Apple Metal HAL backend retained from upstream. │
│        │  │  └─────────────────────────────────────────────────────────────┘
│        │  ├──┌──────────────────────────────────────────────────────────┐
│        │  │  │ conv.rs: Apple Metal HAL backend retained from upstream. │
│        │  │  └──────────────────────────────────────────────────────────┘
│        │  ├──┌────────────────────────────────────────────────────────────┐
│        │  │  │ device.rs: Apple Metal HAL backend retained from upstream. │
│        │  │  └────────────────────────────────────────────────────────────┘
│        │  ├──┌───────────────────────────────────────────────────────────────────────────┐
│        │  │  │ library_from_metallib.rs: Apple Metal HAL backend retained from upstream. │
│        │  │  └───────────────────────────────────────────────────────────────────────────┘
│        │  ├──┌─────────────────────────────────────────────────────────┐
│        │  │  │ mod.rs: Apple Metal HAL backend retained from upstream. │
│        │  │  └─────────────────────────────────────────────────────────┘
│        │  ├──┌─────────────────────────────────────────────────────────────┐
│        │  │  │ surface.rs: Apple Metal HAL backend retained from upstream. │
│        │  │  └─────────────────────────────────────────────────────────────┘
│        │  └──┌──────────────────────────────────────────────────────────┐
│        │     │ time.rs: Apple Metal HAL backend retained from upstream. │
│        │     └──────────────────────────────────────────────────────────┘
│        ├──┌─────────────────────────────────────────────────────┐
│        │  │ noop/: No-op/testing HAL backend retained upstream. │
│        │  └─────────────────────────────────────────────────────┘
│        │  │
│        │  ├──┌───────────────────────────────────┐
│        │  │  │ buffer.rs: No-op/testing backend. │
│        │  │  └───────────────────────────────────┘
│        │  ├──┌────────────────────────────────────┐
│        │  │  │ command.rs: No-op/testing backend. │
│        │  │  └────────────────────────────────────┘
│        │  └──┌────────────────────────────────┐
│        │     │ mod.rs: No-op/testing backend. │
│        │     └────────────────────────────────┘
│        ├──┌──────────────────────────────────────────────────────────────────┐
│        │  │ validation_canary.rs: Internal validation/safety canary support. │
│        │  └──────────────────────────────────────────────────────────────────┘
│        └──┌───────────────────────────────────────────────────────────────────────┐
│           │ vulkan/: Vulkan HAL backend used by the wgpu-based presentation path. │
│           └───────────────────────────────────────────────────────────────────────┘
│           │
│           ├──┌───────────────────────────────────────────────────────┐
│           │  │ adapter.rs: Physical-device enumeration/capabilities. │
│           │  └───────────────────────────────────────────────────────┘
│           ├──┌───────────────────────────────────────────────────────────┐
│           │  │ command.rs: Vulkan command encoder/buffer implementation. │
│           │  └───────────────────────────────────────────────────────────┘
│           ├──┌──────────────────────────────────────────────────────┐
│           │  │ conv.rs: wgpu-to-Vulkan type/flag/format conversion. │
│           │  └──────────────────────────────────────────────────────┘
│           ├──┌───────────────────────────────────────────────────────┐
│           │  │ device.rs: Logical device/resource/pipeline creation. │
│           │  └───────────────────────────────────────────────────────┘
│           ├──┌───────────────────────────────────────────────────────────────────┐
│           │  │ drm.rs: DRM/display integration retained for non-Windows targets. │
│           │  └───────────────────────────────────────────────────────────────────┘
│           ├──┌────────────────────────────────────────────────────────────────────────────┐
│           │  │ instance.rs: Vulkan instance/surface creation and global capability setup. │
│           │  └────────────────────────────────────────────────────────────────────────────┘
│           ├──┌──────────────────────────────────┐
│           │  │ mod.rs: Vulkan HAL backend root. │
│           │  └──────────────────────────────────┘
│           ├──┌──────────────────────────────────────────────┐
│           │  │ sampler.rs: Sampler creation/cache behavior. │
│           │  └──────────────────────────────────────────────┘
│           ├──┌─────────────────────────────────────────────────────────┐
│           │  │ semaphore_list.rs: Semaphore tracking/lifetime support. │
│           │  └─────────────────────────────────────────────────────────┘
│           └──┌──────────────────────────────────────────────┐
│              │ swapchain/: Native Vulkan swapchain support. │
│              └──────────────────────────────────────────────┘
│              │
│              ├──┌─────────────────────────────────────────────┐
│              │  │ mod.rs: Swapchain abstraction/common logic. │
│              │  └─────────────────────────────────────────────┘
│              └──┌──────────────────────────────────────────────────────────────────────────────────────┐
│                 │ native.rs: Native Vulkan swapchain implementation; relevant to desktop presentation. │
│                 └──────────────────────────────────────────────────────────────────────────────────────┘
├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │ third-party-licenses/: Dependency license/provenance archive. These files are legal metadata rather │
│  │ than executable application modules.                                                                │
│  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ ab_glyph-0.2.32/: License/provenance bundle for the dependency ab_glyph-0.2.32; legal metadata, not │
│  │  │ executable code.                                                                                    │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency ab_glyph-0.2.32; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ ab_glyph_rasterizer-0.1.10/: License/provenance bundle for the dependency ab_glyph_rasterizer-0.1.10; │
│  │  │ legal metadata, not executable code.                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency ab_glyph_rasterizer-0.1.10; legal/provenance material │
│  │     │ only.                                                                                               │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ adler2-2.0.1/: License/provenance bundle for the dependency adler2-2.0.1; legal metadata, not │
│  │  │ executable code.                                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-0BSD: License text retained for dependency adler2-2.0.1; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency adler2-2.0.1; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency adler2-2.0.1; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ ahash-0.8.12/: License/provenance bundle for the dependency ahash-0.8.12; legal metadata, not │
│  │  │ executable code.                                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency ahash-0.8.12; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency ahash-0.8.12; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ allocator-api2-0.2.21/: License/provenance bundle for the dependency allocator-api2-0.2.21; legal │
│  │  │ metadata, not executable code.                                                                    │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency allocator-api2-0.2.21; legal/provenance material │
│  │  │  │ only.                                                                                                 │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency allocator-api2-0.2.21; legal/provenance material │
│  │     │ only.                                                                                              │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ android-activity-0.6.1/: License/provenance bundle for the dependency android-activity-0.6.1; legal │
│  │  │ metadata, not executable code.                                                                      │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE: License text retained for dependency android-activity-0.6.1; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency android-activity-0.6.1; legal/provenance material │
│  │  │  │ only.                                                                                                  │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency android-activity-0.6.1; legal/provenance material │
│  │     │ only.                                                                                               │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ android-properties-0.2.2/: License/provenance bundle for the dependency android-properties-0.2.2; │
│  │  │ legal metadata, not executable code.                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency android-properties-0.2.2; legal/provenance material │
│  │     │ only.                                                                                             │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────┐
│  │  │ android_system_properties-0.1.6/: License/provenance bundle for the dependency │
│  │  │ android_system_properties-0.1.6; legal metadata, not executable code.          │
│  │  └────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency android_system_properties-0.1.6; legal/provenance │
│  │  │  │ material only.                                                                                         │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency android_system_properties-0.1.6; legal/provenance │
│  │     │ material only.                                                                                      │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ arboard-3.6.1/: License/provenance bundle for the dependency arboard-3.6.1; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE.txt: License text retained for dependency arboard-3.6.1; legal/provenance material │
│  │  │  │ only.                                                                                             │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT.txt: License text retained for dependency arboard-3.6.1; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ arrayvec-0.7.8/: License/provenance bundle for the dependency arrayvec-0.7.8; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency arrayvec-0.7.8; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency arrayvec-0.7.8; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ ash-0.37.3+1.3.251/: License/provenance bundle for the dependency ash-0.37.3+1.3.251; legal metadata, │
│  │  │ not executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency ash-0.37.3+1.3.251; legal/provenance material │
│  │  │  │ only.                                                                                              │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency ash-0.37.3+1.3.251; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ ash-0.38.0+1.3.281/: License/provenance bundle for the dependency ash-0.38.0+1.3.281; legal metadata, │
│  │  │ not executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency ash-0.38.0+1.3.281; legal/provenance material │
│  │  │  │ only.                                                                                              │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency ash-0.38.0+1.3.281; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ atomic-waker-1.1.2/: License/provenance bundle for the dependency atomic-waker-1.1.2; legal metadata, │
│  │  │ not executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency atomic-waker-1.1.2; legal/provenance material │
│  │  │  │ only.                                                                                              │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency atomic-waker-1.1.2; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-THIRD-PARTY: License text retained for dependency atomic-waker-1.1.2; legal/provenance │
│  │     │ material only.                                                                                 │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ autocfg-1.5.1/: License/provenance bundle for the dependency autocfg-1.5.1; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency autocfg-1.5.1; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency autocfg-1.5.1; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ bit-set-0.5.3/: License/provenance bundle for the dependency bit-set-0.5.3; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency bit-set-0.5.3; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency bit-set-0.5.3; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ bit-set-0.9.1/: License/provenance bundle for the dependency bit-set-0.9.1; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency bit-set-0.9.1; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency bit-set-0.9.1; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ bit-vec-0.6.3/: License/provenance bundle for the dependency bit-vec-0.6.3; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency bit-vec-0.6.3; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency bit-vec-0.6.3; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ bit-vec-0.9.1/: License/provenance bundle for the dependency bit-vec-0.9.1; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency bit-vec-0.9.1; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency bit-vec-0.9.1; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ bitflags-1.3.2/: License/provenance bundle for the dependency bitflags-1.3.2; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency bitflags-1.3.2; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency bitflags-1.3.2; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ bitflags-2.13.1/: License/provenance bundle for the dependency bitflags-2.13.1; legal metadata, not │
│  │  │ executable code.                                                                                    │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency bitflags-2.13.1; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency bitflags-2.13.1; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ bitstream-io-2.6.0/: License/provenance bundle for the dependency bitstream-io-2.6.0; legal metadata, │
│  │  │ not executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency bitstream-io-2.6.0; legal/provenance material │
│  │  │  │ only.                                                                                              │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency bitstream-io-2.6.0; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ block-buffer-0.10.4/: License/provenance bundle for the dependency block-buffer-0.10.4; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency block-buffer-0.10.4; legal/provenance material │
│  │  │  │ only.                                                                                               │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency block-buffer-0.10.4; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ broadcast-parser/: License/provenance bundle for the dependency broadcast-parser; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ HEADER-LICENSES.txt: License text retained for dependency broadcast-parser; legal/provenance material │
│  │  │  │ only.                                                                                                 │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE-2.0: License text retained for dependency broadcast-parser; legal/provenance material │
│  │  │  │ only.                                                                                                │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ PROVENANCE.md: Provenance/legal metadata retained for dependency broadcast-parser; not runtime code. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ bumpalo-3.20.3/: License/provenance bundle for the dependency bumpalo-3.20.3; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency bumpalo-3.20.3; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency bumpalo-3.20.3; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ bytemuck-1.25.2/: License/provenance bundle for the dependency bytemuck-1.25.2; legal metadata, not │
│  │  │ executable code.                                                                                    │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency bytemuck-1.25.2; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency bytemuck-1.25.2; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-ZLIB: License text retained for dependency bytemuck-1.25.2; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ bytemuck_derive-1.12.0/: License/provenance bundle for the dependency bytemuck_derive-1.12.0; legal │
│  │  │ metadata, not executable code.                                                                      │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency bytemuck_derive-1.12.0; legal/provenance material │
│  │  │  │ only.                                                                                                  │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency bytemuck_derive-1.12.0; legal/provenance material │
│  │  │  │ only.                                                                                               │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-ZLIB: License text retained for dependency bytemuck_derive-1.12.0; legal/provenance material │
│  │     │ only.                                                                                                │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ byteorder-lite-0.1.0/: License/provenance bundle for the dependency byteorder-lite-0.1.0; legal │
│  │  │ metadata, not executable code.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency byteorder-lite-0.1.0; legal/provenance material │
│  │     │ only.                                                                                             │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ bytes-1.12.1/: License/provenance bundle for the dependency bytes-1.12.1; legal metadata, not │
│  │  │ executable code.                                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency bytes-1.12.1; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ calloop-0.13.0/: License/provenance bundle for the dependency calloop-0.13.0; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.txt: License text retained for dependency calloop-0.13.0; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ calloop-0.14.4/: License/provenance bundle for the dependency calloop-0.14.4; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.txt: License text retained for dependency calloop-0.14.4; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────┐
│  │  │ calloop-wayland-source-0.4.1/: License/provenance bundle for the dependency │
│  │  │ calloop-wayland-source-0.4.1; legal metadata, not executable code.          │
│  │  └─────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.txt: License text retained for dependency calloop-wayland-source-0.4.1; legal/provenance │
│  │     │ material only.                                                                                   │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ cc-1.4.5/: License/provenance bundle for the dependency cc-1.4.5; legal metadata, not executable code. │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency cc-1.4.5; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency cc-1.4.5; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ cfg-if-1.0.4/: License/provenance bundle for the dependency cfg-if-1.0.4; legal metadata, not │
│  │  │ executable code.                                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency cfg-if-1.0.4; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency cfg-if-1.0.4; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ cfg_aliases-0.1.1/: License/provenance bundle for the dependency cfg_aliases-0.1.1; legal metadata, │
│  │  │ not executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE: License text retained for dependency cfg_aliases-0.1.1; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ NOTICES.md: Provenance/legal metadata retained for dependency cfg_aliases-0.1.1; not runtime code. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ cfg_aliases-0.2.2/: License/provenance bundle for the dependency cfg_aliases-0.2.2; legal metadata, │
│  │  │ not executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE: License text retained for dependency cfg_aliases-0.2.2; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ NOTICES.md: Provenance/legal metadata retained for dependency cfg_aliases-0.2.2; not runtime code. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ cgl-0.3.2/: License/provenance bundle for the dependency cgl-0.3.2; legal metadata, not executable │
│  │  │ code.                                                                                              │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ COPYING: License text retained for dependency cgl-0.3.2; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency cgl-0.3.2; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency cgl-0.3.2; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ clipboard-win-5.4.1/: License/provenance bundle for the dependency clipboard-win-5.4.1; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE: License text retained for dependency clipboard-win-5.4.1; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ SOURCE.txt: Provenance/legal metadata retained for dependency clipboard-win-5.4.1; not runtime code. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ codespan-reporting-0.13.1/: License/provenance bundle for the dependency codespan-reporting-0.13.1; │
│  │  │ legal metadata, not executable code.                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency codespan-reporting-0.13.1; legal/provenance material │
│  │     │ only.                                                                                              │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ com-0.6.0/: License/provenance bundle for the dependency com-0.6.0; legal metadata, not executable │
│  │  │ code.                                                                                              │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency com-0.6.0; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ combine-4.6.8/: License/provenance bundle for the dependency combine-4.6.8; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency combine-4.6.8; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ concurrent-queue-2.5.0/: License/provenance bundle for the dependency concurrent-queue-2.5.0; legal │
│  │  │ metadata, not executable code.                                                                      │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency concurrent-queue-2.5.0; legal/provenance material │
│  │  │  │ only.                                                                                                  │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency concurrent-queue-2.5.0; legal/provenance material │
│  │     │ only.                                                                                               │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ core-foundation-0.9.4/: License/provenance bundle for the dependency core-foundation-0.9.4; legal │
│  │  │ metadata, not executable code.                                                                    │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency core-foundation-0.9.4; legal/provenance material │
│  │  │  │ only.                                                                                                 │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency core-foundation-0.9.4; legal/provenance material │
│  │     │ only.                                                                                              │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ core-foundation-sys-0.8.7/: License/provenance bundle for the dependency core-foundation-sys-0.8.7; │
│  │  │ legal metadata, not executable code.                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency core-foundation-sys-0.8.7; legal/provenance │
│  │  │  │ material only.                                                                                   │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency core-foundation-sys-0.8.7; legal/provenance material │
│  │     │ only.                                                                                                  │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ core-graphics-0.23.2/: License/provenance bundle for the dependency core-graphics-0.23.2; legal │
│  │  │ metadata, not executable code.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ COPYRIGHT: Provenance/legal metadata retained for dependency core-graphics-0.23.2; not runtime code. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency core-graphics-0.23.2; legal/provenance material │
│  │  │  │ only.                                                                                                │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency core-graphics-0.23.2; legal/provenance material │
│  │     │ only.                                                                                             │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ core-graphics-types-0.1.3/: License/provenance bundle for the dependency core-graphics-types-0.1.3; │
│  │  │ legal metadata, not executable code.                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency core-graphics-types-0.1.3; legal/provenance │
│  │  │  │ material only.                                                                                   │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency core-graphics-types-0.1.3; legal/provenance material │
│  │     │ only.                                                                                                  │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ cpufeatures-0.2.17/: License/provenance bundle for the dependency cpufeatures-0.2.17; legal metadata, │
│  │  │ not executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency cpufeatures-0.2.17; legal/provenance material │
│  │  │  │ only.                                                                                              │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency cpufeatures-0.2.17; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ crc32fast-1.5.1/: License/provenance bundle for the dependency crc32fast-1.5.1; legal metadata, not │
│  │  │ executable code.                                                                                    │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency crc32fast-1.5.1; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency crc32fast-1.5.1; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ crossbeam-utils-0.8.23/: License/provenance bundle for the dependency crossbeam-utils-0.8.23; legal │
│  │  │ metadata, not executable code.                                                                      │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency crossbeam-utils-0.8.23; legal/provenance material │
│  │  │  │ only.                                                                                                  │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency crossbeam-utils-0.8.23; legal/provenance material │
│  │     │ only.                                                                                               │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ crunchy-0.2.4/: License/provenance bundle for the dependency crunchy-0.2.4; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency crunchy-0.2.4; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ crypto-common-0.1.7/: License/provenance bundle for the dependency crypto-common-0.1.7; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency crypto-common-0.1.7; legal/provenance material │
│  │  │  │ only.                                                                                               │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency crypto-common-0.1.7; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ cursor-icon-1.2.0/: License/provenance bundle for the dependency cursor-icon-1.2.0; legal metadata, │
│  │  │ not executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency cursor-icon-1.2.0; legal/provenance material │
│  │  │  │ only.                                                                                             │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency cursor-icon-1.2.0; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-ZLIB: License text retained for dependency cursor-icon-1.2.0; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ derivative-2.2.0/: License/provenance bundle for the dependency derivative-2.2.0; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency derivative-2.2.0; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency derivative-2.2.0; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ digest-0.10.7/: License/provenance bundle for the dependency digest-0.10.7; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency digest-0.10.7; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency digest-0.10.7; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ displaydoc-0.2.7/: License/provenance bundle for the dependency displaydoc-0.2.7; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency displaydoc-0.2.7; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency displaydoc-0.2.7; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ dlib-0.5.3/: License/provenance bundle for the dependency dlib-0.5.3; legal metadata, not executable │
│  │  │ code.                                                                                                │
│  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.txt: License text retained for dependency dlib-0.5.3; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ document-features-0.2.12/: License/provenance bundle for the dependency document-features-0.2.12; │
│  │  │ legal metadata, not executable code.                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency document-features-0.2.12; legal/provenance │
│  │  │  │ material only.                                                                                  │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency document-features-0.2.12; legal/provenance material │
│  │     │ only.                                                                                                 │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ downcast-rs-1.2.1/: License/provenance bundle for the dependency downcast-rs-1.2.1; legal metadata, │
│  │  │ not executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency downcast-rs-1.2.1; legal/provenance material │
│  │  │  │ only.                                                                                             │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency downcast-rs-1.2.1; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ dpi-0.1.2/: License/provenance bundle for the dependency dpi-0.1.2; legal metadata, not executable │
│  │  │ code.                                                                                              │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE: License text retained for dependency dpi-0.1.2; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-LIBM-MIT: License text retained for dependency dpi-0.1.2; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ ecolor-0.32.3/: License/provenance bundle for the dependency ecolor-0.32.3; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency ecolor-0.32.3; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency ecolor-0.32.3; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ SOURCE.txt: Provenance/legal metadata retained for dependency ecolor-0.32.3; not runtime code. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ eframe-0.32.3/: License/provenance bundle for the dependency eframe-0.32.3; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency eframe-0.32.3; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency eframe-0.32.3; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ SOURCE.txt: Provenance/legal metadata retained for dependency eframe-0.32.3; not runtime code. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ egui-0.32.3/: License/provenance bundle for the dependency egui-0.32.3; legal metadata, not executable │
│  │  │ code.                                                                                                  │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency egui-0.32.3; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency egui-0.32.3; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ SOURCE.txt: Provenance/legal metadata retained for dependency egui-0.32.3; not runtime code. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ egui-winit-0.32.3/: License/provenance bundle for the dependency egui-winit-0.32.3; legal metadata, │
│  │  │ not executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency egui-winit-0.32.3; legal/provenance material │
│  │  │  │ only.                                                                                             │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency egui-winit-0.32.3; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ SOURCE.txt: Provenance/legal metadata retained for dependency egui-winit-0.32.3; not runtime code. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ egui_glow-0.32.3/: License/provenance bundle for the dependency egui_glow-0.32.3; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency egui_glow-0.32.3; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency egui_glow-0.32.3; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ SOURCE.txt: Provenance/legal metadata retained for dependency egui_glow-0.32.3; not runtime code. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ emath-0.32.3/: License/provenance bundle for the dependency emath-0.32.3; legal metadata, not │
│  │  │ executable code.                                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency emath-0.32.3; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency emath-0.32.3; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ SOURCE.txt: Provenance/legal metadata retained for dependency emath-0.32.3; not runtime code. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ epaint-0.32.3/: License/provenance bundle for the dependency epaint-0.32.3; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency epaint-0.32.3; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency epaint-0.32.3; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ SOURCE.txt: Provenance/legal metadata retained for dependency epaint-0.32.3; not runtime code. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────┐
│  │  │ epaint_default_fonts-0.32.3/: License/provenance bundle for the dependency │
│  │  │ epaint_default_fonts-0.32.3; legal metadata, not executable code.          │
│  │  └────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ fonts/: License/provenance bundle for the dependency fonts; legal metadata, not executable code. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │
│  │     ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ emoji-icon-font-mit-license.txt: License text retained for dependency epaint_default_fonts-0.32.3; │
│  │     │  │ legal/provenance material only.                                                                    │
│  │     │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ Hack-Regular.txt: Provenance/legal metadata retained for dependency epaint_default_fonts-0.32.3; not │
│  │     │  │ runtime code.                                                                                        │
│  │     │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │  │ OFL.txt: Provenance/legal metadata retained for dependency epaint_default_fonts-0.32.3; not runtime │
│  │     │  │ code.                                                                                               │
│  │     │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │        │ UFL.txt: Provenance/legal metadata retained for dependency epaint_default_fonts-0.32.3; not runtime │
│  │        │ code.                                                                                               │
│  │        └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ equivalent-1.0.2/: License/provenance bundle for the dependency equivalent-1.0.2; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency equivalent-1.0.2; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency equivalent-1.0.2; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ errno-0.3.14/: License/provenance bundle for the dependency errno-0.3.14; legal metadata, not │
│  │  │ executable code.                                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency errno-0.3.14; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency errno-0.3.14; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ error-code-3.4.0/: License/provenance bundle for the dependency error-code-3.4.0; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency error-code-3.4.0; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ fax-0.2.7/: License/provenance bundle for the dependency fax-0.2.7; legal metadata, not executable │
│  │  │ code.                                                                                              │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency fax-0.2.7; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ fdeflate-0.3.7/: License/provenance bundle for the dependency fdeflate-0.3.7; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency fdeflate-0.3.7; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency fdeflate-0.3.7; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ find-msvc-tools-0.1.12/: License/provenance bundle for the dependency find-msvc-tools-0.1.12; legal │
│  │  │ metadata, not executable code.                                                                      │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency find-msvc-tools-0.1.12; legal/provenance material │
│  │  │  │ only.                                                                                                  │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency find-msvc-tools-0.1.12; legal/provenance material │
│  │     │ only.                                                                                               │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ flate2-1.1.10/: License/provenance bundle for the dependency flate2-1.1.10; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency flate2-1.1.10; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency flate2-1.1.10; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ foldhash-0.1.5/: License/provenance bundle for the dependency foldhash-0.1.5; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency foldhash-0.1.5; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ foldhash-0.2.0/: License/provenance bundle for the dependency foldhash-0.2.0; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency foldhash-0.2.0; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ foreign-types-0.5.0/: License/provenance bundle for the dependency foreign-types-0.5.0; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency foreign-types-0.5.0; legal/provenance material │
│  │  │  │ only.                                                                                               │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency foreign-types-0.5.0; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ foreign-types-macros-0.2.4/: License/provenance bundle for the dependency foreign-types-macros-0.2.4; │
│  │  │ legal metadata, not executable code.                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency foreign-types-macros-0.2.4; legal/provenance │
│  │  │  │ material only.                                                                                    │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency foreign-types-macros-0.2.4; legal/provenance │
│  │     │ material only.                                                                                 │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ foreign-types-shared-0.3.1/: License/provenance bundle for the dependency foreign-types-shared-0.3.1; │
│  │  │ legal metadata, not executable code.                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency foreign-types-shared-0.3.1; legal/provenance │
│  │  │  │ material only.                                                                                    │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency foreign-types-shared-0.3.1; legal/provenance │
│  │     │ material only.                                                                                 │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ form_urlencoded-1.2.2/: License/provenance bundle for the dependency form_urlencoded-1.2.2; legal │
│  │  │ metadata, not executable code.                                                                    │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency form_urlencoded-1.2.2; legal/provenance material │
│  │  │  │ only.                                                                                                 │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency form_urlencoded-1.2.2; legal/provenance material │
│  │     │ only.                                                                                              │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ four-cc-0.4.0/: License/provenance bundle for the dependency four-cc-0.4.0; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency four-cc-0.4.0; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency four-cc-0.4.0; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ futures-core-0.3.34/: License/provenance bundle for the dependency futures-core-0.3.34; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency futures-core-0.3.34; legal/provenance material │
│  │  │  │ only.                                                                                               │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency futures-core-0.3.34; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ futures-task-0.3.34/: License/provenance bundle for the dependency futures-task-0.3.34; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency futures-task-0.3.34; legal/provenance material │
│  │  │  │ only.                                                                                               │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency futures-task-0.3.34; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ futures-util-0.3.34/: License/provenance bundle for the dependency futures-util-0.3.34; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency futures-util-0.3.34; legal/provenance material │
│  │  │  │ only.                                                                                               │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency futures-util-0.3.34; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ generic-array-0.14.7/: License/provenance bundle for the dependency generic-array-0.14.7; legal │
│  │  │ metadata, not executable code.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency generic-array-0.14.7; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ gethostname-1.1.0/: License/provenance bundle for the dependency gethostname-1.1.0; legal metadata, │
│  │  │ not executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency gethostname-1.1.0; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ getrandom-0.4.3/: License/provenance bundle for the dependency getrandom-0.4.3; legal metadata, not │
│  │  │ executable code.                                                                                    │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency getrandom-0.4.3; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency getrandom-0.4.3; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ gl_generator-0.14.0/: License/provenance bundle for the dependency gl_generator-0.14.0; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE: License text retained for dependency gl_generator-0.14.0; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ SOURCE.txt: Provenance/legal metadata retained for dependency gl_generator-0.14.0; not runtime code. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ glow-0.13.1/: License/provenance bundle for the dependency glow-0.13.1; legal metadata, not executable │
│  │  │ code.                                                                                                  │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency glow-0.13.1; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency glow-0.13.1; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-ZLIB: License text retained for dependency glow-0.13.1; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ glow-0.16.0/: License/provenance bundle for the dependency glow-0.16.0; legal metadata, not executable │
│  │  │ code.                                                                                                  │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency glow-0.16.0; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency glow-0.16.0; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-ZLIB: License text retained for dependency glow-0.16.0; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ glutin-0.32.3/: License/provenance bundle for the dependency glutin-0.32.3; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency glutin-0.32.3; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ glutin-winit-0.5.0/: License/provenance bundle for the dependency glutin-winit-0.5.0; legal metadata, │
│  │  │ not executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency glutin-winit-0.5.0; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ glutin_egl_sys-0.7.1/: License/provenance bundle for the dependency glutin_egl_sys-0.7.1; legal │
│  │  │ metadata, not executable code.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency glutin_egl_sys-0.7.1; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ glutin_wgl_sys-0.5.0/: License/provenance bundle for the dependency glutin_wgl_sys-0.5.0; legal │
│  │  │ metadata, not executable code.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency glutin_wgl_sys-0.5.0; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ glutin_wgl_sys-0.6.1/: License/provenance bundle for the dependency glutin_wgl_sys-0.6.1; legal │
│  │  │ metadata, not executable code.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency glutin_wgl_sys-0.6.1; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ gpu-allocator-0.25.0/: License/provenance bundle for the dependency gpu-allocator-0.25.0; legal │
│  │  │ metadata, not executable code.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency gpu-allocator-0.25.0; legal/provenance material │
│  │  │  │ only.                                                                                                │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency gpu-allocator-0.25.0; legal/provenance material │
│  │     │ only.                                                                                             │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ gpu-allocator-0.28.0/: License/provenance bundle for the dependency gpu-allocator-0.28.0; legal │
│  │  │ metadata, not executable code.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency gpu-allocator-0.28.0; legal/provenance material │
│  │  │  │ only.                                                                                                │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency gpu-allocator-0.28.0; legal/provenance material │
│  │     │ only.                                                                                             │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ gpu-video/: License/provenance bundle for the dependency gpu-video; legal metadata, not executable │
│  │  │ code.                                                                                              │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE: License text retained for dependency gpu-video; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ PATCHES.md: Provenance/legal metadata retained for dependency gpu-video; not runtime code. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ h264-reader/: License/provenance bundle for the dependency h264-reader; legal metadata, not executable │
│  │  │ code.                                                                                                  │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency h264-reader; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency h264-reader; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ half-2.7.1/: License/provenance bundle for the dependency half-2.7.1; legal metadata, not executable │
│  │  │ code.                                                                                                │
│  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency half-2.7.1; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency half-2.7.1; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ hashbrown-0.14.5/: License/provenance bundle for the dependency hashbrown-0.14.5; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency hashbrown-0.14.5; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency hashbrown-0.14.5; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ hashbrown-0.15.5/: License/provenance bundle for the dependency hashbrown-0.15.5; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency hashbrown-0.15.5; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency hashbrown-0.15.5; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ hashbrown-0.16.1/: License/provenance bundle for the dependency hashbrown-0.16.1; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency hashbrown-0.16.1; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency hashbrown-0.16.1; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ hashbrown-0.17.1/: License/provenance bundle for the dependency hashbrown-0.17.1; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency hashbrown-0.17.1; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency hashbrown-0.17.1; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ hassle-rs-0.11.0/: License/provenance bundle for the dependency hassle-rs-0.11.0; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency hassle-rs-0.11.0; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ hermit-abi-0.5.3/: License/provenance bundle for the dependency hermit-abi-0.5.3; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency hermit-abi-0.5.3; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency hermit-abi-0.5.3; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ icu_collections-2.3.0/: License/provenance bundle for the dependency icu_collections-2.3.0; legal │
│  │  │ metadata, not executable code.                                                                    │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency icu_collections-2.3.0; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ icu_locale_core-2.3.0/: License/provenance bundle for the dependency icu_locale_core-2.3.0; legal │
│  │  │ metadata, not executable code.                                                                    │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency icu_locale_core-2.3.0; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ icu_normalizer-2.3.0/: License/provenance bundle for the dependency icu_normalizer-2.3.0; legal │
│  │  │ metadata, not executable code.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency icu_normalizer-2.3.0; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ icu_normalizer_data-2.3.0/: License/provenance bundle for the dependency icu_normalizer_data-2.3.0; │
│  │  │ legal metadata, not executable code.                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency icu_normalizer_data-2.3.0; legal/provenance material │
│  │     │ only.                                                                                              │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ icu_properties-2.3.0/: License/provenance bundle for the dependency icu_properties-2.3.0; legal │
│  │  │ metadata, not executable code.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency icu_properties-2.3.0; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ icu_properties_data-2.3.0/: License/provenance bundle for the dependency icu_properties_data-2.3.0; │
│  │  │ legal metadata, not executable code.                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency icu_properties_data-2.3.0; legal/provenance material │
│  │     │ only.                                                                                              │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ icu_provider-2.3.1/: License/provenance bundle for the dependency icu_provider-2.3.1; legal metadata, │
│  │  │ not executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency icu_provider-2.3.1; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ idna-1.1.0/: License/provenance bundle for the dependency idna-1.1.0; legal metadata, not executable │
│  │  │ code.                                                                                                │
│  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency idna-1.1.0; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency idna-1.1.0; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ idna_adapter-1.2.2/: License/provenance bundle for the dependency idna_adapter-1.2.2; legal metadata, │
│  │  │ not executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency idna_adapter-1.2.2; legal/provenance material │
│  │  │  │ only.                                                                                              │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency idna_adapter-1.2.2; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ image-0.25.10/: License/provenance bundle for the dependency image-0.25.10; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency image-0.25.10; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency image-0.25.10; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ indexmap-2.14.2/: License/provenance bundle for the dependency indexmap-2.14.2; legal metadata, not │
│  │  │ executable code.                                                                                    │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency indexmap-2.14.2; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency indexmap-2.14.2; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ inno-setup/: License/provenance bundle for the dependency inno-setup; legal metadata, not executable │
│  │  │ code.                                                                                                │
│  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license.txt: License text retained for dependency inno-setup; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ itoa-1.0.18/: License/provenance bundle for the dependency itoa-1.0.18; legal metadata, not executable │
│  │  │ code.                                                                                                  │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency itoa-1.0.18; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency itoa-1.0.18; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ jni-sys-0.3.1/: License/provenance bundle for the dependency jni-sys-0.3.1; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency jni-sys-0.3.1; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency jni-sys-0.3.1; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ jni-sys-0.4.1/: License/provenance bundle for the dependency jni-sys-0.4.1; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency jni-sys-0.4.1; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency jni-sys-0.4.1; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ jobserver-0.1.35/: License/provenance bundle for the dependency jobserver-0.1.35; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency jobserver-0.1.35; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency jobserver-0.1.35; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ js-sys-0.3.105/: License/provenance bundle for the dependency js-sys-0.3.105; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency js-sys-0.3.105; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency js-sys-0.3.105; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ khronos-egl-6.0.0/: License/provenance bundle for the dependency khronos-egl-6.0.0; legal metadata, │
│  │  │ not executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency khronos-egl-6.0.0; legal/provenance material │
│  │  │  │ only.                                                                                             │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency khronos-egl-6.0.0; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ khronos_api-3.1.0/: License/provenance bundle for the dependency khronos_api-3.1.0; legal metadata, │
│  │  │ not executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE: License text retained for dependency khronos_api-3.1.0; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ SOURCE.txt: Provenance/legal metadata retained for dependency khronos_api-3.1.0; not runtime code. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ libaribcaption-LICENSE.txt: License text retained for dependency libaribcaption-LICENSE.txt; │
│  │  │ legal/provenance material only.                                                              │
│  │  └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ libc-0.2.189/: License/provenance bundle for the dependency libc-0.2.189; legal metadata, not │
│  │  │ executable code.                                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency libc-0.2.189; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency libc-0.2.189; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ libloading-0.7.4/: License/provenance bundle for the dependency libloading-0.7.4; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency libloading-0.7.4; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ libloading-0.8.9/: License/provenance bundle for the dependency libloading-0.8.9; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency libloading-0.8.9; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ libm-0.2.16/: License/provenance bundle for the dependency libm-0.2.16; legal metadata, not executable │
│  │  │ code.                                                                                                  │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.txt: License text retained for dependency libm-0.2.16; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ libredox-0.1.23/: License/provenance bundle for the dependency libredox-0.1.23; legal metadata, not │
│  │  │ executable code.                                                                                    │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency libredox-0.1.23; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ linux-raw-sys-0.12.1/: License/provenance bundle for the dependency linux-raw-sys-0.12.1; legal │
│  │  │ metadata, not executable code.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ COPYRIGHT: Provenance/legal metadata retained for dependency linux-raw-sys-0.12.1; not runtime code. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency linux-raw-sys-0.12.1; legal/provenance material │
│  │  │  │ only.                                                                                                │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-Apache-2.0_WITH_LLVM-exception: License text retained for dependency linux-raw-sys-0.12.1; │
│  │  │  │ legal/provenance material only.                                                                    │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency linux-raw-sys-0.12.1; legal/provenance material │
│  │     │ only.                                                                                             │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ linux-raw-sys-0.4.15/: License/provenance bundle for the dependency linux-raw-sys-0.4.15; legal │
│  │  │ metadata, not executable code.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ COPYRIGHT: Provenance/legal metadata retained for dependency linux-raw-sys-0.4.15; not runtime code. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency linux-raw-sys-0.4.15; legal/provenance material │
│  │  │  │ only.                                                                                                │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-Apache-2.0_WITH_LLVM-exception: License text retained for dependency linux-raw-sys-0.4.15; │
│  │  │  │ legal/provenance material only.                                                                    │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency linux-raw-sys-0.4.15; legal/provenance material │
│  │     │ only.                                                                                             │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ litemap-0.8.3/: License/provenance bundle for the dependency litemap-0.8.3; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency litemap-0.8.3; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ litrs-1.0.0/: License/provenance bundle for the dependency litrs-1.0.0; legal metadata, not executable │
│  │  │ code.                                                                                                  │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency litrs-1.0.0; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency litrs-1.0.0; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ lock_api-0.4.14/: License/provenance bundle for the dependency lock_api-0.4.14; legal metadata, not │
│  │  │ executable code.                                                                                    │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency lock_api-0.4.14; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency lock_api-0.4.14; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ log-0.4.34/: License/provenance bundle for the dependency log-0.4.34; legal metadata, not executable │
│  │  │ code.                                                                                                │
│  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency log-0.4.34; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency log-0.4.34; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ memchr-2.8.3/: License/provenance bundle for the dependency memchr-2.8.3; legal metadata, not │
│  │  │ executable code.                                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ COPYING: License text retained for dependency memchr-2.8.3; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency memchr-2.8.3; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ memmap2-0.9.11/: License/provenance bundle for the dependency memmap2-0.9.11; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency memmap2-0.9.11; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency memmap2-0.9.11; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ memoffset-0.9.1/: License/provenance bundle for the dependency memoffset-0.9.1; legal metadata, not │
│  │  │ executable code.                                                                                    │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency memoffset-0.9.1; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ miniz_oxide-0.8.9/: License/provenance bundle for the dependency miniz_oxide-0.8.9; legal metadata, │
│  │  │ not executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE: License text retained for dependency miniz_oxide-0.8.9; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE.md: License text retained for dependency miniz_oxide-0.8.9; legal/provenance material │
│  │  │  │ only.                                                                                                │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT.md: License text retained for dependency miniz_oxide-0.8.9; legal/provenance material │
│  │  │  │ only.                                                                                             │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-ZLIB.md: License text retained for dependency miniz_oxide-0.8.9; legal/provenance material │
│  │     │ only.                                                                                              │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ miniz_oxide-0.9.1/: License/provenance bundle for the dependency miniz_oxide-0.9.1; legal metadata, │
│  │  │ not executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE: License text retained for dependency miniz_oxide-0.9.1; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE.md: License text retained for dependency miniz_oxide-0.9.1; legal/provenance material │
│  │  │  │ only.                                                                                                │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT.md: License text retained for dependency miniz_oxide-0.9.1; legal/provenance material │
│  │  │  │ only.                                                                                             │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-ZLIB.md: License text retained for dependency miniz_oxide-0.9.1; legal/provenance material │
│  │     │ only.                                                                                              │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ moxcms-0.8.1/: License/provenance bundle for the dependency moxcms-0.8.1; legal metadata, not │
│  │  │ executable code.                                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE.md: License text retained for dependency moxcms-0.8.1; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSES.md: License text retained for dependency moxcms-0.8.1; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ mpeg4-audio-const-0.2.0/: License/provenance bundle for the dependency mpeg4-audio-const-0.2.0; legal │
│  │  │ metadata, not executable code.                                                                        │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency mpeg4-audio-const-0.2.0; legal/provenance │
│  │  │  │ material only.                                                                                 │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency mpeg4-audio-const-0.2.0; legal/provenance material │
│  │     │ only.                                                                                                │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ mpv/: License/provenance bundle for the dependency mpv; legal metadata, not executable code. │
│  │  └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ Copyright: Provenance/legal metadata retained for dependency mpv; not runtime code. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE.GPL: License text retained for dependency mpv; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.LGPL: License text retained for dependency mpv; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ naga-29.0.4/: License/provenance bundle for the dependency naga-29.0.4; legal metadata, not executable │
│  │  │ code.                                                                                                  │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE.APACHE: License text retained for dependency naga-29.0.4; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.MIT: License text retained for dependency naga-29.0.4; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ nohash-hasher-0.2.0/: License/provenance bundle for the dependency nohash-hasher-0.2.0; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency nohash-hasher-0.2.0; legal/provenance material │
│  │  │  │ only.                                                                                               │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency nohash-hasher-0.2.0; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ num-traits-0.2.19/: License/provenance bundle for the dependency num-traits-0.2.19; legal metadata, │
│  │  │ not executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency num-traits-0.2.19; legal/provenance material │
│  │  │  │ only.                                                                                             │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency num-traits-0.2.19; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ num_enum-0.7.6/: License/provenance bundle for the dependency num_enum-0.7.6; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency num_enum-0.7.6; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-BSD: License text retained for dependency num_enum-0.7.6; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency num_enum-0.7.6; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ num_enum_derive-0.7.6/: License/provenance bundle for the dependency num_enum_derive-0.7.6; legal │
│  │  │ metadata, not executable code.                                                                    │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency num_enum_derive-0.7.6; legal/provenance material │
│  │  │  │ only.                                                                                                 │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-BSD: License text retained for dependency num_enum_derive-0.7.6; legal/provenance material │
│  │  │  │ only.                                                                                              │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency num_enum_derive-0.7.6; legal/provenance material │
│  │     │ only.                                                                                              │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ once_cell-1.21.4/: License/provenance bundle for the dependency once_cell-1.21.4; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency once_cell-1.21.4; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency once_cell-1.21.4; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ orbclient-0.3.55/: License/provenance bundle for the dependency orbclient-0.3.55; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency orbclient-0.3.55; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ ordered-float-5.5.0/: License/provenance bundle for the dependency ordered-float-5.5.0; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency ordered-float-5.5.0; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ owned_ttf_parser-0.25.1/: License/provenance bundle for the dependency owned_ttf_parser-0.25.1; legal │
│  │  │ metadata, not executable code.                                                                        │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency owned_ttf_parser-0.25.1; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ parking_lot-0.12.5/: License/provenance bundle for the dependency parking_lot-0.12.5; legal metadata, │
│  │  │ not executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency parking_lot-0.12.5; legal/provenance material │
│  │  │  │ only.                                                                                              │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency parking_lot-0.12.5; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ parking_lot_core-0.9.12/: License/provenance bundle for the dependency parking_lot_core-0.9.12; legal │
│  │  │ metadata, not executable code.                                                                        │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency parking_lot_core-0.9.12; legal/provenance │
│  │  │  │ material only.                                                                                 │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency parking_lot_core-0.9.12; legal/provenance material │
│  │     │ only.                                                                                                │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ paste-1.0.15/: License/provenance bundle for the dependency paste-1.0.15; legal metadata, not │
│  │  │ executable code.                                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency paste-1.0.15; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency paste-1.0.15; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ percent-encoding-2.3.2/: License/provenance bundle for the dependency percent-encoding-2.3.2; legal │
│  │  │ metadata, not executable code.                                                                      │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency percent-encoding-2.3.2; legal/provenance material │
│  │  │  │ only.                                                                                                  │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency percent-encoding-2.3.2; legal/provenance material │
│  │     │ only.                                                                                               │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ pin-project-1.1.13/: License/provenance bundle for the dependency pin-project-1.1.13; legal metadata, │
│  │  │ not executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency pin-project-1.1.13; legal/provenance material │
│  │  │  │ only.                                                                                              │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency pin-project-1.1.13; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────┐
│  │  │ pin-project-internal-1.1.13/: License/provenance bundle for the dependency │
│  │  │ pin-project-internal-1.1.13; legal metadata, not executable code.          │
│  │  └────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency pin-project-internal-1.1.13; legal/provenance │
│  │  │  │ material only.                                                                                     │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency pin-project-internal-1.1.13; legal/provenance │
│  │     │ material only.                                                                                  │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ pin-project-lite-0.2.17/: License/provenance bundle for the dependency pin-project-lite-0.2.17; legal │
│  │  │ metadata, not executable code.                                                                        │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency pin-project-lite-0.2.17; legal/provenance │
│  │  │  │ material only.                                                                                 │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency pin-project-lite-0.2.17; legal/provenance material │
│  │     │ only.                                                                                                │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ pkg-config-0.3.34/: License/provenance bundle for the dependency pkg-config-0.3.34; legal metadata, │
│  │  │ not executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency pkg-config-0.3.34; legal/provenance material │
│  │  │  │ only.                                                                                             │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency pkg-config-0.3.34; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ plain-0.2.3/: License/provenance bundle for the dependency plain-0.2.3; legal metadata, not executable │
│  │  │ code.                                                                                                  │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency plain-0.2.3; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency plain-0.2.3; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ png-0.18.1/: License/provenance bundle for the dependency png-0.18.1; legal metadata, not executable │
│  │  │ code.                                                                                                │
│  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency png-0.18.1; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency png-0.18.1; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ polling-3.11.0/: License/provenance bundle for the dependency polling-3.11.0; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency polling-3.11.0; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency polling-3.11.0; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ pollster-0.3.0/: License/provenance bundle for the dependency pollster-0.3.0; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency pollster-0.3.0; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency pollster-0.3.0; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ portable-atomic-1.15.0/: License/provenance bundle for the dependency portable-atomic-1.15.0; legal │
│  │  │ metadata, not executable code.                                                                      │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency portable-atomic-1.15.0; legal/provenance material │
│  │  │  │ only.                                                                                                  │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency portable-atomic-1.15.0; legal/provenance material │
│  │     │ only.                                                                                               │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ portable-atomic-util-0.2.8/: License/provenance bundle for the dependency portable-atomic-util-0.2.8; │
│  │  │ legal metadata, not executable code.                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency portable-atomic-util-0.2.8; legal/provenance │
│  │  │  │ material only.                                                                                    │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency portable-atomic-util-0.2.8; legal/provenance │
│  │     │ material only.                                                                                 │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ potential_utf-0.1.6/: License/provenance bundle for the dependency potential_utf-0.1.6; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency potential_utf-0.1.6; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ presser-0.3.1/: License/provenance bundle for the dependency presser-0.3.1; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency presser-0.3.1; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency presser-0.3.1; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ proc-macro-crate-3.5.0/: License/provenance bundle for the dependency proc-macro-crate-3.5.0; legal │
│  │  │ metadata, not executable code.                                                                      │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency proc-macro-crate-3.5.0; legal/provenance material │
│  │  │  │ only.                                                                                                  │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency proc-macro-crate-3.5.0; legal/provenance material │
│  │     │ only.                                                                                               │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ proc-macro2-1.0.107/: License/provenance bundle for the dependency proc-macro2-1.0.107; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency proc-macro2-1.0.107; legal/provenance material │
│  │  │  │ only.                                                                                               │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency proc-macro2-1.0.107; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ profiling-1.0.18/: License/provenance bundle for the dependency profiling-1.0.18; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency profiling-1.0.18; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency profiling-1.0.18; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ SOURCE.txt: Provenance/legal metadata retained for dependency profiling-1.0.18; not runtime code. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ pxfm-0.1.30/: License/provenance bundle for the dependency pxfm-0.1.30; legal metadata, not executable │
│  │  │ code.                                                                                                  │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE.md: License text retained for dependency pxfm-0.1.30; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSES.md: License text retained for dependency pxfm-0.1.30; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ quick-error-2.0.1/: License/provenance bundle for the dependency quick-error-2.0.1; legal metadata, │
│  │  │ not executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency quick-error-2.0.1; legal/provenance material │
│  │  │  │ only.                                                                                             │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency quick-error-2.0.1; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ quick-xml-0.41.0/: License/provenance bundle for the dependency quick-xml-0.41.0; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT.md: License text retained for dependency quick-xml-0.41.0; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ quote-1.0.47/: License/provenance bundle for the dependency quote-1.0.47; legal metadata, not │
│  │  │ executable code.                                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency quote-1.0.47; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency quote-1.0.47; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ raw-window-handle-0.6.2/: License/provenance bundle for the dependency raw-window-handle-0.6.2; legal │
│  │  │ metadata, not executable code.                                                                        │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE.md: License text retained for dependency raw-window-handle-0.6.2; legal/provenance │
│  │  │  │ material only.                                                                                    │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT.md: License text retained for dependency raw-window-handle-0.6.2; legal/provenance │
│  │  │  │ material only.                                                                                 │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-ZLIB.md: License text retained for dependency raw-window-handle-0.6.2; legal/provenance │
│  │     │ material only.                                                                                  │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ raw-window-metal-1.1.0/: License/provenance bundle for the dependency raw-window-metal-1.1.0; legal │
│  │  │ metadata, not executable code.                                                                      │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency raw-window-metal-1.1.0; legal/provenance material │
│  │  │  │ only.                                                                                                  │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency raw-window-metal-1.1.0; legal/provenance material │
│  │     │ only.                                                                                               │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ recfsusb2i/: License/provenance bundle for the dependency recfsusb2i; legal metadata, not executable │
│  │  │ code.                                                                                                │
│  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ GPL-3.0.txt: Provenance/legal metadata retained for dependency recfsusb2i; not runtime code. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ readMe.txt: Provenance/legal metadata retained for dependency recfsusb2i; not runtime code. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ redox_syscall-0.4.1/: License/provenance bundle for the dependency redox_syscall-0.4.1; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency redox_syscall-0.4.1; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ redox_syscall-0.5.18/: License/provenance bundle for the dependency redox_syscall-0.5.18; legal │
│  │  │ metadata, not executable code.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency redox_syscall-0.5.18; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ redox_syscall-0.9.4/: License/provenance bundle for the dependency redox_syscall-0.9.4; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency redox_syscall-0.9.4; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ REGISTRY-LOCKFILE.md: Provenance/legal metadata retained for dependency REGISTRY-LOCKFILE.md; not │
│  │  │ runtime code.                                                                                     │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ renderdoc-sys-1.1.0/: License/provenance bundle for the dependency renderdoc-sys-1.1.0; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency renderdoc-sys-1.1.0; legal/provenance material │
│  │  │  │ only.                                                                                               │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency renderdoc-sys-1.1.0; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ rfc6381-codec-0.2.0/: License/provenance bundle for the dependency rfc6381-codec-0.2.0; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency rfc6381-codec-0.2.0; legal/provenance material │
│  │  │  │ only.                                                                                               │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency rfc6381-codec-0.2.0; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ rfd-0.16.0/: License/provenance bundle for the dependency rfd-0.16.0; legal metadata, not executable │
│  │  │ code.                                                                                                │
│  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency rfd-0.16.0; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ rustc-hash-1.1.0/: License/provenance bundle for the dependency rustc-hash-1.1.0; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency rustc-hash-1.1.0; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency rustc-hash-1.1.0; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ rustc-hash-2.1.3/: License/provenance bundle for the dependency rustc-hash-2.1.3; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency rustc-hash-2.1.3; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency rustc-hash-2.1.3; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ rustc_version-0.4.1/: License/provenance bundle for the dependency rustc_version-0.4.1; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency rustc_version-0.4.1; legal/provenance material │
│  │  │  │ only.                                                                                               │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency rustc_version-0.4.1; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ rustix-0.38.44/: License/provenance bundle for the dependency rustix-0.38.44; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ COPYRIGHT: Provenance/legal metadata retained for dependency rustix-0.38.44; not runtime code. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency rustix-0.38.44; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-Apache-2.0_WITH_LLVM-exception: License text retained for dependency rustix-0.38.44; │
│  │  │  │ legal/provenance material only.                                                              │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency rustix-0.38.44; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ rustix-1.1.4/: License/provenance bundle for the dependency rustix-1.1.4; legal metadata, not │
│  │  │ executable code.                                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ COPYRIGHT: Provenance/legal metadata retained for dependency rustix-1.1.4; not runtime code. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency rustix-1.1.4; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-Apache-2.0_WITH_LLVM-exception: License text retained for dependency rustix-1.1.4; │
│  │  │  │ legal/provenance material only.                                                            │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency rustix-1.1.4; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ rustversion-1.0.23/: License/provenance bundle for the dependency rustversion-1.0.23; legal metadata, │
│  │  │ not executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency rustversion-1.0.23; legal/provenance material │
│  │  │  │ only.                                                                                              │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency rustversion-1.0.23; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ same-file-1.0.6/: License/provenance bundle for the dependency same-file-1.0.6; legal metadata, not │
│  │  │ executable code.                                                                                    │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ COPYING: License text retained for dependency same-file-1.0.6; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency same-file-1.0.6; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ UNLICENSE: License text retained for dependency same-file-1.0.6; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ scoped-tls-1.0.1/: License/provenance bundle for the dependency scoped-tls-1.0.1; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency scoped-tls-1.0.1; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency scoped-tls-1.0.1; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ scopeguard-1.2.0/: License/provenance bundle for the dependency scopeguard-1.2.0; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency scopeguard-1.2.0; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency scopeguard-1.2.0; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ semver-1.0.28/: License/provenance bundle for the dependency semver-1.0.28; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency semver-1.0.28; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency semver-1.0.28; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ serde-1.0.229/: License/provenance bundle for the dependency serde-1.0.229; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency serde-1.0.229; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency serde-1.0.229; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ serde_core-1.0.229/: License/provenance bundle for the dependency serde_core-1.0.229; legal metadata, │
│  │  │ not executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency serde_core-1.0.229; legal/provenance material │
│  │  │  │ only.                                                                                              │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency serde_core-1.0.229; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ serde_derive-1.0.229/: License/provenance bundle for the dependency serde_derive-1.0.229; legal │
│  │  │ metadata, not executable code.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency serde_derive-1.0.229; legal/provenance material │
│  │  │  │ only.                                                                                                │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency serde_derive-1.0.229; legal/provenance material │
│  │     │ only.                                                                                             │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ serde_json-1.0.151/: License/provenance bundle for the dependency serde_json-1.0.151; legal metadata, │
│  │  │ not executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency serde_json-1.0.151; legal/provenance material │
│  │  │  │ only.                                                                                              │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency serde_json-1.0.151; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ sha2-0.10.9/: License/provenance bundle for the dependency sha2-0.10.9; legal metadata, not executable │
│  │  │ code.                                                                                                  │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency sha2-0.10.9; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency sha2-0.10.9; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ shlex-2.0.1/: License/provenance bundle for the dependency shlex-2.0.1; legal metadata, not executable │
│  │  │ code.                                                                                                  │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency shlex-2.0.1; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency shlex-2.0.1; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ simd-adler32-0.3.10/: License/provenance bundle for the dependency simd-adler32-0.3.10; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSES.md: License text retained for dependency simd-adler32-0.3.10; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ simd_cesu8-1.2.0/: License/provenance bundle for the dependency simd_cesu8-1.2.0; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency simd_cesu8-1.2.0; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency simd_cesu8-1.2.0; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ simdutf8-0.1.5/: License/provenance bundle for the dependency simdutf8-0.1.5; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-Apache: License text retained for dependency simdutf8-0.1.5; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency simdutf8-0.1.5; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ slab-0.4.12/: License/provenance bundle for the dependency slab-0.4.12; legal metadata, not executable │
│  │  │ code.                                                                                                  │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency slab-0.4.12; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ slotmap-1.1.1/: License/provenance bundle for the dependency slotmap-1.1.1; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency slotmap-1.1.1; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ smallvec-1.16.0/: License/provenance bundle for the dependency smallvec-1.16.0; legal metadata, not │
│  │  │ executable code.                                                                                    │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency smallvec-1.16.0; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency smallvec-1.16.0; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────┐
│  │  │ smithay-client-toolkit-0.20.0/: License/provenance bundle for the dependency │
│  │  │ smithay-client-toolkit-0.20.0; legal metadata, not executable code.          │
│  │  └──────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.txt: License text retained for dependency smithay-client-toolkit-0.20.0; legal/provenance │
│  │     │ material only.                                                                                    │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ smithay-clipboard-0.7.3/: License/provenance bundle for the dependency smithay-clipboard-0.7.3; legal │
│  │  │ metadata, not executable code.                                                                        │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency smithay-clipboard-0.7.3; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ smol_str-0.2.2/: License/provenance bundle for the dependency smol_str-0.2.2; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency smol_str-0.2.2; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency smol_str-0.2.2; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ stable_deref_trait-1.2.1/: License/provenance bundle for the dependency stable_deref_trait-1.2.1; │
│  │  │ legal metadata, not executable code.                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency stable_deref_trait-1.2.1; legal/provenance │
│  │  │  │ material only.                                                                                  │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency stable_deref_trait-1.2.1; legal/provenance material │
│  │     │ only.                                                                                                 │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ static_assertions-1.1.0/: License/provenance bundle for the dependency static_assertions-1.1.0; legal │
│  │  │ metadata, not executable code.                                                                        │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency static_assertions-1.1.0; legal/provenance │
│  │  │  │ material only.                                                                                 │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency static_assertions-1.1.0; legal/provenance material │
│  │     │ only.                                                                                                │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ syn-1.0.109/: License/provenance bundle for the dependency syn-1.0.109; legal metadata, not executable │
│  │  │ code.                                                                                                  │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency syn-1.0.109; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency syn-1.0.109; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ syn-2.0.119/: License/provenance bundle for the dependency syn-2.0.119; legal metadata, not executable │
│  │  │ code.                                                                                                  │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency syn-2.0.119; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency syn-2.0.119; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ syn-3.0.5/: License/provenance bundle for the dependency syn-3.0.5; legal metadata, not executable │
│  │  │ code.                                                                                              │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency syn-3.0.5; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency syn-3.0.5; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ synstructure-0.13.2/: License/provenance bundle for the dependency synstructure-0.13.2; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency synstructure-0.13.2; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ termcolor-1.4.1/: License/provenance bundle for the dependency termcolor-1.4.1; legal metadata, not │
│  │  │ executable code.                                                                                    │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ COPYING: License text retained for dependency termcolor-1.4.1; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency termcolor-1.4.1; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ UNLICENSE: License text retained for dependency termcolor-1.4.1; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ thiserror-1.0.69/: License/provenance bundle for the dependency thiserror-1.0.69; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency thiserror-1.0.69; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency thiserror-1.0.69; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ thiserror-2.0.20/: License/provenance bundle for the dependency thiserror-2.0.20; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency thiserror-2.0.20; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency thiserror-2.0.20; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ thiserror-impl-1.0.69/: License/provenance bundle for the dependency thiserror-impl-1.0.69; legal │
│  │  │ metadata, not executable code.                                                                    │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency thiserror-impl-1.0.69; legal/provenance material │
│  │  │  │ only.                                                                                                 │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency thiserror-impl-1.0.69; legal/provenance material │
│  │     │ only.                                                                                              │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ thiserror-impl-2.0.20/: License/provenance bundle for the dependency thiserror-impl-2.0.20; legal │
│  │  │ metadata, not executable code.                                                                    │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency thiserror-impl-2.0.20; legal/provenance material │
│  │  │  │ only.                                                                                                 │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency thiserror-impl-2.0.20; legal/provenance material │
│  │     │ only.                                                                                              │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ tiff-0.11.3/: License/provenance bundle for the dependency tiff-0.11.3; legal metadata, not executable │
│  │  │ code.                                                                                                  │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE: License text retained for dependency tiff-0.11.3; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ tests/: License/provenance bundle for the dependency tests; legal metadata, not executable code. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │
│  │     └──┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│  │        │ COPYRIGHT: Provenance/legal metadata retained for dependency tiff-0.11.3; not runtime code. │
│  │        └─────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ tinystr-0.8.4/: License/provenance bundle for the dependency tinystr-0.8.4; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency tinystr-0.8.4; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────┐
│  │  │ toml_datetime-1.1.1+spec-1.1.0/: License/provenance bundle for the dependency │
│  │  │ toml_datetime-1.1.1+spec-1.1.0; legal metadata, not executable code.          │
│  │  └───────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency toml_datetime-1.1.1+spec-1.1.0; legal/provenance │
│  │  │  │ material only.                                                                                        │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency toml_datetime-1.1.1+spec-1.1.0; legal/provenance │
│  │     │ material only.                                                                                     │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────┐
│  │  │ toml_edit-0.25.13+spec-1.1.0/: License/provenance bundle for the dependency │
│  │  │ toml_edit-0.25.13+spec-1.1.0; legal metadata, not executable code.          │
│  │  └─────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency toml_edit-0.25.13+spec-1.1.0; legal/provenance │
│  │  │  │ material only.                                                                                      │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency toml_edit-0.25.13+spec-1.1.0; legal/provenance │
│  │     │ material only.                                                                                   │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────┐
│  │  │ toml_parser-1.1.3+spec-1.1.0/: License/provenance bundle for the dependency │
│  │  │ toml_parser-1.1.3+spec-1.1.0; legal metadata, not executable code.          │
│  │  └─────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency toml_parser-1.1.3+spec-1.1.0; legal/provenance │
│  │  │  │ material only.                                                                                      │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency toml_parser-1.1.3+spec-1.1.0; legal/provenance │
│  │     │ material only.                                                                                   │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ tracing-0.1.44/: License/provenance bundle for the dependency tracing-0.1.44; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency tracing-0.1.44; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ tracing-attributes-0.1.31/: License/provenance bundle for the dependency tracing-attributes-0.1.31; │
│  │  │ legal metadata, not executable code.                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency tracing-attributes-0.1.31; legal/provenance material │
│  │     │ only.                                                                                              │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ tracing-core-0.1.36/: License/provenance bundle for the dependency tracing-core-0.1.36; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE: License text retained for dependency tracing-core-0.1.36; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ src/: License/provenance bundle for the dependency src; legal metadata, not executable code. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │     │
│  │     └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │        │ spin/: License/provenance bundle for the dependency spin; legal metadata, not executable code. │
│  │        └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │        │
│  │        └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │           │ LICENSE: License text retained for dependency tracing-core-0.1.36; legal/provenance material only. │
│  │           └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ ttf-parser-0.25.1/: License/provenance bundle for the dependency ttf-parser-0.25.1; legal metadata, │
│  │  │ not executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency ttf-parser-0.25.1; legal/provenance material │
│  │  │  │ only.                                                                                             │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency ttf-parser-0.25.1; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ typenum-1.20.1/: License/provenance bundle for the dependency typenum-1.20.1; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE: License text retained for dependency typenum-1.20.1; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency typenum-1.20.1; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency typenum-1.20.1; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ unicode-ident-1.0.24/: License/provenance bundle for the dependency unicode-ident-1.0.24; legal │
│  │  │ metadata, not executable code.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency unicode-ident-1.0.24; legal/provenance material │
│  │  │  │ only.                                                                                                │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency unicode-ident-1.0.24; legal/provenance material │
│  │  │  │ only.                                                                                             │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-UNICODE: License text retained for dependency unicode-ident-1.0.24; legal/provenance material │
│  │     │ only.                                                                                                 │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────┐
│  │  │ unicode-segmentation-1.13.3/: License/provenance bundle for the dependency │
│  │  │ unicode-segmentation-1.13.3; legal metadata, not executable code.          │
│  │  └────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ COPYRIGHT: Provenance/legal metadata retained for dependency unicode-segmentation-1.13.3; not runtime │
│  │  │  │ code.                                                                                                 │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency unicode-segmentation-1.13.3; legal/provenance │
│  │  │  │ material only.                                                                                     │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency unicode-segmentation-1.13.3; legal/provenance │
│  │     │ material only.                                                                                  │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ unicode-width-0.1.14/: License/provenance bundle for the dependency unicode-width-0.1.14; legal │
│  │  │ metadata, not executable code.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ COPYRIGHT: Provenance/legal metadata retained for dependency unicode-width-0.1.14; not runtime code. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency unicode-width-0.1.14; legal/provenance material │
│  │  │  │ only.                                                                                                │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency unicode-width-0.1.14; legal/provenance material │
│  │     │ only.                                                                                             │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ unicode-xid-0.2.6/: License/provenance bundle for the dependency unicode-xid-0.2.6; legal metadata, │
│  │  │ not executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency unicode-xid-0.2.6; legal/provenance material │
│  │  │  │ only.                                                                                             │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency unicode-xid-0.2.6; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ url-2.5.8/: License/provenance bundle for the dependency url-2.5.8; legal metadata, not executable │
│  │  │ code.                                                                                              │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency url-2.5.8; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency url-2.5.8; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ utf8_iter-1.0.4/: License/provenance bundle for the dependency utf8_iter-1.0.4; legal metadata, not │
│  │  │ executable code.                                                                                    │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ COPYRIGHT: Provenance/legal metadata retained for dependency utf8_iter-1.0.4; not runtime code. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency utf8_iter-1.0.4; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency utf8_iter-1.0.4; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ version_check-0.9.5/: License/provenance bundle for the dependency version_check-0.9.5; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency version_check-0.9.5; legal/provenance material │
│  │  │  │ only.                                                                                               │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency version_check-0.9.5; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ walkdir-2.5.0/: License/provenance bundle for the dependency walkdir-2.5.0; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ COPYING: License text retained for dependency walkdir-2.5.0; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency walkdir-2.5.0; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ UNLICENSE: License text retained for dependency walkdir-2.5.0; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wasm-bindgen-0.2.128/: License/provenance bundle for the dependency wasm-bindgen-0.2.128; legal │
│  │  │ metadata, not executable code.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency wasm-bindgen-0.2.128; legal/provenance material │
│  │  │  │ only.                                                                                                │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency wasm-bindgen-0.2.128; legal/provenance material │
│  │     │ only.                                                                                             │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────┐
│  │  │ wasm-bindgen-futures-0.4.78/: License/provenance bundle for the dependency │
│  │  │ wasm-bindgen-futures-0.4.78; legal metadata, not executable code.          │
│  │  └────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency wasm-bindgen-futures-0.4.78; legal/provenance │
│  │  │  │ material only.                                                                                     │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency wasm-bindgen-futures-0.4.78; legal/provenance │
│  │     │ material only.                                                                                  │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wasm-bindgen-macro-0.2.128/: License/provenance bundle for the dependency wasm-bindgen-macro-0.2.128; │
│  │  │ legal metadata, not executable code.                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency wasm-bindgen-macro-0.2.128; legal/provenance │
│  │  │  │ material only.                                                                                    │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency wasm-bindgen-macro-0.2.128; legal/provenance │
│  │     │ material only.                                                                                 │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wasm-bindgen-macro-support-0.2.128/: License/provenance bundle for the dependency │
│  │  │ wasm-bindgen-macro-support-0.2.128; legal metadata, not executable code.          │
│  │  └───────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency wasm-bindgen-macro-support-0.2.128; │
│  │  │  │ legal/provenance material only.                                                          │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency wasm-bindgen-macro-support-0.2.128; legal/provenance │
│  │     │ material only.                                                                                         │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────┐
│  │  │ wasm-bindgen-shared-0.2.128/: License/provenance bundle for the dependency │
│  │  │ wasm-bindgen-shared-0.2.128; legal metadata, not executable code.          │
│  │  └────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency wasm-bindgen-shared-0.2.128; legal/provenance │
│  │  │  │ material only.                                                                                     │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency wasm-bindgen-shared-0.2.128; legal/provenance │
│  │     │ material only.                                                                                  │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wayland-backend-0.3.17/: License/provenance bundle for the dependency wayland-backend-0.3.17; legal │
│  │  │ metadata, not executable code.                                                                      │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.txt: License text retained for dependency wayland-backend-0.3.17; legal/provenance material │
│  │     │ only.                                                                                               │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wayland-client-0.31.15/: License/provenance bundle for the dependency wayland-client-0.31.15; legal │
│  │  │ metadata, not executable code.                                                                      │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.txt: License text retained for dependency wayland-client-0.31.15; legal/provenance material │
│  │     │ only.                                                                                               │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wayland-csd-frame-0.3.0/: License/provenance bundle for the dependency wayland-csd-frame-0.3.0; legal │
│  │  │ metadata, not executable code.                                                                        │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency wayland-csd-frame-0.3.0; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wayland-cursor-0.31.14/: License/provenance bundle for the dependency wayland-cursor-0.31.14; legal │
│  │  │ metadata, not executable code.                                                                      │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.txt: License text retained for dependency wayland-cursor-0.31.14; legal/provenance material │
│  │     │ only.                                                                                               │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wayland-protocols-0.32.13/: License/provenance bundle for the dependency wayland-protocols-0.32.13; │
│  │  │ legal metadata, not executable code.                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.txt: License text retained for dependency wayland-protocols-0.32.13; legal/provenance material │
│  │     │ only.                                                                                                  │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wayland-protocols-experimental-20250721.0.1/: License/provenance bundle for the dependency │
│  │  │ wayland-protocols-experimental-20250721.0.1; legal metadata, not executable code.          │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.txt: License text retained for dependency wayland-protocols-experimental-20250721.0.1; │
│  │     │ legal/provenance material only.                                                                │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────┐
│  │  │ wayland-protocols-misc-0.3.12/: License/provenance bundle for the dependency │
│  │  │ wayland-protocols-misc-0.3.12; legal metadata, not executable code.          │
│  │  └──────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.txt: License text retained for dependency wayland-protocols-misc-0.3.12; legal/provenance │
│  │     │ material only.                                                                                    │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────┐
│  │  │ wayland-protocols-wlr-0.3.12/: License/provenance bundle for the dependency │
│  │  │ wayland-protocols-wlr-0.3.12; legal metadata, not executable code.          │
│  │  └─────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.txt: License text retained for dependency wayland-protocols-wlr-0.3.12; legal/provenance │
│  │     │ material only.                                                                                   │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wayland-scanner-0.31.11/: License/provenance bundle for the dependency wayland-scanner-0.31.11; legal │
│  │  │ metadata, not executable code.                                                                        │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.txt: License text retained for dependency wayland-scanner-0.31.11; legal/provenance material │
│  │     │ only.                                                                                                │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wayland-sys-0.31.11/: License/provenance bundle for the dependency wayland-sys-0.31.11; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.txt: License text retained for dependency wayland-sys-0.31.11; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ web-sys-0.3.105/: License/provenance bundle for the dependency web-sys-0.3.105; legal metadata, not │
│  │  │ executable code.                                                                                    │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency web-sys-0.3.105; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency web-sys-0.3.105; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ web-time-1.1.0/: License/provenance bundle for the dependency web-time-1.1.0; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency web-time-1.1.0; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency web-time-1.1.0; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ webbrowser-1.2.4/: License/provenance bundle for the dependency webbrowser-1.2.4; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency webbrowser-1.2.4; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency webbrowser-1.2.4; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ weezl-0.1.12/: License/provenance bundle for the dependency weezl-0.1.12; legal metadata, not │
│  │  │ executable code.                                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency weezl-0.1.12; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency weezl-0.1.12; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wgpu-0.19.4/: License/provenance bundle for the dependency wgpu-0.19.4; legal metadata, not executable │
│  │  │ code.                                                                                                  │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE.APACHE: License text retained for dependency wgpu-0.19.4; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.MIT: License text retained for dependency wgpu-0.19.4; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wgpu-29.0.0/: License/provenance bundle for the dependency wgpu-29.0.0; legal metadata, not executable │
│  │  │ code.                                                                                                  │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE.APACHE: License text retained for dependency wgpu-29.0.0; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.MIT: License text retained for dependency wgpu-29.0.0; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wgpu-core-0.19.4/: License/provenance bundle for the dependency wgpu-core-0.19.4; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE.APACHE: License text retained for dependency wgpu-core-0.19.4; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.MIT: License text retained for dependency wgpu-core-0.19.4; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wgpu-core-29.0.4/: License/provenance bundle for the dependency wgpu-core-29.0.4; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE.APACHE: License text retained for dependency wgpu-core-29.0.4; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.MIT: License text retained for dependency wgpu-core-29.0.4; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wgpu-core-deps-windows-linux-android-29.0.4/: License/provenance bundle for the dependency │
│  │  │ wgpu-core-deps-windows-linux-android-29.0.4; legal metadata, not executable code.          │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE.APACHE: License text retained for dependency wgpu-core-deps-windows-linux-android-29.0.4; │
│  │  │  │ legal/provenance material only.                                                                   │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.MIT: License text retained for dependency wgpu-core-deps-windows-linux-android-29.0.4; │
│  │     │ legal/provenance material only.                                                                │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wgpu-hal-0.19.5/: License/provenance bundle for the dependency wgpu-hal-0.19.5; legal metadata, not │
│  │  │ executable code.                                                                                    │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE.APACHE: License text retained for dependency wgpu-hal-0.19.5; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.MIT: License text retained for dependency wgpu-hal-0.19.5; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wgpu-hal-29.0.4/: License/provenance bundle for the dependency wgpu-hal-29.0.4; legal metadata, not │
│  │  │ executable code.                                                                                    │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ A865R-PATCH.md: Provenance/legal metadata retained for dependency wgpu-hal-29.0.4; not runtime code. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE.APACHE: License text retained for dependency wgpu-hal-29.0.4; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.MIT: License text retained for dependency wgpu-hal-29.0.4; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wgpu-naga-bridge-29.0.4/: License/provenance bundle for the dependency wgpu-naga-bridge-29.0.4; legal │
│  │  │ metadata, not executable code.                                                                        │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE.APACHE: License text retained for dependency wgpu-naga-bridge-29.0.4; legal/provenance │
│  │  │  │ material only.                                                                                 │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.MIT: License text retained for dependency wgpu-naga-bridge-29.0.4; legal/provenance material │
│  │     │ only.                                                                                                │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wgpu-types-0.19.2/: License/provenance bundle for the dependency wgpu-types-0.19.2; legal metadata, │
│  │  │ not executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE.APACHE: License text retained for dependency wgpu-types-0.19.2; legal/provenance material │
│  │  │  │ only.                                                                                             │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.MIT: License text retained for dependency wgpu-types-0.19.2; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ wgpu-types-29.0.4/: License/provenance bundle for the dependency wgpu-types-29.0.4; legal metadata, │
│  │  │ not executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE.APACHE: License text retained for dependency wgpu-types-29.0.4; legal/provenance material │
│  │  │  │ only.                                                                                             │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE.MIT: License text retained for dependency wgpu-types-29.0.4; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ widestring-1.2.1/: License/provenance bundle for the dependency widestring-1.2.1; legal metadata, not │
│  │  │ executable code.                                                                                      │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency widestring-1.2.1; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency widestring-1.2.1; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ winapi-0.3.9/: License/provenance bundle for the dependency winapi-0.3.9; legal metadata, not │
│  │  │ executable code.                                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency winapi-0.3.9; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency winapi-0.3.9; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ winapi-util-0.1.11/: License/provenance bundle for the dependency winapi-util-0.1.11; legal metadata, │
│  │  │ not executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ COPYING: License text retained for dependency winapi-util-0.1.11; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency winapi-util-0.1.11; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ UNLICENSE: License text retained for dependency winapi-util-0.1.11; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-0.52.0/: License/provenance bundle for the dependency windows-0.52.0; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-0.52.0; legal/provenance material │
│  │  │  │ only.                                                                                              │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-0.52.0; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-0.58.0/: License/provenance bundle for the dependency windows-0.58.0; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-0.58.0; legal/provenance material │
│  │  │  │ only.                                                                                              │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-0.58.0; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-0.62.2/: License/provenance bundle for the dependency windows-0.62.2; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-0.62.2; legal/provenance material │
│  │  │  │ only.                                                                                              │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-0.62.2; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-collections-0.3.2/: License/provenance bundle for the dependency windows-collections-0.3.2; │
│  │  │ legal metadata, not executable code.                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-collections-0.3.2; legal/provenance │
│  │  │  │ material only.                                                                                       │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-collections-0.3.2; legal/provenance material │
│  │     │ only.                                                                                                  │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-core-0.52.0/: License/provenance bundle for the dependency windows-core-0.52.0; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-core-0.52.0; legal/provenance │
│  │  │  │ material only.                                                                                 │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-core-0.52.0; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-core-0.58.0/: License/provenance bundle for the dependency windows-core-0.58.0; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-core-0.58.0; legal/provenance │
│  │  │  │ material only.                                                                                 │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-core-0.58.0; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-core-0.62.2/: License/provenance bundle for the dependency windows-core-0.62.2; legal │
│  │  │ metadata, not executable code.                                                                │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-core-0.62.2; legal/provenance │
│  │  │  │ material only.                                                                                 │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-core-0.62.2; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-future-0.3.2/: License/provenance bundle for the dependency windows-future-0.3.2; legal │
│  │  │ metadata, not executable code.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-future-0.3.2; legal/provenance │
│  │  │  │ material only.                                                                                  │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-future-0.3.2; legal/provenance material │
│  │     │ only.                                                                                             │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-implement-0.58.0/: License/provenance bundle for the dependency windows-implement-0.58.0; │
│  │  │ legal metadata, not executable code.                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-implement-0.58.0; legal/provenance │
│  │  │  │ material only.                                                                                      │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-implement-0.58.0; legal/provenance material │
│  │     │ only.                                                                                                 │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-implement-0.60.2/: License/provenance bundle for the dependency windows-implement-0.60.2; │
│  │  │ legal metadata, not executable code.                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-implement-0.60.2; legal/provenance │
│  │  │  │ material only.                                                                                      │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-implement-0.60.2; legal/provenance material │
│  │     │ only.                                                                                                 │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-interface-0.58.0/: License/provenance bundle for the dependency windows-interface-0.58.0; │
│  │  │ legal metadata, not executable code.                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-interface-0.58.0; legal/provenance │
│  │  │  │ material only.                                                                                      │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-interface-0.58.0; legal/provenance material │
│  │     │ only.                                                                                                 │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-interface-0.59.3/: License/provenance bundle for the dependency windows-interface-0.59.3; │
│  │  │ legal metadata, not executable code.                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-interface-0.59.3; legal/provenance │
│  │  │  │ material only.                                                                                      │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-interface-0.59.3; legal/provenance material │
│  │     │ only.                                                                                                 │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-link-0.2.1/: License/provenance bundle for the dependency windows-link-0.2.1; legal metadata, │
│  │  │ not executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-link-0.2.1; legal/provenance material │
│  │  │  │ only.                                                                                                  │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-link-0.2.1; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-numerics-0.3.1/: License/provenance bundle for the dependency windows-numerics-0.3.1; legal │
│  │  │ metadata, not executable code.                                                                      │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-numerics-0.3.1; legal/provenance │
│  │  │  │ material only.                                                                                    │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-numerics-0.3.1; legal/provenance material │
│  │     │ only.                                                                                               │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-result-0.2.0/: License/provenance bundle for the dependency windows-result-0.2.0; legal │
│  │  │ metadata, not executable code.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-result-0.2.0; legal/provenance │
│  │  │  │ material only.                                                                                  │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-result-0.2.0; legal/provenance material │
│  │     │ only.                                                                                             │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-result-0.4.1/: License/provenance bundle for the dependency windows-result-0.4.1; legal │
│  │  │ metadata, not executable code.                                                                  │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-result-0.4.1; legal/provenance │
│  │  │  │ material only.                                                                                  │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-result-0.4.1; legal/provenance material │
│  │     │ only.                                                                                             │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-strings-0.1.0/: License/provenance bundle for the dependency windows-strings-0.1.0; legal │
│  │  │ metadata, not executable code.                                                                    │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-strings-0.1.0; legal/provenance │
│  │  │  │ material only.                                                                                   │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-strings-0.1.0; legal/provenance material │
│  │     │ only.                                                                                              │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-strings-0.5.1/: License/provenance bundle for the dependency windows-strings-0.5.1; legal │
│  │  │ metadata, not executable code.                                                                    │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-strings-0.5.1; legal/provenance │
│  │  │  │ material only.                                                                                   │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-strings-0.5.1; legal/provenance material │
│  │     │ only.                                                                                              │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-sys-0.52.0/: License/provenance bundle for the dependency windows-sys-0.52.0; legal metadata, │
│  │  │ not executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-sys-0.52.0; legal/provenance material │
│  │  │  │ only.                                                                                                  │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-sys-0.52.0; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-sys-0.59.0/: License/provenance bundle for the dependency windows-sys-0.59.0; legal metadata, │
│  │  │ not executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-sys-0.59.0; legal/provenance material │
│  │  │  │ only.                                                                                                  │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-sys-0.59.0; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-sys-0.60.2/: License/provenance bundle for the dependency windows-sys-0.60.2; legal metadata, │
│  │  │ not executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-sys-0.60.2; legal/provenance material │
│  │  │  │ only.                                                                                                  │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-sys-0.60.2; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-sys-0.61.2/: License/provenance bundle for the dependency windows-sys-0.61.2; legal metadata, │
│  │  │ not executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-sys-0.61.2; legal/provenance material │
│  │  │  │ only.                                                                                                  │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-sys-0.61.2; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-targets-0.52.6/: License/provenance bundle for the dependency windows-targets-0.52.6; legal │
│  │  │ metadata, not executable code.                                                                      │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-targets-0.52.6; legal/provenance │
│  │  │  │ material only.                                                                                    │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-targets-0.52.6; legal/provenance material │
│  │     │ only.                                                                                               │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-targets-0.53.5/: License/provenance bundle for the dependency windows-targets-0.53.5; legal │
│  │  │ metadata, not executable code.                                                                      │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-targets-0.53.5; legal/provenance │
│  │  │  │ material only.                                                                                    │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-targets-0.53.5; legal/provenance material │
│  │     │ only.                                                                                               │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows-threading-0.2.1/: License/provenance bundle for the dependency windows-threading-0.2.1; legal │
│  │  │ metadata, not executable code.                                                                        │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows-threading-0.2.1; legal/provenance │
│  │  │  │ material only.                                                                                     │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows-threading-0.2.1; legal/provenance material │
│  │     │ only.                                                                                                │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows_aarch64_gnullvm-0.52.6/: License/provenance bundle for the dependency │
│  │  │ windows_aarch64_gnullvm-0.52.6; legal metadata, not executable code.          │
│  │  └───────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows_aarch64_gnullvm-0.52.6; │
│  │  │  │ legal/provenance material only.                                                          │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows_aarch64_gnullvm-0.52.6; legal/provenance │
│  │     │ material only.                                                                                     │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows_aarch64_gnullvm-0.53.1/: License/provenance bundle for the dependency │
│  │  │ windows_aarch64_gnullvm-0.53.1; legal metadata, not executable code.          │
│  │  └───────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows_aarch64_gnullvm-0.53.1; │
│  │  │  │ legal/provenance material only.                                                          │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows_aarch64_gnullvm-0.53.1; legal/provenance │
│  │     │ material only.                                                                                     │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows_aarch64_msvc-0.52.6/: License/provenance bundle for the dependency │
│  │  │ windows_aarch64_msvc-0.52.6; legal metadata, not executable code.          │
│  │  └────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows_aarch64_msvc-0.52.6; legal/provenance │
│  │  │  │ material only.                                                                                         │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows_aarch64_msvc-0.52.6; legal/provenance │
│  │     │ material only.                                                                                  │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows_aarch64_msvc-0.53.1/: License/provenance bundle for the dependency │
│  │  │ windows_aarch64_msvc-0.53.1; legal metadata, not executable code.          │
│  │  └────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows_aarch64_msvc-0.53.1; legal/provenance │
│  │  │  │ material only.                                                                                         │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows_aarch64_msvc-0.53.1; legal/provenance │
│  │     │ material only.                                                                                  │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows_i686_gnu-0.52.6/: License/provenance bundle for the dependency windows_i686_gnu-0.52.6; legal │
│  │  │ metadata, not executable code.                                                                        │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows_i686_gnu-0.52.6; legal/provenance │
│  │  │  │ material only.                                                                                     │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows_i686_gnu-0.52.6; legal/provenance material │
│  │     │ only.                                                                                                │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows_i686_gnu-0.53.1/: License/provenance bundle for the dependency windows_i686_gnu-0.53.1; legal │
│  │  │ metadata, not executable code.                                                                        │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows_i686_gnu-0.53.1; legal/provenance │
│  │  │  │ material only.                                                                                     │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows_i686_gnu-0.53.1; legal/provenance material │
│  │     │ only.                                                                                                │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows_i686_gnullvm-0.52.6/: License/provenance bundle for the dependency │
│  │  │ windows_i686_gnullvm-0.52.6; legal metadata, not executable code.          │
│  │  └────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows_i686_gnullvm-0.52.6; legal/provenance │
│  │  │  │ material only.                                                                                         │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows_i686_gnullvm-0.52.6; legal/provenance │
│  │     │ material only.                                                                                  │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows_i686_gnullvm-0.53.1/: License/provenance bundle for the dependency │
│  │  │ windows_i686_gnullvm-0.53.1; legal metadata, not executable code.          │
│  │  └────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows_i686_gnullvm-0.53.1; legal/provenance │
│  │  │  │ material only.                                                                                         │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows_i686_gnullvm-0.53.1; legal/provenance │
│  │     │ material only.                                                                                  │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows_i686_msvc-0.52.6/: License/provenance bundle for the dependency windows_i686_msvc-0.52.6; │
│  │  │ legal metadata, not executable code.                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows_i686_msvc-0.52.6; legal/provenance │
│  │  │  │ material only.                                                                                      │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows_i686_msvc-0.52.6; legal/provenance material │
│  │     │ only.                                                                                                 │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows_i686_msvc-0.53.1/: License/provenance bundle for the dependency windows_i686_msvc-0.53.1; │
│  │  │ legal metadata, not executable code.                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows_i686_msvc-0.53.1; legal/provenance │
│  │  │  │ material only.                                                                                      │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows_i686_msvc-0.53.1; legal/provenance material │
│  │     │ only.                                                                                                 │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows_x86_64_gnu-0.52.6/: License/provenance bundle for the dependency windows_x86_64_gnu-0.52.6; │
│  │  │ legal metadata, not executable code.                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows_x86_64_gnu-0.52.6; legal/provenance │
│  │  │  │ material only.                                                                                       │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows_x86_64_gnu-0.52.6; legal/provenance material │
│  │     │ only.                                                                                                  │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows_x86_64_gnu-0.53.1/: License/provenance bundle for the dependency windows_x86_64_gnu-0.53.1; │
│  │  │ legal metadata, not executable code.                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows_x86_64_gnu-0.53.1; legal/provenance │
│  │  │  │ material only.                                                                                       │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows_x86_64_gnu-0.53.1; legal/provenance material │
│  │     │ only.                                                                                                  │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows_x86_64_gnullvm-0.52.6/: License/provenance bundle for the dependency │
│  │  │ windows_x86_64_gnullvm-0.52.6; legal metadata, not executable code.          │
│  │  └──────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows_x86_64_gnullvm-0.52.6; │
│  │  │  │ legal/provenance material only.                                                         │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows_x86_64_gnullvm-0.52.6; legal/provenance │
│  │     │ material only.                                                                                    │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows_x86_64_gnullvm-0.53.1/: License/provenance bundle for the dependency │
│  │  │ windows_x86_64_gnullvm-0.53.1; legal metadata, not executable code.          │
│  │  └──────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows_x86_64_gnullvm-0.53.1; │
│  │  │  │ legal/provenance material only.                                                         │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows_x86_64_gnullvm-0.53.1; legal/provenance │
│  │     │ material only.                                                                                    │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows_x86_64_msvc-0.52.6/: License/provenance bundle for the dependency windows_x86_64_msvc-0.52.6; │
│  │  │ legal metadata, not executable code.                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows_x86_64_msvc-0.52.6; legal/provenance │
│  │  │  │ material only.                                                                                        │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows_x86_64_msvc-0.52.6; legal/provenance │
│  │     │ material only.                                                                                 │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ windows_x86_64_msvc-0.53.1/: License/provenance bundle for the dependency windows_x86_64_msvc-0.53.1; │
│  │  │ legal metadata, not executable code.                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ license-apache-2.0: License text retained for dependency windows_x86_64_msvc-0.53.1; legal/provenance │
│  │  │  │ material only.                                                                                        │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ license-mit: License text retained for dependency windows_x86_64_msvc-0.53.1; legal/provenance │
│  │     │ material only.                                                                                 │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ winit-0.30.13/: License/provenance bundle for the dependency winit-0.30.13; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency winit-0.30.13; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ winnow-1.0.4/: License/provenance bundle for the dependency winnow-1.0.4; legal metadata, not │
│  │  │ executable code.                                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency winnow-1.0.4; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ writeable-0.6.4/: License/provenance bundle for the dependency writeable-0.6.4; legal metadata, not │
│  │  │ executable code.                                                                                    │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency writeable-0.6.4; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ x11rb-0.13.2/: License/provenance bundle for the dependency x11rb-0.13.2; legal metadata, not │
│  │  │ executable code.                                                                              │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency x11rb-0.13.2; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency x11rb-0.13.2; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ x11rb-protocol-0.13.2/: License/provenance bundle for the dependency x11rb-protocol-0.13.2; legal │
│  │  │ metadata, not executable code.                                                                    │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency x11rb-protocol-0.13.2; legal/provenance material │
│  │  │  │ only.                                                                                                 │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency x11rb-protocol-0.13.2; legal/provenance material │
│  │     │ only.                                                                                              │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ xcursor-0.3.11/: License/provenance bundle for the dependency xcursor-0.3.11; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency xcursor-0.3.11; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ xkbcommon-dl-0.4.2/: License/provenance bundle for the dependency xkbcommon-dl-0.4.2; legal metadata, │
│  │  │ not executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency xkbcommon-dl-0.4.2; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ xkeysym-0.2.1/: License/provenance bundle for the dependency xkeysym-0.2.1; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency xkeysym-0.2.1; legal/provenance material only. │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency xkeysym-0.2.1; legal/provenance material only. │
│  │  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-ZLIB: License text retained for dependency xkeysym-0.2.1; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ xml-rs-0.8.29/: License/provenance bundle for the dependency xml-rs-0.8.29; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency xml-rs-0.8.29; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ yoke-0.8.3/: License/provenance bundle for the dependency yoke-0.8.3; legal metadata, not executable │
│  │  │ code.                                                                                                │
│  │  └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency yoke-0.8.3; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ yoke-derive-0.8.2/: License/provenance bundle for the dependency yoke-derive-0.8.2; legal metadata, │
│  │  │ not executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency yoke-derive-0.8.2; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ zerocopy-0.8.56/: License/provenance bundle for the dependency zerocopy-0.8.56; legal metadata, not │
│  │  │ executable code.                                                                                    │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency zerocopy-0.8.56; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-BSD: License text retained for dependency zerocopy-0.8.56; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency zerocopy-0.8.56; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ zerocopy-derive-0.8.56/: License/provenance bundle for the dependency zerocopy-derive-0.8.56; legal │
│  │  │ metadata, not executable code.                                                                      │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency zerocopy-derive-0.8.56; legal/provenance material │
│  │  │  │ only.                                                                                                  │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-BSD: License text retained for dependency zerocopy-derive-0.8.56; legal/provenance material │
│  │  │  │ only.                                                                                               │
│  │  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency zerocopy-derive-0.8.56; legal/provenance material │
│  │     │ only.                                                                                               │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ zerofrom-0.1.8/: License/provenance bundle for the dependency zerofrom-0.1.8; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency zerofrom-0.1.8; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ zerofrom-derive-0.1.7/: License/provenance bundle for the dependency zerofrom-derive-0.1.7; legal │
│  │  │ metadata, not executable code.                                                                    │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency zerofrom-derive-0.1.7; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ zerotrie-0.2.5/: License/provenance bundle for the dependency zerotrie-0.2.5; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency zerotrie-0.2.5; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ zerovec-0.11.8/: License/provenance bundle for the dependency zerovec-0.11.8; legal metadata, not │
│  │  │ executable code.                                                                                  │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency zerovec-0.11.8; legal/provenance material only. │
│  │     └───────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ zerovec-derive-0.11.6/: License/provenance bundle for the dependency zerovec-derive-0.11.6; legal │
│  │  │ metadata, not executable code.                                                                    │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency zerovec-derive-0.11.6; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ zlib-rs-0.6.7/: License/provenance bundle for the dependency zlib-rs-0.6.7; legal metadata, not │
│  │  │ executable code.                                                                                │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE: License text retained for dependency zlib-rs-0.6.7; legal/provenance material only. │
│  │     └──────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ zmij-1.0.23/: License/provenance bundle for the dependency zmij-1.0.23; legal metadata, not executable │
│  │  │ code.                                                                                                  │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  └──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-MIT: License text retained for dependency zmij-1.0.23; legal/provenance material only. │
│  │     └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ zune-core-0.5.3/: License/provenance bundle for the dependency zune-core-0.5.3; legal metadata, not │
│  │  │ executable code.                                                                                    │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  │
│  │  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-APACHE: License text retained for dependency zune-core-0.5.3; legal/provenance material only. │
│  │  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │  │ LICENSE-MIT: License text retained for dependency zune-core-0.5.3; legal/provenance material only. │
│  │  │  └────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │  └──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │     │ LICENSE-ZLIB: License text retained for dependency zune-core-0.5.3; legal/provenance material only. │
│  │     └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  └──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │ zune-jpeg-0.5.15/: License/provenance bundle for the dependency zune-jpeg-0.5.15; legal metadata, not │
│     │ executable code.                                                                                      │
│     └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     │
│     ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │ LICENSE-APACHE: License text retained for dependency zune-jpeg-0.5.15; legal/provenance material only. │
│     │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     ├──┌─────────────────────────────────────────────────────────────────────────────────────────────────────┐
│     │  │ LICENSE-MIT: License text retained for dependency zune-jpeg-0.5.15; legal/provenance material only. │
│     │  └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
│     └──┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│        │ LICENSE-ZLIB: License text retained for dependency zune-jpeg-0.5.15; legal/provenance material only. │
│        └──────────────────────────────────────────────────────────────────────────────────────────────────────┘
├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │ docs/: Historical architecture, reverse-engineering, validation, release, firmware, BDA, Vulkan, │
│  │ power, UI, and hardware research records.                                                        │
│  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  │
│  ├──┌────────────────────────────────────────────────────────────┐
│  │  │ ALPHA20-VALIDATION.md: Documentation: Alpha 20 validation. │
│  │  └────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────┐
│  │  │ ALPHA21-VALIDATION.md: Documentation: Alpha 21 validation. │
│  │  └────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────┐
│  │  │ ALPHA22-VALIDATION.md: Documentation: Alpha 22 validation. │
│  │  └────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────┐
│  │  │ ALPHA23-VALIDATION.md: Documentation: Alpha 23 validation — 2026-09-11. │
│  │  └─────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────┐
│  │  │ ALPHA24-VALIDATION.md: Documentation: Alpha 24 validation — 2026-09-11. │
│  │  └─────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────┐
│  │  │ ALPHA26-VALIDATION.md: Documentation: Alpha 26 — Native console validation. │
│  │  └─────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────┐
│  │  │ ALPHA27-VALIDATION.md: Documentation: Live TV! 0.8.0-alpha.27. │
│  │  └────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────┐
│  │  │ ALPHA28-VALIDATION.md: Documentation: Live TV! 0.8.0-alpha.28. │
│  │  └────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────┐
│  │  │ ALPHA29-VALIDATION.md: Documentation: Live TV! 0.8.0-alpha.29. │
│  │  └────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────┐
│  │  │ ALPHA30-VALIDATION.md: Documentation: Live TV! 0.8.0-alpha.30. │
│  │  └────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────┐
│  │  │ ALPHA31-VALIDATION.md: Documentation: Live TV! 0.8.0-alpha.31. │
│  │  └────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────┐
│  │  │ ALPHA32-VALIDATION.md: Documentation: Live TV! 0.8.0-alpha.32. │
│  │  └────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ ALPHA33-ICON-VERIFICATION.json: JSON configuration, evidence, manifest, or generated data for ALPHA33 │
│  │  │ ICON VERIFICATION.                                                                                    │
│  │  └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────┐
│  │  │ ALPHA33-VALIDATION.md: Documentation: Live TV! 0.8.0-alpha.33. │
│  │  └────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────┐
│  │  │ ALPHA34-VALIDATION.md: Documentation: Live TV! 0.8.0-alpha.34. │
│  │  └────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────┐
│  │  │ ALPHA35-VALIDATION.md: Documentation: Alpha.35 validation. │
│  │  └────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ ALPHA36-HARDWARE.json: JSON configuration, evidence, manifest, or generated data for ALPHA36 HARDWARE. │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ ALPHA36-VALIDATION.md: Documentation: Alpha.36 — hardware-tested recording stability. │
│  │  └───────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────┐
│  │  │ ALPHA37-VALIDATION.md: Documentation: Alpha.37 — unified release. │
│  │  └───────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ ALPHA37-VERIFICATION.json: JSON configuration, evidence, manifest, or generated data for ALPHA37 │
│  │  │ VERIFICATION.                                                                                    │
│  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ ALPHA38-INVESTIGATION.md: Documentation: Alpha.38 investigation — interim package, Vulkan repair │
│  │  │ incomplete.                                                                                      │
│  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────┐
│  │  │ ALPHA38-VALIDATION.md: Documentation: Alpha.38 — interim Vulkan package. │
│  │  └──────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────┐
│  │  │ ALPHA39-VALIDATION.md: Documentation: Alpha.39 validation. │
│  │  └────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────┐
│  │  │ ALPHA40-VALIDATION.md: Documentation: Alpha.40 validation. │
│  │  └────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────┐
│  │  │ ALPHA41-VALIDATION.md: Documentation: Alpha.41 validation — 2026-09-13. │
│  │  └─────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────┐
│  │  │ ALPHA42-VALIDATION.md: Documentation: Alpha.42 validation. │
│  │  └────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────┐
│  │  │ API.md: Documentation: Rust application API, version 2. │
│  │  └─────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────┐
│  │  │ ASPECT-OSD-RBI-ALPHA10.md: Documentation: A865R TV alpha 10 changes. │
│  │  └──────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ AVERTV-FULLSCREEN-COMPARISON.md: Documentation: Original AVerTV fullscreen comparison. │
│  │  └────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────┐
│  │  │ AVERTV_UPDATE_0.7.0.md: Documentation: AVerTV compatibility follow-up to 0.7.0. │
│  │  └─────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────┐
│  │  │ BDA_COMPATIBILITY.md: Documentation: Experimental userspace BDA adapter. │
│  │  └──────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────┐
│  │  │ DESIGN.md: Documentation: Windows userspace and open firmware design. │
│  │  └───────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ firmware-comparison.json: JSON configuration, evidence, manifest, or generated data for firmware │
│  │  │ comparison.                                                                                      │
│  │  └──────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────────┐
│  │  │ GPU-PRESENTATION-DESIGN.md: Documentation: Multithreaded GPU frame-buffer design. │
│  │  └───────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────────────┐
│  │  │ NATIVE_PLAYER_PHASE2.md: Documentation: Native player inspection and validation. │
│  │  └──────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ OFDM_RECONSTRUCTION.md: Documentation: OFDM service reconstruction — open firmware 0.1.3.0. │
│  │  └─────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────────────────────┐
│  │  │ OPEN_FIRMWARE.md: Documentation: Open firmware test results, 2026-09-10. │
│  │  └──────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────┐
│  │  │ POWER_MANAGEMENT.md: Documentation: A865R power-management reconstruction. │
│  │  └────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────┐
│  │  │ RECEPTION.md: Documentation: Reception and playback, 0.5.0. │
│  │  └─────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────┐
│  │  │ RELEASE_0.7.0.md: Documentation: Local development release 0.7.0. │
│  │  └───────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────┐
│  │  │ REMOTE_CONTROL.md: Documentation: Infrared remote development, 0.6.0. │
│  │  └───────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────────┐
│  │  │ RESEARCH.md: Documentation: Evidence from the supplied A865R files. │
│  │  └─────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ RF_INTEGRATION_0.1.4.md: Documentation: Open firmware 0.1.4.0 — reception and standby results. │
│  │  └────────────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌─────────────────────────────────────────────────────────────────┐
│  │  │ START_MENU_REPAIR.md: Documentation: Start Menu adapter repair. │
│  │  └─────────────────────────────────────────────────────────────────┘
│  ├──┌───────────────────────────────────────────────────────────────────────────────┐
│  │  │ TRANSPARENT-OSD-ALPHA11.md: Documentation: AVerTV transparent OSD inspection. │
│  │  └───────────────────────────────────────────────────────────────────────────────┘
│  ├──┌────────────────────────────────────────────────────────────────────────────────────────┐
│  │  │ TV-CAMARA-GPU-DIAGNOSIS.md: Documentation: TV Câmara GPU crash diagnosis — 2026-09-12. │
│  │  └────────────────────────────────────────────────────────────────────────────────────────┘
│  ├──┌──────────────────────────────────────────────────────────┐
│  │  │ VALIDATION.md: Documentation: Release validation, 0.5.0. │
│  │  └──────────────────────────────────────────────────────────┘
│  └──┌────────────────────────────────────────────────────────────────────────────────┐
│     │ VULKAN-VIDEO-PREVIEW.md: Documentation: Vulkan Video preview — 0.8.0-alpha.24. │
│     └────────────────────────────────────────────────────────────────────────────────┘
└──┌───────────────────────────────────────────────────────────────────────────────────────────────────┐
   │ images/: Project artwork masters, exported icons, screenshots, and image-generation/export helper │
   │ scripts.                                                                                          │
   └───────────────────────────────────────────────────────────────────────────────────────────────────┘
   │
   ├──┌──────────────────────────────────────────────────────────────────────────────────────────┐
   │  │ draw_volar_ruby_etched.py: Python developer/analysis utility for draw volar ruby etched. │
   │  └──────────────────────────────────────────────────────────────────────────────────────────┘
   ├──┌──────────────────────────────────────────────────────────────────┐
   │  │ export_svg.py: Python developer/analysis utility for export svg. │
   │  └──────────────────────────────────────────────────────────────────┘
   ├──┌───────────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-1024.png: Artwork/UI/image asset: open-volar-s-1024.png. │
   │  └───────────────────────────────────────────────────────────────────────┘
   ├──┌─────────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-128.png: Artwork/UI/image asset: open-volar-s-128.png. │
   │  └─────────────────────────────────────────────────────────────────────┘
   ├──┌───────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-16.png: Artwork/UI/image asset: open-volar-s-16.png. │
   │  └───────────────────────────────────────────────────────────────────┘
   ├──┌───────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-20.png: Artwork/UI/image asset: open-volar-s-20.png. │
   │  └───────────────────────────────────────────────────────────────────┘
   ├──┌───────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-24.png: Artwork/UI/image asset: open-volar-s-24.png. │
   │  └───────────────────────────────────────────────────────────────────┘
   ├──┌─────────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-256.png: Artwork/UI/image asset: open-volar-s-256.png. │
   │  └─────────────────────────────────────────────────────────────────────┘
   ├──┌───────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-32.png: Artwork/UI/image asset: open-volar-s-32.png. │
   │  └───────────────────────────────────────────────────────────────────┘
   ├──┌───────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-40.png: Artwork/UI/image asset: open-volar-s-40.png. │
   │  └───────────────────────────────────────────────────────────────────┘
   ├──┌───────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-48.png: Artwork/UI/image asset: open-volar-s-48.png. │
   │  └───────────────────────────────────────────────────────────────────┘
   ├──┌─────────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-512.png: Artwork/UI/image asset: open-volar-s-512.png. │
   │  └─────────────────────────────────────────────────────────────────────┘
   ├──┌───────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-64.png: Artwork/UI/image asset: open-volar-s-64.png. │
   │  └───────────────────────────────────────────────────────────────────┘
   ├──┌───────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-96.png: Artwork/UI/image asset: open-volar-s-96.png. │
   │  └───────────────────────────────────────────────────────────────────┘
   ├──┌───────────────────────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-steep-1024.png: Artwork/UI/image asset: open-volar-s-steep-1024.png. │
   │  └───────────────────────────────────────────────────────────────────────────────────┘
   ├──┌─────────────────────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-steep-128.png: Artwork/UI/image asset: open-volar-s-steep-128.png. │
   │  └─────────────────────────────────────────────────────────────────────────────────┘
   ├──┌───────────────────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-steep-16.png: Artwork/UI/image asset: open-volar-s-steep-16.png. │
   │  └───────────────────────────────────────────────────────────────────────────────┘
   ├──┌───────────────────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-steep-20.png: Artwork/UI/image asset: open-volar-s-steep-20.png. │
   │  └───────────────────────────────────────────────────────────────────────────────┘
   ├──┌───────────────────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-steep-24.png: Artwork/UI/image asset: open-volar-s-steep-24.png. │
   │  └───────────────────────────────────────────────────────────────────────────────┘
   ├──┌─────────────────────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-steep-256.png: Artwork/UI/image asset: open-volar-s-steep-256.png. │
   │  └─────────────────────────────────────────────────────────────────────────────────┘
   ├──┌───────────────────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-steep-32.png: Artwork/UI/image asset: open-volar-s-steep-32.png. │
   │  └───────────────────────────────────────────────────────────────────────────────┘
   ├──┌───────────────────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-steep-40.png: Artwork/UI/image asset: open-volar-s-steep-40.png. │
   │  └───────────────────────────────────────────────────────────────────────────────┘
   ├──┌───────────────────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-steep-48.png: Artwork/UI/image asset: open-volar-s-steep-48.png. │
   │  └───────────────────────────────────────────────────────────────────────────────┘
   ├──┌─────────────────────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-steep-512.png: Artwork/UI/image asset: open-volar-s-steep-512.png. │
   │  └─────────────────────────────────────────────────────────────────────────────────┘
   ├──┌───────────────────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-steep-64.png: Artwork/UI/image asset: open-volar-s-steep-64.png. │
   │  └───────────────────────────────────────────────────────────────────────────────┘
   ├──┌───────────────────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-steep-96.png: Artwork/UI/image asset: open-volar-s-steep-96.png. │
   │  └───────────────────────────────────────────────────────────────────────────────┘
   ├──┌─────────────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-steep.ico: Artwork/UI/image asset: open-volar-s-steep.ico. │
   │  └─────────────────────────────────────────────────────────────────────────┘
   ├──┌─────────────────────────────────────────────────────────────────────────┐
   │  │ open-volar-s-steep.svg: Artwork/UI/image asset: open-volar-s-steep.svg. │
   │  └─────────────────────────────────────────────────────────────────────────┘
   ├──┌─────────────────────────────────────────────────────────────┐
   │  │ open-volar-s.ico: Artwork/UI/image asset: open-volar-s.ico. │
   │  └─────────────────────────────────────────────────────────────┘
   ├──┌─────────────────────────────────────────────────────────────┐
   │  │ open-volar-s.svg: Artwork/UI/image asset: open-volar-s.svg. │
   │  └─────────────────────────────────────────────────────────────┘
   ├──┌─────────────────────────────────────────────────┐
   │  │ README.md: Documentation: Open Volar S artwork. │
   │  └─────────────────────────────────────────────────┘
   └──┌────────────────────────────────────────────────────────────────────────────────────┐
      │ screenshots/: Documentation screenshots of the application and receiver interface. │
      └────────────────────────────────────────────────────────────────────────────────────┘
      │
      ├──┌─────────────────────────────────────────────────────────────────┐
      │  │ live-tv-window.png: Artwork/UI/image asset: live-tv-window.png. │
      │  └─────────────────────────────────────────────────────────────────┘
      ├──┌───────────────────────────────────────────────┐
      │  │ README.md: Documentation: Interface previews. │
      │  └───────────────────────────────────────────────┘
      └──┌─────────────────────────────────────────────────────────────────┐
         │ receiver-panel.png: Artwork/UI/image asset: receiver-panel.png. │
         └─────────────────────────────────────────────────────────────────┘
```
