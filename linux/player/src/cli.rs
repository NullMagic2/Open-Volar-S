//! Strict command-line interface for the project's native playback helper.
use crate::{control::Runtime,playback};
use std::path::PathBuf;
pub fn run(args:Vec<String>)->Result<(),Box<dyn std::error::Error>>{
    let mut runtime=Runtime::default();let mut window=None;let mut ipc:Option<PathBuf>=None;let mut source=None;let mut positional=false;
    for arg in args{
        if arg=="--"{positional=true;continue;}
        if positional||!arg.starts_with("--"){
            if source.replace(arg).is_some(){return Err("Specify one playback source".into());}continue;
        }
        let(key,value)=arg.strip_prefix("--").unwrap().split_once('=').ok_or("Player options require --name=value")?;
        match key{
            "wid"=>window=Some(value.parse::<u64>()?),
            "input-ipc-server"|"ipc"=>ipc=Some(value.into()),
            "program"=>runtime.settings.program=Some(value.parse::<u16>()?),
            "renderer" if matches!(value,"auto"|"vulkan")=>{},
            "volume"|"audio-mode"|"deinterlace-mode"|"output-size"|"picture"|"video-aspect-override"|"icc-profile"|"screenshot-directory"|"sub-visibility"=>{
                let parsed=if matches!(key,"icc-profile"|"screenshot-directory"|"video-aspect-override"){serde_json::json!(value)}else{serde_json::from_str(value).unwrap_or_else(|_|serde_json::json!(value))};
                runtime.command(&serde_json::json!(["set_property",key,parsed]))?;
            },
            _=>return Err(format!("Unsupported native-player option: {key}").into()),
        }
    }
    playback::play_options(source.ok_or("Missing playback source")?,runtime,window,ipc.as_deref())
}
