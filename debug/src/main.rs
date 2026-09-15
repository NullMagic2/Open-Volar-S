#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use a865r_media::playback;
use a865r_media::native_player;
mod television;

use a865r::api::{ColorProfile, DeinterlaceMode, PlaybackSettings, RenderBackend, Resolution};
use a865r::{BulkPipe, Device, FirmwareImage, Result as UsbResult, Transport};
use eframe::egui;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    sync::{mpsc, Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const REFERENCE_SHA: &str = "4b066157d0eb1a088e55daaf339418fc6596b5f76c4583d06d5eddb3a272e921";

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

fn file_metadata(path: &Path) -> std::io::Result<Value> {
    let mut input = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    let mut bytes = 0u64;
    loop {
        let n = input.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        bytes += n as u64;
    }
    Ok(
        json!({"name": path.file_name().unwrap_or_default().to_string_lossy(),
              "size": bytes, "sha256": format!("{:x}", hasher.finalize())}),
    )
}

fn firmware_report(bytes: Vec<u8>) -> UsbResult<Value> {
    let image = FirmwareImage::from_scatter_bytes(bytes)?;
    let sha = format!("{:x}", Sha256::digest(image.bytes()));
    let regions: Vec<Value> = image
        .regions()
        .map(|r| {
            let reset = if r.address == 0x4100 && r.data.len() >= 3 && r.data[0] == 2 {
                Some(format!(
                    "0x{:04x}",
                    u16::from_be_bytes([r.data[1], r.data[2]])
                ))
            } else {
                None
            };
            json!({"core": format!("{:?}", r.core), "address": format!("0x{:04x}", r.address),
               "length": r.data.len(), "reset_jump": reset})
        })
        .collect();
    Ok(json!({"size": image.bytes().len(), "sha256": sha,
              "matches_supplied_reference_fingerprint": sha == REFERENCE_SHA,
              "scatter_records": image.segment_count(), "regions": regions,
              "hardware_tested": false, "payload_bytes_exported": false}))
}

/// Records only the transactions issued by the GUI's read-oriented probe.
struct LoggedTransport {
    inner: Box<dyn Transport>,
    events: Arc<Mutex<Vec<Value>>>,
}

impl LoggedTransport {
    fn record(&self, value: Value) {
        if let Ok(mut events) = self.events.lock() {
            events.push(value);
        }
    }
}

impl Transport for LoggedTransport {
    fn open(&mut self, vid: u16, pid: u16) -> UsbResult<()> {
        let result = self.inner.open(vid, pid);
        self.record(json!({"time_ms": now_ms(), "operation": "open", "vid": vid,
                           "pid": pid, "error": result.as_ref().err().map(ToString::to_string)}));
        result
    }
    fn cycle_port(&mut self) -> UsbResult<()> {
        Err(a865r::Error::Unsupported(
            "port cycling is excluded from the debug GUI".into(),
        ))
    }
    fn bulk_pipes(&self) -> &[BulkPipe] {
        self.inner.bulk_pipes()
    }
    fn set_command_pipes(&mut self, out: u8, input: u8) -> UsbResult<()> {
        let result = self.inner.set_command_pipes(out, input);
        self.record(
            json!({"time_ms": now_ms(), "operation": "select_pipes", "out": out,
                           "in": input, "error": result.as_ref().err().map(ToString::to_string)}),
        );
        result
    }
    fn exchange(&mut self, request: &[u8], expected: usize, timeout: u32) -> UsbResult<Vec<u8>> {
        let start = now_ms();
        let result = self.inner.exchange(request, expected, timeout);
        self.record(
            json!({"operation": "exchange", "time_ms": start, "elapsed_ms": now_ms()-start,
            "request_hex": hex(request), "expected_response_bytes": expected, "timeout_ms": timeout,
            "response_hex": result.as_ref().ok().map(|r| hex(r)),
            "error": result.as_ref().err().map(ToString::to_string)}),
        );
        result
    }
    fn read_bulk(&mut self, _pipe: u8, _size: usize, _timeout: u32) -> UsbResult<Vec<u8>> {
        Err(a865r::Error::Unsupported(
            "stream capture is excluded from the debug GUI".into(),
        ))
    }
    fn description(&self) -> String {
        self.inner.description()
    }
}

fn run_hidden(command: &mut Command) -> std::io::Result<std::process::Output> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    command.output()
}

fn command_result(command: &mut Command) -> Value {
    match run_hidden(command) {
        Ok(out) => json!({"success": out.status.success(), "exit_code": out.status.code(),
                         "stdout": String::from_utf8_lossy(&out.stdout),
                         "stderr": String::from_utf8_lossy(&out.stderr)}),
        Err(e) => json!({"success": false, "error": e.to_string()}),
    }
}

