//! Bounded MPEG-TS PES assembly for the native player. Discontinuities discard
//! partial data rather than feeding a decoder bytes from different pictures.
#[derive(Default)]
pub struct Pes {
    data: Vec<u8>,
    cc: Option<u8>,
    started: bool,
}
#[derive(Debug)]
pub struct Packet { pub bytes: Vec<u8>, pub pts: Option<i64> }
impl Pes {
    fn take(&mut self)->Option<Packet>{
        let mut data=std::mem::take(&mut self.data);
        if data.len()<9||data[..3]!=[0,0,1]||data[6]&0xc0!=0x80{return None;}
        let end=9+data[8] as usize;
        if end>data.len(){return None;}
        let pts=if data[7]&0x80!=0&&end>=14{
            let p=&data[9..14];
            if p[0]&1==0||p[2]&1==0||p[4]&1==0{return None;}
            Some(((((p[0] as u64>>1)&7)<<30)|((p[1] as u64)<<22)|((p[2] as u64>>1)<<15)|((p[3] as u64)<<7)|(p[4] as u64>>1)) as i64)
        }else{None};
        let size=((data[4] as usize)<<8)|data[5] as usize;
        if size>0{if size+6>data.len(){return None;}data.truncate(size+6);}
        if end>data.len(){return None;}
        Some(Packet{bytes:data.split_off(end),pts:pts.map(|n|n*1000/9)})
    }
    pub fn push(&mut self,packet:&[u8])->Option<Packet>{
        if packet.len()!=188||packet[0]!=0x47{return None;}
        let control=(packet[3]>>4)&3;
        if packet[1]&0x80!=0||packet[3]&0xc0!=0||control==0{self.data.clear();self.started=false;self.cc=None;return None;}
        let offset=if control&2!=0{5+packet[4] as usize}else{4};
        if offset>188{self.data.clear();self.started=false;return None;}
        if control&2!=0&&packet[4]>0&&packet[5]&0x80!=0{self.cc=None;self.data.clear();self.started=false;}
        if control&1==0{return None;}
        let cc=packet[3]&15;
        if self.cc==Some(cc){return None;}
        if self.cc.is_some_and(|last|(last+1)&15!=cc){self.data.clear();self.started=false;}
        self.cc=Some(cc);
        let result=if packet[1]&0x40!=0 {let result=self.take();self.started=true;result}else{None};
        if self.started {
            if self.data.len()+188>4*1024*1024{self.data.clear();self.started=false;}
            else{self.data.extend_from_slice(&packet[offset..]);}
        }
        result
    }
    pub fn finish(&mut self)->Option<Packet>{self.take()}
}

#[cfg(test)]mod tests{
    use super::*;
    fn packet(cc:u8,start:bool,data:&[u8])->[u8;188]{
        assert!(data.len()<=182);let mut p=[0xff;188];p[0]=0x47;p[1]=if start{0x40}else{0};p[2]=100;p[3]=0x30|cc;p[4]=(183-data.len()) as u8;p[5]=0;
        p[188-data.len()..].copy_from_slice(data);p
    }
    fn pes()->Vec<u8>{vec![0,0,1,0xc0,0,10,0x80,0x80,5,0x21,0,5,0xbf,0x21,0x12,0x34]}
    #[test]fn assemble_split_pes_and_ignore_duplicate_transport(){
        let b=pes();let mut p=Pes::default();let first=packet(0,true,&b[..10]);
        assert!(p.push(&first).is_none());assert!(p.push(&first).is_none());assert!(p.push(&packet(1,false,&b[10..])).is_none());
        let out=p.push(&packet(2,true,&b)).unwrap();assert_eq!(out.bytes,vec![0x12,0x34]);assert_eq!(out.pts,Some(10_000_000));
    }
    #[test]fn missing_packet_and_truncated_finite_pes_are_discarded(){
        let b=pes();let mut p=Pes::default();p.push(&packet(0,true,&b[..10]));p.push(&packet(2,false,&b[10..]));assert!(p.finish().is_none());
        p.push(&packet(3,true,&b[..12]));assert!(p.finish().is_none());
        p.push(&packet(4,true,&b));assert_eq!(p.finish().unwrap().bytes,vec![0x12,0x34]);
    }
    #[test]fn transport_error_and_bad_pts_do_not_reach_decoder(){
        let mut b=pes();let mut p=Pes::default();let mut packet=packet(0,true,&b);packet[1]|=0x80;p.push(&packet);assert!(p.finish().is_none());
        b[11]&=!1;p.push(&super::tests::packet(1,true,&b));assert!(p.finish().is_none());
    }
}

/// Recover packet boundaries after USB startup bytes or a damaged chunk.
/// Require three sync bytes when acquiring lock, rather than treating a payload
/// byte equal to 0x47 as a packet header. Buffered memory stays below 50 KiB.
pub struct Packets<R>{input:R,bytes:Vec<u8>,at:usize,eof:bool,locked:bool}
impl<R:std::io::Read> Packets<R>{
    pub fn new(input:R)->Self{Self{input,bytes:Vec::new(),at:0,eof:false,locked:false}}
    pub fn next(&mut self)->std::io::Result<Option<[u8;188]>>{
        let mut skipped=0;
        loop{
            if self.bytes.len()-self.at<188*3&&!self.eof{
                self.bytes.drain(..self.at);self.at=0;
                let mut data=[0;188*256];let n=self.input.read(&mut data)?;
                self.eof=n==0;self.bytes.extend_from_slice(&data[..n]);
                if !self.eof{continue;}
            }
            let data=&self.bytes[self.at..];if data.len()<188{return Ok(None);}
            if data[0]!=0x47{self.locked=false;}
            if !self.locked{
                let valid=data[0]==0x47&&(data.len()<376&&self.eof||data.get(188)==Some(&0x47))&&(data.len()<564&&self.eof||data.get(376)==Some(&0x47));
                if !valid{self.at+=1;skipped+=1;if skipped>2*1024*1024{return Err(std::io::Error::new(std::io::ErrorKind::InvalidData,"No transport synchronization"));}continue;}
                self.locked=true;
            }
            let packet=data[..188].try_into().unwrap();self.at+=188;return Ok(Some(packet));
        }
    }
}
#[cfg(test)]mod framing_tests{
    use super::Packets;
    #[test]fn startup_noise_and_midstream_damage_do_not_end_playback(){
        let packet=|cc:u8|{let mut p=[0xff;188];p[..4].copy_from_slice(&[0x47,0,100,0x10|cc]);p};
        let mut data=vec![0;700];for i in 0..4{data.extend(packet(i));}
        data.extend([3u8,4,0x47,9,8]);for i in 4..8{data.extend(packet(i));}
        let mut r=Packets::new(&data[..]);for i in 0..8{assert_eq!(r.next().unwrap(),Some(packet(i)));}assert!(r.next().unwrap().is_none());
    }
}
