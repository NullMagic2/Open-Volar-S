//! Summary: Provides the Rust command-line utility used to probe and validate the A865R driver core.

use a865r::{
    execution_probe_image, Device, EepromLayout, Error, FirmwareImage, Result, TsAnalyzer,
};
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

/// Prints command syntax and the current reverse-engineering boundary.
fn print_usage() {
    println!(
        "a865rctl - open-source AVerMedia A865R research utility\n\n\
Usage:\n\
  a865rctl wine-bridge <private-config-file> (Linux)\n\
  a865rctl wine-setup <Linux-helper-path> (Wine installer)\n\
  a865rctl wine-remove <Linux-helper-path> (Wine uninstaller)\n\
  a865rctl probe\n\
  a865rctl diagnose [firmware.bin]\n\
  a865rctl extract-firmware <AVer857BDA.sys> <firmware.bin>\n\
  a865rctl initialize <firmware.bin>\n\
  a865rctl probe-reference-processors <firmware.bin>\n\
  a865rctl build-open-fw-probe <output.fw>\n\
  a865rctl probe-open-fw\n\
  a865rctl reset-cold\n\
  a865rctl cycle-usb\n\
  a865rctl read-reg <address> <length>\n\
  a865rctl write-reg <address> <byte> [byte ...]\n\
  a865rctl i2c-read <7bit-address> <length>\n\
  a865rctl i2c-write <7bit-address> <byte> [byte ...]\n\
  a865rctl i2c-write-read <7bit-address> <read-length> [prefix-byte ...]\n\
  a865rctl dump-eeprom <output.bin> [af9035|it9135]\n\
  a865rctl capture <seconds> <output.ts> [pipe-id]\n\
  a865rctl analyze-ts <input.ts>\n\n\
  a865rctl scan-frequencies <first-khz> <last-khz> <step-khz>\n\
  a865rctl receiver-init\n\
  a865rctl receive <frequency-khz> <seconds> <new-output.ts>\n\
  a865rctl test-lifecycle <frequency-khz> <new-output-prefix>\n\
  a865rctl scan-uhf <first-channel> <last-channel>\n\n\
Numbers accept decimal or 0x-prefixed hexadecimal notation.\n\
receive accepts the tested open 0.1.4.0 or matching reference firmware; capture reads an already-active TS endpoint.\n\
probe-open-fw runs only independently authored code from RAM and requires a cold device."
    );
}

/// Parses an unsigned integer in decimal or 0x-prefixed hexadecimal notation.
fn parse_u64(text: &str) -> Result<u64> {
    let (radix, digits) =
        if let Some(rest) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            (16, rest)
        } else {
            (10, text)
        };
    u64::from_str_radix(digits, radix)
        .map_err(|_| Error::InvalidArgument(format!("invalid numeric value: {text}")))
}

/// Parses a numeric value and checks that it fits in u8.
fn parse_u8(text: &str, label: &str) -> Result<u8> {
    parse_u64(text)?
        .try_into()
        .map_err(|_| Error::InvalidArgument(format!("{label} is outside the u8 range")))
}

/// Parses a numeric value and checks that it fits in u32.
fn parse_u32(text: &str, label: &str) -> Result<u32> {
    parse_u64(text)?
        .try_into()
        .map_err(|_| Error::InvalidArgument(format!("{label} is outside the u32 range")))
}

/// Formats a firmware tuple as dotted decimal.
fn firmware_string(version: [u8; 4]) -> String {
    format!(
        "{}.{}.{}.{}",
        version[0], version[1], version[2], version[3]
    )
}

