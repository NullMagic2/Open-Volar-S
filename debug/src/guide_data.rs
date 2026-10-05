//! Broadcast guide data shared by the Windows and Linux interfaces.
use serde_json::{json,Value};
use std::path::PathBuf;
pub fn key(v:&Value)->(u64,u64,u64,u64,u64) { (number(v,"frequency_khz"),number(v,"network_id"),number(v,"transport_id"),number(v,"program_id"),number(v,"event_id")) }
fn number(v:&Value,k:&str)->u64 {v[k].as_u64().unwrap_or(0)}
pub fn events(stats:&a865r::TsStats,frequency:u32)->Value {
    json!(stats.events.values().filter(|e|stats.streams.iter().any(|s|s.program_number==e.service_id&&matches!(s.stream_type,1|2|0x1b|0x24))).map(|e|json!({
        "frequency_khz":frequency,"program_id":e.service_id,"transport_id":e.transport_id,"network_id":e.network_id,
        "event_id":e.event_id,"start":e.start,"duration":e.duration,"name":e.name,"description":e.description,
        "channel_name":stats.service_names.get(&e.service_id),"channel_number":stats.channel_numbers.get(&e.service_id),
        "minimum_age":e.minimum_age,"running":e.running,"following":e.following,"language":e.language
    })).collect::<Vec<_>>())
}
/// Title of the program on now, shared by the Windows and Linux interfaces.
pub fn current_title(events:&Value,frequency:u32,program:u32)->Option<String>{
    events.as_array()?.iter().filter(|e|e["frequency_khz"].as_u64()==Some(frequency as u64)
        && e["program_id"].as_u64()==Some(program as u64) && e["running"]==true && e["following"]!=true
        && e["name"].as_str().is_some_and(|s|!s.trim().is_empty()))
        .max_by_key(|e|e["start"].as_i64().unwrap_or(0))
        .and_then(|e|e["name"].as_str()).map(|s|s.split_whitespace().collect::<Vec<_>>().join(" "))
}
/// Present/following status describes the moment it was received; a saved
/// guide must not keep showing an old program as on now or next.
pub fn clear_status(events:&mut [Value]) {
    for e in events.iter_mut().filter_map(Value::as_object_mut) {e.insert("running".into(),json!(false));e.insert("following".into(),json!(false));}
}
/// Merge received events into a guide. The latest program announced as on now
/// (or next) for a service supersedes older ones of that service, so stale
/// status from earlier reception cannot persist. Shared by Windows and Linux.
pub fn merge(events:&mut Vec<Value>,incoming:&[Value])->bool {
    let service=|v:&Value|(number(v,"frequency_khz"),number(v,"program_id"));
    let now=|v:&Value|v["running"]==true&&v["following"]!=true;
    let next=|v:&Value|v["following"]==true;
    let mut winners=Vec::new();
    for (flag,has) in [("running",&now as &dyn Fn(&Value)->bool),("following",&next)] {
        let mut latest=std::collections::HashMap::new();
        for e in incoming.iter().filter(|e|e.is_object()&&has(e)) {
            let start=e["start"].as_i64().unwrap_or(0);
            let entry=latest.entry(service(e)).or_insert((start,key(e)));
            if start>entry.0 {*entry=(start,key(e));}
        }
        winners.push((flag,latest));
    }
    // Normalize before comparing, so repeated identical reception is not a change.
    let normalize=|mut e:Value|{
        for (flag,latest) in &winners {
            if let Some((_,winner))=latest.get(&service(&e)) {if e[*flag]==true&&key(&e)!=*winner {e[*flag]=json!(false);}}
        }e
    };
    let mut changed=false;
    for event in incoming.iter().filter(|e|e.is_object()).cloned().map(&normalize) {
        if let Some(old)=events.iter_mut().find(|o|key(o)==key(&event)){if *old!=event{*old=event;changed=true;}}
        else{events.push(event);changed=true;}
    }
    for e in events.iter_mut() {
        let normalized=normalize(e.clone());
        if *e!=normalized {*e=normalized;changed=true;}
    }
    if changed {
        events.sort_by_key(|v|v["start"].as_i64().unwrap_or(0));
        if events.len()>8192 {events.drain(..events.len()-8192);}
    }
    changed
}
pub fn station(v:&Value)->String {format!("{} – {}",v["channel_number"].as_u64().map(|v|v.to_string()).unwrap_or_else(||"—".into()),v["channel_name"].as_str().unwrap_or("TV"))}
pub fn date(seconds:i64)->String {
    let days=seconds.div_euclid(86400);let time=seconds.rem_euclid(86400);let z=days+719468;
    let era=z.div_euclid(146097);let doe=z-era*146097;let yoe=(doe-doe/1460+doe/36524-doe/146096)/365;
    let doy=doe-(365*yoe+yoe/4-yoe/100);let mp=(5*doy+2)/153;let day=doy-(153*mp+2)/5+1;let month=mp+if mp<10{3}else{-9};
    format!("{day:02}/{month:02} {:02}:{:02}",time/3600,time/60%60)
}
pub fn range(v:&Value)->String {let start=v["start"].as_i64().unwrap_or(0);format!("{} – {}",date(start),date(start.saturating_add(v["duration"].as_i64().unwrap_or(0))))}
/// Same layout as the Windows guide's detail pane, plus the age rating when broadcast.
pub fn detail(v:&Value,untitled:&str,undescribed:&str)->String {
    let name=v["name"].as_str().filter(|n|!n.trim().is_empty()).unwrap_or(untitled);
    let description=v["description"].as_str().filter(|d|!d.trim().is_empty()).unwrap_or(undescribed);
    let age=v["minimum_age"].as_u64().map(|a|format!("\n{a}+")).unwrap_or_default();
    format!("{name}\n{}\n{}{age}\n\n{description}",station(v),range(v))
}
pub struct Cache {pub events:Vec<Value>,pub revision:u64,path:PathBuf,last:Value}
impl Cache {
    pub fn load(path:PathBuf)->Self {
        let mut events:Vec<Value>=std::fs::read(&path).ok().filter(|b|b.len()<=16*1024*1024).and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default();
        events.retain(Value::is_object);clear_status(&mut events);events.sort_by_key(|v|v["start"].as_i64().unwrap_or(0));
        if events.len()>8192 {events.drain(..events.len()-8192);}
        Self{events,revision:0,path,last:Value::Null}
    }
    pub fn ingest(&mut self,events:&Value)->bool {
        let Some(list)=events.as_array().filter(|l|!l.is_empty()) else{return false};
        if self.last==*events{return false}self.last=events.clone();
        let changed=merge(&mut self.events,list);
        if changed {
            self.revision=self.revision.wrapping_add(1);
            let tmp=self.path.with_extension("epg.tmp");
            if let Some(parent)=self.path.parent(){let _=std::fs::create_dir_all(parent);}
            if let Ok(bytes)=serde_json::to_vec(&self.events){if std::fs::write(&tmp,bytes).is_ok(){let _=std::fs::rename(&tmp,&self.path);}}
        }changed
    }
    pub fn current_title(&self,frequency:u32,program:u32)->Option<String>{current_title(&json!(self.events),frequency,program)}
}
#[cfg(test)]mod tests {use super::*;
 #[test]fn conflicting_status_in_reception_settles_instead_of_changing_every_update(){
  // The same multiplex snapshot arriving repeatedly must not count as a guide change.
  let incoming=[json!({"frequency_khz":1,"program_id":2,"event_id":1,"start":10,"name":"Earlier","running":true}),
   json!({"frequency_khz":1,"program_id":2,"event_id":2,"start":20,"name":"Later","running":true})];
  let mut events=Vec::new();assert!(merge(&mut events,&incoming));
  for _ in 0..3 {assert!(!merge(&mut events,&incoming));}
  assert_eq!(current_title(&json!(events),1,2).as_deref(),Some("Later"));
 }
 #[test]fn newer_present_and_following_supersede_stale_status(){
  let mut events=vec![json!({"frequency_khz":1,"program_id":2,"event_id":1,"start":10,"name":"Night show","running":true}),
   json!({"frequency_khz":1,"program_id":2,"event_id":2,"start":20,"name":"Old next","following":true}),
   json!({"frequency_khz":1,"program_id":3,"event_id":9,"start":10,"name":"Other service","running":true})];
  assert!(merge(&mut events,&[json!({"frequency_khz":1,"program_id":2,"event_id":5,"start":500,"name":"Afternoon show","running":true}),
   json!({"frequency_khz":1,"program_id":2,"event_id":6,"start":600,"name":"Evening show","running":false,"following":true})]));
  assert_eq!(current_title(&json!(events),1,2).as_deref(),Some("Afternoon show"));
  let flagged=|flag:&str|events.iter().filter(|e|e[flag]==true).map(|e|e["name"].as_str().unwrap()).collect::<Vec<_>>();
  assert_eq!(flagged("running"),["Other service","Afternoon show"]);assert_eq!(flagged("following"),["Evening show"]);
  let mut saved=events.clone();clear_status(&mut saved);assert!(saved.iter().all(|e|e["running"]==false&&e["following"]==false));
 }
 #[test]fn current_title_ignores_next_untitled_and_other_services(){let events=json!([
  {"frequency_khz":1,"program_id":2,"name":"Old","running":true,"start":1},
  {"frequency_khz":1,"program_id":2,"name":"  Current\nshow ","running":true,"start":2},
  {"frequency_khz":1,"program_id":2,"name":" ","running":true,"start":3},
  {"frequency_khz":1,"program_id":2,"name":"Next","running":true,"following":true,"start":4},
  {"frequency_khz":9,"program_id":2,"name":"Other multiplex","running":true,"start":5}]);
  assert_eq!(current_title(&events,1,2).as_deref(),Some("Current show"));assert_eq!(current_title(&events,1,3),None);assert_eq!(current_title(&Value::Null,1,2),None);}
 #[test]fn updates_survive_restart_and_do_not_merge_different_multiplexes(){let p=std::env::temp_dir().join(format!("ovs-guide-{}.json",std::process::id()));let _=std::fs::remove_file(&p);let mut c=Cache::load(p.clone());let mut a=json!({"frequency_khz":1,"program_id":2,"event_id":3,"start":100,"name":"First","running":true});assert!(c.ingest(&json!([a])));a["frequency_khz"]=json!(2);assert!(c.ingest(&json!([a])));a["name"]=json!("Updated");assert!(c.ingest(&json!([a])));assert_eq!(c.events.len(),2);assert!(!c.ingest(&json!([])));let c=Cache::load(p.clone());assert_eq!(c.events[1]["name"],"Updated");assert_eq!(c.current_title(2,2),None,"saved status is not current after a restart");assert!(detail(&c.events[1],"","").starts_with("Updated\n"));let _=std::fs::remove_file(p);}
}
