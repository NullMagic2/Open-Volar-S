//! Snapshot destinations and CPU image encoding, shared by both video presenters.
use std::path::{Path,PathBuf};
pub fn default_folder()->PathBuf {
    crate::recordings::default_folder().parent().unwrap().join("snapshots")
}
pub fn load(settings:&serde_json::Value)->PathBuf {
    settings["snapshot_folder"].as_str().map(PathBuf::from).filter(|p|p.is_absolute()).unwrap_or_else(default_folder)
}
pub fn save_bmp(path:&Path,bmp:&[u8])->Result<(),String> {
    if let Some(parent)=path.parent() {std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
    // Preserve the explicit BMP paths used by existing diagnostic commands.
    if path.extension().is_some_and(|e|e.eq_ignore_ascii_case("bmp")) {
        return std::fs::write(path,bmp).map_err(|e|e.to_string());
    }
    let image=image::load_from_memory_with_format(bmp,image::ImageFormat::Bmp).map_err(|e|e.to_string())?;
    let temporary=path.with_extension("png.part");
    let result=image.save_with_format(&temporary,image::ImageFormat::Png).map_err(|e|e.to_string())
        .and_then(|_|std::fs::rename(&temporary,path).map_err(|e|e.to_string()));
    if result.is_err() {let _=std::fs::remove_file(&temporary);}
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saves_real_png_and_preserves_diagnostic_bmp() {
        let folder=std::env::temp_dir().join(format!("live-tv-snapshot-test-{}-{}",std::process::id(),crate::millis()));
        let mut bmp=Vec::new();bmp.extend(b"BM");bmp.extend(70u32.to_le_bytes());bmp.extend([0;4]);bmp.extend(54u32.to_le_bytes());bmp.extend(40u32.to_le_bytes());bmp.extend(2i32.to_le_bytes());bmp.extend(2i32.to_le_bytes());bmp.extend(1u16.to_le_bytes());bmp.extend(24u16.to_le_bytes());bmp.extend([0;24]);
        // Bottom-up BGR rows, each padded to eight bytes.
        bmp.extend([255,0,0,255,255,255,0,0,0,0,255,0,255,0,0,0]);
        let png=folder.join("frame.png");save_bmp(&png,&bmp).unwrap();
        let image=image::open(&png).unwrap().to_rgb8();assert_eq!(image.dimensions(),(2,2));
        assert_eq!(image.get_pixel(0,0).0,[255,0,0]);assert_eq!(image.get_pixel(1,0).0,[0,255,0]);assert_eq!(image.get_pixel(0,1).0,[0,0,255]);
        let diagnostic=folder.join("frame.bmp");save_bmp(&diagnostic,&bmp).unwrap();assert_eq!(std::fs::read(diagnostic).unwrap(),bmp);
        assert!(save_bmp(&folder.join("bad.png"),b"invalid").is_err());
        std::fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn destinations_validate_and_preserve_saved_folders() {
        let path=std::env::temp_dir().join("Live TV snapshots");
        assert_eq!(load(&serde_json::json!({"snapshot_folder":path})),path);
        assert_eq!(load(&serde_json::json!({"snapshot_folder":"relative"})),default_folder());
        assert!(crate::recordings::validate_folder("").is_err());assert!(crate::recordings::validate_folder("relative").is_err());
        assert_eq!(crate::recordings::validate_folder(&path.to_string_lossy()).unwrap(),path);
    }
}