/// Prints all fields collected by the safe device probe.
fn print_probe(info: &a865r::DeviceInfo) {
    println!("Device:             AVerMedia A865R (USB\\VID_07CA&PID_B865)");
    println!("Transport:          {}", info.transport);
    println!("Chip type:          0x{:04X}", info.chip_type);
    println!("Chip version:       0x{:02X}", info.chip_version);
    println!("Pre-chip version:   0x{:02X}", info.prechip_version);
    println!(
        "Firmware:           {}",
        firmware_string(info.firmware_version)
    );
    println!(
        "Firmware running:   {}",
        if info.firmware_running {
            "yes"
        } else {
            "no / cold"
        }
    );
    println!("Command OUT pipe:   0x{:02X}", info.command_out_pipe);
    println!("Command IN pipe:    0x{:02X}", info.command_in_pipe);
    match info.transport_stream_pipe {
        Some(pipe) => println!("Likely TS pipe:     0x{pipe:02X}"),
        None => println!("Likely TS pipe:     not identified"),
    }
    println!("Bulk endpoints:");
    for pipe in &info.bulk_pipes {
        println!(
            "  0x{:02X} {} max-packet={}",
            pipe.id,
            if pipe.input { "IN " } else { "OUT" },
            pipe.maximum_packet_size
        );
    }

    if let Some(eeprom) = &info.eeprom {
        println!(
            "EEPROM layout:      {:?} @ 0x{:06X}",
            eeprom.layout, eeprom.base_address
        );
        println!("EEPROM TS mode:     {}", eeprom.summary.ts_mode);
        println!(
            "EEPROM dual mode:   {}",
            if eeprom.summary.dual_mode {
                "yes"
            } else {
                "no"
            }
        );
        println!("EEPROM tuner ID:    0x{:02X}", eeprom.summary.tuner_id);
        println!("EEPROM tuner IF:    {} kHz", eeprom.summary.tuner_if_khz);
        println!(
            "EEPROM IR mode/type: 0x{:02X}/0x{:02X}",
            eeprom.summary.ir_mode, eeprom.summary.ir_type
        );
    } else if let Some(note) = &info.eeprom_probe_note {
        println!("EEPROM:             unavailable ({note})");
    }
}

/// Connects to the tuner and prints the non-destructive identification result.
fn command_probe() -> Result<()> {
    let mut device = Device::new();
    let info = device.connect_and_probe()?;
    print_probe(&info);
    Ok(())
}

/// Reads and validates an extracted IT9175 scatter firmware file.
fn read_firmware(path: &Path) -> Result<FirmwareImage> {
    let bytes = std::fs::read(path)?;
    FirmwareImage::from_scatter_bytes(bytes)
}

/// Extracts the exact scatter image from the supported original AVerMedia kernel driver.
fn command_extract_firmware(driver_path: &Path, output_path: &Path) -> Result<()> {
    let driver = std::fs::read(driver_path)?;
    let firmware = FirmwareImage::extract_from_avermedia_driver(&driver)?;
    std::fs::write(output_path, firmware.bytes())?;
    println!(
        "Extracted {} bytes in {} validated scatter records to {}.",
        firmware.bytes().len(),
        firmware.segment_count(),
        output_path.display()
    );
    Ok(())
}

/// Writes the reproducibly generated, independently authored MCS-51 execution probe.
fn command_build_open_firmware_probe(output_path: &Path) -> Result<()> {
    let firmware = execution_probe_image()?;
    std::fs::write(output_path, firmware.bytes())?;
    println!(
        "Built {} open firmware bytes in {} scatter records at {}.",
        firmware.bytes().len(),
        firmware.segment_count(),
        output_path.display()
    );
    println!("Source-assembled code with reconstructed board data; no vendor executable payload.");
    Ok(())
}

/// Boots the reversible open MCS-51 probe and verifies that the OFDM core executes it.
fn command_probe_open_firmware() -> Result<()> {
    let mut device = Device::new();
    let info = device.connect_and_probe()?;
    print_probe(&info);
    println!();
    println!("Uploading independently authored execution probe...");
    let report = device.probe_open_firmware_execution()?;
    println!("Open firmware boot verified: both versions, forwarded register reads, scheduler {:?}. Uploaded {} bytes in {} records.",report.scheduler_ticks,report.bytes_uploaded,report.records_uploaded);
    println!(
        "Boot acknowledgement: {}.",
        if report.boot_acknowledged {
            "received"
        } else {
            "lost during reset; link recovered afterward"
        }
    );
    println!(
        "Link firmware query after probe: {}.",
        firmware_string(report.firmware_query)
    );
    println!(
        "OFDM firmware query after probe: {}.",
        firmware_string(report.ofdm_firmware_query)
    );
    println!("Physically unplug and reconnect the tuner before another firmware test.");
    Ok(())
}

