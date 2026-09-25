//! Interaction rules shared by the Windows and Linux frontends.
use serde_json::Value;
// Broadcast channel first; use frequency/program as deterministic subchannel ties.
// Unknown channel numbers follow numbered services.
pub fn channel_sort_key(service:&Value)->(u64,u64,u64) {
    (service["channel_number"].as_u64().unwrap_or(u64::MAX),
     service["frequency_khz"].as_u64().unwrap_or(u64::MAX),
     service["program_id"].as_u64().unwrap_or(u64::MAX))
}
pub fn channel_order(services:&[Value],count:usize)->Vec<usize> {
    let mut order:Vec<_>=(0..count).collect();
    order.sort_by_key(|&i|channel_sort_key(services.get(i).unwrap_or(&Value::Null)));
    order
}
pub fn sort_channels(services:&mut Vec<Value>,selected:usize)->usize {
    let order=channel_order(services,services.len());
    let new_selected=order.iter().position(|&i|i==selected).unwrap_or(0);
    *services=order.iter().map(|&i|services[i].clone()).collect();
    new_selected
}
pub fn channel_number(service:Option<&Value>,index:usize)->u64 {
    service.and_then(|s|s["channel_number"].as_u64()).unwrap_or(index as u64+1)
}
pub fn channel_match(services:&[Value],count:usize,digits:&str)->Option<usize> {
    let number=digits.parse::<u64>().ok()?;
    (0..count).find(|&i|channel_number(services.get(i),i)==number)
}
pub fn drag_volume(volume:f32,previous:f32,current:f32)->f32 {
    let pi=std::f32::consts::PI;
    let delta=(current-previous+pi).rem_euclid(2.*pi)-pi;
    (volume+delta/pi*100.).clamp(0.,100.)
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn numeric_channel_order_preserves_selection_and_subchannels(){
        let mut services=vec![
            serde_json::json!({"channel_number":13,"frequency_khz":500000,"program_id":2}),
            serde_json::json!({"channel_number":4,"frequency_khz":600000,"program_id":2}),
            serde_json::json!({"channel_number":4,"frequency_khz":600000,"program_id":1}),
            serde_json::json!({"channel_number":2,"frequency_khz":700000,"program_id":1}),
            serde_json::json!({"frequency_khz":400000,"program_id":1}),
        ];
        let selected=services[0].clone();
        assert_eq!(channel_order(&services,services.len()),vec![3,2,1,0,4]);
        let index=sort_channels(&mut services,0);
        assert_eq!(index,3);assert_eq!(services[index],selected);
        assert_eq!(channel_match(&services,services.len(),"13"),Some(3));
        assert_eq!(sort_channels(&mut services,index),index);
        assert_eq!(sort_channels(&mut vec![],99),0);
    }
    #[test]fn channel_numbers_are_not_list_positions(){
        let services=vec![serde_json::json!({"channel_number":4}),serde_json::json!({"channel_number":13})];
        assert_eq!(channel_match(&services,2,"04"),Some(0));
        assert_eq!(channel_match(&services,2,"13"),Some(1));
        assert_eq!(channel_match(&services,2,"2"),None);
    }
    #[test]fn dial_crosses_angle_seam_without_jumping(){
        let v=drag_volume(50.,179_f32.to_radians(),(-179_f32).to_radians());
        assert!((v-51.111).abs()<0.01);
        assert_eq!(drag_volume(99.,0.,1.),100.);
        assert_eq!(drag_volume(1.,1.,0.),0.);
    }
}
