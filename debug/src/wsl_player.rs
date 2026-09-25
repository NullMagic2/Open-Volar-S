#[cfg(target_os="linux")]
#[path="../../linux/debug/wsl_player.rs"]
mod player;
#[cfg(target_os="linux")]
fn main(){if let Err(error)=player::run(){eprintln!("[ERROR]: WSL playback: {error}");std::process::exit(1);}}
#[cfg(not(target_os="linux"))]
fn main(){eprintln!("This frontend runs only inside WSL.");std::process::exit(1);}