fn collect_probe(include_power: bool) -> Value {
    let events = Arc::new(Mutex::new(Vec::new()));
    let transport = LoggedTransport {
        inner: a865r::transport::default_transport(),
        events: events.clone(),
    };
    let mut device = Device::with_transport(Box::new(transport));
    let probe = match device.connect_and_probe() {
        Ok(info) => {
            let ofdm = if info.firmware_running && info.chip_type == 0x9175 {
                match device.query_processor_firmware_version(0x80, 1000) {
                    Ok(version) => json!({"version":version}),
                    Err(e) => json!({"error":e.to_string()}),
                }
            } else {
                json!({"skipped":"Requires running IT9175 firmware"})
            };
            let power = if include_power
                && info.firmware_running
                && info.chip_type == 0x9175
                && ofdm.get("version").is_some()
            {
                let mut values = Vec::new();
                for &(address, name) in POWER_REGISTERS {
                    let started = now_ms();
                    match device.read_registers(address, 1) {
                        Ok(bytes) => values.push(json!({"address":format!("0x{address:06x}"),"name":name,"time_ms":started,"value":bytes[0],"hex":hex(&bytes)})),
                        Err(e) => {
                            values.push(json!({"address":format!("0x{address:06x}"),"name":name,"time_ms":started,"error":e.to_string()}));
                            break;
                        }
                    }
                }
                json!({"registers":values,"complete":values.len()==POWER_REGISTERS.len() && values.last().is_some_and(|v|v.get("error").is_none()),
                    "interpretation":"Sequential, non-atomic raw samples. Register names are inferred from the fingerprinted vendor driver; values do not prove suspend or reception."})
            } else {
                json!({"skipped":"Select Power snapshot with both IT9175 cores responding"})
            };
            json!({"success": true, "chip_type": format!("0x{:04x}", info.chip_type),
            "ofdm_firmware":ofdm,"power_snapshot":power,
            "chip_version": info.chip_version, "prechip_version": info.prechip_version,
            "link_firmware": info.firmware_version, "firmware_running": info.firmware_running,
            "command_out": info.command_out_pipe, "command_in": info.command_in_pipe,
            "candidate_ts_pipe": info.transport_stream_pipe,
            "endpoints": info.bulk_pipes.iter().map(|p| json!({"address":p.id,"input":p.input,
                "max_packet_size":p.maximum_packet_size})).collect::<Vec<_>>(),
            "eeprom": info.eeprom.map(|e| json!({"layout":format!("{:?}",e.layout),
                "base":e.base_address,"bytes_hex":hex(&e.bytes),"tuner_id":e.summary.tuner_id,
                "if_khz":e.summary.tuner_if_khz,"ts_mode":e.summary.ts_mode})),
            "eeprom_note": info.eeprom_probe_note})
        }
        Err(error) => json!({"success": false, "error": error.to_string(),
            "next_step": "Check that the tuner is connected. Vendor BDA ownership does not expose this project's WinUSB interface; capture vendor-driver traffic in Wireshark separately."}),
    };
    let windows = if cfg!(windows) {
        let system = std::env::var_os("SystemRoot")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("C:\\Windows"));
        command_result(Command::new(system.join("System32/WindowsPowerShell/v1.0/powershell.exe"))
            .args(["-NoProfile", "-NonInteractive", "-Command",
                "[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new(); Get-PnpDevice -PresentOnly -ErrorAction Stop | Where-Object { $_.InstanceId -like 'USB\\VID_07CA&PID_B865*' } | Select-Object Status,Class,FriendlyName,InstanceId,Problem | ConvertTo-Json -Depth 3"]))
    } else {
        json!({"success": false, "error": "Windows-only hardware backend"})
    };
    json!({"probe": probe, "windows_device_inventory": windows, "usb_transactions": events.lock().unwrap().clone(),
           "scope": "Read-oriented identification and optional power-register samples; no firmware upload, tuning, reset or driver installation."})
}

// Evidence: AVer857BDA.sys SHA-256 80dfe4cd... at VA 22f28 and 2388c.
// Read-only checkpoints; no claim that the board-specific power contract is complete.
const POWER_REGISTERS: &[(u32, &str)] = &[
    (0x00d8bf, "LINK D8BF bit 0: set in demod power-off"),
    (0x00e00c, "LINK E00C: demod power branch"),
    (0x00d8c7, "LINK D8C7 bit 0: conditional single-chip branch"),
    (0x00d8bb, "LINK D8BB bit 0: conditional board/NIM branch"),
    (0x00d91b, "LINK D91B: board routing/power branch"),
    (0x00d91c, "LINK D91C: board routing/power branch"),
    (0x80004c, "OFDM 004C: shutdown request/ack checkpoint"),
    (0x80fb24, "OFDM FB24 bit 3: set after shutdown polling"),
    (0x80fbb9, "OFDM FBB9 bit 5: demod power branch"),
    (0x80f84f, "OFDM F84F: pulsed during demod power changes"),
    (0x80fba8, "OFDM FBA8: cleared during power changes"),
    (0x80ec40, "OFDM EC40: tuner enable branch"),
    (0x80ec3f, "OFDM EC3F: tuner power-off tail"),
];

