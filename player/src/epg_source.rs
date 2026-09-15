use serde_json::{json, Value};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
pub fn current_title(events:&Value,frequency:u32,program:u32)->Option<String>{
    events.as_array()?.iter().filter(|e|e["frequency_khz"].as_u64()==Some(frequency as u64)
        && e["program_id"].as_u64()==Some(program as u64) && e["running"]==true && e["following"]!=true
        && e["name"].as_str().is_some_and(|s|!s.trim().is_empty()))
        .max_by_key(|e|e["start"].as_i64().unwrap_or(0))
        .and_then(|e|e["name"].as_str()).map(|s|s.split_whitespace().collect::<Vec<_>>().join(" "))
}
#[cfg(test)]mod now_tests {use super::*;
    #[test]fn selects_current_title_only_for_the_selected_service(){let events=json!([
        {"frequency_khz":1,"program_id":2,"name":"Old","running":true,"start":1},
        {"frequency_khz":1,"program_id":2,"name":"  Current\nshow ","running":true,"start":2},
        {"frequency_khz":1,"program_id":2,"name":"Next","running":true,"following":true,"start":3},
        {"frequency_khz":9,"program_id":2,"name":"Other multiplex","running":true,"start":4}]);
        assert_eq!(current_title(&events,1,2).as_deref(),Some("Current show"));assert_eq!(current_title(&events,1,3),None);}
    #[test]fn missing_epg_has_no_placeholder(){assert_eq!(current_title(&Value::Null,1,2),None);assert_eq!(current_title(&json!([]),1,2),None);}
}
fn tracks(stats:&a865r::TsStats)->Value {
    json!(stats.streams.iter().filter(|s|matches!(s.stream_type,0x0f|0x11|0x03|0x04)).map(|s|
        json!({"program_id":s.program_number,"pid":s.pid,"stream_type":s.stream_type,"language":stats.audio_languages.get(&s.pid)})).collect::<Vec<_>>())
}
fn captions(stats:&a865r::TsStats)->Value {
    json!(stats.streams.iter().filter_map(|s|stats.caption_profiles.get(&s.pid).map(|p|json!({"program_id":s.program_number,"pid":s.pid,"profile":p}))).collect::<Vec<_>>())
}
pub fn events(stats: &a865r::TsStats, frequency: u32) -> Value {
    json!(stats.events.values().filter(|e|stats.streams.iter().any(|stream|stream.program_number==e.service_id&&matches!(stream.stream_type,0x1b|0x02|0x24))).map(|e|json!({
        "frequency_khz":frequency,"program_id":e.service_id,"transport_id":e.transport_id,"network_id":e.network_id,
        "event_id":e.event_id,"start":e.start,"duration":e.duration,"name":e.name,"description":e.description,
        "channel_name":stats.service_names.get(&e.service_id),"channel_number":stats.channel_numbers.get(&e.service_id),
        "minimum_age":e.minimum_age,"running":e.running,"following":e.following,"language":e.language
    })).collect::<Vec<_>>())
}
pub fn observe(
    sample: &[u8],
    frequency: u32,
    control: a865r_media::playback::Control,
) -> a865r_bda::StreamObserver {
    let mut analyzer = a865r::TsAnalyzer::new();
    for chunk in sample.chunks(188 * 512) {
        analyzer.push(chunk);
    }
    control.set_epg(events(analyzer.stats(), frequency));
    control.set_audio_tracks(tracks(analyzer.stats()));
    control.set_caption_tracks(captions(analyzer.stats()));
    let state = Arc::new(Mutex::new((analyzer, Instant::now())));
    Arc::new(move |bytes| {
        let mut state = state.lock().unwrap();
        state.0.push(bytes);
        if state.1.elapsed() >= Duration::from_secs(2) {
            control.set_epg(events(state.0.stats(), frequency));
            control.set_audio_tracks(tracks(state.0.stats()));
            control.set_caption_tracks(captions(state.0.stats()));
            state.1 = Instant::now();
        }
    })
}
