//! Recover one explicitly selected service from a multiplex recording without
//! transcoding. The original and any existing output are never overwritten.
use a865r_media::playback::Control;
use std::{fs::File,io::Read,path::Path};
fn main()->Result<(),Box<dyn std::error::Error>>{
    let args:Vec<_>=std::env::args_os().collect();
    if args.len()!=4{return Err("Usage: record_selected_service INPUT.ts OUTPUT.ts PROGRAM_ID".into());}
    let program=args[3].to_str().ok_or("Invalid program")?.parse()?;
    let mut input=File::open(&args[1])?;let control=Control::default();
    control.start_selected_recording(Path::new(&args[2]),program)?;
    let mut buffer=vec![0;188*305];
    loop{let count=input.read(&mut buffer)?;if count==0{break;}control.record_chunk(&buffer[..count])?;}
    control.stop_recording()?;
    println!("Saved selected service {program} to {}",args[2].to_string_lossy());Ok(())
}
