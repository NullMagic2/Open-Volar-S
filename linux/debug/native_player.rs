//! Linux MPEG-TS playback and live viewing through the Rust A865R receiver.
use crate::playback::{Control, Options};
use a865r::{Device, Error, FirmwareImage, Result};
use a865r::ts::TsAnalyzer;
use serde_json::{json, Value};
use crate::service_stream::{ProgramStream,clean_live_start,aligned_live_start,selected_program};
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize,Ordering};
use std::sync::{mpsc,Arc,Mutex};
use std::thread;
use std::time::{Duration,Instant};

/// Select the project's native helper on Linux; WSL retains its Windows host.
pub fn player_command()->Command {
    if crate::wsl_video::is_wsl(){return crate::wsl_video::player_command();}
    let path=std::env::var_os("A865R_NATIVE_PLAYER").map(PathBuf::from).unwrap_or_else(||{
        let sibling=std::env::current_exe().unwrap_or_default().with_file_name("open-volar-s-player");
        if sibling.is_file(){sibling}else{PathBuf::from("open-volar-s-player")}
    });Command::new(path)
}
pub fn resolved_renderer(mode:u64) -> u64 {
    if !crate::wsl_video::is_wsl(){return 1;}
    if mode != 0 { return mode.min(2); }
    static VULKAN: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *VULKAN.get_or_init(|| {
        let mut child=match Command::new(crate::color::player_path())
            .args(["--no-config","--no-terminal","--ao=null","--vo=gpu-next",
                "--gpu-api=vulkan","--gpu-context=x11vk","--frames=1",
                "av://lavfi:testsrc2=size=16x16:rate=1"])
            .stdout(Stdio::null()).stderr(Stdio::null()).spawn(){Ok(child)=>child,Err(_)=>return false};
        let start=Instant::now();
        loop {
            match child.try_wait(){
                Ok(Some(status))=>return status.success(),
                Err(_)=>return false,
                Ok(None) if start.elapsed()>Duration::from_secs(4)=>{
                    let _=child.kill();let _=child.wait();return false;
                },
                Ok(None)=>thread::sleep(Duration::from_millis(40)),
            }
        }
    }) {1} else {2}
}
pub fn renderer_args(mode:u64)->Vec<&'static str>{
    if crate::wsl_video::is_wsl(){return match mode{
        2=>vec!["--vo=gpu-next","--gpu-api=opengl","--gpu-context=x11egl","--gpu-sw=yes","--hwdec=auto-safe"],
        _=>vec!["--vo=gpu-next","--gpu-api=vulkan","--gpu-context=x11vk","--hwdec=auto-safe"],
    };}
    vec!["--renderer=vulkan"]
}

