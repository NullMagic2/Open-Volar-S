use super::*;
fn packet(pid:u16,payload:&[u8])->Vec<u8>{
    let mut p=vec![0xff;188];p[..4].copy_from_slice(&[0x47,0x40|((pid>>8) as u8),pid as u8,0x10]);
    p[4..4+payload.len()].copy_from_slice(payload);p
}
fn table(pid:u16,mut section:Vec<u8>)->Vec<u8>{
    section.extend_from_slice(&mpeg_crc(&section).to_be_bytes());
    let mut payload=vec![0];payload.extend(section);packet(pid,&payload)
}
fn pmt(program:u16,pid:u16,video:u16)->Vec<u8>{
    let mut section=vec![2,0xb0,28,(program>>8) as u8,program as u8,0xc1,0,0,
        0xe0|((video+3)>>8) as u8,(video+3) as u8,0xf0,0];
    for (kind,pid) in [(0x1b,video),(0x11,video+1),(6,video+2)]{
        section.extend_from_slice(&[kind,0xe0|(pid>>8) as u8,pid as u8,0xf0,0]);
    }
    table(pid,section)
}
fn multiplex()->Vec<u8>{
    let mut data=table(0,vec![0,0xb0,17,0,1,0xc1,0,0,0x42,0x58,0xf0,0,0x42,0x40,0xf0,0x10]);
    for (program,pmt_pid,video) in [(16984,0x1000,0x1001),(16960,0x1010,0x1011)]{
        data.extend(pmt(program,pmt_pid,video));
        // An incomplete reference picture before the new sequence is discarded.
        data.extend(packet(video,&[0,0,1,0xe0,0,0,0,0,1,0x41,0x99]));
        data.extend(packet(video,&[0,0,1,0xe0,0,0,0,0,1,0x67,0x11,0,0,1,0x68,0x22,0,0,1,0x65,0x33]));
        data.extend(packet(video+1,&[0,0,1,0xc0,0,7,0x80,0,0,0x56,0xe0,1,0x20]));
        data.extend(packet(video+2,&[0,0,1,0xbd,0,1,0x88]));
        data.extend(packet(video+3,&[0xaa]));
        data.extend(packet(video,&[0,0,1,0xe0,0,0,0,0,1,0x41,0x44]));
    }
    data
}
#[test]fn incomplete_secondary_audio_is_not_committed_at_stop(){
    let mut r=SelectedRecording::new(16960).unwrap();
    let mut out=r.push(&multiplex()).unwrap();
    let partial=packet(0x1012,&[0,0,1,0xc0,0x04,0,0x77]);
    out.extend(r.push(&partial).unwrap());
    out.extend(r.push(&packet(0x1011,&[0,0,1,0xe0,0,0,0,0,1,0x41,0x66])).unwrap());
    assert!(!out.chunks_exact(188).any(|p|p==partial));
    assert!(r.ready());
}
#[test]fn audio_crossing_video_boundary_is_committed_as_a_whole(){
    let mut r=SelectedRecording::new(16960).unwrap();r.push(&multiplex()).unwrap();
    let first=packet(0x1012,&[0,0,1,0xc0,1,44,0x77]);
    assert!(r.push(&first).unwrap().is_empty());
    let out=r.push(&packet(0x1011,&[0,0,1,0xe0,0,0,0,0,1,0x41,0x66])).unwrap();
    assert!(!out.chunks_exact(188).any(|p|packet_pid(p)==0x1012));
    let mut last=packet(0x1012,&[0x88]);last[1]&=!0x40;
    let out=r.push(&last).unwrap();
    assert_eq!(out,[first,last].concat());
}
#[test]fn recording_contains_only_explicit_service_with_audio_captions_and_pcr(){
    let data=multiplex();
    for (program,video,pmt_pid) in [(16960,0x1011,0x1010),(16984,0x1001,0x1000)]{
        let mut recorder=SelectedRecording::new(program).unwrap();let mut out=Vec::new();
        for chunk in data.chunks(101){out.extend(recorder.push(chunk).unwrap());}
        assert!(recorder.ready());assert_eq!(out.len()%188,0);
        let mut analyzer=a865r::TsAnalyzer::new();analyzer.push(&out);let stats=analyzer.stats();
        assert_eq!(stats.programs.keys().copied().collect::<Vec<_>>(),[program as u16]);
        assert_eq!(stats.psi_crc_errors,0);
        let pids:BTreeSet<_>=out.chunks_exact(188).map(packet_pid).collect();
        assert_eq!(pids,BTreeSet::from([0,pmt_pid,video,video+1,video+2,video+3]));
        assert_eq!(out.chunks_exact(188).filter(|p|packet_pid(p)==video).count(),1,"Partial leading picture leaked into recording");
        assert!(out.chunks_exact(188).any(|p|p==&pmt(program as u16,pmt_pid,video)[..]),"Original PMT/descriptors must be preserved");
    }
}
#[test]fn missing_service_never_falls_back_to_first_or_mobile(){
    let mut recording=SelectedRecording::new(123).unwrap();
    assert!(recording.push(&multiplex()).unwrap().is_empty());assert!(!recording.ready());
    assert!(SelectedRecording::new(0).is_err());assert!(SelectedRecording::new(65536).is_err());
}
#[test]fn existing_recording_is_never_truncated(){
    let path=std::env::temp_dir().join(format!("ovs-protected-recording-{}.ts",std::process::id()));
    std::fs::write(&path,b"keep this recording").unwrap();
    let control=crate::playback::Control::default();
    assert_eq!(control.start_selected_recording(&path,16960).unwrap_err().kind(),io::ErrorKind::AlreadyExists);
    assert_eq!(std::fs::read(&path).unwrap(),b"keep this recording");std::fs::remove_file(path).unwrap();
}