/// Refuses the disproven software reset path before touching the hardware.
fn command_reset_cold() -> Result<()> {
    Err(Error::Unsupported(
        "no verified software cold reset exists for the A865R. Hardware testing proved that LINK command 0x23 powers down the open command processor and can remove the USB interface; physically unplug and reconnect the tuner to restore firmware 0.0.0.0. Use cycle-usb only for ordinary re-enumeration"
            .to_string(),
    ))
}

/// Cycles the parent hub and waits for the device to return in any probeable firmware state.
fn cycle_usb_and_wait() -> Result<(a865r::DeviceInfo, Duration)> {
    let mut device = Device::new();
    device.cycle_usb_port()?;
    drop(device);

    let started = Instant::now();
    let deadline = started + Duration::from_secs(15);
    let mut last_error = None;
    // Give Plug and Play time to remove the old devnode before accepting a newly opened path.
    thread::sleep(Duration::from_millis(500));
    while Instant::now() < deadline {
        let mut candidate = Device::new();
        match candidate.connect_and_probe() {
            Ok(info) => {
                return Ok((info, started.elapsed()));
            }
            Err(error) => last_error = Some(error.to_string()),
        }
        thread::sleep(Duration::from_millis(250));
    }

    Err(Error::Transport(format!(
        "the hub accepted the cycle request, but the A865R did not return in a probeable state within 15 seconds; last probe: {}",
        last_error.unwrap_or_else(|| "device interface not present".to_string())
    )))
}

/// Cycles the parent hub port and reports the resulting state without calling it a cold reset.
fn command_cycle_usb() -> Result<()> {
    println!("Cycling the A865R's parent USB-hub port...");
    let (info, elapsed) = cycle_usb_and_wait()?;
    println!();
    print_probe(&info);
    println!();
    if info.firmware_version == [0, 0, 0, 0] && !info.firmware_running {
        println!(
            "Cold state observed after USB cycle in {:.2?}: firmware is 0.0.0.0.",
            elapsed
        );
    } else {
        println!(
            "USB cycle completed in {:.2?}, but firmware remains {}. The hub re-enumerated the device without clearing RAM.",
            elapsed,
            firmware_string(info.firmware_version)
        );
    }
    Ok(())
}

/// Uploads firmware to a cold device and prints the verified post-boot state.
fn command_initialize(firmware_path: &Path) -> Result<()> {
    let firmware = read_firmware(firmware_path)?;
    let mut device = Device::new();
    let before = device.connect_and_probe()?;
    print_probe(&before);
    let report = device.load_firmware(&firmware)?;
    println!();
    if report.segments_uploaded == 0 {
        println!("Firmware was already running; no records were uploaded.");
    } else {
        println!(
            "Firmware booted: uploaded {} bytes in {} records; version {}.",
            report.bytes_uploaded,
            report.segments_uploaded,
            firmware_string(report.firmware_version)
        );
    }
    println!();
    print_probe(device.info()?);
    Ok(())
}

