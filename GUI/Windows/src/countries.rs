use serde_json::{json, Value};
/// ISDB-T adoption: https://www.dibeg.org/world/ . Scan centers cover this board's UHF range.
/// These are editable search presets, not claims about each country's allocated TV spectrum.
pub fn defaults() -> Vec<Value> {
    let profiles:Vec<_>=["Brazil","Argentina","Bolivia","Chile","Costa Rica","Ecuador","El Salvador","Guatemala","Honduras","Nicaragua","Paraguay","Peru","Uruguay","Venezuela"]
        .iter().map(|name|json!({"name":name,"standard":"ISDB-T","supported":true,"first":473143,"last":695143,"step":6000})).collect();
    profiles
}
pub fn scan(profile: &Value) -> Result<Vec<u32>, String> {
    if profile["standard"] != "ISDB-T" || profile["supported"] != true {
        return Err(format!(
            "{} uses {}. This ISDB-T tuner cannot receive it.",
            profile["name"].as_str().unwrap_or("This country"),
            profile["standard"]
                .as_str()
                .unwrap_or("another TV standard")
        ));
    }
    let read = |name| {
        profile[name]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| "Invalid country frequencies".to_owned())
    };
    a865r::channel_plan::custom_scan(read("first")?, read("last")?, read("step")?)
        .map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_and_unsupported_system() {
        let p = defaults();
        assert_eq!(p[0]["name"], "Brazil");
        assert_eq!(scan(&p[0]).unwrap().len(), 38);
        assert!(p.iter().all(|p| scan(p).is_ok()));
        assert!(p.iter().all(|p| p["name"] != "United States"));
    }
}