fn nonblocking_stdin(input:&std::process::ChildStdin)->io::Result<()> {
    let flags=unsafe { libc::fcntl(input.as_raw_fd(),libc::F_GETFL) };
    if flags<0{return Err(io::Error::last_os_error());}
    if unsafe { libc::fcntl(input.as_raw_fd(),libc::F_SETFL,flags|libc::O_NONBLOCK) }<0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
fn write_player(input:&mut std::process::ChildStdin,data:&[u8],control:&Control)->io::Result<()> {
    let start=Instant::now();let mut written=0;
    while written<data.len(){
        if control.cancel.load(Ordering::Relaxed){
            return Err(io::Error::new(io::ErrorKind::Interrupted,"playback stopped"));
        }
        match input.write(&data[written..]){
            Ok(0)=>return Err(io::Error::new(io::ErrorKind::WriteZero,"player input closed")),
            Ok(n)=>{written+=n;},
            Err(error) if error.kind()==io::ErrorKind::WouldBlock=>{
                if start.elapsed()>Duration::from_secs(30){
                    return Err(io::Error::new(io::ErrorKind::TimedOut,"player cannot keep up with live transport data"));
                }
                thread::sleep(Duration::from_millis(2));
            },
            Err(error)=>return Err(error),
        }
    }
    Ok(())
}

// Keep USB reads independent of the video renderer. WSLg may pause mpv while
// presenting a frame; a synchronous pipe write would stall the tuner too.
const PLAYER_QUEUE_LIMIT:usize=64*1024*1024;
struct PlayerPipe {
    sender:Option<mpsc::Sender<Vec<u8>>>,
    pending:Arc<AtomicUsize>,
    error:Arc<Mutex<Option<String>>>,
    writer:Option<thread::JoinHandle<()>>,
}
impl PlayerPipe {
    fn new(mut input:std::process::ChildStdin,control:Control)->io::Result<Self>{
        nonblocking_stdin(&input)?;
        let (sender,receiver)=mpsc::channel::<Vec<u8>>();
        let pending=Arc::new(AtomicUsize::new(0));
        let error=Arc::new(Mutex::new(None));
        let queued=Arc::clone(&pending);
        let failed=Arc::clone(&error);
        let writer=thread::spawn(move||{
            while let Ok(data)=receiver.recv(){
                queued.fetch_sub(data.len(),Ordering::Relaxed);
                if let Err(cause)=write_player(&mut input,&data,&control){
                    *failed.lock().unwrap()=Some(cause.to_string());
                    break;
                }
            }
        });
        Ok(Self{sender:Some(sender),pending,error,writer:Some(writer)})
    }
    fn push(&self,data:Vec<u8>)->Result<()> {
        if let Some(cause)=self.error.lock().unwrap().as_ref(){
            return Err(Error::Protocol(format!("Live TV player input: {cause}")));
        }
        if data.is_empty(){return Ok(());}
        let len=data.len();
        let old=self.pending.fetch_add(len,Ordering::Relaxed);
        if old.saturating_add(len)>PLAYER_QUEUE_LIMIT {
            self.pending.fetch_sub(len,Ordering::Relaxed);
            return Err(Error::Protocol("Live TV player remains behind the transport stream".into()));
        }
        if self.sender.as_ref().is_none_or(|sender|sender.send(data).is_err()){
            self.pending.fetch_sub(len,Ordering::Relaxed);
            let cause=self.error.lock().unwrap().clone().unwrap_or_else(||"input pipe closed".into());
            return Err(Error::Protocol(format!("Live TV player input: {cause}")));
        }
        Ok(())
    }
    fn finish(mut self,child:&mut Child){
        self.sender.take();
        if child.try_wait().ok().flatten().is_none(){crate::wsl_video::stop_player(child);}
        if let Some(writer)=self.writer.take(){let _=writer.join();}
    }
}

fn wait_for_saved_file(mut child: Child, control: Control) -> Value {
    control.status("Playing saved transport stream with the native player");
    loop {
        if control.cancel.load(Ordering::Relaxed) {
            crate::wsl_video::stop_player(&mut child);
            return json!({"success": true, "stopped": true});
        }
        match child.try_wait() {
            Ok(Some(status)) => return json!({"success": status.success(), "exit_code": status.code()}),
            Ok(None) => thread::sleep(Duration::from_millis(100)),
            Err(error) => return json!({"success": false, "error": error.to_string()}),
        }
    }
}

fn watch(frequency: u32, folder: PathBuf, options: Options, control: Control) -> Value {
    let result = (|| -> Result<Value> {
        a865r::channel_plan::validate_frequency(frequency)?;
        std::fs::create_dir_all(&folder)?;
        control.status("Connecting to the A865R tuner…");
        let mut device = Device::new();
        let info = device.connect_and_probe()?.clone();
        if !info.firmware_running {
            control.status("Loading open firmware into RAM…");
            let image = a865r::execution_probe_image()?;
            device.load_firmware(&FirmwareImage::from_scatter_bytes(image.bytes().to_vec())?)?;
        }
        let mut receiver = device.receiver()?;
        control.status("Initializing tuner…");
        receiver.initialize()?;
        let stream_result = (|| -> Result<Value> {
            control.status(format!("Tuning {:.3} MHz…", frequency as f64 / 1000.0));
            let tune = receiver.tune(frequency, 6000)?;
            if !tune.mpeg_locked {
                return Err(Error::Protocol("No MPEG lock. Check the antenna or scan another frequency.".into()));
            }
            let mut command = player_command();
            if crate::wsl_video::is_wsl(){
            command.args(["--no-config", "--cache=yes", "--cache-secs=2", "--demuxer-lavf-analyzeduration=3", "--demuxer-lavf-probesize=1000000", "--force-window=immediate", "--osc=no", "--osd-level=0", "--input-default-bindings=no", "--input-vo-keyboard=no", "--title=Live TV! — Open Volar S"]);
            command.args(renderer_args(resolved_renderer(0)));
            match options.deinterlacing {
                a865r::api::DeinterlaceMode::Off => {}
                a865r::api::DeinterlaceMode::SingleRate => {
                    command.arg("--vf=lavfi=[bwdif=mode=send_frame:deint=interlaced]");
                }
                a865r::api::DeinterlaceMode::DoubleRate => {
                    command.arg("--vf=lavfi=[bwdif=mode=send_field:deint=interlaced]");
                }
            }
            }else{
                command.args(renderer_args(1));
                command.arg(format!("--deinterlace-mode={}",match options.deinterlacing{a865r::api::DeinterlaceMode::Off=>0,a865r::api::DeinterlaceMode::SingleRate=>1,_=>2}));
                if let Some(program)=options.program_id{command.arg(format!("--program={program}"));}
            }
            if let Some(window) = std::env::var_os("A865R_MPV_WID") {
                command.arg(format!("--wid={}", window.to_string_lossy()));
            }
            if let Some(ipc) = std::env::var_os("A865R_MPV_IPC") {
                command.arg(format!("--input-ipc-server={}", ipc.to_string_lossy()));
            }
            if let Some(folder) = std::env::var_os("A865R_MPV_SCREENSHOT_DIR") {
                let folder=PathBuf::from(folder);
                std::fs::create_dir_all(&folder)?;
                command.arg(format!("--screenshot-directory={}",folder.display()));
            }
            if crate::wsl_video::is_wsl(){
                command.arg("--audio-channels=auto");if let Ok(filter)=std::env::var("A865R_MPV_AF"){command.arg(format!("--af={filter}"));}
            }else if let Ok(mode)=std::env::var("A865R_PLAYER_AUDIO_MODE"){command.arg(format!("--audio-mode={mode}"));}
            if let Ok(volume)=std::env::var("A865R_MPV_VOLUME"){command.arg(format!("--volume={volume}"));}
            if let Ok(captions)=std::env::var("A865R_MPV_CAPTIONS"){command.arg(format!("--sub-visibility={captions}"));}
            if let Ok(args)=std::env::var("A865R_MPV_VIDEO_ARGS"){if let Ok(args)=serde_json::from_str::<Vec<String>>(&args){command.args(args);}}
            let mut child = command.arg("-").stdin(Stdio::piped())
                .stdout(Stdio::null()).stderr(Stdio::inherit()).spawn().map_err(|error| {
                    Error::Protocol(format!("Cannot start the native Live TV player: {error}. Install open-volar-s-player or set A865R_NATIVE_PLAYER."))
                })?;
            let input = child.stdin.take().ok_or_else(|| Error::Protocol("Native player did not open its input pipe".into()))?;
            let player=PlayerPipe::new(input,control.clone())?;
            control.status(format!("Live TV! · {:.3} MHz",frequency as f64 / 1000.0));
            let mut analyzer=TsAnalyzer::new();
            let mut updated=Instant::now()-Duration::from_secs(2);
            let mut staged=Vec::new();
            let mut started=false;
            let mut selected_stream:Option<ProgramStream>=None;
            let signal_control=control.clone();
            let capture = receiver.stream_chunks_monitored(86400, &control.cancel, |data| {
                control.record_chunk(data)?;
                analyzer.push(data);
                if let Some(program)=selected_program(options.program_id,analyzer.stats()) {
                    let mut captions=control.caption_stream.lock().unwrap();
                    captions.select(analyzer.stats(),program);captions.push(data);
                }
                if updated.elapsed()>=Duration::from_secs(2) {
                    let stats=analyzer.stats();
                    control.set_epg(crate::guide_data::events(stats,frequency));
                    let services:Vec<Value>=stats.programs.keys().filter_map(|id|{
                        let video=stats.streams.iter().find(|s|s.program_number==*id&&matches!(s.stream_type,1|2|0x1b|0x24))?;
                        let audio=stats.streams.iter().find(|s|s.program_number==*id&&matches!(s.stream_type,3|4|0x0f|0x11));
                        Some(json!({"program_id":id,"frequency_khz":frequency,"channel_number":stats.channel_numbers.get(id),"name":stats.service_names.get(id).cloned().unwrap_or_else(||format!("TV service {id}")),"video_pid":video.pid,"audio_pid":audio.map(|s|s.pid)}))
                    }).collect();
                    control.set_scan_services(json!(services));
                    if let Some(program)=selected_program(options.program_id,&stats){
                        let streams:Vec<_>=stats.streams.iter().filter(|s|s.program_number==program).collect();
                        let video=streams.iter().find(|s|matches!(s.stream_type,1|2|0x1b|0x24)).map(|s|s.pid);
                        let audio=streams.iter().find(|s|matches!(s.stream_type,3|4|0x0f|0x11)).map(|s|s.pid);
                        control.set_service(json!({"program_id":program,"frequency_khz":frequency,"video_pid":video,"audio_pid":audio}));
                    }
                    updated=Instant::now();
                }
                if child.try_wait()?.is_some() {
                    return Err(Error::Protocol("Live TV player exited".into()));
                }
                if !started {
                    staged.extend_from_slice(data);
                    let stats=analyzer.stats();
                    let program=selected_program(options.program_id,&stats);
                    let start=program.and_then(|id| {
                        let video=stats.streams.iter().find(|stream| {
                            stream.program_number==id && stream.stream_type==0x1b
                        })?;
                        clean_live_start(&staged,video.pid,stats.programs.get(&id)?.pmt_pid)
                    });
                    if let Some(start)=start.or_else(|| {
                        (staged.len()>=12*1024*1024 && program.is_some())
                            .then(||aligned_live_start(&staged))
                    }) {
                        let id=program.expect("selected program checked above");
                        let info=stats.programs.get(&id).expect("program checked above");
                        let mut filtered=ProgramStream::new(id,info.pmt_pid,info.pcr_pid,&stats);
                        let packets=filtered.push(&staged[start..]);
                        player.push(packets)?;
                        selected_stream=Some(filtered);
                        staged.clear();
                        started=true;
                    }else if staged.len()>=24*1024*1024 && program.is_none(){
                        return Err(Error::Protocol(format!("Selected TV service {:?} is absent from this frequency",options.program_id)));
                    }
                } else if let Some(filtered)=selected_stream.as_mut(){
                    let packets=filtered.push(data);
                    player.push(packets)?;
                }
                Ok(())
            }, |signal| {
                if let Ok(report)=signal {signal_control.set_signal(report.quality_percent);}
            });
            let _=control.stop_recording();
            player.finish(&mut child);
            let _ = child.wait();
            let capture = capture?;
            Ok(json!({"success": true, "frequency_khz": frequency, "bytes": capture.bytes_written,
                "packets": capture.stats.packets, "stopped": control.cancel.load(Ordering::Relaxed)}))
        })();
        let stop = receiver.stop();
        match (stream_result, stop) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (_, Err(error)) => Err(error),
        }
    })();
    result.unwrap_or_else(|error| json!({"success": false, "error": error.to_string()}))
}