/// Uses the known-good reference image only as a control for processor-routing diagnostics.
///
/// This does not combine reference and open code. It verifies which harmless firmware-query and
/// memory-read operations the physical A865R actually supports before those operations are used as
/// pass/fail checkpoints for independently authored firmware.
fn command_probe_reference_processors(firmware_path: &Path) -> Result<()> {
    const REFERENCE_VERSION: [u8; 4] = [3, 0, 3, 0];

    let firmware = read_firmware(firmware_path)?;
    let mut device = Device::new();
    let before = device.connect_and_probe()?;
    print_probe(&before);
    if before.firmware_running && before.firmware_version != REFERENCE_VERSION {
        return Err(Error::Unsupported(format!(
            "processor routing must be measured against reference firmware 3.0.3.0; the device is already running {}. Physically unplug and reconnect it before retrying",
            firmware_string(before.firmware_version)
        )));
    }

    let load = device.load_firmware(&firmware)?;
    println!();
    println!(
        "Reference control: firmware {}, uploaded {} bytes in {} records.",
        firmware_string(load.firmware_version),
        load.bytes_uploaded,
        load.segments_uploaded
    );
    if load.firmware_version != REFERENCE_VERSION {
        return Err(Error::Protocol(format!(
            "reference control reported {}, expected 3.0.3.0",
            firmware_string(load.firmware_version)
        )));
    }

    println!("Processor firmware-query controls:");
    for (label, mailbox) in [
        ("LINK encoded mailbox", 0x00u8),
        ("OFDM encoded mailbox", 0x80u8),
        ("OFDM raw selector", 0x08u8),
    ] {
        match device.query_processor_firmware_version(mailbox, 800) {
            Ok(version) => println!("  {label:20} 0x{mailbox:02X}: {}", firmware_string(version)),
            Err(error) => println!("  {label:20} 0x{mailbox:02X}: ERROR: {error}"),
        }
    }

    // Recover from any preceding candidate timeout through a known LINK query, then independently
    // test the mailbox-0x80 register-forwarding path used by the open execution marker.
    let _ = device.query_processor_firmware_version(0x00, 800);
    match device.read_registers(0x804191, 4) {
        Ok(value) => println!(
            "OFDM XDATA 0x4191..0x4194 through address 0x804191: {:02X?}",
            value
        ),
        Err(error) => {
            println!("OFDM XDATA 0x4191..0x4194 through address 0x804191: ERROR: {error}")
        }
    }
    println!(
        "This was a reference-control measurement only; no proprietary bytes were included in the open image generator."
    );
    Ok(())
}

/// Runs the safe probe, optionally cold-boots firmware, and reports the next hardware milestone.
fn command_diagnose(firmware_path: Option<&Path>) -> Result<()> {
    let mut device = Device::new();
    let initial = device.connect_and_probe()?;
    print_probe(&initial);

    if !initial.firmware_running {
        if let Some(path) = firmware_path {
            let firmware = read_firmware(path)?;
            let report = device.load_firmware(&firmware)?;
            println!();
            println!(
                "Cold boot succeeded: {} bytes, {} records, firmware {}.",
                report.bytes_uploaded,
                report.segments_uploaded,
                firmware_string(report.firmware_version)
            );
            println!();
            println!("Post-boot probe:");
            print_probe(device.info()?);
        }
    }

    let info = device.info()?.clone();
    println!();
    println!("Diagnostic assessment:");
    if info.firmware_running {
        println!("  [OK] Firmware query returned a non-zero version.");
    } else {
        println!("  [!] Firmware is cold. Extract it from AVer857BDA.sys, then pass the resulting file to diagnose.");
    }
    if info.transport_stream_pipe.is_some() {
        println!("  [OK] A second bulk IN endpoint is available as a TS candidate.");
    } else {
        println!("  [!] No separate TS endpoint was identified automatically.");
    }
    if let Some(eeprom) = &info.eeprom {
        println!(
            "  [OK] Read-only EEPROM window produced a 256-byte snapshot (tuner ID 0x{:02X}).",
            eeprom.summary.tuner_id
        );
    } else {
        println!("  [i] EEPROM configuration could not be decoded automatically; the core probe still succeeded.");
    }
    println!(
        "  [i] No tuner frequency or demodulator tuning state was changed by this diagnostic."
    );
    Ok(())
}

/// Reads and prints a raw register range.
fn command_read_reg(address_text: &str, length_text: &str) -> Result<()> {
    let address = parse_u32(address_text, "register address")?;
    let length: usize = parse_u64(length_text)?
        .try_into()
        .map_err(|_| Error::InvalidArgument("read length is too large".to_string()))?;
    let mut device = Device::new();
    let _ = device.connect_and_probe()?;
    let bytes = device.read_registers(address, length)?;
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            print!(" ");
        }
        print!("{byte:02X}");
    }
    println!();
    Ok(())
}

