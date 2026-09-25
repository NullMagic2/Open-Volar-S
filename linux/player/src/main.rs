#[path = "../../../GUI/shared/aspect.rs"]
mod aspect;
#[path = "../../../GUI/shared/geometry.rs"]
mod canvas;
mod cli;
mod index;
mod color;
mod source;
mod control;
mod playback;
mod window;
mod video;
#[path = "../../../GUI/Windows/src/picture.rs"]
mod picture;
mod latm;
mod sound;
mod aac;
#[path = "../../../GUI/shared/audio.rs"]
mod audio_modes;
mod transport;
use gpu_video::{VulkanInstance,parameters::{VulkanAdapterDescriptor,VulkanDeviceDescriptor}};
fn main(){
    if let Err(error)=run(){eprintln!("Native player: {error}");std::process::exit(1);}
}
fn run()->Result<(),Box<dyn std::error::Error>>{
    if std::env::args().nth(1).as_deref()==Some("--play") {
        let args:Vec<_>=std::env::args().collect();
        let ipc=args.iter().find_map(|a|a.strip_prefix("--ipc=")).map(std::path::PathBuf::from);
        return playback::play(std::env::args().nth(2).ok_or("Missing TS file")?,args.iter().any(|s|s=="--mute"),ipc.as_deref());
    }
    if std::env::args().nth(1).as_deref()==Some("--test-sound") {
        let pcm=std::fs::read(std::env::args().nth(2).ok_or("Missing PCM file")?)?;
        let mut output=sound::Output::new(48000,2)?;let start=std::time::Instant::now();let mut latency=0f64;
        for chunk in pcm.chunks(480*8){output.write(chunk)?;latency=latency.max(output.latency()?.unwrap_or(0.));}
        output.drain()?;
        println!("Native audio output: {:.3}s PCM, {:.3}s elapsed, maximum latency {:.3}s",pcm.len() as f64/(48000.*8.),start.elapsed().as_secs_f64(),latency);
        return Ok(());
    }
    if std::env::args().nth(1).as_deref()==Some("--test-audio") {
        let data=std::fs::read(std::env::args().nth(2).ok_or("Missing TS file")?)?;
        let mut analyzer=a865r::TsAnalyzer::new();analyzer.push(&data);
        let pid=std::env::args().nth(3).and_then(|s|s.parse().ok()).or_else(||analyzer.stats().streams.iter().find(|s|s.stream_type==0x0f).map(|s|s.pid)).ok_or("No ADTS AAC stream")?;
        let mut decoder=aac::Decoder::new()?;let mut adts=aac::Adts::default();let mut pes=transport::Pes::default();let mut samples=0;let mut peak=0f32;
        let mut output=Vec::new();let mut rate=0;let mut channels=0;
        for p in data.chunks_exact(188) {
            if ((p[1] as u16&31)<<8)|p[2] as u16!=pid{continue;}
            if let Some(p)=pes.push(p){for (config,frame) in adts.push(&p.bytes){
                let mut pcm=decoder.decode(&config,&frame)?;
                audio_modes::mix(&mut pcm.bytes,audio_modes::Format{channels:pcm.channels,samples:audio_modes::Samples::F32},audio_modes::Mode::from_index(std::env::args().nth(4).and_then(|s|s.parse().ok()).unwrap_or(0)))?;
                for s in pcm.bytes.chunks_exact(4){peak=peak.max(f32::from_le_bytes(s.try_into().unwrap()).abs());}
                samples+=pcm.bytes.len()/4;rate=pcm.rate;channels=pcm.channels;output.extend(pcm.bytes);
            }}
        }
        println!("Native AAC: PID={pid} rate={rate} channels={channels} samples={samples} peak={peak}");
        if samples==0||peak==0.{return Err("No decoded audio".into());}
        std::fs::write("/tmp/ovs-native-audio.f32",output)?;
        return Ok(());
    }
    if std::env::args().nth(1).is_some_and(|s|s!="--test-render"&&s!="--test-decode"){
        return cli::run(std::env::args().skip(1).collect());
    }
    let instance=VulkanInstance::new()?;
    let adapter=instance.create_adapter(&VulkanAdapterDescriptor::default())?;
    println!("Adapter: {:?}",adapter.info());
    let device=adapter.create_device(&VulkanDeviceDescriptor::default())?;
    if std::env::args().nth(1).as_deref()==Some("--test-render") {
        let data=std::fs::read(std::env::args().nth(2).ok_or("Missing TS file")?)?;
        let mut analyzer=a865r::TsAnalyzer::new();analyzer.push(&data);
        let pid=analyzer.stats().streams.iter().find(|s|s.stream_type==0x1b).map(|s|s.pid).ok_or("No H.264 stream")?;
        let mut decoder=gpu_video::broadcast::Decoder::new(&device)?;
        let mut renderer=video::Renderer::new(device.wgpu_device(),device.wgpu_queue());
        let mut pes=transport::Pes::default();let mut frames=std::collections::VecDeque::new();let mut count=0;
        for p in data.chunks_exact(188){
            if ((p[1] as u16&31)<<8)|p[2] as u16!=pid{continue;}
            if let Some(p)=pes.push(p){for frame in decoder.decode(&p.bytes,p.pts)?{
                frames.push_back(frame);
                if frames.len()>=3{
                    for field in 0..2{renderer.render(&frames[1],Some(&frames[0]),Some(&frames[2]),field,(2560,1440))?;
                        if count==30{renderer.screenshot(std::path::Path::new(&format!("/tmp/ovs-native-shared-field-{field}.png")))?;}
                    }
                    frames.pop_front();count+=1;
                    if count>=60{println!("Rendered 120 fields with shared Windows shader at 2560x1440; GPU decode and presentation processing, no FFmpeg");return Ok(());}
                }
            }}
        }
        return Err(format!("Only rendered {count} pictures").into());
    }
    if let Some(path)=std::env::args().nth(2){
        let data=std::fs::read(path)?;
        let mut analyzer=a865r::TsAnalyzer::new();analyzer.push(&data);
        let pid=std::env::args().nth(3).and_then(|p|p.parse::<u16>().ok()).or_else(||analyzer.stats().streams.iter().find(|s|s.stream_type==0x1b).map(|s|s.pid)).ok_or("No H.264 stream")?;
        println!("Video PID: {pid}");
        let offset=(0..188).find(|&i|data.get(i)==Some(&0x47)&&data.get(i+188)==Some(&0x47)).ok_or("No TS sync")?;
        let mut decoder=gpu_video::broadcast::Decoder::new_bytes(&device)?;
        let mut pes=transport::Pes::default();let mut frames=0;
        for p in data[offset..].chunks_exact(188){
            if ((p[1] as u16&31)<<8)|p[2] as u16!=pid{continue;}
            if let Some(p)=pes.push(p){
                for f in decoder.decode(&p.bytes,p.pts)? {
                    frames+=1;
                    println!("Frame {frames}: {}x{} pts={} interlaced={} top_first={}",f.texture.width,f.texture.height,f.pts,f.interlaced,f.top_first);
                    if frames==30 {
                        let w=f.texture.width;let h=f.texture.height;
                        let mut rgba=Vec::with_capacity((w*h*4) as usize);
                        for y in 0..h as usize {for x in 0..w as usize{
                            let yy=(f.texture.frame[y*w as usize+x] as f32-16.)/219.;
                            let at=(w*h) as usize+(y/2)*w as usize+(x/2)*2;
                            let u=(f.texture.frame[at] as f32-128.)/224.;let v=(f.texture.frame[at+1] as f32-128.)/224.;
                            for n in [yy+1.5748*v,yy-0.187324*u-0.468124*v,yy+1.8556*u]{rgba.push((n.clamp(0.,1.)*255.) as u8);}rgba.push(255);
                        }}
                        image::save_buffer("/tmp/ovs-native-decoded.png",&rgba,w,h,image::ColorType::Rgba8)?;
                    }
                    if frames>=90{println!("Decoded 90 broadcast frames with Vulkan Video; no FFmpeg/GStreamer/mpv.");return Ok(());}
                }
            }
        }
        return Err(format!("Only decoded {frames} frames").into());
    }
    println!("Decode capabilities: {:?}",device.decode_capabilities());
    let _decoder=gpu_video::broadcast::Decoder::new(&device)?;
    println!("Native broadcast decoder initialized");
    Ok(())
}
