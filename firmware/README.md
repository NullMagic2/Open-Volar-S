# Open firmware 0.1.4.0

Both LINK and OFDM cores, generated from this workspace's Rust 8051 assembler source. The image is also built into the driver. It is loaded into volatile RAM on cold startup, not flashed to EEPROM. Source: crates/liba865r/src/open_firmware.rs and open_firmware/*.rs. Build: `cargo run -p a865rctl --release -- build-open-fw-probe firmware/a865r-open-0.1.4.0.fw`.

Hardware reception and limitations: docs/RF_INTEGRATION_0.1.4.md. Numeric startup/calibration tables were reconstructed from the reference; this is not a clean-room implementation. Proprietary executable firmware is not distributed.