/// Writes explicitly supplied bytes to a raw register range.
fn command_write_reg(address_text: &str, values: &[String]) -> Result<()> {
    let address = parse_u32(address_text, "register address")?;
    let mut data = Vec::with_capacity(values.len());
    for value in values {
        data.push(parse_u8(value, "register byte")?);
    }
    let mut device = Device::new();
    let _ = device.connect_and_probe()?;
    device.write_registers(address, &data)?;
    println!("Wrote {} byte(s) beginning at 0x{address:06X}.", data.len());
    Ok(())
}

/// Reads bytes from a 7-bit I2C peripheral through the firmware-managed bridge.
fn command_i2c_read(address_text: &str, length_text: &str) -> Result<()> {
    let address = parse_u8(address_text, "I2C address")?;
    let length: usize = parse_u64(length_text)?
        .try_into()
        .map_err(|_| Error::InvalidArgument("I2C read length is too large".to_string()))?;
    let mut device = Device::new();
    let _ = device.connect_and_probe()?;
    let bytes = device.i2c_read(address, length)?;
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            print!(" ");
        }
        print!("{byte:02X}");
    }
    println!();
    Ok(())
}

/// Writes explicitly supplied bytes to a 7-bit I2C peripheral.
fn command_i2c_write(address_text: &str, values: &[String]) -> Result<()> {
    let address = parse_u8(address_text, "I2C address")?;
    let mut data = Vec::with_capacity(values.len());
    for value in values {
        data.push(parse_u8(value, "I2C byte")?);
    }
    let mut device = Device::new();
    let _ = device.connect_and_probe()?;
    device.i2c_write(address, &data)?;
    println!(
        "Wrote {} byte(s) to I2C address 0x{address:02X}.",
        data.len()
    );
    Ok(())
}

/// Performs a repeated-start style I2C write-prefix followed by a read.
fn command_i2c_write_read(
    address_text: &str,
    read_length_text: &str,
    prefix_values: &[String],
) -> Result<()> {
    let address = parse_u8(address_text, "I2C address")?;
    let read_length: usize = parse_u64(read_length_text)?
        .try_into()
        .map_err(|_| Error::InvalidArgument("I2C read length is too large".to_string()))?;
    let mut prefix = Vec::with_capacity(prefix_values.len());
    for value in prefix_values {
        prefix.push(parse_u8(value, "I2C prefix byte")?);
    }
    let mut device = Device::new();
    let _ = device.connect_and_probe()?;
    let bytes = device.i2c_write_read(address, &prefix, read_length)?;
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            print!(" ");
        }
        print!("{byte:02X}");
    }
    println!();
    Ok(())
}

/// Dumps a complete read-only EEPROM window to disk using auto-detection or an explicit layout.
fn command_dump_eeprom(path: &Path, layout_text: Option<&str>) -> Result<()> {
    let explicit_layout = match layout_text {
        Some("af9035") => Some(EepromLayout::Af9035),
        Some("it9135") => Some(EepromLayout::It9135),
        Some(other) => {
            return Err(Error::InvalidArgument(format!(
                "unknown EEPROM layout: {other}"
            )))
        }
        None => None,
    };

    let mut device = Device::new();
    let info = device.connect_and_probe()?;
    let eeprom = if let Some(layout) = explicit_layout {
        device.read_eeprom_with_layout(layout)?
    } else {
        info.eeprom.ok_or_else(|| {
            Error::Unsupported(
                "automatic EEPROM decoding failed; retry with an explicit af9035 or it9135 layout"
                    .to_string(),
            )
        })?
    };
    std::fs::write(path, &eeprom.bytes)?;
    println!(
        "Wrote {} EEPROM bytes from 0x{:06X} to {}.",
        eeprom.bytes.len(),
        eeprom.base_address,
        path.display()
    );
    Ok(())
}