fn exports_root(root: &Path) -> PathBuf {
    // Installer marker separates immutable shared program files from each user's data.
    if root.join("installed-mode.txt").is_file() {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            return PathBuf::from(local).join("A865R/Debug/exports");
        }
        return std::env::temp_dir().join("A865R/Debug/exports");
    }
    root.join("debug/exports")
}

fn project_root() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        for dir in exe.ancestors().skip(1).take(5) {
            if dir.join("tools/usb_trace.py").is_file() {
                return dir.to_path_buf();
            }
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn default_python() -> String {
    if let Some(p) = std::env::var_os("A865R_PYTHON") {
        return p.to_string_lossy().into_owned();
    }
    if let Some(home) = std::env::var_os("USERPROFILE") {
        let p = PathBuf::from(home)
            .join(".cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe");
        if p.is_file() {
            return p.to_string_lossy().into_owned();
        }
    }
    "python".into()
}

fn write_json(path: &Path, value: &Value) -> std::io::Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)
}

struct JobResult {
    title: String,
    value: Value,
    error: Option<String>,
}

struct DebugApp {
    root: PathBuf,
    session: PathBuf,
    log: String,
    notes: String,
    firmware: String,
    capture: String,
    python: String,
    bus: String,
    device: String,
    section: String,
    interface: String,
    busy: bool,
    job_number: u32,
    jobs: Vec<Value>,
    receive: Option<mpsc::Receiver<JobResult>>,
    status: String,
    screenshot: Option<PathBuf>,
    frames: u32,
    playback: PlaybackSettings,
    ffmpeg: String,
    frequency: u32,
    custom_plan: bool,
    scan_first: u32,
    scan_last: u32,
    scan_step: u32,
    record_seconds: u32,
    control: Option<playback::Control>,
    stations: Vec<u32>,
    remote_label: String,
}

impl DebugApp {
    fn new() -> Self {
        let root = project_root();
        let session =
            exports_root(&root).join(format!("session-{}-{}", now_ms(), std::process::id()));
        Self {
            root,
            session,
            log: "Ready. Choose a diagnostic action. Results and failures appear here.\n".into(),
            notes: String::new(),
            firmware: String::new(),
            capture: String::new(),
            python: default_python(),
            bus: String::new(),
            device: String::new(),
            section: String::new(),
            interface: String::new(),
            busy: false,
            job_number: 0,
            jobs: Vec::new(),
            receive: None,
            status: "Ready".into(),
            screenshot: None,
            frames: 0,
            playback: PlaybackSettings::default(),
            ffmpeg: playback::Options::default()
                .ffmpeg
                .to_string_lossy()
                .into_owned(),
            frequency: 521143,
            custom_plan: false,
            scan_first: 473143,
            scan_last: 695143,
            scan_step: 6000,
            record_seconds: 30,
            control: None,
            stations: Vec::new(),
            remote_label: "Play / pause".into(),
        }
    }