pub fn launch(
    options: Options,
    file: Option<PathBuf>,
    frequency: Option<u32>,
    folder: PathBuf,
    control: Control,
) -> Value {
    if let Some(frequency) = frequency {
        return watch(frequency, folder, options, control);
    }
    let Some(path) = file else {
        return json!({"success": false, "error": "Choose a recording or a frequency"});
    };
    match player_command()
        .args(renderer_args(resolved_renderer(0)))
        .arg(path)
        .spawn()
    {
        Ok(child) => wait_for_saved_file(child, control),
        Err(error) => json!({"success": false, "error": format!("Cannot start the native player: {error}")}),
    }
}

#[cfg(test)]
mod tests {
    use super::{aligned_live_start,clean_live_start,renderer_args,selected_program,ProgramStream};
    use a865r::ts::{TsAnalyzer,TsStats,ProgramInfo,StreamInfo};

    #[test]
    fn selected_hd_program_excludes_mobile_packets(){
        let mut stats=TsStats::default();
        for (program,pmt,video) in [(16960,0x1000,4113),(16984,0x1001,4097)]{
            stats.programs.insert(program,ProgramInfo{program_number:program,pmt_pid:pmt,pcr_pid:Some(video)});
            stats.streams.insert(StreamInfo{program_number:program,pid:video,stream_type:0x1b});
        }
        assert_eq!(selected_program(Some(16960),&stats),Some(16960));
        assert_eq!(selected_program(Some(16961),&stats),None,"A missing HD service must not fall back to mobile");
        let mut filter=ProgramStream::new(16960,0x1000,Some(4113),&stats);
        fn packet(pid:u16)->[u8;188]{
            let mut p=[0xff;188];p[0]=0x47;p[1]=((pid>>8)&0x1f) as u8;p[2]=pid as u8;p[3]=0x10;p
        }
        let mut input=Vec::new();
        for pid in [0,0x1001,4097,0x1000,4113,4097]{input.extend_from_slice(&packet(pid));}
        let split=200;
        let mut output=filter.push(&input[..split]);
        output.extend_from_slice(&filter.push(&input[split..]));
        assert_eq!(output.len()%188,0);
        let pids:Vec<u16>=output.chunks_exact(188)
            .map(|p|(u16::from(p[1]&0x1f)<<8)|u16::from(p[2])).collect();
        assert_eq!(pids,[0,0,0x1000,4113]);
        let mut analyzer=TsAnalyzer::new();analyzer.push(&output);
        assert_eq!(analyzer.stats().psi_crc_errors,0,"Synthesized PAT must have valid CRC");
        assert!(analyzer.stats().programs.contains_key(&16960));
        assert!(!analyzer.stats().programs.contains_key(&16984));
    }


