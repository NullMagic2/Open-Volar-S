use serde_json::{json,Value};
use sha2::{Digest,Sha256};
use std::io::Read;

const ITERATIONS:u32=210_000;
pub const AGES:[u8;6]=[0,10,12,14,16,18];
pub fn channel_key(frequency:u32,program:u32)->String{format!("{frequency}:{program}")}

fn bytes(v:&Value)->Option<Vec<u8>>{
    v.as_array()?.iter().map(|n|u8::try_from(n.as_u64()?).ok()).collect()
}
fn hmac(password:&[u8],message:&[u8])->[u8;32]{
    let mut key=[0u8;64];
    if password.len()>64 {key[..32].copy_from_slice(&Sha256::digest(password));}
    else {key[..password.len()].copy_from_slice(password);}
    let mut inner=[0x36u8;64];let mut outer=[0x5cu8;64];
    for i in 0..64 {inner[i]^=key[i];outer[i]^=key[i];}
    let mut hash=Sha256::new();hash.update(inner);hash.update(message);
    let middle=hash.finalize();
    let mut hash=Sha256::new();hash.update(outer);hash.update(middle);
    hash.finalize().into()
}
fn derive(password:&str,salt:&[u8])->[u8;32]{
    let mut block=Vec::with_capacity(salt.len()+4);
    block.extend_from_slice(salt);block.extend_from_slice(&1u32.to_be_bytes());
    let mut previous=hmac(password.as_bytes(),&block);
    let mut result=previous;
    for _ in 1..ITERATIONS {
        previous=hmac(password.as_bytes(),&previous);
        for i in 0..32 {result[i]^=previous[i];}
    }
    result
}
pub fn set_password(password:&str)->Result<Value,String>{
    if !(4..=128).contains(&password.chars().count()){
        return Err("Use a password of 4–128 characters.".into());
    }
    let mut salt=[0u8;16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file|file.read_exact(&mut salt))
        .map_err(|e|format!("Cannot create password salt: {e}"))?;
    Ok(json!({"salt":salt,"hash":derive(password,&salt)}))
}
pub fn verify(config:&Value,password:&str)->bool{
    let Some(salt)=bytes(&config["password"]["salt"]) else{return false;};
    let Some(expected)=bytes(&config["password"]["hash"]) else{return false;};
    if salt.len()!=16||expected.len()!=32{return false;}
    let actual=derive(password,&salt);
    actual.iter().zip(expected).fold(0u8,|diff,(a,b)|diff|(a^b))==0
}
pub fn blocked(config:&Value,frequency:u32,program:u32,rating:Option<u8>,unlocked:bool)->bool{
    if config["enabled"]!=true||unlocked{return false;}
    let key=channel_key(frequency,program);
    if config["channels"].as_array().is_some_and(|items|items.iter().any(|n|n.as_str()==Some(&key))){return true;}
    rating.map(|age|age as u64>config["max_age"].as_u64().unwrap_or(18))
        .unwrap_or(config["block_unrated"]==true)
}
pub fn rating(events:&Value,frequency:u32,program:u32)->Option<u8>{
    events.as_array()?.iter()
        .filter(|e|e["frequency_khz"].as_u64()==Some(frequency as u64)
            && e["program_id"].as_u64()==Some(program as u64)
            && e["running"]==true && e["following"]!=true)
        .filter_map(|e|e["minimum_age"].as_u64().and_then(|n|u8::try_from(n).ok())).max()
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn policy_rules(){
        let p=json!({"enabled":true,"max_age":12,"block_unrated":true,"channels":["500000:1"]});
        assert!(!blocked(&p,500000,2,Some(12),false));
        assert!(blocked(&p,500000,2,Some(14),false));
        assert!(blocked(&p,500000,2,None,false));
        assert!(blocked(&p,500000,1,Some(0),false));
        assert!(!blocked(&p,500000,1,None,true));
    }
    #[test]fn windows_password_format(){
        let p=set_password("example password").unwrap();
        assert!(verify(&json!({"password":p}),"example password"));
        assert!(!verify(&json!({"password":p}),"different"));
    }
}