    fn player_options(&self) -> playback::Options {
        playback::Options {
            ffmpeg: PathBuf::from(&self.ffmpeg),
            resolution: self.playback.effective_resolution(),
            gpu: self.playback.backend != RenderBackend::Cpu,
            threads: self.playback.cpu_threads,
            program_id: None,
            deinterlacing: self.playback.deinterlacing,
            color_profile: self.playback.color_profile.clone(),
        }
    }
    fn start_tv(&mut self, action: television::Action) {
        let firmware = PathBuf::from(&self.firmware);
        let folder = self.session.join(format!("tv-{}", now_ms()));
        let options = self.player_options();
        let control = playback::Control::default();
        self.control = Some(control.clone());
        self.start("TV reception", move || {
            television::run(action, firmware, folder, options, control)
        });
    }
    fn television_ui(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Infrared remote through the tuner", |ui| {
            ui.label("Learn the code for one button. Stop TV first, then point the remote at the tuner.");
            ui.horizontal(|ui| {
                ui.label("Button name"); ui.text_edit_singleline(&mut self.remote_label);
                if ui.button("Listen for 30 seconds").clicked() {
                    let label=self.remote_label.clone();
                    let folder=self.session.join(format!("remote-{}",now_ms()));
                    let control=playback::Control::default(); self.control=Some(control.clone());
                    self.start("Infrared button learning",move||a865r_media::remote::learn(30,&label,&folder,&control));
                }
            });
            ui.small("Experimental: exports received codes and errors. Universal handset compatibility and button actions are not yet verified.");
        });
        ui.strong("Watch TV");
        ui.small("Original A865R · ISDB-T, 6 MHz · UHF 470–698 MHz · open firmware supported");
        ui.horizontal(|ui| {
            ui.label("Monitor colors");
            ui.selectable_value(
                &mut self.playback.color_profile,
                ColorProfile::Monitor,
                "Use monitor profile",
            );
            ui.selectable_value(
                &mut self.playback.color_profile,
                ColorProfile::Disabled,
                "Off",
            );
            if ui.button("Choose ICC/ICM…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Color profile", &["icc", "icm"])
                    .pick_file()
                {
                    match a865r_media::color::validate_profile(&path) {
                        Ok(_) => self.playback.color_profile = ColorProfile::File(path),
                        Err(error) => self.status = error.to_string(),
                    }
                }
            }
        });
        if let ColorProfile::File(path) = &self.playback.color_profile {
            ui.small(path.display().to_string());
        }
        ui.horizontal(|ui| {
            if ui.button("Device capabilities & versions").clicked() {
                self.start("Device capabilities", television::query_device);
            }
            if let Some(control) = &self.control {
                if ui.button("Export current playback status").clicked() {
                    let status = control.snapshot();
                    self.start("Current playback status", move || status);
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Frequency (kHz)");
            ui.add(egui::DragValue::new(&mut self.frequency).range(470000..=697999));
            if !self.stations.is_empty() {
                egui::ComboBox::from_id_salt("stations")
                    .selected_text("Found frequencies")
                    .show_ui(ui, |ui| {
                        for &f in &self.stations {
                            ui.selectable_value(
                                &mut self.frequency,
                                f,
                                format!("{:.3} MHz", f as f64 / 1000.0),
                            );
                        }
                    });
            }
            if ui.button("Watch").clicked() {
                self.start_tv(television::Action::Watch {
                    frequency: self.frequency,
                });
            }
            ui.label("Seconds");
            ui.add(egui::DragValue::new(&mut self.record_seconds).range(1..=3600));
            if ui.button("Record TV").clicked() {
                self.start_tv(television::Action::Record {
                    frequency: self.frequency,
                    seconds: self.record_seconds,
                });
            }
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.playback.upscaling_enabled, "Upscaling");
            egui::ComboBox::from_id_salt("resolution")
                .selected_text(self.playback.resolution.label())
                .show_ui(ui, |ui| {
                    for resolution in Resolution::ALL {
                        ui.selectable_value(
                            &mut self.playback.resolution,
                            resolution,
                            resolution.label(),
                        );
                    }
                });
            egui::ComboBox::from_id_salt("renderer")
                .selected_text(format!("{:?}", self.playback.backend))
                .show_ui(ui, |ui| {
                    for (backend, label) in [
                        (RenderBackend::Auto, "Auto · Vulkan with CPU fallback"),
                        (RenderBackend::Cpu, "CPU · multithreaded"),
                        (RenderBackend::Gpu, "Vulkan GPU · CPU fallback"),
                    ] {
                        ui.selectable_value(&mut self.playback.backend, backend, label);
                    }
                });
            ui.label("CPU workers");
            ui.add(egui::DragValue::new(&mut self.playback.cpu_threads).range(1..=64));
            if ui.button("Play recording…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("TV recording", &["ts", "mkv", "mp4"])
                    .pick_file()
                {
                    let options = self.player_options();
                    let folder = self.session.join(format!("playback-{}", now_ms()));
                    let control = playback::Control::default();
                    self.control = Some(control.clone());
                    self.start("Play recording", move || {
                        playback::play_file(options, path, folder, control)
                    });
                }
            }
        });
        ui.collapsing("Scan channels and playback settings",|ui| {
            egui::ComboBox::from_id_salt("deinterlace").selected_text(self.playback.deinterlacing.label()).show_ui(ui,|ui| {for mode in [DeinterlaceMode::DoubleRate,DeinterlaceMode::SingleRate,DeinterlaceMode::Off] {ui.selectable_value(&mut self.playback.deinterlacing,mode,mode.label());}});
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.custom_plan,false,"Brazil UHF");ui.radio_value(&mut self.custom_plan,true,"Custom ISDB-T frequencies");
                if ui.button("Scan").clicked() {
                    let frequencies=if self.custom_plan {a865r::channel_plan::custom_scan(self.scan_first,self.scan_last,self.scan_step)} else {Ok(a865r::channel_plan::brazil_uhf())};
                    match frequencies {Ok(f)=>self.start_tv(television::Action::Scan(f)),Err(e)=>self.status=e.to_string()}
                }
            });
            if self.custom_plan {ui.horizontal(|ui| {for (label,value) in [("First kHz",&mut self.scan_first),("Last kHz",&mut self.scan_last),("Step kHz",&mut self.scan_step)] {ui.label(label);ui.add(egui::DragValue::new(value));}});}
            file_row(ui,"FFmpeg executable",&mut self.ffmpeg,&["exe"]);
            ui.small("The native Live TV diagnostic player applies the monitor profile during presentation. Changes apply to the next playback session.");
            if ui.button("Export capabilities & playback settings").clicked() {
                let report=json!({"capabilities":television::capabilities_json(),"playback":{"resolution":self.playback.resolution.dimensions(),"upscaling_enabled":self.playback.upscaling_enabled,"effective_resolution":self.playback.effective_resolution().dimensions(),"backend":format!("{:?}",self.playback.backend),"cpu_threads":self.playback.cpu_threads,"deinterlacing":format!("{:?}",self.playback.deinterlacing),"color_profile":a865r_media::color::report_json(&a865r::api::ColorProfileStatus::new(self.playback.color_profile.clone()))}});
                self.start("Capabilities API",move || report);
            }
        });
        ui.separator();
    }

    fn start(&mut self, title: &str, job: impl FnOnce() -> Value + Send + 'static) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.status = format!("{title}...");
        let title = title.to_string();
        let (send, receive) = mpsc::channel();
        self.receive = Some(receive);
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job));
            let message = match result {
                Ok(value) => JobResult {
                    title,
                    value,
                    error: None,
                },
                Err(_) => JobResult {
                    title,
                    value: json!({"success":false}),
                    error: Some("Diagnostic worker failed unexpectedly".into()),
                },
            };
            let _ = send.send(message);
        });
    }

    fn save_export(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.session)?;
        write_json(
            &self.session.join("manifest.json"),
            &json!({"schema":1,"app_version":env!("CARGO_PKG_VERSION"),
            "exported_unix_ms":now_ms(),"target":"07ca:b865 / A865R / IT9175", "os":std::env::consts::OS,
            "architecture":std::env::consts::ARCH, "notes":self.notes,"jobs":self.jobs,
            "limits":"Original A865R reference and open 0.1.4.0 reception verified locally on RF22. Other channels and USB/system sleep need separate validation. TV actions may save requested recordings; proprietary firmware is not copied."}),
        )?;
        fs::write(self.session.join("diagnostic-log.txt"), &self.log)?;
        fs::write(self.session.join("notes.txt"), &self.notes)?;
        fs::write(self.session.join("README.txt"), "Share this entire session folder. manifest.json records actions and results; numbered JSON files hold diagnostics; capture JSONL files contain decoded candidate commands. Failed probes are useful evidence. Raw USBPcap files and proprietary firmware are not copied. TV actions can include startup samples and requested recordings. Device instance identifiers can appear in Windows inventory.\n")?;
        Ok(())
    }

    fn poll(&mut self) {
        let message = self.receive.as_ref().and_then(|r| r.try_recv().ok());
        if let Some(result) = message {
            self.accept_result(result);
        }
    }

    fn accept_result(&mut self, result: JobResult) {
        self.busy = false;
        self.control = None;
        if let Some(stations) = result.value.get("stations").and_then(Value::as_array) {
            self.stations = stations
                .iter()
                .filter(|s| s["locked"] == true)
                .filter_map(|s| s["frequency_khz"].as_u64().map(|n| n as u32))
                .collect();
            if let Some(&first) = self.stations.first() {
                self.frequency = first;
            }
        }
        self.receive = None;
        self.job_number += 1;
        let name = format!("{:03}-diagnostic.json", self.job_number);
        self.log.push_str(&format!(
            "\n--- {} ---\n{}\n",
            result.title,
            serde_json::to_string_pretty(&result.value).unwrap_or_default()
        ));
        if let Some(error) = &result.error {
            self.log.push_str(error);
        }
        self.jobs.push(json!({"title":result.title,"file":name,"worker_error":result.error,"time_ms":now_ms()}));
        let saved = fs::create_dir_all(&self.session)
            .and_then(|_| write_json(&self.session.join(&name), &result.value))
            .and_then(|_| self.save_export());
        self.status = match saved {
            Ok(_) => "Diagnostic finished; results saved. Check the result for success or failure."
                .into(),
            Err(e) => format!("Result available, but export failed: {e}"),
        };
    }

    fn analyze_capture(&mut self, inventory_only: bool) {
        let path = PathBuf::from(&self.capture);
        let python = self.python.clone();
        let script = self.root.join("tools/usb_trace.py");
        let output = self
            .session
            .join(format!("capture-{}-commands.jsonl", now_ms()));
        let fields = [
            ("--bus", self.bus.clone()),
            ("--device", self.device.clone()),
            ("--section", self.section.clone()),
            ("--interface", self.interface.clone()),
        ];
        self.start(if inventory_only { "Capture inventory" } else { "Analyze capture" }, move || {
            if let Err(e) = fs::create_dir_all(output.parent().unwrap()) { return json!({"error":e.to_string()}); }
            let metadata = file_metadata(&path).map_err(|e| e.to_string());
            let mut cmd = Command::new(python);
            cmd.arg(&script).arg(&path);
            if inventory_only { cmd.arg("--inventory"); }
            else {
                cmd.arg("--output").arg(&output);
                for (flag, value) in fields { if !value.trim().is_empty() { cmd.arg(flag).arg(value.trim()); } }
            }
            let result = command_result(&mut cmd);
            let decoded = result.get("stdout").and_then(Value::as_str).and_then(|s| serde_json::from_str::<Value>(s).ok());
            json!({"capture":metadata.unwrap_or_else(|e|json!({"error":e})),"analysis":decoded,"process":result,
                "output_file":if inventory_only {None} else {Some(output.file_name().unwrap().to_string_lossy().to_string())},
                "hint":"If automatic detection is ambiguous, use Inventory, then enter the tuner bus/device and optional section/interface below."})
        });
    }
}

