//! Explicit diagnostic recording without a window, decoder or graphics device.
use std::path::{Path,PathBuf};
use a865r_media::{playback::Control,television};
use serde_json::json;
fn arguments(args:&[String])->Result<(u32,u32,PathBuf),String>{
    if args.len()!=3 {return Err("Usage: --capture-only FREQUENCY_KHZ SECONDS OUTPUT_DIRECTORY".into());}
    let frequency=args[0].parse().map_err(|_|"Invalid frequency")?;
    a865r::channel_plan::validate_frequency(frequency).map_err(|e|e.to_string())?;
    let seconds=args[1].parse::<u32>().map_err(|_|"Invalid duration")?;
    if !(1..=30).contains(&seconds){return Err("Diagnostic capture must be 1–30 seconds".into());}
    Ok((frequency,seconds,args[2].clone().into()))
}
pub fn run(args:&[String],data:&Path)->i32{
    let Ok((frequency,seconds,folder))=arguments(args) else {return 2;};
    // Raw multiplex capture contains every service. Preserve the parental gate.
    let settings:serde_json::Value=std::fs::read(data.join("settings.json")).ok().and_then(|v|serde_json::from_slice(&v).ok()).unwrap_or_default();
    crate::parental::load(settings["parental"].clone());
    if crate::parental::active(){return 3;}
    if std::fs::create_dir_all(&folder).is_err() || folder.join("recording.ts").exists(){return 2;}
    let result=television::run(television::Action::Record{frequency,seconds},PathBuf::new(),folder.clone(),Default::default(),Control::default());
    let ok=result["success"]==true;
    let report=json!({"capture":result,"decoder_started":false,"renderer_started":false,"duration_limit_seconds":seconds});
    let _=std::fs::write(folder.join("capture-only.json"),report.to_string());
    if ok{0}else{1}
}
#[cfg(test)] mod tests{
    #[test] fn capture_is_explicit_and_bounded(){
        let args=|n:&str|vec!["521143".into(),n.into(),"capture".into()];
        for n in ["0","31","-1","bad"]{assert!(super::arguments(&args(n)).is_err());}
        assert_eq!(super::arguments(&args("15")).unwrap().1,15);
        assert!(super::arguments(&[]).is_err());
    }
}
