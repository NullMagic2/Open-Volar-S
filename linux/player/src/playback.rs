//! Continuous native A/V playback. Video follows the audio device's audible PTS.
use std::{sync::{Arc,Mutex,mpsc,atomic::{AtomicBool,Ordering}},collections::VecDeque,time::{Duration,Instant},io::Read};
use gpu_video::{VulkanInstance,parameters::{VulkanAdapterDescriptor,VulkanDeviceDescriptor},broadcast::Frame};
use crate::{source,control,aac,transport,latm,sound,video,window,audio_modes};
enum Event{Video(Frame),Error(String),End}
struct AudioFrame{pcm:aac::Pcm,pts:f64}
#[derive(Default)]struct Clock{point:Option<(f64,Instant)>,end:f64,error:Option<String>,paused:Option<f64>,finished:bool}
impl Clock{fn position(&self)->Option<f64>{self.paused.or_else(||self.point.map(|(p,t)|(p+t.elapsed().as_secs_f64()).min(self.end)))}}
fn produce(source:Arc<source::Source>,cancel:Arc<AtomicBool>,device:Arc<gpu_video::VulkanDevice>,target_position:Option<f64>,tx:mpsc::SyncSender<Event>,audio_tx:mpsc::SyncSender<AudioFrame>,state:control::Shared)->Result<(),String>{
    let settings=state.lock().unwrap().settings.clone();
    let initial_offset=source.range().0;let mut file=source.reader(initial_offset,cancel.clone());
    let mut start=Vec::new();let mut analyzer=a865r::TsAnalyzer::new();let mut chunk=[0;188*256];
    loop{
        let n=file.read(&mut chunk).map_err(|e|e.to_string())?;if n==0{break;}start.extend_from_slice(&chunk[..n]);analyzer.push(&chunk[..n]);
        if analyzer.stats().streams.iter().any(|s|s.stream_type==0x1b&&settings.video_pid.is_none_or(|pid|pid==s.pid)&&settings.program.is_none_or(|p|p==s.program_number)){break;}
        if start.len()>4*1024*1024{return Err("No program table in transport stream".into());}
    }
    let offset=(0..start.len().saturating_sub(376)).find(|&i|start.get(i)==Some(&0x47)&&start.get(i+188)==Some(&0x47)&&start.get(i+376)==Some(&0x47)).ok_or("No transport stream sync")?;
    let streams=&analyzer.stats().streams;
    let video=streams.iter().find(|s|s.stream_type==0x1b&&settings.video_pid.is_none_or(|pid|pid==s.pid)&&settings.program.is_none_or(|p|p==s.program_number)).ok_or("No supported H.264 service")?;
    let audio=streams.iter().find(|s|s.program_number==video.program_number&&matches!(s.stream_type,0x0f|0x11)&&settings.audio_pid.is_none_or(|pid|pid==s.pid));
    {let mut r=state.lock().unwrap();r.tracks=serde_json::json!(streams.iter().filter(|s|s.program_number==video.program_number).map(|s|serde_json::json!({"id":s.pid,"demux-id":s.pid,"type":if s.stream_type==0x1b{"video"}else if matches!(s.stream_type,0x0f|0x11){"audio"}else{"sub"},"selected":s.pid==video.pid||audio.is_some_and(|a|a.pid==s.pid)})).collect::<Vec<_>>());}
    eprintln!("Native playback: service={} video={} audio={:?}",video.program_number,video.pid,audio.map(|s|(s.pid,s.stream_type)));
    let seek=target_position.and_then(|p|source.seek(video.pid,p));
    if target_position.is_some()&&seek.is_none(){return Err("Requested position is not indexed yet".into());}
    let anchor=seek.map(|(_,t)|t).or_else(||source.timeline(video.pid).map(|(first,_,_)|first)).unwrap_or(0.);
    let target=seek.map(|(_,t)|t);
    let decode_offset=seek.map(|(mark,_)|mark.offset).unwrap_or(initial_offset+offset as u64);
    let file=source.reader(decode_offset,cancel);
    let align=|pts:Option<i64>|pts.map(|pts|{let wrap=((1u64<<33)*1000/9) as i64;pts+((anchor*1e7-pts as f64)/wrap as f64).round() as i64*wrap});let mut decoder=gpu_video::broadcast::Decoder::new(&device).map_err(|e|e.to_string())?;
    let(mut vp,mut ap)=(transport::Pes::default(),transport::Pes::default());
    let mut audio_decoder=aac::Decoder::new()?;let mut adts=aac::Adts::default();let mut latm=latm::Latm::default();let mut audio_pts=None;
    let mut feed_audio=|p:transport::Packet|->Result<(),String>{
        let frames=if audio.is_some_and(|s|s.stream_type==0x11){latm.push(&p.bytes)}else{adts.push(&p.bytes).into_iter().map(Ok).collect()};
        if let Some(pts)=p.pts{audio_pts=Some(align(Some(pts)).unwrap() as f64/1e7);}
        for frame in frames{
            let(config,frame)=frame?;let mut pcm=audio_decoder.decode(&config,&frame)?;
            if pcm.bytes.is_empty(){continue;}
            let pts=audio_pts.ok_or("AAC stream has no timestamps")?;
            audio_pts=Some(pts+pcm.bytes.len() as f64/(pcm.channels as f64*4.*pcm.rate as f64));
            let mut pts=pts;
            if let Some(target)=target{
                if audio_pts.unwrap()<=target{continue;}
                if pts<target{let frames=((target-pts)*pcm.rate as f64).ceil() as usize;let bytes=(frames*pcm.channels*4).min(pcm.bytes.len());pcm.bytes.drain(..bytes);pts+=bytes as f64/(pcm.channels as f64*4.*pcm.rate as f64);}
            }
            if pcm.bytes.is_empty(){continue;}
            audio_tx.send(AudioFrame{pcm,pts}).map_err(|_|"Playback stopped")?;
        }Ok(())
    };
    let mut packets=transport::Packets::new(file);
    while let Some(packet)=packets.next().map_err(|e|e.to_string())?{
        let pid=((packet[1] as u16&31)<<8)|packet[2] as u16;
        if pid==video.pid{if let Some(p)=vp.push(&packet){for f in decoder.decode(&p.bytes,align(p.pts)).map_err(|e|e.to_string())?{if target.is_none_or(|t|f.pts as f64/1e7+f.duration as f64/1e7>t){send_video(&tx,f)?;}}}}
        else if audio.is_some_and(|s|s.pid==pid){if let Some(p)=ap.push(&packet){feed_audio(p)?;}}
    }
    if let Some(p)=ap.finish(){feed_audio(p)?;}
    if let Some(p)=vp.finish(){for f in decoder.decode(&p.bytes,align(p.pts)).map_err(|e|e.to_string())?{if target.is_none_or(|t|f.pts as f64/1e7+f.duration as f64/1e7>t){send_video(&tx,f)?;}}}
    for f in decoder.flush().map_err(|e|e.to_string())?{if target.is_none_or(|t|f.pts as f64/1e7+f.duration as f64/1e7>t){send_video(&tx,f)?;}}
    Ok(())
}
// A hidden/covered surface can block presentation. Never let its queue stop
// audio decoding or transport consumption; discard display frames only.
fn send_video(tx:&mpsc::SyncSender<Event>,frame:Frame)->Result<(),String>{
    match tx.try_send(Event::Video(frame)){
        Ok(())|Err(mpsc::TrySendError::Full(_))=>Ok(()),
        Err(mpsc::TrySendError::Disconnected(_))=>Err("Playback stopped".into()),
    }
}
fn audio_worker(rx:mpsc::Receiver<AudioFrame>,clock:Arc<Mutex<Clock>>,stop:Arc<AtomicBool>,state:control::Shared){
    let run=||->Result<(),String>{
        let mut output:Option<sound::Output>=None;let mut pending:Option<AudioFrame>=None;let mut offset=0;let mut paused=false;
        loop{
            if stop.load(Ordering::Relaxed){break;}
            let settings=state.lock().unwrap().settings.clone();
            if settings.paused!=paused{
                if let Some(out)=output.as_mut(){out.cork(settings.paused)?;}
                let mut c=clock.lock().unwrap();
                if settings.paused{c.paused=c.position();}else if let Some(p)=c.paused.take(){c.point=Some((p,Instant::now()));}
                paused=settings.paused;
            }
            if paused{std::thread::sleep(Duration::from_millis(2));continue;}
            if pending.is_none(){match rx.recv_timeout(Duration::from_millis(5)){
                Ok(f)=>{pending=Some(f);offset=0;},Err(mpsc::RecvTimeoutError::Timeout)=>continue,Err(mpsc::RecvTimeoutError::Disconnected)=>break,
            }}
            let frame=pending.as_ref().unwrap();let p=&frame.pcm;
            if output.as_ref().is_none_or(|o|o.rate!=p.rate||o.channels!=p.channels){output=Some(sound::Output::new(p.rate,p.channels)?);}
            let end=(offset+480*p.channels*4).min(p.bytes.len());let mut bytes=p.bytes[offset..end].to_vec();
            audio_modes::mix(&mut bytes,audio_modes::Format{channels:p.channels,samples:audio_modes::Samples::F32},settings.audio).map_err(|e|e.to_string())?;
            for sample in bytes.chunks_exact_mut(4){let v=f32::from_le_bytes(sample.try_into().unwrap())*settings.volume/100.;sample.copy_from_slice(&v.to_le_bytes());}
            let out=output.as_mut().unwrap();let written=out.try_write(&bytes)?;
            if written==0{std::thread::sleep(Duration::from_millis(2));continue;}
            offset+=written;
            let end=frame.pts+offset as f64/(p.channels as f64*4.*p.rate as f64);
            if let Some(latency)=out.latency()?{
                let point=end-latency;let mut c=clock.lock().unwrap();c.point=Some((point,Instant::now()));c.end=end;
            }
            if offset==p.bytes.len(){pending=None;}
        }
        if let Some(mut output)=output{if stop.load(Ordering::Relaxed){output.flush()?;}else{output.drain()?;}}
        Ok(())
    };
    if let Err(e)=run(){clock.lock().unwrap().error=Some(e);}
    clock.lock().unwrap().finished=true;
}
fn clone_frame(f:&Frame)->Frame{Frame{texture:f.texture.clone(),display_aspect:f.display_aspect,pts:f.pts,duration:f.duration,interlaced:f.interlaced,top_first:f.top_first,bt709:f.bt709,full:f.full}}
struct Displayed{frame:Frame,previous:Option<Frame>,next:Option<Frame>,field:u32}
fn draw(renderer:&mut video::Renderer,window:&mut window::Window,displayed:&Displayed,settings:&control::Settings,state:&control::Shared)->Result<bool,String>{
    let f=&displayed.frame;let source_size=(f.texture.width(),f.texture.height());
    let cap=match settings.size{1=>(2560,1440),2=>(3840,2160),_=>source_size};
    let aspect=if settings.aspect>0.{renderer.aperture=None;((settings.aspect*10000.).round() as u32,10000)}else{
        renderer.aperture=Some(crate::aspect::AspectRatio::Auto.aperture(source_size,f.display_aspect));
        crate::aspect::AspectRatio::Auto.display(source_size,f.display_aspect)
    };
    // Use precisely the Windows fitting rule: resolution caps never set the DAR.
    let(_,processing)=crate::canvas::geometry(aspect,window.size(),cap);
    renderer.render(f,displayed.previous.as_ref(),displayed.next.as_ref(),if settings.deinterlace==2{displayed.field}else{0},processing)?;
    let presented=window.present(renderer,aspect)?;{let mut s=state.lock().unwrap();s.osd=window.dimensions(aspect);s.video_position=Some(f.pts as f64/1e7-s.start);}Ok(presented)
}
pub fn play(path:String,muted:bool,ipc:Option<&std::path::Path>)->Result<(),Box<dyn std::error::Error>>{
    let mut runtime=control::Runtime::default();if muted{runtime.settings.volume=0.;}
    play_options(path,runtime,None,ipc)
}
pub fn play_options(path:String,runtime:control::Runtime,window_id:Option<u64>,ipc:Option<&std::path::Path>)->Result<(),Box<dyn std::error::Error>>{
    let state=Arc::new(Mutex::new(runtime));
    let _server=ipc.map(|p|control::Server::start(p,state.clone())).transpose()?;
    let instance=VulkanInstance::new()?;let mut window=window::Window::new(&instance,window_id)?;
    let adapter=instance.create_adapter(&VulkanAdapterDescriptor{compatible_surface:window.surface.as_ref(),..Default::default()})?;
    let device=adapter.create_device(&VulkanDeviceDescriptor::default())?;window.configure(&device);
    let mut renderer=video::Renderer::new(device.wgpu_device(),device.wgpu_queue());
    let source=if path=="-"{source::Source::live(std::io::stdin(),2*1024*1024*1024)?}
        else if let Some(address)=path.strip_prefix("tcp://127.0.0.1:"){
            let port=address.parse::<u16>()?;let stream=std::net::TcpStream::connect_timeout(&std::net::SocketAddr::from(([127,0,0,1],port)),Duration::from_secs(5))?;
            source::Source::live(stream,2*1024*1024*1024)?
        }else{source::Source::file(std::path::Path::new(&path))?};
    loop{
        let target=state.lock().unwrap().seek_request.take();
        if !session(source.clone(),device.clone(),&mut window,&mut renderer,state.clone(),target)?{break;}
    }
    Ok(())
}
fn session(source:Arc<source::Source>,device:Arc<gpu_video::VulkanDevice>,window:&mut window::Window,renderer:&mut video::Renderer,state:control::Shared,target:Option<f64>)->Result<bool,Box<dyn std::error::Error>>{
    let(tx,rx)=mpsc::sync_channel(12);let decode_device=device.clone();
    let(atx,arx)=mpsc::sync_channel(8);let clock=Arc::new(Mutex::new(Clock::default()));let stop=Arc::new(AtomicBool::new(false));
    let(c,s,r)=(clock.clone(),stop.clone(),state.clone());let audio=std::thread::spawn(move||audio_worker(arx,c,s,r));
    let producer_state=state.clone();let producer_stop=stop.clone();let producer_source=source.clone();
    let producer=std::thread::spawn(move||{if let Err(e)=produce(producer_source,producer_stop,decode_device,target,tx.clone(),atx,producer_state){let _=tx.send(Event::Error(e));}let _=tx.send(Event::End);});
    let mut frames:VecDeque<Frame>=VecDeque::new();let mut previous=None;let mut field=0;
    let mut displayed:Option<Displayed>=None;let mut revision=u64::MAX;let mut drawn_size=(0,0);let mut profile=None;let mut ended=false;let mut rendered=0;let mut late=0;let mut max_lateness=0f64;let mut wall=None;let began=Instant::now();let mut has_audio=true;
    let result=(||->Result<(),Box<dyn std::error::Error>>{
        loop{
            if window.closed()||state.lock().unwrap().seek_request.is_some(){break;}
            if let Some(e)=clock.lock().unwrap().error.clone(){return Err(e.into());}
            while frames.len()<4&&!ended{
                match rx.try_recv(){
                    Ok(Event::Video(f))=>{wall.get_or_insert((f.pts as f64/1e7,Instant::now()));frames.push_back(f);},
                    Ok(Event::Error(e))=>return Err(e.into()),Ok(Event::End)=>ended=true,
                    Err(mpsc::TryRecvError::Empty)=>break,Err(mpsc::TryRecvError::Disconnected)=>ended=true,
                }
            }
            if has_audio&&state.lock().unwrap().tracks.as_array().is_some_and(|t|!t.is_empty()&&!t.iter().any(|t|t["type"]=="audio")){has_audio=false;}
            let clock_position=clock.lock().unwrap().position();
            let position=if has_audio{clock_position}else{wall.map(|(p,t)|p+t.elapsed().as_secs_f64())};
            let(settings,new_revision,overlays,snapshots)={let mut s=state.lock().unwrap();
                let pid=s.tracks.as_array().and_then(|t|t.iter().find(|t|t["type"]=="video"&&t["selected"]==true)).and_then(|t|t["id"].as_u64()).map(|p|p as u16);
                if let Some((first,start,end))=pid.and_then(|p|source.timeline(p)){s.start=first;s.duration=(end-first).max(0.);s.cache_range=Some(((start-first).max(0.),s.duration));}
                s.position=position.map(|p|(p-s.start).max(0.)).unwrap_or(target.unwrap_or(0.));
                (s.settings.clone(),s.revision,s.overlays.clone(),if displayed.is_some(){std::mem::take(&mut s.snapshots)}else{Default::default()})};
            window.set_overlays(&renderer,&overlays);
            if profile.as_ref()!=Some(&settings.profile){
                let color_result=(||->Result<(),String>{
                    let data=match settings.profile.as_str(){"monitor"=>window.monitor_profile(),"off"|""=>None,path=>{
                        use std::io::Read;
                        let file=std::fs::File::open(path).map_err(|e|e.to_string())?;
                        let mut data=Vec::new();file.take(16*1024*1024+1).read_to_end(&mut data).map_err(|e|e.to_string())?;Some(data)
                    }};
                    renderer.set_color(data.as_deref())
                })();
                if let Err(e)=color_result{state.lock().unwrap().error=Some(format!("Display colour profile: {e}"));}
                profile=Some(settings.profile.clone());
            }
            renderer.picture=settings.picture;renderer.deinterlace=settings.deinterlace!=0;
            if new_revision!=revision||drawn_size!=window.size(){
                if let Some(last)=&displayed{draw(renderer,window,last,&settings,&state)?;}
                revision=new_revision;drawn_size=window.size();
            }
            for path in snapshots{
                if let Some(canvas)=&window.canvas{
                    if let Err(e)=renderer.save_texture(canvas,&path){state.lock().unwrap().error=Some(format!("Snapshot failed: {e}"));}
                }
            }
            if settings.paused{
                if displayed.is_none(){if let Some(f)=frames.front(){let last=Displayed{frame:clone_frame(f),previous:None,next:frames.get(1).map(clone_frame),field:0};draw(renderer,window,&last,&settings,&state)?;displayed=Some(last);}}
                std::thread::sleep(Duration::from_millis(2));continue;
            }
            if let Some(f)=frames.front().filter(|_|frames.len()>1||ended){
                let fields=if f.interlaced&&settings.deinterlace==2{2}else{1};
                if field>=fields{previous=frames.pop_front();field=0;continue;}
                let pts=(f.pts as f64+f.duration as f64*field as f64/fields as f64)/1e7;
                if let Some(position)=position{
                    if position>=pts{
                        let lateness=position-pts;max_lateness=max_lateness.max(lateness);
                        // Drop late fields, retaining the neighboring original frame for reconstruction.
                        if lateness<0.10||frames.len()==1{
                            let last=Displayed{frame:clone_frame(f),previous:previous.as_ref().map(clone_frame),next:frames.get(1).map(clone_frame),field};
                            if draw(renderer,window,&last,&settings,&state)?{rendered+=1;}displayed=Some(last);
                        }else{late+=1;}
                        field+=1;if field==fields{previous=frames.pop_front();field=0;}
                    }
                }
            }
            if ended&&frames.is_empty(){break;}
            if began.elapsed()>Duration::from_secs(30)&&rendered==0{return Err("Native playback produced no presented frames".into());}
            // At EOF, final video can extend beyond the last audio access unit.
            if has_audio&&clock.lock().unwrap().finished{
                if let Some(f)=frames.front(){wall=Some((f.pts as f64/1e7,Instant::now()));has_audio=false;}
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        eprintln!("Native A/V: fields={rendered}, dropped={late}, maximum scheduling lateness={max_lateness:.3}s, elapsed={:.3}s",began.elapsed().as_secs_f64());Ok(())
    })();
    if result.is_err()||!ended||!frames.is_empty(){stop.store(true,Ordering::Relaxed);}drop(rx);let _=producer.join();let _=audio.join();result?;Ok(state.lock().unwrap().seek_request.is_some())
}