fn file_row(ui: &mut egui::Ui, label: &str, value: &mut String, extensions: &[&str]) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::TextEdit::singleline(value).desired_width(620.0));
        if ui.button("Browse...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(label, extensions)
                .pick_file()
            {
                *value = path.to_string_lossy().into_owned();
            }
        }
    });
}

impl eframe::App for DebugApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
        if let Some(control) = &self.control {
            if let Ok(status) = control.message.lock() {
                if !status.is_empty() {
                    self.status = status.clone();
                }
            }
            if ctx.input(|i| i.viewport().close_requested()) {
                control.stop();
            }
        }
        if self.screenshot.is_some() {
            self.frames += 1;
            ctx.request_repaint_after(Duration::from_millis(30));
            if self.frames == 10 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            }
            let screenshot = ctx.input(|i| {
                i.events.iter().find_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let Some(image) = screenshot {
                let pixels: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                if let Some(path) = self.screenshot.take() {
                    match image::save_buffer(
                        &path,
                        &pixels,
                        image.width() as u32,
                        image.height() as u32,
                        image::ColorType::Rgba8,
                    ) {
                        Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                        Err(e) => self.status = format!("Screenshot failed: {e}"),
                    }
                }
            }
        }
        if self.busy {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(10.0);
            ui.heading("AVerTV Volar S  /  Debug desk");
            ui.label(format!("Windows userspace TV and diagnostics  •  A865R / 07CA:B865  •  v{}",env!("CARGO_PKG_VERSION")));
            ui.add_space(6.0);
            ui.label("Watch and record TV, compare firmware, and export development evidence.");
            ui.small("Open firmware 0.1.4.0 receives TV on the tested A865R. Leave the firmware path empty to use it. Reception is verified locally on RF22.");
            ui.add_space(8.0);
        });
        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if self.busy {
                    ui.spinner();
                }
                if let Some(control) = &self.control {
                    if ui.button("Stop").clicked() {
                        control.stop();
                    }
                }
                ui.label(&self.status);
            });
            if let Some(control) = &self.control {
                let state = control.snapshot();
                if !state.is_null() {
                    ui.small(format!(
                        "Driver {} · API {} · Output {} · Colors: {} · {}",
                        a865r::api::DRIVER_VERSION,
                        a865r::api::API_VERSION,
                        state["output_resolution"],
                        state["color_profile"]["state"]
                            .as_str()
                            .unwrap_or("pending"),
                        state["color_profile"]["profile_name"]
                            .as_str()
                            .unwrap_or("No confirmed profile")
                    ));
                }
            }
            ui.small(format!("Exports: {}", self.session.display()));
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_enabled_ui(!self.busy, |ui| {
                self.television_ui(ui);
                ui.horizontal(|ui| {
                    if ui.button("Probe tuner").clicked() { self.start("Probe tuner", || collect_probe(false)); }
                    if ui.button("Power snapshot").on_hover_text("Read power-management checkpoints without changing them").clicked() { self.start("Power snapshot", || collect_probe(true)); }
                    if ui.button("Export report").clicked() {
                        self.status = match self.save_export() { Ok(_) => format!("Exported to {}",self.session.display()), Err(e)=>format!("Export failed: {e}") };
                    }
                    if ui.button("Open exports folder").clicked() {
                        match self.save_export() {
                            Ok(_) => { if let Err(e) = Command::new("explorer.exe").arg(&self.session).spawn() { self.status = e.to_string(); } }
                            Err(e) => self.status = e.to_string(),
                        }
                    }
                });
                ui.separator();
                file_row(ui,"Firmware",&mut self.firmware,&["fw","bin"]);
                ui.horizontal(|ui| {
                    if ui.add_enabled(!self.firmware.is_empty(),egui::Button::new("Inspect firmware")).clicked() {
                        let path=PathBuf::from(&self.firmware);
                        self.start("Inspect firmware",move || match fs::read(&path) {
                            Ok(bytes)=>match firmware_report(bytes) {Ok(v)=>v,Err(e)=>json!({"error":e.to_string()})},
                            Err(e)=>json!({"error":e.to_string()}),
                        });
                    }
                    if ui.button("Compare with original .sys...").clicked() {
                        if let Some(driver)=rfd::FileDialog::new().add_filter("Original driver",&["sys"]).pick_file() {
                            let reference=PathBuf::from(&self.firmware);
                            self.start("Compare driver firmware",move || {
                                let result=(||->UsbResult<Value>{
                                    let data=fs::read(&driver)?;
                                    let embedded=FirmwareImage::extract_from_avermedia_driver(&data)?;
                                    let comparison=fs::read(&reference)?;
                                    Ok(json!({"driver":file_metadata(&driver)?,"firmware":firmware_report(comparison.clone())?,
                                        "embedded_firmware":firmware_report(embedded.bytes().to_vec())?,
                                        "byte_identical":embedded.bytes()==comparison.as_slice()}))
                                })();
                                result.unwrap_or_else(|e|json!({"error":e.to_string(),"hint":"Select a firmware file and the extracted x64 AVer857BDA.sys first."}))
                            });
                        }
                    }
                });
                ui.separator();
                file_row(ui,"USB capture",&mut self.capture,&["pcap","pcapng","cap"]);
                ui.horizontal(|ui| {
                    if ui.add_enabled(!self.capture.is_empty(),egui::Button::new("Inventory capture")).clicked(){self.analyze_capture(true);}
                    if ui.add_enabled(!self.capture.is_empty(),egui::Button::new("Analyze capture")).clicked(){self.analyze_capture(false);}
                    ui.small("Select a saved Wireshark/USBPcap file.");
                });
                ui.collapsing("Capture settings",|ui|{
                    ui.label("Leave selectors blank for automatic 07ca:b865 detection. Otherwise copy values from Inventory.");
                    ui.horizontal(|ui|{
                        for (label,value) in [("Bus",&mut self.bus),("Device",&mut self.device),("Section",&mut self.section),("Interface",&mut self.interface)] {
                            ui.label(label);ui.add(egui::TextEdit::singleline(value).desired_width(65.0));
                        }
                    });
                    file_row(ui,"Python executable",&mut self.python,&["exe"]);
                });
            });
            ui.collapsing("Your observations (included in exports)",|ui|{
                ui.label("Connection state, original driver behavior, channel/frequency, and what you clicked in Wireshark.");
                ui.add(egui::TextEdit::multiline(&mut self.notes).desired_rows(3).desired_width(f32::INFINITY));
            });
            ui.separator();
            ui.horizontal(|ui|{ui.strong("Results");if ui.button("Copy results").clicked(){ctx.copy_text(self.log.clone());}});
            egui::ScrollArea::both().auto_shrink([false,false]).stick_to_bottom(true).show(ui,|ui|{
                ui.add(egui::TextEdit::multiline(&mut self.log).font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY).interactive(false));
            });
        });
    }
}

