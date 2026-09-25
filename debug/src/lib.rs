//! Public playback backend for applications using the A865R userspace driver.
pub mod color;
pub mod playback;
mod service_stream;
pub mod recording_export;
pub mod remote;
pub mod television;
const REFERENCE_SHA: &str = "4b066157d0eb1a088e55daaf339418fc6596b5f76c4583d06d5eddb3a272e921";

#[cfg(windows)]
#[path = "../../windows/debug/native_player.rs"]
pub mod native_player;
#[cfg(target_os = "linux")]
#[path = "../../linux/debug/wsl_video.rs"]
pub mod wsl_video;
#[cfg(target_os = "linux")]
#[path = "../../linux/debug/native_player.rs"]
pub mod native_player;

pub mod guide_data;

#[cfg(target_os="linux")] pub mod caption_stream;
