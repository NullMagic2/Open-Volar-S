//! ICC policy and renderer-confirmed reporting. Original TS recordings are never color transformed.
use a865r::api::{ColorProfile, ColorProfileState, ColorProfileStatus};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

pub fn validate_profile(path: &Path) -> io::Result<String> {
    let metadata = fs::metadata(path)?;
    if !(132..=32 * 1024 * 1024).contains(&metadata.len()) {
        return Err(io::Error::other(
            "ICC file size is outside supported bounds",
        ));
    }
    let data = fs::read(path)?;
    let declared = u32::from_be_bytes(data[..4].try_into().unwrap()) as usize;
    if declared < 132
        || declared > data.len()
        || &data[36..40] != b"acsp"
        || &data[16..20] != b"RGB "
    {
        return Err(io::Error::other(
            "Expected an RGB ICC/ICM profile; WCS and non-RGB profiles are unsupported",
        ));
    }
    let tags = u32::from_be_bytes(data[128..132].try_into().unwrap()) as usize;
    if tags > (declared - 132) / 12 {
        return Err(io::Error::other("ICC tag table is truncated"));
    }
    for tag in data[132..132 + tags * 12].chunks_exact(12) {
        let offset = u32::from_be_bytes(tag[4..8].try_into().unwrap()) as usize;
        let length = u32::from_be_bytes(tag[8..12].try_into().unwrap()) as usize;
        if offset.checked_add(length).is_none_or(|end| end > declared) {
            return Err(io::Error::other("ICC tag is outside the profile"));
        }
    }
    Ok(format!("{:x}", Sha256::digest(&data)))
}

pub fn player_path() -> PathBuf {
    if let Some(path) = std::env::var_os("A865R_MPV") {
        return path.into();
    }
    let exe = std::env::current_exe().unwrap_or_default();
    for path in [
        exe.with_file_name("mpv.exe"),
        exe.with_file_name("runtime").join("mpv.exe"),
        PathBuf::from("debug/runtime/mpv.exe"),
    ] {
        if path.is_file() {
            return path;
        }
    }
    PathBuf::from("mpv.exe")
}

pub fn configure(command: &mut Command, requested: &ColorProfile) -> io::Result<()> {
    command.args([
        "--no-config",
        "--load-scripts=no",
        "--vo=gpu-next",
        "--icc-intent=1",
        "--icc-cache=no",
        "--target-colorspace-hint=auto",
    ]);
    match requested {
        ColorProfile::Monitor => {
            command.arg("--icc-profile-auto=yes");
        }
        ColorProfile::Disabled => {
            command.args(["--icc-profile-auto=no", "--icc-profile="]);
        }
        ColorProfile::File(path) => {
            validate_profile(path)?;
            command.arg("--icc-profile-auto=no").arg(format!(
                "--icc-profile={}",
                fs::canonicalize(path)?.display()
            ));
        }
    }
    Ok(())
}

/// A detected path alone is not proof that the renderer loaded the ICC transform.
/// On monitor changes the new path invalidates old success until a new open confirmation.
pub fn parse_renderer_log(requested: ColorProfile, text: &str) -> ColorProfileStatus {
    let mut state = ColorProfileStatus::new(requested.clone());
    if requested == ColorProfile::Disabled {
        return state;
    }
    let mut selected = match &requested {
        ColorProfile::File(p) => Some(fs::canonicalize(p).unwrap_or(p.clone())),
        _ => None,
    };
    for line in text.lines() {
        if requested == ColorProfile::Monitor && line.contains("[vo/gpu-next/win32]") {
            if let Some((_, path)) = line.split_once("color-profile: ") {
                selected = Some(PathBuf::from(path.trim()));
                state.state = ColorProfileState::Pending;
                state.active_path = None;
                state.sha256 = None;
            }
        }
        if line.contains("[vo/gpu-next/libplacebo]") && line.contains("Opened ICC profile:") {
            state.state = ColorProfileState::Applied;
            state.active_path = selected.clone();
            state.detail = None;
        }
        if line.to_ascii_lowercase().contains("icc")
            && (line.contains("[e]") || line.contains("[fatal]"))
        {
            state.state = ColorProfileState::Failed;
            state.active_path = None;
            state.sha256 = None;
            state.detail = Some(line.to_owned());
        }
    }
    if state.state == ColorProfileState::Applied {
        if let Some(path) = &state.active_path {
            state.sha256 = validate_profile(path).ok();
        }
    }
    state
}

pub fn report_json(state: &ColorProfileStatus) -> Value {
    json!({"requested":match state.requested {ColorProfile::Monitor=>"monitor",ColorProfile::File(_)=>"file",ColorProfile::Disabled=>"disabled"},
        "requested_path":match &state.requested {ColorProfile::File(path)=>Some(path),_=>None},
        "state":format!("{:?}",state.state).to_ascii_lowercase(),"applied":state.state==ColorProfileState::Applied,
        "active_path":state.active_path,"profile_name":state.active_path.as_ref().and_then(|p|p.file_name()).map(|s|s.to_string_lossy()),
        "sha256":state.sha256,"intent":"relative_colorimetric","detail":state.detail,"evidence":"mpv/libplacebo renderer log; profile path detection alone is insufficient"})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detection_is_not_application_and_monitor_changes_invalidate_success() {
        let detect = "[v][vo/gpu-next/win32] color-profile: C:\\first.icm\n";
        let opened = "[v][vo/gpu-next/libplacebo] Opened ICC profile:\n";
        assert_eq!(
            parse_renderer_log(ColorProfile::Monitor, detect).state,
            ColorProfileState::Pending
        );
        assert_eq!(
            parse_renderer_log(ColorProfile::Monitor, &format!("{detect}{opened}")).state,
            ColorProfileState::Applied
        );
        let changed =
            format!("{detect}{opened}[v][vo/gpu-next/win32] color-profile: C:\\second.icm\n");
        let report = parse_renderer_log(ColorProfile::Monitor, &changed);
        assert_eq!(report.state, ColorProfileState::Pending);
        assert_eq!(report.active_path, None);
    }
    #[test]
    fn disabled_never_claims_an_applied_profile() {
        assert_eq!(
            parse_renderer_log(
                ColorProfile::Disabled,
                "[v][vo/gpu-next/libplacebo] Opened ICC profile:"
            )
            .state,
            ColorProfileState::Disabled
        );
    }
}