/// Captures bytes from the selected stream endpoint and prints MPEG-TS diagnostics.
fn command_capture(seconds_text: &str, path: &Path, pipe_text: Option<&str>) -> Result<()> {
    let seconds = parse_u32(seconds_text, "capture duration")?;
    let pipe = pipe_text
        .map(|text| parse_u8(text, "pipe ID"))
        .transpose()?;
    let mut device = Device::new();
    let info = device.connect_and_probe()?;
    let selected = pipe.or(info.transport_stream_pipe);
    println!(
        "Capturing from {} for {seconds} second(s)...",
        selected
            .map(|value| format!("0x{value:02X}"))
            .unwrap_or_else(|| "unknown pipe".to_string())
    );
    let report = device.capture_transport_stream(path, seconds, pipe)?;
    print_ts_stats(&report.stats);
    println!("Bytes written:       {}", report.bytes_written);
    println!("Elapsed:             {:.2?}", report.elapsed);
    if report.bytes_written == 0 {
        println!("No stream bytes arrived. A cold or untuned device is expected to behave this way at this stage.");
    }
    Ok(())
}

/// Reads an existing `.ts` file and runs the same streaming analyzer used during USB capture.
fn command_analyze_ts(path: &Path) -> Result<()> {
    let mut input = File::open(path)?;
    let mut analyzer = TsAnalyzer::new();
    let mut buffer = vec![0u8; 188 * 512];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        analyzer.push(&buffer[..count]);
    }
    let stats = analyzer.finish();
    print_ts_stats(&stats);
    Ok(())
}

/// Prints MPEG-TS synchronization, continuity, PID, PAT, and PMT statistics.
fn print_ts_stats(stats: &a865r::TsStats) {
    println!("TS bytes seen:       {}", stats.bytes_seen);
    println!("TS packets:          {}", stats.packets);
    println!("Sync losses:         {}", stats.sync_losses);
    println!("Continuity errors:   {}", stats.continuity_errors);
    println!("Transport errors:    {}", stats.transport_error_packets);
    if !stats.programs.is_empty() {
        println!("Programs:");
        for program in stats.programs.values() {
            println!(
                "  program={} PMT=0x{:04X} PCR={}",
                program.program_number,
                program.pmt_pid,
                program
                    .pcr_pid
                    .map(|pid| format!("0x{pid:04X}"))
                    .unwrap_or_else(|| "unknown".to_string())
            );
        }
    }
    if !stats.streams.is_empty() {
        println!("Elementary streams:");
        for stream in &stats.streams {
            println!(
                "  program={} PID=0x{:04X} type=0x{:02X}",
                stream.program_number, stream.pid, stream.stream_type
            );
        }
    }
    if !stats.pid_packets.is_empty() {
        let mut top: Vec<(u16, u64)> = stats
            .pid_packets
            .iter()
            .map(|(pid, count)| (*pid, *count))
            .collect();
        top.sort_by_key(|entry| std::cmp::Reverse(entry.1));
        println!("Top PIDs:");
        for (pid, count) in top.into_iter().take(12) {
            println!("  0x{pid:04X}: {count} packets");
        }
    }
}

