//! Bounded ISDB caption PES extraction; reception never waits for caption rendering.
use std::collections::VecDeque;
#[derive(Clone,Debug)]pub struct Packet {pub pts_ms:i64,pub bytes:Vec<u8>}
#[derive(Default)]pub struct Stream {
 pub pid:Option<u16>,pub profile:u16,pub revision:u64,pub packets:VecDeque<Packet>,
 pub timeline:Option<i64>,clock_pid:Option<u16>,timestamp_offset:Option<i64>,
 pending:Vec<u8>,pes:Vec<u8>,cc:Option<u8>,queued_bytes:usize,
}
impl Stream {
 pub fn select(&mut self,stats:&a865r::TsStats,program:u16){
  self.clock_pid=stats.streams.iter().find(|s|s.program_number==program&&matches!(s.stream_type,1|2|0x1b|0x24)).map(|s|s.pid);
  let next=stats.streams.iter().find_map(|s|(s.program_number==program).then(||stats.caption_profiles.get(&s.pid).map(|p|(s.pid,*p))).flatten());
  if next.map(|p|p.0)!=self.pid || next.map(|p|p.1).unwrap_or(8)!=self.profile {self.pid=next.map(|p|p.0);self.profile=next.map(|p|p.1).unwrap_or(8);self.pes.clear();self.cc=None;self.packets.clear();self.queued_bytes=0;self.timestamp_offset=None;self.timeline=None;self.revision+=1;}
 }
 pub fn push(&mut self,bytes:&[u8]) {
  self.pending.extend_from_slice(bytes);let mut used=0;
  while self.pending.len()-used>=188 {
   if self.pending[used]!=0x47{used+=1;continue}
   let mut packet=[0u8;188];packet.copy_from_slice(&self.pending[used..used+188]);used+=188;self.packet(&packet);
  }
  self.pending.drain(..used);
 }
 fn packet(&mut self,p:&[u8;188]) {
  let pid=(((p[1]&31) as u16)<<8)|p[2] as u16;
  if Some(pid)==self.clock_pid&&p[1]&0xc0==0x40&&p[3]&0xd0==0x10 {let at=if p[3]&32!=0{5+p[4] as usize}else{4};if at+14<=188 {let q=&p[at..];if q[..3]==[0,0,1]&&q[7]&0x80!=0{let t=&q[9..14];self.timeline=Some((((t[0]&14) as i64)<<29|((t[1] as i64)<<22)|(((t[2]&254) as i64)<<14)|((t[3] as i64)<<7)|((t[4] as i64)>>1))/90);}}}
  if Some((((p[1]&31) as u16)<<8)|p[2] as u16)!=self.pid{return}
  if p[1]&0x80!=0||p[3]&0xc0!=0 {self.pes.clear();self.cc=None;return}
  let mode=(p[3]>>4)&3;if mode==0{return}let mut offset=4;
  if mode&2!=0 {offset=5+p[4] as usize;if offset>188{self.pes.clear();self.cc=None;return}if p[4]>0&&p[5]&0x80!=0{self.pes.clear();self.cc=None;}}
  if mode&1==0||offset>=188{return}let cc=p[3]&15;
  if self.cc==Some(cc){return}if self.cc.is_some_and(|old|(old+1)&15!=cc){self.pes.clear();}
  self.cc=Some(cc);let start=p[1]&0x40!=0;
  if start {self.finish();self.pes.clear();}else if self.pes.is_empty(){return}
  if self.pes.len()+188-offset>65_542{self.pes.clear();return}
  self.pes.extend_from_slice(&p[offset..]);
  if self.pes.len()>=6 {let n=u16::from_be_bytes([self.pes[4],self.pes[5]]) as usize;if n>0&&self.pes.len()>=n+6 {self.pes.truncate(n+6);self.finish();self.pes.clear();}}
 }
 fn finish(&mut self){
  let p=&self.pes;if p.len()<14||p[..3]!=[0,0,1]||p[6]&0xc0!=0x80||p[7]&0x80==0||p[8]<5{return}
  let declared=u16::from_be_bytes([p[4],p[5]]) as usize;if declared!=0&&p.len()!=declared+6{return}
  let end=9+p[8] as usize;if end>=p.len(){return}let t=&p[9..14];if t[0]&1==0||t[2]&1==0||t[4]&1==0{return}
  let pts=(((t[0]&14) as u64)<<29)|((t[1] as u64)<<22)|(((t[2]&254) as u64)<<14)|((t[3] as u64)<<7)|((t[4] as u64)>>1);
  // Some Brazilian multiplexes retain an unrelated encoder clock in caption
  // PES. Preserve valid timestamps; align only gross clock discontinuities.
  let raw=(pts/90) as i64;let mut pts_ms=raw+self.timestamp_offset.unwrap_or(0);
  if let Some(video)=self.timeline {const WRAP:i64=(1i64<<33)/90;pts_ms+=((video-pts_ms+WRAP/2).div_euclid(WRAP))*WRAP;
   if (pts_ms-video).abs()>60_000 {self.timestamp_offset=Some(video-raw);pts_ms=video;}
  }
  let packet=Packet{pts_ms,bytes:p[end..].to_vec()};self.queued_bytes+=packet.bytes.len();self.packets.push_back(packet);
  while self.packets.len()>4096||self.queued_bytes>8*1024*1024{if let Some(p)=self.packets.pop_front(){self.queued_bytes-=p.bytes.len();self.revision+=1;}}
 }
 pub fn drain(&mut self)->Vec<Packet>{self.queued_bytes=0;self.packets.drain(..).collect()}
}
#[cfg(test)]mod tests {use super::*;
 fn ts(cc:u8,start:bool,data:&[u8])->Vec<u8>{let mut p=vec![0xff;188];p[0]=0x47;p[1]=if start{0x40}else{0};p[2]=42;p[3]=0x30|cc;p[4]=(183-data.len()) as u8;if p[4]>0{p[5]=0;}let offset=5+p[4] as usize;p[offset..].copy_from_slice(data);p}
 #[test]fn unrelated_caption_clock_is_aligned_but_valid_timestamps_survive(){
  let p=[0,0,1,0xbd,0,11,0x80,0x80,5,0x21,0,5,0xbf,0x21,0x80,0xff,0x00];
  let mut s=Stream::default();s.pid=Some(42);s.timeline=Some(100_000);s.push(&ts(0,true,&p));assert_eq!(s.drain()[0].pts_ms,100_000);s.timeline=Some(100_020);s.push(&ts(1,true,&p));assert_eq!(s.drain()[0].pts_ms,100_000,"Keep a stable encoder offset");
  let mut s=Stream::default();s.pid=Some(42);s.timeline=Some(1200);s.push(&ts(0,true,&p));assert_eq!(s.drain()[0].pts_ms,1000);
 }
 #[test]fn reconstructs_pts_and_rejects_broken_continuity(){let mut s=Stream::default();s.pid=Some(42);let p=[0,0,1,0xbd,0,11,0x80,0x80,5,0x21,0,5,0xbf,0x21,0x80,0xff,0x00];let data=ts(0,true,&p[..10]);for chunk in data.chunks(17){s.push(chunk);}s.push(&ts(1,false,&p[10..]));assert_eq!(s.packets.len(),1);assert_eq!(s.packets[0].pts_ms,1000);assert_eq!(s.packets[0].bytes,[0x80,0xff,0]);s.drain();s.push(&ts(2,true,&p[..10]));s.push(&ts(4,false,&p[10..]));assert!(s.packets.is_empty());}
}
