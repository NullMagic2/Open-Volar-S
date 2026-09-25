use super::*;
pub fn shader(picture:Picture,size:u64)->String{
 let source=a865r_media::recording_export::shader(&picture.json());
 let resize=match size{1=>"//!WIDTH 2560\n//!HEIGHT 1440\n",2=>"//!WIDTH 3840\n//!HEIGHT 2160\n",_=>""};
 source.replacen("//!HOOK MAIN\n",&format!("//!HOOK MAIN\n{resize}"),1)
}
