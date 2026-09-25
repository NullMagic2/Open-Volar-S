//! Sparse transport index and 33-bit broadcast-clock unwrapping for native seek.
use std::collections::{BTreeMap,VecDeque};
const WRAP:i64=1<<33;
#[derive(Default)]pub struct Clock{last:Option<i64>}
impl Clock{
    pub fn ticks(&mut self,raw:i64)->i64{
        let ticks=if let Some(last)=self.last{let epoch=(last-raw+WRAP/2).div_euclid(WRAP);raw+epoch*WRAP}else{raw};
        self.last=Some(self.last.map_or(ticks,|last|last.max(ticks)));ticks
    }
}
#[derive(Clone,Copy,Debug)]pub struct Mark{pub pts:f64,pub offset:u64}
#[derive(Default)]struct Track{clock:Clock,marks:VecDeque<Mark>,pub first:Option<f64>,pub last:Option<f64>}
#[derive(Default)]pub struct Index{pending:Vec<u8>,at:u64,analyzer:a865r::TsAnalyzer,tracks:BTreeMap<u16,Track>}
impl Index{
    pub fn push(&mut self,bytes:&[u8],oldest:u64){
        self.analyzer.push(bytes);self.pending.extend_from_slice(bytes);let mut consumed=0;
        while consumed+188<=self.pending.len(){
            let packet=&self.pending[consumed..consumed+188];if packet[0]!=0x47{consumed+=1;continue;}
            let pid=((packet[1] as u16&31)<<8)|packet[2] as u16;
            if self.analyzer.stats().streams.iter().any(|s|s.pid==pid&&s.stream_type==0x1b){
                if let Some(raw)=timestamp(packet){
                    let track=self.tracks.entry(pid).or_default();let pts=track.clock.ticks(raw) as f64/90000.;
                    track.first.get_or_insert(pts);track.last=Some(track.last.map_or(pts,|p|p.max(pts)));
                    if track.marks.back().is_none_or(|m|pts-m.pts>=0.5){track.marks.push_back(Mark{pts,offset:self.at+consumed as u64});}
                    while track.marks.front().is_some_and(|m|m.offset<oldest)||track.marks.len()>200000{track.marks.pop_front();}
                }
            }
            consumed+=188;
        }
        self.pending.drain(..consumed);self.at+=consumed as u64;
    }
    pub fn range(&self,pid:u16)->Option<(f64,f64,f64)>{let t=self.tracks.get(&pid)?;Some((t.first?,t.marks.front()?.pts,t.last?))}
    pub fn seek(&self,pid:u16,relative:f64)->Option<(Mark,f64)>{
        let t=self.tracks.get(&pid)?;let first=t.first?;let target=(first+relative).clamp(t.marks.front()?.pts,t.last?);
        // Decode from before the target to reconstruct references and seek to the
        // requested timestamp, rather than starting presentation at an arbitrary GOP.
        let mark=t.marks.iter().rev().find(|m|m.pts<=target-3.).or_else(||t.marks.front())?;
        Some((*mark,target))
    }
}
pub fn timestamp(p:&[u8])->Option<i64>{
    if p.len()!=188||p[0]!=0x47||p[1]&0xc0!=0x40||p[3]&0xc0!=0||p[3]&0x10==0{return None;}
    let offset=if p[3]&0x20!=0{5+p[4] as usize}else{4};let b=p.get(offset..)?;
    if b.len()<14||b[..3]!=[0,0,1]||b[6]&0xc0!=0x80||b[7]&0x80==0||b[8]<5{return None;}
    let t=&b[9..14];if t[0]&1==0||t[2]&1==0||t[4]&1==0{return None;}
    Some((((t[0] as i64>>1)&7)<<30)|((t[1] as i64)<<22)|((t[2] as i64>>1)<<15)|((t[3] as i64)<<7)|(t[4] as i64>>1))
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn clock_wrap_preserves_reordered_frames(){let mut c=Clock::default();assert_eq!(c.ticks(WRAP-20),WRAP-20);assert_eq!(c.ticks(10),WRAP+10);assert_eq!(c.ticks(WRAP-10),WRAP-10);assert_eq!(c.ticks(20),WRAP+20);}
    #[test]fn malformed_transport_has_no_timestamp(){assert!(timestamp(&[]).is_none());assert!(timestamp(&[0x47;188]).is_none());}
}
