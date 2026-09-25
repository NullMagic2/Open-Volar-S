#[cfg(target_os = "linux")]
mod linux_main;
#[cfg(target_os = "linux")]
fn main() { linux_main::run(); }
#[cfg(not(target_os = "linux"))]
fn main() { eprintln!("Live TV! GTK frontend is available on Linux."); }
