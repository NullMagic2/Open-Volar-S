//! Wine-only presentation: Windows controls, Linux mpv embedded in the Wine window.
use a865r::transport::wine_bridge::{WineTransport,unix_path};
use a865r_media::playback::Control;
use serde_json::{json,Value};
use std::{io::Write,path::PathBuf,sync::{atomic::Ordering,mpsc},time::{Duration,Instant}};
use windows::{core::w,Win32::{Foundation::*,Graphics::Gdi::MapWindowPoints,UI::WindowsAndMessaging::*}};
use crate::native::Command;
fn error(e:impl std::fmt::Display)->String {e.to_string()}
fn geometry(surface:usize)->Value {
    unsafe {
        let window=HWND(surface as _);let root=GetAncestor(window,GA_ROOT);
        let mut rect=RECT::default();let _=GetClientRect(window,&mut rect);
        let mut point=[POINT{x:0,y:0}];MapWindowPoints(window,root,&mut point);
        let client=GetPropW(root,w!("__wine_x11_client_window")).0 as usize;
        let parent=if client!=0 {client}else{GetPropW(root,w!("__wine_x11_whole_window")).0 as usize};
        json!({"parent":parent,
            "x":point[0].x,"y":point[0].y,"width":(rect.right-rect.left).max(1),"height":(rect.bottom-rect.top).max(1)})
    }
}
fn rpc(client:&mut WineTransport,op:u8,value:Value)->Result<Value,String> {
    let bytes=client.call(op,&serde_json::to_vec(&value).map_err(error)?).map_err(error)?;
    serde_json::from_slice(&bytes).map_err(error)
}
fn command(client:&mut WineTransport,value:Value)->Result<Value,String>{rpc(client,7,json!({"command":value}))}
fn property(client:&mut WineTransport,name:&str)->Value {
    command(client,json!(["get_property",name])).ok().map(|v|v["data"].clone()).unwrap_or(Value::Null)
}
fn live(listener:std::net::TcpListener,frequency:u32,program:Option<u32>,control:Control)->Result<(),String> {
    use a865r::{Device,execution_probe_image};
    let mut device=Device::new();let info=device.connect_and_probe().map_err(error)?;
    control.set_device_info(a865r_media::television::capabilities_for(Some(&info)));
    if !info.firmware_running {device.load_firmware(&execution_probe_image().map_err(error)?).map_err(error)?;}
    let mut receiver=device.receiver().map_err(error)?;receiver.initialize().map_err(error)?;
    let result=(||{
        control.status(format!("Tuning {:.3} MHz…",frequency as f64/1000.));
        if !receiver.tune(frequency,6000).map_err(error)?.mpeg_locked {return Err("No TV signal at this frequency".into());}
        control.set_signal(receiver.signal_quality().ok().flatten());
        listener.set_nonblocking(true).map_err(error)?;
        let started=Instant::now();
        let mut stream=loop {
            match listener.accept(){Ok((s,_))=>break s,Err(e) if e.kind()==std::io::ErrorKind::WouldBlock=>{},Err(e)=>return Err(error(e))}
            if control.cancel.load(Ordering::Relaxed){return Ok(());}
            if started.elapsed()>Duration::from_secs(15){return Err("Linux playback did not connect to the TV stream".into());}
            std::thread::sleep(Duration::from_millis(30));
        };
        stream.set_write_timeout(Some(Duration::from_secs(2))).map_err(error)?;
        let mut sample=Vec::new();let mut announced=false;
        let observer=crate::epg_source::observe(&[],frequency,control.clone());
        let mut gate=Instant::now()-Duration::from_secs(1);
        control.status("Playing through Wine / Linux mpv");
        receiver.stream_chunks_until_stopped(&control.cancel,|data|{
            observer(data);
            if gate.elapsed()>=Duration::from_secs(1) {
                crate::parental::check(&control.snapshot()["epg"],frequency,program.unwrap_or(0)).map_err(a865r::Error::Unsupported)?;
                gate=Instant::now();
            }
            control.record_chunk(data)?;
            if !announced {
                sample.extend_from_slice(data);
                if sample.len()>=188*5000 {
                    let services=a865r_media::television::discover_ts(&sample,frequency);
                    if let Some(service)=services.iter().find(|s|program.is_none_or(|p|s["program_id"].as_u64()==Some(p as u64))) {
                        control.set_service(service.clone());control.set_audio_tracks(service["audio_tracks"].clone());announced=true;
                    }
                    sample.clear();
                }
            }
            stream.write_all(data)?;Ok(())
        }).map_err(error)?;Ok(())
    })();
    let stopped=receiver.stop().map_err(error);
    result.and(stopped)
}
pub fn run(file:Option<PathBuf>,frequency:u32,program:Option<u32>,surface:usize,folder:PathBuf,
    control:Control,commands:&mpsc::Receiver<Command>,volume:u32,deinterlace:a865r::api::DeinterlaceMode,
    growing:bool,service_hint:Option<Value>,resolution:a865r::api::Resolution,color:a865r::api::ColorProfile)->Value {
    let (tx,rx)=mpsc::channel();let mut worker=None;
    let result=(||->Result<Value,String>{
        let config=std::env::var_os("OPEN_VOLAR_S_WINE_BRIDGE").map(PathBuf::from).unwrap_or_else(||r"C:\open-volar-s-bridge.txt".into());
        let mut client=WineTransport::new(config);client.connect().map_err(error)?;
        let mut start=geometry(surface);
        if start["parent"].as_u64().unwrap_or(0)==0 {return Err("Wine playback requires its X11 driver (XWayland is supported).".into());}
        crate::parental::check(&control.snapshot()["epg"],frequency,program.unwrap_or(0))?;
        let live_input=file.is_none();
        let source=if let Some(path)=&file {unix_path(path).map_err(error)?} else {
            let listener=std::net::TcpListener::bind("127.0.0.1:0").map_err(error)?;
            let source=format!("tcp://{}",listener.local_addr().map_err(error)?);
            let c=control.clone();worker=Some(std::thread::spawn(move ||{let _=tx.send(live(listener,frequency,program,c));}));source
        };
        if let Some(service)=service_hint{control.set_service(service);}
        start["source"]=json!(source);start["program"]=json!(program);start["volume"]=json!(volume);
        start["deinterlace"]=json!(deinterlace!=a865r::api::DeinterlaceMode::Off);start["follow"]=json!(growing);
        let _=std::fs::write(folder.join("wine-window.json"),start.to_string());
        let backend=rpc(&mut client,6,start)?;
        std::fs::write(folder.join("wine-playback.json"),backend.to_string()).map_err(error)?;
        control.set_native_diagnostic(backend);
        control.set_shader_state(json!({"enabled":true,"name":"Linux mpv GPU shaders"}));
        match color {
            a865r::api::ColorProfile::Monitor=>{command(&mut client,json!(["set_property","icc-profile-auto",true]))?;},
            a865r::api::ColorProfile::File(path)=>{command(&mut client,json!(["set_property","icc-profile",unix_path(&path).map_err(error)?]))?;},
            a865r::api::ColorProfile::Disabled=>{command(&mut client,json!(["set_property","icc-profile-auto",false]))?;},
        }
        let mut shader_revision=0u64;let mut subtitles=None;let mut service_selected=!live_input;
        let mut pending_picture=None;let mut ready_polls=0u32;
        let mut last_poll=Instant::now()-Duration::from_secs(1);let mut last_geometry=Value::Null;
        let mut paused=false;let mut frames=0u64;
        while !control.cancel.load(Ordering::Relaxed) {
            if let Ok(result)=rx.try_recv(){result?;if live_input {return Err("Tuner stream stopped".into());}}
            for c in commands.try_iter() {
                match c {
                    Command::Pause=>{command(&mut client,json!(["cycle","pause"]))?;paused=!paused;},
                    Command::Volume(v)=>{command(&mut client,json!(["set_property","volume",v.min(100)]))?;},
                    Command::Seek(s)=>{command(&mut client,json!(["seek",s,"absolute"]))?;},
                    Command::Step(n)=>{command(&mut client,json!([if n>=0{"frame-step"}else{"frame-back-step"}]))?;},
                    Command::GoLive=>{command(&mut client,json!(["seek",100,"absolute-percent"]))?;},
                    Command::AspectRatio(r)=>{let ratio=if r==crate::aspect::AspectRatio::Auto{-1.0}else{let(w,h)=r.dimensions((16,9));w as f64/h as f64};command(&mut client,json!(["set_property","video-aspect-override",ratio]))?;},
                    Command::Picture(p)=>{pending_picture=Some(p);},
                    Command::Snapshot(path)=>{command(&mut client,json!(["screenshot-to-file",unix_path(&path).map_err(error)?,"video"]))?;},
                    Command::AudioTrack(pid)=>{
                        if let Some(tracks)=property(&mut client,"track-list").as_array(){if let Some(t)=tracks.iter().find(|t|t["type"]=="audio"&&t["src-id"].as_u64()==Some(pid as u64)){command(&mut client,json!(["set_property","aid",t["id"]]))?;}}
                    },
                    Command::AudioMode(mode)=>{let filter=match mode {crate::audio::Mode::Mono=>"lavfi=[pan=stereo|c0=0.5*c0+0.5*c1|c1=0.5*c0+0.5*c1]",crate::audio::Mode::Left=>"lavfi=[pan=stereo|c0=c0|c1=c0]",crate::audio::Mode::Right=>"lavfi=[pan=stereo|c0=c1|c1=c1]",_=>""};command(&mut client,json!(["set_property","af",filter]))?;},
                    Command::Resize|Command::Repaint|Command::VideoHdr(_)=>{},
                }
            }
            let g=geometry(surface);if g!=last_geometry {rpc(&mut client,8,g.clone())?;last_geometry=g;}
            if last_poll.elapsed()>Duration::from_millis(500) {
                rpc(&mut client,8,geometry(surface))?;
                let show=control.captions_enabled.load(Ordering::Relaxed);
                if subtitles!=Some(show){command(&mut client,json!(["set_property","sub-visibility",show]))?;subtitles=Some(show);}
                if !service_selected {
                    // Use the selected service's actual TS PIDs. Track order and
                    // resolution are not stable across multiplexes or startup.
                    let state=control.snapshot();let service=&state["service"];
                    if let (Some(video_pid),Some(tracks))=(service["video_pid"].as_u64(),property(&mut client,"track-list").as_array()) {
                        let video=tracks.iter().find(|t|t["type"]=="video"&&t["src-id"].as_u64()==Some(video_pid));
                        let audio=tracks.iter().find(|t|t["type"]=="audio"&&t["src-id"].as_u64()==service["audio_pid"].as_u64());
                        if let Some(video)=video {
                            if audio.is_some() || service["audio_pid"].is_null() {
                                command(&mut client,json!(["set_property","vid",video["id"]]))?;
                                command(&mut client,json!(["set_property","aid",audio.map(|t|t["id"].clone()).unwrap_or(json!("no"))]))?;
                                service_selected=true;
                            }
                        }
                    }
                }
                let width=property(&mut client,"width").as_u64().unwrap_or(0) as u32;
                let height=property(&mut client,"height").as_u64().unwrap_or(0) as u32;
                control.set_video_size(width,height);
                if width>0 && height>0 && service_selected && property(&mut client,"vo-configured")==true {ready_polls+=1;} else {ready_polls=0;}
                // mpv must have created its video output before loading a user
                // shader. Loading it during probing can leave a black first FBO.
                if ready_polls>=2 {
                    if let Some(p)=pending_picture.take() {
                        if p==crate::picture::Picture::default() && resolution.dimensions().is_none() {
                            command(&mut client,json!(["set_property","glsl-shaders",[]]))?;
                        } else {
                            let mut source=a865r_media::recording_export::shader(&p.json());
                            if let Some((w,h))=resolution.dimensions(){source=source.replacen("//!HOOK MAIN\n",&format!("//!HOOK MAIN\n//!WIDTH {w}\n//!HEIGHT {h}\n"),1);}
                            shader_revision+=1;let path=folder.join(format!("wine-picture-{shader_revision}.glsl"));
                            std::fs::write(&path,source).map_err(error)?;
                            command(&mut client,json!(["set_property","glsl-shaders",[unix_path(&path).map_err(error)?]]))?;
                        }
                    }
                }
                let position=property(&mut client,"time-pos").as_f64().unwrap_or(0.);
                let duration=property(&mut client,"duration").as_f64().unwrap_or(0.);
                control.set_timeline(position,duration,!live_input,paused);
                frames=property(&mut client,"estimated-frame-number").as_u64().unwrap_or(frames);
                let report=json!({"backend":"Wine / Linux mpv","video_size":[width,height],"position":position,"frames":frames,"live":live_input});
                control.set_native_diagnostic(report.clone());let _=std::fs::write(folder.join("wine-playback.json"),report.to_string());
                if property(&mut client,"idle-active")==true && width==0 {return Err("Linux player could not open the TV stream".into());}
                last_poll=Instant::now();
            }
            std::thread::sleep(Duration::from_millis(30));
        }
        Ok(json!({"success":true,"stopped":true,"backend":"Wine / Linux mpv","frames":frames}))
    })();
    control.cancel.store(true,Ordering::Relaxed);
    if let Some(worker)=worker{let _=worker.join();}
    result.unwrap_or_else(|e|json!({"success":false,"error":e}))
}
