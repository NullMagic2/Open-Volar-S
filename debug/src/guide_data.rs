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
pub fn station(v:&Value)->String {format!("{} – {}",v["channel_number"].as_u64().map(|v|v.to_string()).unwrap_or_else(||"—".into()),v["channel_name"].as_str().unwrap_or("TV"))}
pub fn date(seconds:i64)->String {
    let days=seconds.div_euclid(86400);let time=seconds.rem_euclid(86400);let z=days+719468;
    let era=z.div_euclid(146097);let doe=z-era*146097;let yoe=(doe-doe/1460+doe/36524-doe/146096)/365;
    let doy=doe-(365*yoe+yoe/4-yoe/100);let mp=(5*doy+2)/153;let day=doy-(153*mp+2)/5+1;let month=mp+if mp<10{3}else{-9};
    format!("{day:02}/{month:02} {:02}:{:02}",time/3600,time/60%60)
}
pub fn range(v:&Value)->String {let start=v["start"].as_i64().unwrap_or(0);format!("{} – {}",date(start),date(start.saturating_add(v["duration"].as_i64().unwrap_or(0))))}
pub fn detail(v:&Value)->String {format!("{}\n{}\n{}\n{}\n\n{}",station(v),range(v),v["name"].as_str().unwrap_or(""),v["minimum_age"].as_u64().map(|v|format!("{v}+")).unwrap_or_default(),v["description"].as_str().unwrap_or(""))}
pub struct Cache {pub events:Vec<Value>,pub revision:u64,path:PathBuf,last:Value}
impl Cache {
    pub fn load(path:PathBuf)->Self {
        let mut events:Vec<Value>=std::fs::read(&path).ok().filter(|b|b.len()<=16*1024*1024).and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default();
        events.retain(Value::is_object);events.sort_by_key(|v|v["start"].as_i64().unwrap_or(0));
        if events.len()>8192 {events.drain(..events.len()-8192);}
        Self{events,revision:0,path,last:Value::Null}
    }
    pub fn ingest(&mut self,events:&Value)->bool {
        let Some(list)=events.as_array().filter(|l|!l.is_empty()) else{return false};
        if self.last==*events{return false}self.last=events.clone();let mut changed=false;
        for event in list.iter().filter(|e|e.is_object()) {
            if let Some(old)=self.events.iter_mut().find(|o|key(o)==key(event)){if old!=event{*old=event.clone();changed=true;}}
            else{self.events.push(event.clone());changed=true;}
        }
        if changed {
            self.events.sort_by_key(|v|v["start"].as_i64().unwrap_or(0));if self.events.len()>8192{self.events.drain(..self.events.len()-8192);}
            self.revision=self.revision.wrapping_add(1);
            let tmp=self.path.with_extension("epg.tmp");
            if let Some(parent)=self.path.parent(){let _=std::fs::create_dir_all(parent);}
            if let Ok(bytes)=serde_json::to_vec(&self.events){if std::fs::write(&tmp,bytes).is_ok(){let _=std::fs::rename(&tmp,&self.path);}}
        }changed
    }
    pub fn current_title(&self,frequency:u32,program:u32)->Option<String>{self.events.iter().filter(|e|number(e,"frequency_khz")==frequency as u64&&number(e,"program_id")==program as u64&&e["running"]==true&&e["following"]!=true).max_by_key(|e|e["start"].as_i64().unwrap_or(0)).and_then(|e|e["name"].as_str()).map(|s|s.split_whitespace().collect::<Vec<_>>().join(" "))}
}
#[cfg(test)]mod tests {use super::*;
 #[test]fn updates_survive_restart_and_do_not_merge_different_multiplexes(){let p=std::env::temp_dir().join(format!("ovs-guide-{}.json",std::process::id()));let _=std::fs::remove_file(&p);let mut c=Cache::load(p.clone());let mut a=json!({"frequency_khz":1,"program_id":2,"event_id":3,"start":100,"name":"First","running":true});assert!(c.ingest(&json!([a])));a["frequency_khz"]=json!(2);assert!(c.ingest(&json!([a])));a["name"]=json!("Updated");assert!(c.ingest(&json!([a])));assert_eq!(c.events.len(),2);assert!(!c.ingest(&json!([])));let c=Cache::load(p.clone());assert_eq!(c.current_title(2,2).as_deref(),Some("Updated"));assert!(detail(&c.events[1]).contains("Updated"));let _=std::fs::remove_file(p);}
}
