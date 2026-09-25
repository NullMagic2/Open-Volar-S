//! Bounded disk-backed live transport cache, independent of decoder backpressure.
//! The backing file is private and unlinked immediately; no cache survives exit.
use std::{fs::{File,OpenOptions},io::{self,Read},os::unix::fs::{FileExt,OpenOptionsExt},sync::{Arc,Mutex,Condvar,atomic::{AtomicBool,Ordering}},time::{Duration,SystemTime,UNIX_EPOCH}};
const PACKET:u64=188;
struct Storage{file:File,capacity:Option<u64>,end:u64,done:bool,error:Option<String>,index:crate::index::Index,index_done:bool}
pub struct Source{storage:Mutex<Storage>,changed:Condvar}
pub struct Reader{source:Arc<Source>,pub position:u64,cancel:Arc<AtomicBool>}
impl Source{
    pub fn file(path:&std::path::Path)->io::Result<Arc<Self>>{
        let file=File::open(path)?;let scan=file.try_clone()?;let end=file.metadata()?.len();
        let source=Arc::new(Self{storage:Mutex::new(Storage{file,capacity:None,end,done:true,error:None,index:Default::default(),index_done:false}),changed:Condvar::new()});
        let weak=Arc::downgrade(&source);std::thread::spawn(move||{
            let mut at=0;let mut bytes=vec![0;188*2048];
            while let Some(source)=weak.upgrade(){
                match scan.read_at(&mut bytes,at){
                    Ok(0)=>{source.storage.lock().unwrap().index_done=true;source.changed.notify_all();break;},
                    Ok(n)=>{source.storage.lock().unwrap().index.push(&bytes[..n],0);source.changed.notify_all();at+=n as u64;},
                    Err(e)=>{let mut s=source.storage.lock().unwrap();s.error=Some(e.to_string());s.index_done=true;source.changed.notify_all();break;},
                }
            }
        });Ok(source)
    }
    pub fn live(input:impl Read+Send+'static,capacity:u64)->io::Result<Arc<Self>>{
        let source=Self::cache(capacity)?;let target=source.clone();std::thread::spawn(move||{
            let mut input=input;let mut buffer=vec![0;188*256];
            let result=(||->io::Result<()>{loop{let n=input.read(&mut buffer)?;if n==0{break;}target.append(&buffer[..n])?;}Ok(())})();
            let mut s=target.storage.lock().unwrap();s.done=true;s.error=result.err().map(|e|e.to_string());target.changed.notify_all();
        });Ok(source)
    }
    fn cache(capacity:u64)->io::Result<Arc<Self>>{
        let capacity=capacity/PACKET*PACKET;
        if capacity<PACKET*4{return Err(io::Error::other("Transport cache too small"));}
        let stamp=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        let path=std::env::temp_dir().join(format!("open-volar-s-cache-{}-{stamp}",std::process::id()));
        let file=OpenOptions::new().read(true).write(true).create_new(true).mode(0o600).open(&path)?;
        std::fs::remove_file(path)?;
        Ok(Arc::new(Self{storage:Mutex::new(Storage{file,capacity:Some(capacity),end:0,done:false,error:None,index:Default::default(),index_done:false}),changed:Condvar::new()}))
    }
    fn append(&self,mut data:&[u8])->io::Result<()>{
        let whole=data;
        let mut s=self.storage.lock().unwrap();let cap=s.capacity.ok_or_else(||io::Error::other("Not a live cache"))?;
        while !data.is_empty(){let at=s.end%cap;let n=(cap-at).min(data.len() as u64) as usize;s.file.write_all_at(&data[..n],at)?;s.end+=n as u64;data=&data[n..];}
        let oldest=s.end.saturating_sub(cap);s.index.push(whole,oldest);
        self.changed.notify_all();Ok(())
    }
    pub fn range(&self)->(u64,u64,bool){let s=self.storage.lock().unwrap();let start=s.capacity.map(|c|s.end.saturating_sub(c).div_ceil(PACKET)*PACKET).unwrap_or(0);(start,s.end,s.done)}
    pub fn timeline(&self,pid:u16)->Option<(f64,f64,f64)>{self.storage.lock().unwrap().index.range(pid)}
    pub fn seek(&self,pid:u16,position:f64)->Option<(crate::index::Mark,f64)>{self.storage.lock().unwrap().index.seek(pid,position)}
    pub fn reader(self:&Arc<Self>,position:u64,cancel:Arc<AtomicBool>)->Reader{Reader{source:self.clone(),position,cancel}}
}
impl Read for Reader{
    fn read(&mut self,out:&mut [u8])->io::Result<usize>{
        if out.is_empty(){return Ok(0);}
        let mut s=self.source.storage.lock().unwrap();
        loop{
            if self.cancel.load(Ordering::Relaxed){return Err(io::Error::new(io::ErrorKind::ConnectionAborted,"Playback stopped"));}
            if let Some(error)=&s.error{return Err(io::Error::other(error.clone()));}
            let oldest=s.capacity.map(|c|s.end.saturating_sub(c)).unwrap_or(0);
            if self.position<oldest{return Err(io::Error::other("Playback position expired from the live cache"));}
            if self.position<s.end{
                let at=s.capacity.map(|c|self.position%c).unwrap_or(self.position);
                let contiguous=s.capacity.map(|c|c-at).unwrap_or(s.end-self.position);
                let n=(out.len() as u64).min(s.end-self.position).min(contiguous) as usize;
                let read=s.file.read_at(&mut out[..n],at)?;self.position+=read as u64;return Ok(read);
            }
            if s.done{return Ok(0);}
            s=self.source.changed.wait_timeout(s,Duration::from_millis(20)).unwrap().0;
        }
    }
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn ring_wrap_retains_exact_newest_bytes_and_rejects_expired_reader(){
        let s=Source::cache(188*4).unwrap();let cancel=Arc::new(AtomicBool::new(false));let mut old=s.reader(0,cancel.clone());
        let data:Vec<_>=(0..188*6).map(|n|(n%251) as u8).collect();s.append(&data).unwrap();assert!(old.read(&mut [0;1]).is_err());
        let(start,end,_)=s.range();assert_eq!((start,end),(188*2,188*6));let mut r=s.reader(start,cancel);let mut actual=vec![0;188*4];r.read_exact(&mut actual).unwrap();assert_eq!(actual,&data[188*2..]);
    }
    #[test]fn a_waiting_live_reader_can_be_cancelled(){
        let s=Source::cache(188*4).unwrap();let cancel=Arc::new(AtomicBool::new(false));let mut r=s.reader(0,cancel.clone());
        let task=std::thread::spawn(move||r.read_exact(&mut [0;188]));cancel.store(true,Ordering::Relaxed);assert_eq!(task.join().unwrap().unwrap_err().kind(),io::ErrorKind::ConnectionAborted);
    }
}
