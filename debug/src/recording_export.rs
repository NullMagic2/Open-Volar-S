//! One recording export contract for Windows, Linux and WSL.
//! Capture stays lossless; GPU processing runs only after the writer closes.
use crate::playback::Control;
use a865r::api::{Resolution,DeinterlaceMode};
use serde_json::{json,Value};
use std::{fs,io::Read,path::{Path,PathBuf},process::{Command,Stdio},time::{Duration,Instant}};
#[path="../../GUI/Windows/src/picture.rs"] mod picture;
#[path="recording_captions.rs"] mod captions;
static EXPORT_OWNER:std::sync::Mutex<()>=std::sync::Mutex::new(());

#[derive(Clone,Debug)]
pub struct Settings {pub resolution:Resolution,pub deinterlacing:DeinterlaceMode,pub picture:Value,pub program:u32}
impl Settings {
    pub fn json(&self)->Value{json!({"program":self.program,"dimensions":self.resolution.dimensions(),
        "deinterlacing":match self.deinterlacing{DeinterlaceMode::Off=>"off",DeinterlaceMode::SingleRate=>"single",DeinterlaceMode::DoubleRate=>"double"},"picture":self.picture})}
}
pub fn shader(value:&Value)->String{
    let p=picture::Picture::load(value);let [s,b,c,t]=p.uniform();let strength=p.effect_strength();
    // Match the scalar picture transform: bypass the tone curve when disabled.
    let effect=if p.hdr_effect {format!(r#"
 y=dot(rgb,weights);float mapped=y+{strength:.8}*y*(1.0-y)*(2.0*y-1.0);
 vec3 chroma=rgb-vec3(y);float scale=1.0;
 for(int i=0;i<3;i++) {{
  if(chroma[i]>0.000001) scale=min(scale,(1.0-mapped)/chroma[i]);
  else if(chroma[i]<-0.000001) scale=min(scale,-mapped/chroma[i]);
 }}
 rgb=clamp(vec3(mapped)+chroma*scale,0.0,1.0);
"#)}else{String::new()};
    format!(r#"//!HOOK MAIN
//!BIND HOOKED
//!DESC Live TV recording picture
vec4 hook() {{
 vec4 pixel=HOOKED_tex(HOOKED_pos); vec3 rgb=clamp(pixel.rgb,0.0,1.0);
 const vec3 weights=vec3(0.2126,0.7152,0.0722); float y=dot(rgb,weights);
 rgb=clamp((mix(vec3(y),rgb,{s:.8})-0.5)*{c:.8}+0.5+{b:.8},0.0,1.0);
 rgb=clamp(rgb+2.0*rgb*(1.0-rgb)*{t:.8}*vec3(-0.06,-0.02,0.06),0.0,1.0);
{effect}
 return vec4(rgb,pixel.a);
}}
"#)
}
fn wsl()->bool{
    #[cfg(target_os="linux")] {return crate::wsl_video::is_wsl();}
    #[cfg(not(target_os="linux"))] {false}
}
pub fn encoder_executable(preferred:&Path)->PathBuf{
    if let Some(p)=std::env::var_os("OVS_RECORDING_FFMPEG"){return p.into();}
    if wsl(){PathBuf::from("/mnt/c/Program Files/FFmpeg/bin/ffmpeg.exe")}else{preferred.into()}
}
fn host_path(path:&Path)->Result<String,String>{
    if wsl(){let result=Command::new("wslpath").arg("-w").arg(path).output().map_err(|e|e.to_string())?;
        if !result.status.success(){return Err("Cannot resolve recording path for Windows GPU export".into());}
        Ok(String::from_utf8_lossy(&result.stdout).trim().into())
    }else{Ok(path.to_string_lossy().into())}
}
fn command(exe:&Path)->Command{
    let mut c=Command::new(exe);
    #[cfg(windows)] {use std::os::windows::process::CommandExt;c.creation_flags(0x08000000);}
    c.stdin(Stdio::null()).stdout(Stdio::null());c
}
fn run(exe:&Path,args:&[String],log:&Path,control:&Control)->Result<bool,String>{
    let mut child=command(exe).args(args).stderr(fs::File::create(log).map_err(|e|e.to_string())?)
        .spawn().map_err(|e|format!("Cannot start hardware recording export ({}): {e}",exe.display()))?;
    let began=Instant::now();
    loop{
        if control.finalization_cancel.load(std::sync::atomic::Ordering::Relaxed)||began.elapsed()>Duration::from_secs(86400){
            let _=child.kill();let _=child.wait();return Err("Recording export cancelled/timed out; source capture preserved".into());}
        match child.try_wait(){Ok(Some(s))=>return Ok(s.success()),Err(e)=>{let _=child.kill();let _=child.wait();return Err(e.to_string());},_=>{}}
        std::thread::sleep(Duration::from_millis(100));
    }
}
fn strings(v:&[&str])->Vec<String>{v.iter().map(|s|s.to_string()).collect()}
fn filter(settings:&Settings,vaapi:bool)->String{
    let code=shader(&settings.picture).bytes().map(|b|format!("{b:02x}")).collect::<String>();
    let mut result=if vaapi{"null"}else{"hwdownload,format=nv12,hwupload"}.to_owned();
    match settings.deinterlacing{
        DeinterlaceMode::Off=>{},
        DeinterlaceMode::SingleRate if vaapi=>result.push_str(",deinterlace_vaapi=rate=frame:auto=1"),
        DeinterlaceMode::DoubleRate if vaapi=>result.push_str(",deinterlace_vaapi=rate=field:auto=1"),
        DeinterlaceMode::SingleRate=>result.push_str(",bwdif_vulkan=mode=send_frame:deint=interlaced"),
        DeinterlaceMode::DoubleRate=>result.push_str(",bwdif_vulkan=mode=send_field:deint=interlaced"),
    }
    if vaapi{result.push_str(",hwdownload,format=nv12");}
    result.push_str(",libplacebo=format=nv12");
    if let Some((w,h))=settings.resolution.dimensions(){result.push_str(&format!(":w={w}:h={h}"));}
    result.push_str(&format!(":custom_shader_bin={code}"));
    if vaapi{result.push_str(",format=nv12,hwupload");}else{result.push_str(",hwdownload,format=nv12");}
    result
}
pub fn finish(input:&Path,output:&Path,settings:&Settings,preferred:&Path,control:&Control)->Result<Value,String>{
    if output.exists()||input==output{return Err("Recording output already exists; source capture preserved".into());}
    let _owner=EXPORT_OWNER.lock().unwrap_or_else(|e|e.into_inner());
    if control.finalization_cancel.load(std::sync::atomic::Ordering::Relaxed){return Err("Recording export cancelled; source capture preserved".into());}
    let input=input.canonicalize().map_err(|e|e.to_string())?;
    let output=output.parent().unwrap_or(Path::new(".")).canonicalize().map_err(|e|e.to_string())?.join(output.file_name().ok_or("Missing recording filename")?);
    let folder=output.with_extension("export");fs::create_dir(&folder).map_err(|e|format!("Cannot create recording export folder: {e}"))?;
    fs::write(folder.join("settings.json"),settings.json().to_string()).map_err(|e|e.to_string())?;
    let mut initial=Vec::new();fs::File::open(&input).map_err(|e|e.to_string())?.take(24*1024*1024).read_to_end(&mut initial).map_err(|e|e.to_string())?;
    let mut analyzer=a865r::TsAnalyzer::new();
    for chunk in initial.chunks(188*256){analyzer.push(chunk);
        if analyzer.stats().streams.iter().any(|s|s.program_number as u32==settings.program&&matches!(s.stream_type,1|2|0x1b|0x24)){break;}}
    let stats=analyzer.stats();
    let streams:Vec<_>=stats.streams.iter().filter(|s|s.program_number as u32==settings.program).collect();
    let video=streams.iter().find(|s|matches!(s.stream_type,1|2|0x1b|0x24)).ok_or("Selected service has no video; source capture preserved")?;
    let mut pids=vec![video.pid];pids.extend(streams.iter().filter(|s|matches!(s.stream_type,3|4|0xf|0x11)).map(|s|s.pid));
    let profiles=stats.caption_profiles.iter().filter(|(pid,_)|streams.iter().any(|s|s.pid==**pid)).map(|(p,v)|(*p,*v)).collect::<std::collections::BTreeMap<_,_>>();
    pids.extend(profiles.keys().copied());
    let exe=encoder_executable(preferred);
    let windows=cfg!(windows)||wsl();
    let backends=if windows{vec![("d3d11va","h264_amf"),("d3d11va","h264_nvenc"),("d3d11va","h264_qsv")]}else{
        vec![("vaapi","h264_vaapi"),("cuda","h264_nvenc"),("qsv","h264_qsv")]};
    let mut encoded=None;
    for (index,(decoder,encoder)) in backends.iter().enumerate(){
        control.status(format!("Processing recording with {encoder}…"));
        let candidate=folder.join(format!("{encoder}.ts"));let log=folder.join(format!("{encoder}.log"));
        let mut args=strings(&["-hide_banner","-nostdin","-n","-v","warning","-hwaccel",decoder]);
        args.extend(strings(&["-hwaccel_output_format",match *decoder{"d3d11va"=>"d3d11",other=>other}]));
        if *encoder=="h264_vaapi"{
            let device=std::env::var("OVS_RECORDING_VAAPI_DEVICE").unwrap_or_else(|_|"/dev/dri/renderD128".into());
            args.extend(["-init_hw_device".into(),format!("vaapi=recordgpu:{device}")]);
        }else{args.extend(strings(&["-init_hw_device","vulkan=recordgpu"]));}
        args.extend(strings(&["-filter_hw_device","recordgpu","-i"]));
        args.push(host_path(&input)?);
        for (i,pid) in pids.iter().enumerate(){args.extend(["-map".into(),format!("0:i:{pid}"),"-streamid".into(),format!("{i}:{pid}")]);}
        args.extend(["-vf".into(),filter(settings,*encoder=="h264_vaapi")]);
        args.extend(strings(&["-c:v",encoder,"-b:v","30M","-maxrate","45M","-bufsize","60M","-c:a","copy","-c:s","copy","-c:d","copy","-mpegts_flags","+resend_headers+initial_discontinuity","-mpegts_service_id"]));
        args.push(settings.program.to_string());args.extend(strings(&["-avoid_negative_ts","make_zero","-muxdelay","0"]));args.push(host_path(&candidate)?);
        if run(&exe,&args,&log,control)?{
            encoded=Some((candidate,encoder.to_string()));break;
        }
        if let Some((next_dec,next_enc))=backends.get(index+1){eprintln!("[DEBUG]: {decoder}/{encoder} not available; trying {next_dec}/{next_enc}");}
    }
    let (mut candidate,encoder)=encoded.ok_or_else(||format!("Hardware recording export failed; source capture and settings preserved. See {}. FFmpeg must support Vulkan/libplacebo and a hardware encoder.",folder.display()))?;
    // Verification only; never silently replace the GPU export with CPU encoding.
    let mut args=strings(&["-v","error","-nostdin","-xerror","-i"]);args.push(host_path(&candidate)?);
    args.extend(strings(&["-map","0:v:0","-map","0:a?","-f","null","-"]));
    if !run(&exe,&args,&folder.join("verify.log"),control)?{return Err(format!("Export verification failed; source capture preserved. See {}",folder.display()));}
    if !profiles.is_empty(){let tagged=folder.join("captions.ts");captions::restore_caption_metadata(&candidate,&tagged,&profiles)?;candidate=tagged;}
    // Hard-link publication is atomic and refuses to replace existing files on either OS.
    fs::hard_link(&candidate,&output).map_err(|e|format!("Cannot publish recording: {e}; source capture preserved"))?;
    let _=fs::remove_file(&candidate);
    for (_,encoder) in &backends{let _=fs::remove_file(folder.join(format!("{encoder}.ts")));}
    let report=json!({"success":true,"recording":output,"broadcast_capture":input,"encoder":encoder,"settings":settings.json(),"verified":true});
    fs::write(folder.join("result.json"),report.to_string()).map_err(|e|e.to_string())?;
    control.status("Recording ready.");Ok(report)
}

#[cfg(test)] mod tests{
    use super::*;
    #[test]fn export_uses_gpu_resolution_and_exact_picture_shader(){
        let s=Settings{program:16960,resolution:Resolution::Qhd,deinterlacing:DeinterlaceMode::DoubleRate,picture:json!({"saturation":177,"hdr_effect":true})};
        let f=filter(&s,false);assert!(f.contains("w=2560:h=1440"));assert!(f.contains("bwdif_vulkan=mode=send_field"));
        assert!(f.contains("custom_shader_bin="));assert!(!f.contains("lut3d"));
        assert!(shader(&s.picture).contains("1.76999998"));assert!(shader(&s.picture).contains("0.55000001"));
    }
    #[test]fn existing_output_is_never_replaced(){
        let p=std::env::temp_dir().join(format!("ovs-export-keep-{}",std::process::id()));fs::write(&p,b"keep").unwrap();
        let s=Settings{program:1,resolution:Resolution::Native,deinterlacing:DeinterlaceMode::Off,picture:json!({})};
        assert!(finish(Path::new("missing"),&p,&s,Path::new("missing"),&Control::default()).unwrap_err().contains("already exists"));
        assert_eq!(fs::read(&p).unwrap(),b"keep");let _=fs::remove_file(p);
    }
}