    #[test]
    #[ignore="requires A865R_TEST_TS pointing to a captured multiplex"]
    fn real_hd_sample_filters_out_mobile_program(){
        let input=std::fs::read(std::env::var("A865R_TEST_TS").expect("A865R_TEST_TS"))
            .expect("read transport stream sample");
        let mut analyzer=TsAnalyzer::new();analyzer.push(&input);
        let stats=analyzer.stats();
        let info=stats.programs.get(&16960).expect("Gazeta HD program");
        let mut filter=ProgramStream::new(16960,info.pmt_pid,info.pcr_pid,&stats);
        let output=filter.push(&input);
        let mut selected=TsAnalyzer::new();selected.push(&output);
        assert!(selected.stats().programs.contains_key(&16960));
        assert!(!selected.stats().programs.contains_key(&16984));
        assert!(selected.stats().pid_packets.contains_key(&4113));
        assert!(!selected.stats().pid_packets.contains_key(&4097));
        let path=std::env::var("A865R_TEST_FILTERED_TS").expect("A865R_TEST_FILTERED_TS");
        std::fs::write(path,&output).unwrap();
    }

    #[test]
    fn exposes_platform_renderer() {
        if crate::wsl_video::is_wsl(){
            assert!(renderer_args(1).contains(&"--gpu-api=vulkan"));
            assert!(renderer_args(2).contains(&"--gpu-api=opengl"));
        }else{
            for mode in 0..3{assert_eq!(renderer_args(mode),vec!["--renderer=vulkan"]);}
        }
    }

