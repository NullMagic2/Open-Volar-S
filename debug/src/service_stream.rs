//! Shared service selection for Windows and Linux capture.
use std::collections::BTreeSet;
use std::io;
#[cfg(test)]
#[path="service_stream_tests.rs"]
mod tests;

/// Streaming single-service recorder shared by both operating systems. Buffer
/// only startup: after a complete decodable boundary, packets are copied intact.
pub(crate) struct SelectedRecording {
    program:u16,analyzer:Option<a865r::TsAnalyzer>,raw:Vec<u8>,
    filter:Option<ProgramStream>,pending:Vec<u8>,video:Option<(u16,u8)>,
    pmt:u16,ready:bool,await_pes:BTreeSet<u16>,await_latm:BTreeSet<u16>,
    completed_frame:bool,
    incomplete:std::collections::BTreeMap<u16,(Vec<u8>,Option<usize>)>,
}
impl SelectedRecording {
    pub(crate) fn new(program:u32)->io::Result<Self>{
        let program=u16::try_from(program).ok().filter(|p|*p!=0)
            .ok_or_else(||io::Error::new(io::ErrorKind::InvalidInput,"Select a valid TV service before recording"))?;
        Ok(Self{program,analyzer:Some(a865r::TsAnalyzer::new()),raw:Vec::new(),
            filter:None,pending:Vec::new(),video:None,pmt:0,ready:false,await_pes:BTreeSet::new(),await_latm:BTreeSet::new(),
            completed_frame:false,incomplete:Default::default()})
    }
    pub(crate) fn ready(&self)->bool{self.ready&&self.completed_frame}
    pub(crate) fn push(&mut self,data:&[u8])->io::Result<Vec<u8>>{
        const START_LIMIT:usize=24*1024*1024;
        let packets=if let Some(filter)=self.filter.as_mut(){filter.push(data)}else{
            self.raw.extend_from_slice(data);
            let analyzer=self.analyzer.as_mut().unwrap();analyzer.push(data);
            let stats=analyzer.stats();
            let video=stats.streams.iter().find(|s|s.program_number==self.program&&matches!(s.stream_type,1|2|0x1b|0x24));
            let info=stats.programs.get(&self.program);
            if let (Some(video),Some(info))=(video,info){
                self.video=Some((video.pid,video.stream_type));self.pmt=info.pmt_pid;
                self.await_pes=stats.streams.iter().filter(|s|s.program_number==self.program).map(|s|s.pid).collect();
                self.await_latm=stats.streams.iter().filter(|s|s.program_number==self.program&&s.stream_type==0x11).map(|s|s.pid).collect();
                let mut filter=ProgramStream::new(self.program,info.pmt_pid,info.pcr_pid,&stats);
                let packets=filter.push(&self.raw);
                self.filter=Some(filter);self.raw.clear();self.raw.shrink_to_fit();self.analyzer=None;
                packets
            }else{
                if self.raw.len()>START_LIMIT{return Err(io::Error::new(io::ErrorKind::InvalidData,format!("Selected TV service {} is absent or has no video",self.program)));}
                return Ok(Vec::new());
            }
        };
        if self.ready{let packets=self.complete_pes(&packets);return self.complete_frames(&packets);}
        self.pending.extend_from_slice(&packets);
        let (video,kind)=self.video.unwrap();
        if let Some(start)=recording_boundary(&self.pending,video,kind,self.pmt){
            let pending=std::mem::take(&mut self.pending);
            let mut out=Vec::new();
            // Keep the selected PAT and original PMT (including audio/caption
            // descriptors), but none of the incomplete pictures before the GOP.
            for packet in pending[..start].chunks_exact(188){
                let pid=packet_pid(packet);
                if pid==0||pid==self.pmt{out.extend_from_slice(packet);}
            }
            out.extend_from_slice(&self.complete_pes(&pending[start..]));
            self.ready=true;return self.complete_frames(&out);
        }
        if self.pending.len()>START_LIMIT{return Err(io::Error::new(io::ErrorKind::InvalidData,"No complete sequence headers/keyframe received for the selected TV service"));}
        Ok(Vec::new())
    }
    fn complete_pes(&mut self,packets:&[u8])->Vec<u8>{
        let mut out=Vec::with_capacity(packets.len());
        for packet in packets.chunks_exact(188){
            let pid=packet_pid(packet);
            if self.await_latm.contains(&pid){
                if packet[1]&0x40==0||packet[3]&0x10==0{continue;}
                let payload=if packet[3]&0x20!=0{5+packet[4] as usize}else{4};
                if payload+9>188{continue;}
                let audio=payload+9+packet[payload+8] as usize;
                // LATM useSameStreamMux=1 depends on an earlier configuration.
                // Begin each audio track at its own fresh StreamMuxConfig.
                if audio+4>188||packet[audio]!=0x56||packet[audio+1]&0xe0!=0xe0||packet[audio+3]&0x80!=0{continue;}
                self.await_latm.remove(&pid);
            }
            if self.await_pes.contains(&pid){
                if packet[1]&0x40==0{continue;}
                self.await_pes.remove(&pid);
            }
            out.extend_from_slice(packet);
        }
        out
    }
    fn complete_frames(&mut self,packets:&[u8])->io::Result<Vec<u8>>{
        let video=self.video.unwrap().0;let mut out=Vec::new();
        for packet in packets.chunks_exact(188){
            let pid=packet_pid(packet);let start=packet[1]&0x40!=0;
            let payload=if packet[3]&0x10==0{188}else if packet[3]&0x20!=0{5+packet[4] as usize}else{4};
            if payload+6<=188&&start&&packet[payload..payload+3]==[0,0,1]{
                let length=((packet[payload+4] as usize)<<8)|packet[payload+5] as usize;
                if let Some((mut previous,None))=self.incomplete.remove(&pid){
                    out.append(&mut previous);if pid==video{self.completed_frame=true;}
                }
                // Emit each PES atomically. A global cut through interleaved
                // audio/video would leave another track's PES half-written.
                // PTS/DTS and the packet order within each PID stay intact.
                self.incomplete.insert(pid,(Vec::new(),if length==0{None}else{Some(length+6)}));
            }
            if let Some((pending,remaining))=self.incomplete.get_mut(&pid){
                pending.extend_from_slice(packet);
                if let Some(remaining)=remaining.as_mut().filter(|_|payload<188){
                    *remaining=remaining.saturating_sub(188-payload);
                }
                if *remaining==Some(0){
                    let (mut complete,_)=self.incomplete.remove(&pid).unwrap();out.append(&mut complete);
                    if pid==video{self.completed_frame=true;}
                }
            }else{out.extend_from_slice(packet);}
            if self.incomplete.values().map(|(p,_)|p.len()).sum::<usize>()>24*1024*1024{
                return Err(io::Error::new(io::ErrorKind::InvalidData,"Selected service stopped producing complete video PES packets"));
            }
        }
        Ok(out)
    }
}
fn packet_pid(packet:&[u8])->u16{(u16::from(packet[1]&0x1f)<<8)|u16::from(packet[2])}
fn recording_boundary(data:&[u8],video:u16,kind:u8,pmt:u16)->Option<usize>{
    let (mut pat_seen,mut pmt_seen)=(false,false);
    let (mut pes,mut sequence)=(None,None);let mut pps=false;let mut tail=Vec::new();
    for (index,packet) in data.chunks_exact(188).enumerate(){
        let pid=packet_pid(packet);let offset=index*188;
        if pid==0{pat_seen=true;}if pid==pmt{pmt_seen=true;}
        if pid!=video{continue;}
        if packet[1]&0x40!=0{pes=Some(offset);tail.clear();}
        let adaptation=(packet[3]>>4)&3;if adaptation&1==0{continue;}
        let payload=if adaptation&2!=0{5+usize::from(packet[4])}else{4};
        if payload>=188{continue;}
        let mut probe=std::mem::take(&mut tail);probe.extend_from_slice(&packet[payload..]);
        for nal in probe.windows(4).filter(|v|v[..3]==[0,0,1]){
            if !pat_seen||!pmt_seen{continue;}
            match kind{
                0x1b=>match nal[3]&31{
                    7=>{sequence=pes;pps=false;},8 if sequence.is_some()=>pps=true,
                    5 if pps=>return sequence,_=>{}
                },
                1|2 if nal[3]==0xb3=>return pes,
                0x24=>match (nal[3]>>1)&63{32=>sequence=pes,19..=21 if sequence.is_some()=>return sequence,_=>{}},
                _=>{}
            }
        }
        tail=probe[probe.len().saturating_sub(3)..].to_vec();
    }
    None
}
// Start the player with the selected program tables and a fresh H.264 sequence
// header. A live USB read may begin in the middle of a GOP; feeding those
// incomplete reference frames produces the large mosaic blocks seen at startup.
pub(crate) fn clean_live_start(data: &[u8], video_pid: u16, pmt_pid: u16) -> Option<usize> {
    let alignment=aligned_live_start(data);
    let (mut pat,mut pmt,mut video_start)=(None,None,None);
    let mut tail=Vec::new();
    for offset in (alignment..data.len().saturating_sub(187)).step_by(188) {
        let packet=&data[offset..offset+188];
        if packet[0]!=0x47 { continue; }
        let pid=(u16::from(packet[1]&0x1f)<<8)|u16::from(packet[2]);
        if pid==0 {pat=Some(offset);}
        if pid==pmt_pid {pmt=Some(offset);}
        if pid!=video_pid {continue;}
        if packet[1]&0x40!=0 {video_start=Some(offset);tail.clear();}
        let adaptation=(packet[3]>>4)&3;
        if adaptation&1==0 {continue;}
        let payload=if adaptation&2!=0 {5+usize::from(packet[4])} else {4};
        if payload>=188 {continue;}
        let mut probe=std::mem::take(&mut tail);
        probe.extend_from_slice(&packet[payload..]);
        let sequence=probe.windows(4).any(|bytes| {
            bytes[0]==0 && bytes[1]==0 && bytes[2]==1 && bytes[3]&0x1f==7
        });
        tail=probe[probe.len().saturating_sub(3)..].to_vec();
        if sequence {
            if let (Some(table),Some(map),Some(pes))=(pat,pmt,video_start) {
                if table<pes && map<pes {return Some(table.min(map));}
            }
        }
    }
    None
}

