//! Password-protected channel/rating policy. Temporary unlock is process-local.
use serde_json::{json,Value};
use std::sync::{Mutex,atomic::{AtomicBool,Ordering}};
use windows::{core::w,Win32::Security::Cryptography::*};
pub const AUTH:u16=350;pub const NEW:u16=351;pub const CONFIRM:u16=352;pub const UNLOCK:u16=353;pub const ENABLE:u16=354;pub const UNRATED:u16=355;pub const RATING:u16=356;pub const CHANNEL:u16=357;pub const LOCK:u16=358;pub const STATUS:u16=360;
pub static UNLOCKED:AtomicBool=AtomicBool::new(false);
static POLICY:Mutex<Value>=Mutex::new(Value::Null);
pub fn load(v:Value){*POLICY.lock().unwrap()=v;}
pub fn config()->Value{POLICY.lock().unwrap().clone()}
pub fn active()->bool{config()["enabled"]==true && !UNLOCKED.load(Ordering::Acquire)}
fn derive(password:&str,salt:&[u8])->Result<Vec<u8>,String>{unsafe{
    let mut algorithm=BCRYPT_ALG_HANDLE::default();
    BCryptOpenAlgorithmProvider(&mut algorithm,w!("SHA256"),None,BCRYPT_ALG_HANDLE_HMAC_FLAG).ok().map_err(|e|e.to_string())?;
    let mut output=vec![0u8;32];let result=BCryptDeriveKeyPBKDF2(algorithm,Some(password.as_bytes()),Some(salt),210_000,&mut output,0).ok();
    let _=BCryptCloseAlgorithmProvider(algorithm,0);result.map_err(|e|e.to_string())?;Ok(output)
}}
fn bytes(v:&Value)->Vec<u8>{v.as_array().map(|a|a.iter().filter_map(|n|n.as_u64().and_then(|n|u8::try_from(n).ok())).collect()).unwrap_or_default()}
pub fn set_password(password:&str)->Result<Value,String>{
    if !(4..=128).contains(&password.chars().count()){return Err("Use a password of 4–128 characters.".into());}
    let mut salt=[0u8;16];unsafe{BCryptGenRandom(None,&mut salt,BCRYPT_USE_SYSTEM_PREFERRED_RNG).ok().map_err(|e|e.to_string())?;}
    Ok(json!({"salt":salt,"hash":derive(password,&salt)?}))
}
pub fn verify(password:&str)->bool{
    let v=config();let salt=bytes(&v["password"]["salt"]);let expected=bytes(&v["password"]["hash"]);
    if salt.len()!=16||expected.len()!=32{return false;}
    derive(password,&salt).is_ok_and(|actual|actual.iter().zip(expected).fold(0u8,|v,(a,b)|v|(a^b))==0)
}
pub fn channel_key(frequency:u32,program:u32)->String{format!("{frequency}:{program}")}
pub fn blocked(v:&Value,frequency:u32,program:u32,rating:Option<u8>,unlocked:bool)->bool{
    if v["enabled"]!=true||unlocked{return false;}
    let key=channel_key(frequency,program);
    if v["channels"].as_array().is_some_and(|a|a.iter().any(|n|n.as_str()==Some(&key))){return true;}
    rating.map(|age|age as u64>v["max_age"].as_u64().unwrap_or(18)).unwrap_or(v["block_unrated"]==true)
}
pub fn check(events:&Value,frequency:u32,program:u32)->Result<(),String>{
    let rating=events.as_array().and_then(|a|a.iter().filter(|e|e["program_id"].as_u64()==Some(program as u64)&&e["running"]==true&&!e["following"].as_bool().unwrap_or(false)).filter_map(|e|e["minimum_age"].as_u64()).max()).map(|n|n as u8);
    if blocked(&config(),frequency,program,rating,UNLOCKED.load(Ordering::Acquire)){Err("Parental controls: this program is blocked. Unlock in Settings > Parental.".into())}else{Ok(())}
}
#[cfg(test)]mod tests{use super::*;
    #[test]fn rating_channel_and_unrated_rules(){let v=json!({"enabled":true,"max_age":12,"block_unrated":true,"channels":["500000:1"]});assert!(!blocked(&v,500000,2,Some(12),false));assert!(blocked(&v,500000,2,Some(14),false));assert!(blocked(&v,500000,2,None,false));assert!(blocked(&v,500000,1,Some(0),false));assert!(!blocked(&v,500000,1,None,true));}
    #[test]fn password_is_salted_and_verifiable(){let a=set_password("test-password").unwrap();let b=set_password("test-password").unwrap();assert_ne!(a,b);let salt=bytes(&a["salt"]);assert_eq!(derive("test-password",&salt).unwrap(),bytes(&a["hash"]));assert_ne!(derive("wrong",&salt).unwrap(),bytes(&a["hash"]));}
}
