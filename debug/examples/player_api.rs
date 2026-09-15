//! Run: cargo run -p a865r-debug --example player_api -- recording.ts [cpu|gpu|auto] [native|1080p|1440p|4k]
use a865r::api::{capabilities, PlaybackSettings, RenderBackend, Resolution, DRIVER_VERSION};
use a865r_media::playback::{play_file, Control, Options};
use std::path::PathBuf;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let input = PathBuf::from(args.get(1).ok_or("Supply a recording path")?);
    let mut settings = PlaybackSettings::default();
    settings.backend = match args.get(2).map(String::as_str).unwrap_or("auto") {
        "cpu" => RenderBackend::Cpu,
        "gpu" => RenderBackend::Gpu,
        "auto" => RenderBackend::Auto,
        _ => return Err("Choose cpu, gpu or auto".into()),
    };
    settings.resolution = match args.get(3).map(String::as_str).unwrap_or("4k") {
        "native" => Resolution::Native,
        "1080p" => Resolution::Hd,
        "1440p" => Resolution::Qhd,
        "4k" => Resolution::Uhd,
        _ => return Err("Choose native, 1080p, 1440p or 4k".into()),
    };
    settings.upscaling_enabled = settings.resolution != Resolution::Native;
    println!("Driver {DRIVER_VERSION}; {:?}", capabilities(None));
    let options = Options::from_settings(Options::default().ffmpeg, &settings)?;
    let folder = PathBuf::from("debug/exports").join(format!("api-{}", std::process::id()));
    let result = play_file(options, input, folder, Control::default());
    println!("{result}");
    if result["success"] != true {
        return Err("Playback failed; inspect the exported logs".into());
    }
    Ok(())
}