pub(crate) fn aligned_live_start(data: &[u8]) -> usize {
    (0..data.len().min(1024)).find(|&i| {
        i+752<data.len() && (0..5).all(|packet|data[i+packet*188]==0x47)
    }).unwrap_or(0)
}

// Feed mpv only the selected service. Its automatic MPEG-TS program choice
// otherwise picks the first program in the broadcast PAT (often the mobile feed).
pub(crate) struct ProgramStream {
    allowed:BTreeSet<u16>, program:u16, pmt_pid:u16, carry:Vec<u8>, pat_counter:u8,pat_sent:bool,
}
impl ProgramStream {
    pub(crate) fn new(program:u16,pmt_pid:u16,pcr_pid:Option<u16>,streams:&a865r::ts::TsStats)->Self{
        let mut allowed=BTreeSet::from([pmt_pid]);
        if let Some(pid)=pcr_pid{allowed.insert(pid);}
        allowed.extend(streams.streams.iter().filter(|s|s.program_number==program).map(|s|s.pid));
        Self{allowed,program,pmt_pid,carry:Vec::new(),pat_counter:0,pat_sent:false}
    }
    fn pat(&mut self)->[u8;188]{
        let mut packet=[0xff;188];
        packet[..17].copy_from_slice(&[
            0x47,0x40,0x00,0x10|(self.pat_counter&15),0x00,
            0x00,0xb0,0x0d,0x00,0x01,0xc1,0x00,0x00,
            (self.program>>8) as u8,self.program as u8,
            0xe0|((self.pmt_pid>>8) as u8&0x1f),self.pmt_pid as u8,
        ]);
        let crc=mpeg_crc(&packet[5..17]).to_be_bytes();
        packet[17..21].copy_from_slice(&crc);
        self.pat_counter=(self.pat_counter+1)&15;
        packet
    }
    pub(crate) fn push(&mut self,data:&[u8])->Vec<u8>{
        self.carry.extend_from_slice(data);
        let mut out=Vec::with_capacity(data.len()/2+188);
        if !self.pat_sent{self.pat_sent=true;out.extend_from_slice(&self.pat());}
        let mut offset=0;
        while self.carry.len()-offset>=188 {
            if self.carry[offset]!=0x47 ||
                (self.carry.len()-offset>=376 && self.carry[offset+188]!=0x47){
                offset+=1;continue;
            }
            let packet=&self.carry[offset..offset+188];
            let pid=(u16::from(packet[1]&0x1f)<<8)|u16::from(packet[2]);
            if pid==0{out.extend_from_slice(&self.pat());}
            else if self.allowed.contains(&pid){out.extend_from_slice(packet);}
            offset+=188;
        }
        self.carry.drain(..offset);
        out
    }
}
fn mpeg_crc(bytes:&[u8])->u32{
    let mut crc=0xffff_ffffu32;
    for byte in bytes{
        crc^=u32::from(*byte)<<24;
        for _ in 0..8{crc=if crc&0x8000_0000!=0{(crc<<1)^0x04c1_1db7}else{crc<<1};}
    }
    crc
}
pub(crate) fn selected_program(requested:Option<u32>,stats:&a865r::ts::TsStats)->Option<u16>{
    match requested{
        Some(id)=>u16::try_from(id).ok().filter(|id|stats.programs.contains_key(id)),
        None=>stats.programs.keys().next().copied(),
    }
}
