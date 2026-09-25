//! Graphical DEB/RPM builder and installer. Linux diagnostics remain in Rust.
#[cfg(target_os = "linux")]
mod linux_app {
    use eframe::egui;
    use std::io::{BufRead, BufReader, Read};
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::thread;
    use std::time::Duration;

    enum Event {
        Line(String),
        Done(bool),
    }

    #[derive(Default)]
    struct InstallerApp {
        busy: bool,
        status: String,
        log: String,
        receiver: Option<Receiver<Event>>,
    }

    impl InstallerApp {
        fn start(&mut self, args: &'static [&'static str]) {
            if self.busy {
                return;
            }
            self.busy = true;
            self.status = "Building…".into();
            self.log.push_str(&format!("\n$ bash linux/package.sh {}\n", args.join(" ")));
            let (sender, receiver) = mpsc::channel();
            self.receiver = Some(receiver);
            thread::spawn(move || run_package(args, sender));
        }
    }

    fn read_lines<R: Read + Send + 'static>(source: R, sender: Sender<Event>) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            for line in BufReader::new(source).lines() {
                match line {
                    Ok(line) => { let _ = sender.send(Event::Line(format!("{line}\n"))); }
                    Err(error) => { let _ = sender.send(Event::Line(format!("Read error: {error}\n"))); break; }
                }
            }
        })
    }

    fn run_package(args: &'static [&'static str], sender: Sender<Event>) {
        let source = std::env::var_os("OPEN_VOLAR_S_SOURCE").map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok().filter(|p| p.join("linux/package.sh").is_file()));
        let script = source.map(|p| p.join("linux/package.sh")).unwrap_or_else(||
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent().expect("installer has a linux parent").join("package.sh"));
        if !script.is_file() {
            let _ = sender.send(Event::Line("Source tree required: start this installer from the extracted source directory or set OPEN_VOLAR_S_SOURCE to that directory.\n".into()));
            let _ = sender.send(Event::Done(false));
            return;
        }
        let mut command = Command::new("bash");
        command.arg(script).args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
        match command.spawn() {
            Ok(mut child) => {
                let stdout = read_lines(child.stdout.take().expect("piped stdout"), sender.clone());
                let stderr = read_lines(child.stderr.take().expect("piped stderr"), sender.clone());
                let result = child.wait();
                let _ = stdout.join();
                let _ = stderr.join();
                match result {
                    Ok(status) => { let _ = sender.send(Event::Done(status.success())); }
                    Err(error) => {
                        let _ = sender.send(Event::Line(format!("Package process failed: {error}\n")));
                        let _ = sender.send(Event::Done(false));
                    }
                }
            }
            Err(error) => {
                let _ = sender.send(Event::Line(format!("Could not start package builder: {error}\n")));
                let _ = sender.send(Event::Done(false));
            }
        }
    }

    impl eframe::App for InstallerApp {
        fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            if let Some(receiver) = &self.receiver {
                while let Ok(event) = receiver.try_recv() {
                    match event {
                        Event::Line(line) => self.log.push_str(&line),
                        Event::Done(success) => {
                            self.busy = false;
                            self.status = if success { "Completed — packages are in dist/" } else { "Failed — see log" }.into();
                        }
                    }
                }
            }
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Open Volar S for Linux");
                ui.label("Build the Rust receiver tools and Debug Desk with a DKMS USB driver. Install the native package here.");
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.add_enabled(!self.busy, egui::Button::new("Build DEB")).clicked() {
                        self.start(&["--format", "deb"]);
                    }
                    if ui.add_enabled(!self.busy, egui::Button::new("Build RPM")).clicked() {
                        self.start(&["--format", "rpm"]);
                    }
                    if ui.add_enabled(!self.busy, egui::Button::new("Build both")).clicked() {
                        self.start(&["--format", "all"]);
                    }
                    if ui.add_enabled(!self.busy, egui::Button::new("Build and install here")).clicked() {
                        self.start(&["--format", "native", "--install", "auto"]);
                    }
                });
                ui.add_space(8.0);
                ui.label(if self.status.is_empty() { "Ready" } else { &self.status });
                ui.separator();
                egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                    ui.add(egui::TextEdit::multiline(&mut self.log)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .interactive(false));
                });
            });
            if self.busy {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }
    }

    pub fn run() -> eframe::Result {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([820.0, 540.0]),
            ..Default::default()
        };
        eframe::run_native(
            "Open Volar S — Linux Installer",
            options,
            Box::new(|_| Ok(Box::new(InstallerApp::default()))),
        )
    }
}

#[cfg(target_os = "linux")]
fn main() -> eframe::Result { linux_app::run() }

#[cfg(not(target_os = "linux"))]
fn main() { eprintln!("The graphical package installer runs on Linux."); }
