//! Native PulseAudio output, including lossless pause/resume and device timing.
//! PipeWire desktops expose the same PulseAudio endpoint. All API access is
//! serialized by the library's threaded mainloop lock; this owner stays on the
//! audio worker, while control requests travel through the player's state.
use libc::{c_char,c_int,c_void};
use std::{ptr::{null,null_mut,NonNull},time::{Duration,Instant}};
#[repr(C)]struct Spec{format:c_int,rate:u32,channels:u8}
#[repr(C)]struct Map{channels:u8,positions:[c_int;32]}
#[repr(C)]struct Attributes{maxlength:u32,tlength:u32,prebuf:u32,minreq:u32,fragsize:u32}
type Callback=Option<unsafe extern "C" fn(*mut c_void,c_int,*mut c_void)>;
#[link(name="libpulse.so.0",kind="dylib",modifiers="+verbatim")]
unsafe extern "C"{
    fn pa_threaded_mainloop_new()->*mut c_void;
    fn pa_threaded_mainloop_start(m:*mut c_void)->c_int;
    fn pa_threaded_mainloop_stop(m:*mut c_void);
    fn pa_threaded_mainloop_free(m:*mut c_void);
    fn pa_threaded_mainloop_lock(m:*mut c_void);
    fn pa_threaded_mainloop_unlock(m:*mut c_void);
    fn pa_threaded_mainloop_get_api(m:*mut c_void)->*mut c_void;
    fn pa_context_new(api:*mut c_void,name:*const c_char)->*mut c_void;
    fn pa_context_connect(c:*mut c_void,server:*const c_char,flags:u32,api:*const c_void)->c_int;
    fn pa_context_get_state(c:*const c_void)->c_int;
    fn pa_context_errno(c:*const c_void)->c_int;
    fn pa_context_disconnect(c:*mut c_void);
    fn pa_context_unref(c:*mut c_void);
    fn pa_stream_new(c:*mut c_void,name:*const c_char,spec:*const Spec,map:*const Map)->*mut c_void;
    fn pa_stream_connect_playback(s:*mut c_void,device:*const c_char,attr:*const Attributes,flags:u32,volume:*const c_void,sync:*mut c_void)->c_int;
    fn pa_stream_get_state(s:*const c_void)->c_int;
    fn pa_stream_writable_size(s:*const c_void)->usize;
    fn pa_stream_write(s:*mut c_void,data:*const c_void,bytes:usize,free:Option<unsafe extern "C" fn(*mut c_void)>,offset:i64,seek:c_int)->c_int;
    fn pa_stream_get_latency(s:*mut c_void,time:*mut u64,negative:*mut c_int)->c_int;
    fn pa_stream_cork(s:*mut c_void,cork:c_int,callback:Callback,data:*mut c_void)->*mut c_void;
    fn pa_stream_flush(s:*mut c_void,callback:Callback,data:*mut c_void)->*mut c_void;
    fn pa_stream_drain(s:*mut c_void,callback:Callback,data:*mut c_void)->*mut c_void;
    fn pa_stream_disconnect(s:*mut c_void)->c_int;
    fn pa_stream_unref(s:*mut c_void);
    fn pa_operation_get_state(o:*const c_void)->c_int;
    fn pa_operation_cancel(o:*mut c_void);
    fn pa_operation_unref(o:*mut c_void);
}
struct Lock(*mut c_void);
impl Drop for Lock{fn drop(&mut self){unsafe{pa_threaded_mainloop_unlock(self.0);}}}
pub struct Output{main:NonNull<c_void>,started:bool,context:*mut c_void,stream:*mut c_void,pub rate:u32,pub channels:usize}
impl Output{
    fn lock(&self)->Lock{unsafe{pa_threaded_mainloop_lock(self.main.as_ptr());}Lock(self.main.as_ptr())}
    fn error(&self,action:&str)->String{format!("Linux audio {action} (PulseAudio error {})",unsafe{pa_context_errno(self.context)})}
    pub fn new(rate:u32,channels:usize)->Result<Self,String>{unsafe{
        if !matches!(channels,1|2|6)||!(8000..=192000).contains(&rate){return Err("Unsupported speaker format".into());}
        let main=NonNull::new(pa_threaded_mainloop_new()).ok_or("Cannot allocate native audio mainloop")?;
        let mut out=Self{main,started:false,context:null_mut(),stream:null_mut(),rate,channels};
        if pa_threaded_mainloop_start(main.as_ptr())<0{return Err("Cannot start native audio mainloop".into());}out.started=true;
        {
            let _lock=out.lock();out.context=pa_context_new(pa_threaded_mainloop_get_api(main.as_ptr()),c"Live TV!".as_ptr());
            if out.context.is_null(){return Err("Cannot allocate native audio context".into());}
            if pa_context_connect(out.context,null(),0,null())<0{return Err(out.error("connection failed"));}
        }
        let began=Instant::now();loop{
            let state={let _lock=out.lock();pa_context_get_state(out.context)};
            if state==4{break;}if state>=5||began.elapsed()>Duration::from_secs(5){return Err(out.error("connection unavailable"));}
            std::thread::sleep(Duration::from_millis(2));
        }
        {
            let _lock=out.lock();let spec=Spec{format:5,rate,channels:channels as u8};
            let mut map=Map{channels:channels as u8,positions:[-1;32]};map.positions[..channels].copy_from_slice(match channels{1=>&[0],2=>&[1,2],_=>&[1,2,3,7,5,6]});
            out.stream=pa_stream_new(out.context,c"Television audio".as_ptr(),&spec,&map);
            if out.stream.is_null(){return Err(out.error("stream allocation failed"));}
            let frame=channels as u32*4;
            let attr=Attributes{maxlength:rate*frame/2,tlength:rate*frame/10,prebuf:u32::MAX,minreq:rate*frame/100,fragsize:u32::MAX};
            // Interpolated device timing, automatic timing refresh and a bounded
            // requested total latency; retain server channel mapping.
            if pa_stream_connect_playback(out.stream,null(),&attr,0x200a,null(),null_mut())<0{return Err(out.error("stream connection failed"));}
        }
        let began=Instant::now();loop{
            let state={let _lock=out.lock();pa_stream_get_state(out.stream)};
            if state==2{break;}if state>=3||began.elapsed()>Duration::from_secs(5){return Err(out.error("stream unavailable"));}
            std::thread::sleep(Duration::from_millis(2));
        }
        Ok(out)
    }}
    pub fn try_write(&mut self,pcm:&[u8])->Result<usize,String>{
        if pcm.len()%(self.channels*4)!=0{return Err("Incomplete PCM frame".into());}
        let _lock=self.lock();unsafe{
            let available=pa_stream_writable_size(self.stream);if available==usize::MAX{return Err(self.error("write unavailable"));}
            let n=available.min(pcm.len())/(self.channels*4)*(self.channels*4);
            if n!=0&&pa_stream_write(self.stream,pcm.as_ptr().cast(),n,None,0,0)<0{return Err(self.error("write failed"));}Ok(n)
        }
    }
    pub fn write(&mut self,mut pcm:&[u8])->Result<(),String>{
        let began=Instant::now();while !pcm.is_empty(){let n=self.try_write(pcm)?;pcm=&pcm[n..];if n==0{if began.elapsed()>Duration::from_secs(5){return Err("Native audio output timed out".into());}std::thread::sleep(Duration::from_millis(2));}}Ok(())
    }
    pub fn latency(&self)->Result<Option<f64>,String>{
        let _lock=self.lock();let(mut time,mut negative)=(0,0);
        if unsafe{pa_stream_get_latency(self.stream,&mut time,&mut negative)}<0{
            // PA_ERR_NODATA is normal before the server's first timing update.
            // Keep feeding PCM and wait for measured timing; never invent a clock.
            if unsafe{pa_context_errno(self.context)}==16{return Ok(None);}
            return Err(self.error("timing not available"));
        }
        Ok(Some(time as f64/1e6*if negative==0{1.}else{-1.}))
    }
    fn operation(&mut self,action:impl FnOnce(*mut c_void)->*mut c_void)->Result<(),String>{
        let op={let _lock=self.lock();action(self.stream)};let op=NonNull::new(op).ok_or_else(||self.error("control failed"))?;
        let began=Instant::now();let result=loop{
            let _lock=self.lock();let state=unsafe{pa_operation_get_state(op.as_ptr())};
            if state!=0{break if state==1{Ok(())}else{Err(self.error("control cancelled"))};}
            if unsafe{pa_stream_get_state(self.stream)}>=3||began.elapsed()>Duration::from_secs(5){unsafe{pa_operation_cancel(op.as_ptr());}break Err(self.error("control timed out"));}
            drop(_lock);std::thread::sleep(Duration::from_millis(2));
        };
        {let _lock=self.lock();unsafe{pa_operation_unref(op.as_ptr());}}result
    }
    pub fn cork(&mut self,paused:bool)->Result<(),String>{self.operation(|s|unsafe{pa_stream_cork(s,paused as c_int,None,null_mut())})}
    pub fn flush(&mut self)->Result<(),String>{self.operation(|s|unsafe{pa_stream_flush(s,None,null_mut())})}
    pub fn drain(&mut self)->Result<(),String>{self.operation(|s|unsafe{pa_stream_drain(s,None,null_mut())})}
}
impl Drop for Output{fn drop(&mut self){unsafe{
    if self.started{
        {let _lock=self.lock();if !self.stream.is_null(){pa_stream_disconnect(self.stream);pa_stream_unref(self.stream);}if !self.context.is_null(){pa_context_disconnect(self.context);pa_context_unref(self.context);}}
        pa_threaded_mainloop_stop(self.main.as_ptr());
    }
    pa_threaded_mainloop_free(self.main.as_ptr());
}}}