/// Dispatches command-line arguments to the requested operation.
fn run() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    #[cfg(windows)]
    if matches!(args.get(1).map(String::as_str),Some("wine-setup"|"wine-remove")) {
        let path=args.get(2).ok_or_else(||Error::InvalidArgument("Missing Linux helper path".into()))?;
        return a865r::transport::wine_bridge::install_helper(Path::new(path),args[1]=="wine-remove");
    }
    #[cfg(target_os="linux")]
    if args.get(1).map(String::as_str)==Some("wine-bridge") {
        let path=args.get(2).ok_or_else(||Error::InvalidArgument("Usage: a865rctl wine-bridge CONFIG_FILE".into()))?;
        return a865r::transport::wine_bridge::serve(Path::new(path));
    }
    match args.as_slice() {
        [_, command, frequency, output]
            if command == "test-lifecycle" || command == "test-lifecycle-open" =>
        {
            let frequency = parse_u32(frequency, "frequency")?;
            let mut device = Device::new();
            print_probe(&device.connect_and_probe()?);
            let mut receiver = device.receiver()?;
            if command == "test-lifecycle-open" {
                receiver.initialize_open_firmware_experiment()?;
            } else {
                receiver.initialize()?;
            }
            let result = (|| -> Result<()> {
                for cycle in 0..3 {
                    let tune = receiver.tune(frequency, 6000)?;
                    println!(
                        "Cycle {cycle} tune: {tune:?}; quality: {:?}",
                        receiver.signal_quality()?
                    );
                    if !tune.mpeg_locked {
                        return Err(Error::Protocol("No MPEG lock in lifecycle test".into()));
                    }
                    let path = format!("{output}.cycle-{cycle}.ts");
                    let capture = receiver.record(
                        Path::new(&path),
                        3,
                        &std::sync::atomic::AtomicBool::new(false),
                    )?;
                    println!("Cycle {cycle}: {} bytes", capture.bytes_written);
                    print_ts_stats(&capture.stats);
                    if cycle < 2 {
                        receiver.suspend()?;
                        println!("Cycle {cycle}: suspend acknowledged");
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        receiver.resume()?;
                        println!(
                            "Cycle {cycle}: resumed and recalibrated {:?}",
                            receiver.calibration()
                        );
                    }
                }
                Ok(())
            })();
            let stop = receiver.stop();
            result.and(stop)
        }
        [_, command] if command == "receiver-init" => {
            let mut device = Device::new();
            print_probe(&device.connect_and_probe()?);
            let mut receiver = device.receiver()?;
            receiver.initialize()?;
            println!("Receiver initialized: {:?}", receiver.calibration());
            receiver.stop()
        }
        [_, command, frequency, seconds, output]
            if command == "receive" || command == "receive-open" =>
        {
            let frequency = parse_u32(frequency, "frequency")?;
            let seconds = parse_u32(seconds, "seconds")?;
            let mut device = Device::new();
            print_probe(&device.connect_and_probe()?);
            let mut receiver = device.receiver()?;
            if command == "receive-open" {
                receiver.initialize_open_firmware_experiment()?
            } else {
                receiver.initialize()?
            };
            println!("Calibration: {:?}", receiver.calibration());
            let result = (|| -> Result<()> {
                let tune = receiver.tune(frequency, 6000)?;
                println!("Tune: {tune:?}");
                println!(
                    "Relative firmware quality: {:?}",
                    receiver.signal_quality()?
                );
                if !tune.mpeg_locked {
                    return Err(Error::Protocol(
                        "No MPEG lock at the requested frequency".into(),
                    ));
                }
                let capture = receiver.record(
                    Path::new(output),
                    seconds,
                    &std::sync::atomic::AtomicBool::new(false),
                )?;
                print_ts_stats(&capture.stats);
                println!("Quality after capture: {:?}", receiver.signal_quality()?);
                Ok(())
            })();
            let stop = receiver.stop();
            result.and(stop)
        }
        [_, command, first, last, step] if command == "scan-frequencies" => {
            let frequencies = a865r::channel_plan::custom_scan(
                parse_u32(first, "first kHz")?,
                parse_u32(last, "last kHz")?,
                parse_u32(step, "step kHz")?,
            )?;
            let mut device = Device::new();
            print_probe(&device.connect_and_probe()?);
            let mut receiver = device.receiver()?;
            receiver.initialize()?;
            let result = (|| -> Result<()> {
                for frequency in frequencies {
                    println!("{:?}", receiver.tune(frequency, 6000)?);
                }
                Ok(())
            })();
            let stop = receiver.stop();
            result.and(stop)
        }
        [_, command, first, last] if command == "scan-uhf" => {
            let first = parse_u8(first, "first channel")?;
            let last = parse_u8(last, "last channel")?;
            a865r::receiver::brazil_uhf_frequency(first)?;
            a865r::receiver::brazil_uhf_frequency(last)?;
            if first > last {
                return Err(Error::InvalidArgument(
                    "First channel exceeds last channel".into(),
                ));
            }
            let mut device = Device::new();
            print_probe(&device.connect_and_probe()?);
            let mut receiver = device.receiver()?;
            receiver.initialize()?;
            println!("Calibration: {:?}", receiver.calibration());
            let result = (|| -> Result<()> {
                for channel in first..=last {
                    let tune =
                        receiver.tune(a865r::receiver::brazil_uhf_frequency(channel)?, 2500)?;
                    println!("RF {channel}: {tune:?}");
                }
                Ok(())
            })();
            let stop = receiver.stop();
            result.and(stop)
        }
        [_, command, reference, component] if command == "probe-components" => {
            let open_link = match component.as_str() {
                "open-link" => true,
                "open-ofdm" => false,
                _ => {
                    return Err(Error::InvalidArgument(
                        "Choose open-link or open-ofdm".into(),
                    ))
                }
            };
            let reference = read_firmware(Path::new(reference))?;
            let mut device = Device::new();
            print_probe(&device.connect_and_probe()?);
            println!("Component isolation only: one original proprietary core + one open core; not a full open firmware.");
            let report = device.probe_firmware_components(&reference, open_link)?;
            println!("Component test: {report:?}");
            Ok(())
        }
        [_, command] if command == "check-open-fw" => {
            let mut device = Device::new();
            print_probe(&device.connect_and_probe()?);
            println!(
                "Open health (LINK, OFDM, scheduler before/after): {:?}",
                device.check_open_firmware_health()?
            );
            Ok(())
        }
        [_, command] if command == "capabilities" => {
            println!("{:#?}", a865r::api::capabilities(None));
            Ok(())
        }
        [_, command] if command == "probe" => command_probe(),
        [_, command] if command == "diagnose" => command_diagnose(None),
        [_, command, firmware] if command == "diagnose" => {
            command_diagnose(Some(&PathBuf::from(firmware)))
        }
        [_, command, driver, output] if command == "extract-firmware" => {
            command_extract_firmware(&PathBuf::from(driver), &PathBuf::from(output))
        }
        [_, command, firmware] if command == "initialize" => {
            command_initialize(&PathBuf::from(firmware))
        }
        [_, command, firmware] if command == "probe-reference-processors" => {
            command_probe_reference_processors(&PathBuf::from(firmware))
        }
        [_, command, output] if command == "build-open-fw-probe" => {
            command_build_open_firmware_probe(&PathBuf::from(output))
        }
        [_, command] if command == "probe-open-fw" => command_probe_open_firmware(),
        [_, command] if command == "reset-cold" => command_reset_cold(),
        [_, command] if command == "cycle-usb" => command_cycle_usb(),
        [_, command, address, length] if command == "read-reg" => command_read_reg(address, length),
        [_, command, address, values @ ..] if command == "write-reg" && !values.is_empty() => {
            command_write_reg(address, values)
        }
        [_, command, address, length] if command == "i2c-read" => command_i2c_read(address, length),
        [_, command, address, values @ ..] if command == "i2c-write" && !values.is_empty() => {
            command_i2c_write(address, values)
        }
        [_, command, address, read_length, prefix @ ..] if command == "i2c-write-read" => {
            command_i2c_write_read(address, read_length, prefix)
        }
        [_, command, output] if command == "dump-eeprom" => {
            command_dump_eeprom(&PathBuf::from(output), None)
        }
        [_, command, output, layout] if command == "dump-eeprom" => {
            command_dump_eeprom(&PathBuf::from(output), Some(layout))
        }
        [_, command, seconds, output] if command == "capture" => {
            command_capture(seconds, &PathBuf::from(output), None)
        }
        [_, command, seconds, output, pipe] if command == "capture" => {
            command_capture(seconds, &PathBuf::from(output), Some(pipe))
        }
        [_, command, input] if command == "analyze-ts" => command_analyze_ts(&PathBuf::from(input)),
        _ => {
            print_usage();
            Err(Error::InvalidArgument("invalid command line".to_string()))
        }
    }
}

/// Reports errors cleanly to the shell without exposing Rust panics.
fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(2);
    }
}