fn main() -> eframe::Result {
    let mut app = DebugApp::new();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut watch_frequency = None;
    let mut play_path = None;
    let mut play_duration = 0u64;
    let mut play_export = None;
    let mut export = None;
    let mut probe = false;
    let mut power = false;
    let mut remote_seconds = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--remote-seconds" if i + 1 < args.len() => {
                remote_seconds = Some(args[i + 1].parse::<u32>().unwrap_or(0));
                i += 1;
            }
            "--probe" => probe = true,
            "--cpu" => app.playback.backend = RenderBackend::Cpu,
            "--watch" if i + 1 < args.len() => {
                watch_frequency = Some(args[i + 1].parse::<u32>().unwrap_or(0));
                i += 1;
            }
            "--play" | "--play-seconds" | "--play-export" if i + 1 < args.len() => {
                match args[i].as_str() {
                    "--play" => play_path = Some(PathBuf::from(&args[i + 1])),
                    "--play-seconds" => play_duration = args[i + 1].parse().unwrap_or(0),
                    _ => play_export = Some(PathBuf::from(&args[i + 1])),
                };
                i += 1;
            }
            "--capabilities" => {
                println!("{}", television::capabilities_json());
                return Ok(());
            }
            "--query-device" if i + 1 < args.len() => {
                let report = television::query_device();
                if let Err(error) = write_json(Path::new(&args[i + 1]), &report) {
                    eprintln!("{error}");
                    std::process::exit(2);
                }
                println!("{report}");
                return Ok(());
            }
            "--query-status" if i + 1 < args.len() => {
                match fs::read_to_string(Path::new(&args[i + 1]).join("status.json")) {
                    Ok(report) => println!("{report}"),
                    Err(error) => {
                        eprintln!("{error}");
                        std::process::exit(2);
                    }
                }
                return Ok(());
            }
            "--color-profile" if i + 1 < args.len() => {
                app.playback.color_profile = match args[i + 1].as_str() {
                    "monitor" => ColorProfile::Monitor,
                    "off" => ColorProfile::Disabled,
                    path => ColorProfile::File(PathBuf::from(path)),
                };
                i += 1;
            }
            "--collect-default-export" => export = Some(app.session.clone()),
            "--power-snapshot" => {
                probe = true;
                power = true;
            }
            "--firmware" | "--capture" | "--collect-export" | "--screenshot"
                if i + 1 < args.len() =>
            {
                let value = args[i + 1].clone();
                match args[i].as_str() {
                    "--firmware" => app.firmware = value,
                    "--capture" => app.capture = value,
                    "--collect-export" => export = Some(PathBuf::from(value)),
                    _ => app.screenshot = Some(PathBuf::from(value)),
                }
                i += 1;
            }
            _ => {
                eprintln!("Unknown or incomplete argument: {}", args[i]);
                std::process::exit(2);
            }
        }
        i += 1;
    }
    if let Some(seconds) = remote_seconds {
        let folder = export.unwrap_or_else(|| app.session.join("remote"));
        let report = a865r_media::remote::learn(
            seconds,
            &app.remote_label,
            &folder,
            &playback::Control::default(),
        );
        println!("{report}");
        if report["success"] != true {
            std::process::exit(1)
        }
        return Ok(());
    }
    if let Some(frequency) = watch_frequency {
        let folder = play_export.unwrap_or_else(|| app.session.join("live-tv"));
        let control = playback::Control::default();
        if play_duration > 0 {
            let cancel = control.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(play_duration));
                cancel.stop();
            });
        }
        let result = television::run(
            television::Action::Watch { frequency },
            PathBuf::from(&app.firmware),
            folder.clone(),
            app.player_options(),
            control,
        );
        let _ = write_json(&folder.join("result.json"), &result);
        if result["success"] != true {
            std::process::exit(1);
        }
        return Ok(());
    }
    if let Some(path) = play_path {
        let folder = play_export.unwrap_or_else(|| app.session.join("playback"));
        let control = playback::Control::default();
        if play_duration > 0 {
            let cancel = control.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(play_duration));
                cancel.stop();
            });
        }
        let result = playback::play_file(app.player_options(), path, folder.clone(), control);
        let _ = write_json(&folder.join("result.json"), &result);
        if result["success"] != true {
            std::process::exit(1);
        }
        return Ok(());
    }
    if let Some(path) = export {
        app.session = path;
        app.accept_result(JobResult {
            title: "Read-oriented tuner probe".into(),
            value: collect_probe(power),
            error: None,
        });
        if !app.firmware.is_empty() {
            let result = fs::read(&app.firmware)
                .map_err(a865r::Error::from)
                .and_then(firmware_report);
            app.accept_result(JobResult {
                title: "Firmware structure".into(),
                value: result.unwrap_or_else(|e| json!({"error":e.to_string()})),
                error: None,
            });
        }
        if app.save_export().is_err() {
            std::process::exit(2);
        }
        return Ok(());
    }
    if probe {
        app.start("Probe tuner", move || collect_probe(power));
    }
    let options = eframe::NativeOptions {
        centered: true,
        viewport: egui::ViewportBuilder::default()
            .with_icon(eframe::icon_data::from_png_bytes(include_bytes!("../../player/assets/app-icon.png")).expect("valid application icon"))
            .with_inner_size([1160.0, 940.0])
            .with_min_inner_size([850.0, 650.0]),
        ..Default::default()
    };
    eframe::run_native(
        "AVerTV Volar S - Debug desk",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::light());
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn firmware_report_rejects_invalid_and_does_not_export_payload() {
        assert!(firmware_report(vec![]).is_err());
        let report = firmware_report(vec![3, 0, 0, 1, 0x41, 0, 3, 2, 0x12, 0xbf]).unwrap();
        assert_eq!(report["regions"][0]["reset_jump"], "0x12bf");
        assert_eq!(report["matches_supplied_reference_fingerprint"], false);
        assert_eq!(report["payload_bytes_exported"], false);
    }
    #[test]
    fn failed_diagnostics_are_exported_and_notes_preserved() {
        let mut app = DebugApp::new();
        app.session = std::env::temp_dir().join(format!(
            "a865r-export-test-{}-{}",
            now_ms(),
            std::process::id()
        ));
        app.notes = "Device not connected\nSecond line".into();
        app.log = "USB transport error: device missing".into();
        app.jobs
            .push(json!({"title":"Probe tuner","error":"not connected"}));
        app.save_export().unwrap();
        let manifest: Value =
            serde_json::from_slice(&fs::read(app.session.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest["notes"], app.notes);
        assert!(fs::read_to_string(app.session.join("diagnostic-log.txt"))
            .unwrap()
            .contains("device missing"));
        // Remove only files created by this test, then the empty test directory.
        for name in [
            "manifest.json",
            "diagnostic-log.txt",
            "notes.txt",
            "README.txt",
        ] {
            fs::remove_file(app.session.join(name)).unwrap();
        }
        fs::remove_dir(app.session).unwrap();
    }
}
