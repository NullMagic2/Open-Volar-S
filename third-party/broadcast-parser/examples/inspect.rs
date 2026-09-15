//! CPU-only H264 parser and reference-sequence check. Never creates a GPU device.
use broadcast_parser::{Event, Parser};
use std::collections::HashSet;
fn main() -> Result<(), String> {
    let path = std::env::args()
        .nth(1)
        .ok_or("Provide an Annex-B H264 file")?;
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut parser = Parser::new()?;
    let (mut pictures, mut fields, mut displayed, mut idrs, mut max_refs, mut missing) =
        (0, 0, 0, 0, 0, 0);
    let mut started = false;
    let mut independent_starts=0;
    let mut decoded = HashSet::new();
    let mut last_pts = None;
    let mut regressions = 0;
    for (bytes, eos) in data
        .chunks(65536)
        .map(|b| (b, false))
        .chain(std::iter::once((&[][..], true)))
    {
        parser.parse(bytes, None, eos, |event| {
            match event {
                Event::Picture(p) => {
                    pictures += 1;
                    if pictures <= 3 { println!("picture={pictures} id={} idr={} intra={} reference={} fields={}/{}/{} frame_num={} poc={:?} refs={} size={} slices={:?} prefix={:02x?} sps={:?} pps={:?}",p.id,p.idr,p.intra,p.reference,p.field,p.bottom,p.second,p.frame_num,p.poc,p.reference_count,p.bytes.len(),p.slices,&p.bytes[..p.bytes.len().min(12)],p.sps,p.pps); }
                    fields += p.field;
                    idrs += p.idr;
                    if p.idr!=0 {println!("IDR picture={} idr_pic_id={}",pictures,p.idr_pic_id);}
                    if (!started && broadcast_parser::independent_start(p.idr!=0,p.intra!=0,p.second!=0,p.reference_count as usize)) || (p.idr!=0 && p.second==0) {
                        independent_starts+=1;
                        started = true;
                        decoded.clear();
                    }
                    let refs = &p.references[..p.reference_count as usize];
                    max_refs = max_refs.max(refs.len());
                    if started {
                        for r in refs {
                            if !decoded.contains(&r.id) {
                                missing += 1;
                            }
                        }
                        decoded.retain(|id| {
                            refs.iter().any(|r| r.id == *id) || p.second != 0 && p.id == *id
                        });
                        decoded.insert(p.id);
                    }
                }
                Event::Display { pts, .. } => {
                    displayed += 1;
                    if last_pts.is_some_and(|last| pts < last) {
                        regressions += 1;
                    }
                    last_pts = Some(pts);
                }
            }
            Ok(())
        })?;
    }
    println!(
        "pictures={pictures} fields={fields} displayed={displayed} idrs={idrs} max_reference_slots={max_refs} independent_starts={independent_starts} missing_references_after_start={missing} timestamp_regressions={regressions}"
    );
    if displayed == 0 || independent_starts == 0 || missing != 0 || regressions != 0 {
        return Err("Parser regression check failed".into());
    }
    Ok(())
}