    #[test]
    fn blocked_player_input_can_be_cancelled() {
        use std::sync::atomic::Ordering;
        let mut child=std::process::Command::new("sleep").arg("5")
            .stdin(std::process::Stdio::piped()).spawn().unwrap();
        let mut input=child.stdin.take().unwrap();
        super::nonblocking_stdin(&input).unwrap();
        let control=crate::playback::Control::default();
        let signal=control.clone();
        std::thread::spawn(move||{
            std::thread::sleep(std::time::Duration::from_millis(100));
            signal.cancel.store(true,Ordering::Relaxed);
        });
        let start=std::time::Instant::now();
        let error=super::write_player(&mut input,&vec![0;1024*1024],&control).unwrap_err();
        assert_eq!(error.kind(),std::io::ErrorKind::Interrupted);
        assert!(start.elapsed()<std::time::Duration::from_secs(1));
        let _=child.kill();let _=child.wait();
    }

    #[test]
    fn slow_player_does_not_block_usb_capture_queue() {
        let mut child=std::process::Command::new("sleep").arg("5")
            .stdin(std::process::Stdio::piped()).spawn().unwrap();
        let input=child.stdin.take().unwrap();
        let player=super::PlayerPipe::new(input,crate::playback::Control::default()).unwrap();
        let start=std::time::Instant::now();
        for _ in 0..100 { player.push(vec![0;64*1024]).unwrap(); }
        assert!(start.elapsed()<std::time::Duration::from_secs(1));
        player.finish(&mut child);
        let _=child.wait();
    }

    #[test]
    fn waits_for_selected_video_sequence_header() {
        fn packet(pid:u16,start:bool,payload:&[u8])->[u8;188] {
            let mut packet=[0xff;188];
            packet[..4].copy_from_slice(&[
                0x47,((pid>>8) as u8&0x1f)|if start{0x40}else{0},pid as u8,0x10
            ]);
            packet[4..4+payload.len()].copy_from_slice(payload);
            packet
        }
        let mut data=vec![0xff;512];
        for value in [packet(0,true,&[0]),packet(0x50,true,&[0]),
            packet(0x65,true,&[0,0,1,0x41]),packet(0x65,false,&[1]),
            packet(0,true,&[0]),packet(0x50,true,&[0]),
            packet(0x65,true,&[0,0,1,0x27])]
        {data.extend_from_slice(&value);}
        assert_eq!(aligned_live_start(&data),512);
        assert_eq!(clean_live_start(&data,0x65,0x50),Some(512+4*188));
        assert_eq!(clean_live_start(&data,0xc9,0x50),None);
    }
}
