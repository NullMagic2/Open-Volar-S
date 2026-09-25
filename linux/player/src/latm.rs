//! Bounded LOAS/LATM framing for the AAC broadcast transport (ISO/IEC 14496-3).
//! Each elementary PID has independent configuration and partial-frame state.
#[derive(Default)]
pub struct Latm { pending:Vec<u8>,config:Vec<u8>,subframes:usize }
struct Bits<'a>{data:&'a [u8],at:usize}
impl Bits<'_>{
    fn read(&mut self,n:usize)->Result<u32,String>{
        if n>32||self.at+n>self.data.len()*8{return Err("Truncated LATM data".into());}
        let mut v=0;for _ in 0..n{v=(v<<1)|((self.data[self.at/8]>>(7-self.at%8))&1) as u32;self.at+=1;}Ok(v)
    }
    fn skip(&mut self,n:usize)->Result<(),String>{if n>self.data.len()*8-self.at{return Err("Truncated LATM data".into());}self.at+=n;Ok(())}
    fn bytes(&mut self,n:usize)->Result<Vec<u8>,String>{if n>8192{return Err("Oversized LATM payload".into());}(0..n).map(|_|self.read(8).map(|n|n as u8)).collect()}
    fn value(&mut self)->Result<usize,String>{let n=self.read(2)? as usize+1;Ok(self.read(n*8)? as usize)}
    fn frequency(&mut self)->Result<(),String>{if self.read(4)?==15{self.read(24)?;}Ok(())}
    fn object(&mut self)->Result<u32,String>{let n=self.read(5)?;if n==31{Ok(32+self.read(6)?)}else{Ok(n)}}
    fn asc(&mut self)->Result<(),String>{
        let mut object=self.object()?;self.frequency()?;let channels=self.read(4)?;
        if object==5||object==29{self.frequency()?;object=self.object()?;if object==22{self.read(4)?;}}
        if !matches!(object,1|2|3|4|6|7|17|19|20|21|22|23){return Err(format!("Unsupported LATM AAC object {object}"));}
        self.read(1)?;if self.read(1)?!=0{self.read(14)?;}let extension=self.read(1)?;
        if channels==0{return Err("LATM program-config elements are not supported".into());}
        if object==6||object==20{self.read(3)?;}
        if extension!=0{
            if object==22{self.read(16)?;}
            if matches!(object,17|19|20|23){self.read(3)?;}
            if self.read(1)?!=0{return Err("Unsupported LATM extension".into());}
        }
        if matches!(object,17|19|20|21|22|23)&&self.read(2)?!=0{return Err("Unsupported AAC error protection".into());}
        Ok(())
    }
}
impl Latm{
    pub fn push(&mut self,bytes:&[u8])->Vec<Result<(Vec<u8>,Vec<u8>),String>>{
        if self.pending.len()+bytes.len()>4*1024*1024{self.pending.clear();return vec![Err("Oversized LATM input".into())];}
        self.pending.extend_from_slice(bytes);let mut at=0;let mut elements=Vec::new();
        while at+3<=self.pending.len(){
            let b=&self.pending[at..];if b[0]!=0x56||b[1]&0xe0!=0xe0{at+=1;continue;}
            let n=((b[1] as usize&31)<<8)|b[2] as usize;
            if at+3+n>self.pending.len(){break;}
            elements.push(b[3..3+n].to_vec());at+=3+n;
        }
        self.pending.drain(..at);let mut frames=Vec::new();
        for e in elements{match self.element(&e){Ok(f)=>frames.extend(f.into_iter().map(Ok)),Err(e)=>{self.config.clear();frames.push(Err(e));}}}frames
    }
    fn element(&mut self,data:&[u8])->Result<Vec<(Vec<u8>,Vec<u8>)>,String>{
        let mut b=Bits{data,at:0};
        if b.read(1)?==0{
            let version=b.read(1)?;
            if version!=0 {if b.read(1)?!=0{return Err("Unsupported LATM mux version".into());}b.value()?;}
            if b.read(1)?==0{return Err("Unsupported asynchronous LATM multiplex".into());}
            let subframes=b.read(6)? as usize+1;
            if b.read(4)?!=0||b.read(3)?!=0{return Err("Unsupported LATM program/layer multiplex".into());}
            let explicit=if version!=0{Some(b.value()?)}else{None};let start=b.at;
            if let Some(n)=explicit{b.skip(n)?;}else{b.asc()?;}
            let bits=b.at-start;
            if bits==0||bits>2048{return Err("Invalid AAC configuration length".into());}
            let mut config=vec![0u8;bits.div_ceil(8)];
            for i in 0..bits{config[i/8]|=((data[(start+i)/8]>>(7-(start+i)%8))&1)<<(7-i%8);}
            if b.read(3)?!=0{return Err("Unsupported LATM frame length type".into());}
            b.read(8)?; // Buffer fullness, does not affect access unit framing.
            if b.read(1)?!=0{
                if version!=0{b.value()?;}else{loop{let more=b.read(1)?;b.read(8)?;if more==0{break;}}}
            }
            if b.read(1)?!=0{b.read(8)?;}
            self.config=config;self.subframes=subframes;
        }
        // Tuning joins a running multiplex. Wait for the next configuration block.
        if self.config.is_empty(){return Ok(Vec::new());}
        let mut frames=Vec::new();
        for _ in 0..self.subframes{
            let mut n=0;loop{let size=b.read(8)? as usize;n+=size;if n>8192{return Err("Oversized LATM access unit".into());}if size<255{break;}}
            let bytes=b.bytes(n)?;if !bytes.is_empty(){frames.push((self.config.clone(),bytes));}
        }
        Ok(frames)
    }
}
#[cfg(test)]mod tests{
    use super::*;
    fn element(config:bool)->Vec<u8>{
        let mut bits=Vec::new();let mut put=|n:u32,len:usize|{for i in (0..len).rev(){bits.push(((n>>i)&1) as u8);}};
        put(if config{0}else{1},1);
        if config{put(0,1);put(1,1);put(0,6);put(0,4);put(0,3);put(0x1190,16);put(0,3);put(255,8);put(0,1);put(0,1);}
        put(2,8);put(0x1234,16);
        let mut b=vec![0;bits.len().div_ceil(8)];for (i,bit) in bits.iter().enumerate(){b[i/8]|=bit<<(7-i%8);}
        let mut loas=vec![0x56,0xe0|((b.len()>>8) as u8),b.len() as u8];loas.extend(b);loas
    }
    #[test]fn split_configuration_and_repeated_payload(){
        let mut p=Latm::default();let e=element(true);
        assert!(p.push(&e[..4]).is_empty());
        assert_eq!(p.push(&e[4..]),vec![Ok((vec![0x11,0x90],vec![0x12,0x34]))]);
        assert_eq!(p.push(&element(false)),vec![Ok((vec![0x11,0x90],vec![0x12,0x34]))]);
    }
    #[test]fn wait_for_configuration_but_reject_truncation(){
        let mut p=Latm::default();assert!(p.push(&element(false)).is_empty());
        assert!(p.push(&element(true))[0].is_ok());
        assert!(p.element(&[0]).is_err());assert!(p.element(&[]).is_err());
    }
}
