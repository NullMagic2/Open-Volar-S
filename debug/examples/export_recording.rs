//! Export/recover a closed selected-service capture using its saved settings.
use a865r::api::{Resolution,DeinterlaceMode};
use a865r_media::{recording_export::{self,Settings},playback::{Control,Options}};
fn main()->Result<(),Box<dyn std::error::Error>>{
    let args:Vec<_>=std::env::args_os().collect();
    if args.len()!=4{return Err("Usage: export_recording INPUT OUTPUT SETTINGS.json".into());}
    let v:serde_json::Value=serde_json::from_slice(&std::fs::read(&args[3])?)?;
    let resolution=match v["dimensions"][0].as_u64(){Some(1920)=>Resolution::Hd,Some(2560)=>Resolution::Qhd,Some(3840)=>Resolution::Uhd,_=>Resolution::Native};
    let deinterlacing=match v["deinterlacing"].as_str(){Some("double")=>DeinterlaceMode::DoubleRate,Some("off")=>DeinterlaceMode::Off,_=>DeinterlaceMode::SingleRate};
    let s=Settings{program:v["program"].as_u64().ok_or("Missing program")? as u32,resolution,deinterlacing,picture:v["picture"].clone()};
    println!("{}",recording_export::finish(args[1].as_ref(),args[2].as_ref(),&s,&Options::default().ffmpeg,&Control::default())?);Ok(())
}
