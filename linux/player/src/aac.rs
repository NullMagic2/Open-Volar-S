//! Native AAC decoding through FAAD2, without an external player or FFmpeg.
//! FAAD2 is copyright (c) Nero AG, www.nero.com (GPL); see THIRD_PARTY.md.
use libc::{c_long,c_ulong,c_void};
use std::{ffi::CStr,ptr::NonNull};
#[repr(C)]
struct Configuration { object:u8, rate:c_ulong, output:u8, downmix:u8, old_adts:u8, no_implicit_sbr:u8 }
#[repr(C)]
#[derive(Default)]
struct Info {
    consumed:c_ulong,samples:c_ulong,channels:u8,error:u8,rate:c_ulong,
    sbr:u8,object:u8,header:u8,front:u8,side:u8,back:u8,lfe:u8,
    positions:[u8;32], positions_tail:[u8;32], ps:u8,
}
#[link(name="libfaad.so.2",kind="dylib",modifiers="+verbatim")]
unsafe extern "C" {
    fn NeAACDecOpen()->*mut c_void;
    fn NeAACDecClose(h:*mut c_void);
    fn NeAACDecGetCurrentConfiguration(h:*mut c_void)->*mut Configuration;
    fn NeAACDecSetConfiguration(h:*mut c_void,c:*mut Configuration)->u8;
    fn NeAACDecInit2(h:*mut c_void,b:*mut u8,len:c_ulong,rate:*mut c_ulong,channels:*mut u8)->libc::c_char;
    fn NeAACDecDecode(h:*mut c_void,i:*mut Info,b:*mut u8,len:c_ulong)->*mut c_void;
    fn NeAACDecGetErrorMessage(code:u8)->*const libc::c_char;
    fn NeAACDecPostSeekReset(h:*mut c_void,frame:c_long);
}
pub struct Decoder { handle:NonNull<c_void>, config:Vec<u8> }
pub struct Pcm { pub bytes:Vec<u8>,pub rate:u32,pub channels:usize }
impl Decoder {
    pub fn new()->Result<Self,String>{unsafe{
        let handle=NonNull::new(NeAACDecOpen()).ok_or("Cannot initialize AAC decoder")?;
        let result=Self{handle,config:Vec::new()};
        let c=NeAACDecGetCurrentConfiguration(handle.as_ptr());
        if c.is_null(){return Err("AAC decoder returned no configuration".into());}
        (*c).output=4; // FAAD_FMT_FLOAT
        (*c).downmix=0; // Preserve original speaker channels before our mixer.
        if NeAACDecSetConfiguration(handle.as_ptr(),c)==0{return Err("AAC float output unavailable".into());}
        Ok(result)
    }}
    pub fn reset(&mut self){unsafe{NeAACDecPostSeekReset(self.handle.as_ptr(),-1);}}
    pub fn decode(&mut self,config:&[u8],frame:&[u8])->Result<Pcm,String>{unsafe{
        if config.is_empty()||config.len()>256||frame.is_empty()||frame.len()>65536{return Err("Invalid AAC access unit".into());}
        if self.config!=config{
            let mut c=config.to_vec();let mut rate=0;let mut channels=0;
            if NeAACDecInit2(self.handle.as_ptr(),c.as_mut_ptr(),c.len() as c_ulong,&mut rate,&mut channels)!=0{
                self.config.clear();return Err("Unsupported AAC stream configuration".into());
            }
            self.config=config.to_vec();
        }
        // Decoder API takes a mutable pointer even though input remains encoded data.
        let mut input=frame.to_vec();let mut info=Info::default();
        let data=NeAACDecDecode(self.handle.as_ptr(),&mut info,input.as_mut_ptr(),input.len() as c_ulong);
        if info.error!=0{
            let e=NeAACDecGetErrorMessage(info.error);
            return Err(if e.is_null(){format!("AAC error {}",info.error)}else{CStr::from_ptr(e).to_string_lossy().into_owned()});
        }
        let n=info.samples as usize;let channels=info.channels as usize;
        if n==0{return Ok(Pcm{bytes:Vec::new(),rate:info.rate as u32,channels});}
        if data.is_null()||!matches!(channels,1|2|6)||n>64*2048||n%channels!=0||!(8000..=192000).contains(&info.rate){return Err("Invalid or unsupported decoded AAC format".into());}
        // FAAD channel order is AAC order, not Windows/Pulse speaker order.
        let positions=&info.positions[..channels];
        let canonical:&[u8]=match channels{1=>&[1],2=>&[2,3],_=>&[2,3,1,9,6,7]};
        let mut order=Vec::new();
        for &speaker in canonical{
            let idx=positions.iter().position(|&p|p==speaker||speaker==6&&p==4||speaker==7&&p==5)
                .ok_or_else(||format!("Unsupported AAC channel layout {positions:?}"))?;
            order.push(idx);
        }
        let samples=std::slice::from_raw_parts(data.cast::<f32>(),n);
        let mut bytes=Vec::with_capacity(n*4);
        for frame in samples.chunks_exact(channels){for &i in &order{
            let v=if frame[i].is_finite(){frame[i].clamp(-1.,1.)}else{0.};
            bytes.extend_from_slice(&v.to_le_bytes());
        }}
        Ok(Pcm{bytes,rate:info.rate as u32,channels})
    }}
}
impl Drop for Decoder{fn drop(&mut self){unsafe{NeAACDecClose(self.handle.as_ptr());}}}

/// Incremental ADTS framing; the PES boundary need not be an AAC frame boundary.
#[derive(Default)]
pub struct Adts { pending:Vec<u8> }
impl Adts {
    pub fn push(&mut self,bytes:&[u8])->Vec<(Vec<u8>,Vec<u8>)>{
        let mut frames=Vec::new();
        if self.pending.len()+bytes.len()>4*1024*1024{self.pending.clear();return frames;}
        self.pending.extend_from_slice(bytes);let mut at=0;
        while at+7<=self.pending.len(){
            let b=&self.pending[at..];
            if b[0]!=0xff||b[1]&0xf6!=0xf0{at+=1;continue;}
            let len=((b[3] as usize&3)<<11)|((b[4] as usize)<<3)|(b[5] as usize>>5);
            let header=if b[1]&1!=0{7}else{9};
            if len<header||b[6]&3!=0{at+=1;continue;}
            if at+len>self.pending.len(){break;}
            let object=(b[2]>>6)+1;let frequency=(b[2]>>2)&15;
            let channels=((b[2]&1)<<2)|(b[3]>>6);
            if frequency<13 {
                frames.push((vec![(object<<3)|(frequency>>1),((frequency&1)<<7)|(channels<<3)],b[header..len].to_vec()));
            }
            at+=len;
        }
        self.pending.drain(..at);frames
    }
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn adts_split_and_garbage_resynchronization(){
        let frame=[0xff,0xf1,0x4c,0x80,0x01,0x3f,0xfc,0x12,0x34];
        let mut p=Adts::default();assert!(p.push(&[4,2,1]).is_empty());
        assert!(p.push(&frame[..5]).is_empty());
        assert_eq!(p.push(&frame[5..]),vec![(vec![0x11,0x90],vec![0x12,0x34])]);
        assert!(p.push(&[]).is_empty());
    }
}
