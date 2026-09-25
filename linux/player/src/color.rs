//! Build the same 33-cubed sRGB-to-monitor LUT used by the Windows renderer,
//! using Linux's Little CMS rather than the Windows colour-management API.
use libc::c_void;
#[link(name="lcms2")]
unsafe extern "C"{
    fn cmsCreate_sRGBProfile()->*mut c_void;
    fn cmsOpenProfileFromMem(data:*const c_void,size:u32)->*mut c_void;
    fn cmsCloseProfile(profile:*mut c_void)->i32;
    fn cmsGetColorSpace(profile:*mut c_void)->u32;
    fn cmsCreateTransform(input:*mut c_void,input_format:u32,output:*mut c_void,output_format:u32,intent:u32,flags:u32)->*mut c_void;
    fn cmsDeleteTransform(transform:*mut c_void);
    fn cmsDoTransform(transform:*mut c_void,input:*const c_void,output:*mut c_void,count:u32);
}
struct Profile(*mut c_void);
impl Drop for Profile{fn drop(&mut self){if !self.0.is_null(){unsafe{cmsCloseProfile(self.0);}}}}
struct Transform(*mut c_void);
impl Drop for Transform{fn drop(&mut self){if !self.0.is_null(){unsafe{cmsDeleteTransform(self.0);}}}}
pub fn lut(data:&[u8])->Result<Vec<u8>,String>{
    if data.len()<128||data.len()>16*1024*1024||&data[36..40]!=b"acsp"{return Err("Invalid ICC profile".into());}
    unsafe{
        let source=Profile(cmsCreate_sRGBProfile());let target=Profile(cmsOpenProfileFromMem(data.as_ptr().cast(),data.len() as u32));
        if source.0.is_null()||target.0.is_null()||cmsGetColorSpace(target.0)!=0x52474220{return Err("ICC profile must describe an RGB display".into());}
        // FLOAT_SH(1) | COLORSPACE_SH(PT_RGB) | CHANNELS_SH(3) | BYTES_SH(4)
        let format=(1<<22)|(4<<16)|(3<<3)|4;
        let transform=Transform(cmsCreateTransform(source.0,format,target.0,format,1,0));
        if transform.0.is_null(){return Err("Cannot build the monitor ICC transform".into());}
        let input:Vec<f32>=(0..33*33*33).flat_map(|i|[(i%33)as f32/32.,((i/33)%33)as f32/32.,(i/33/33)as f32/32.]).collect();
        let mut output=vec![0f32;input.len()];cmsDoTransform(transform.0,input.as_ptr().cast(),output.as_mut_ptr().cast(),33*33*33);
        if output.iter().any(|f|!f.is_finite()){return Err("ICC transform returned invalid colours".into());}
        Ok(output.chunks_exact(3).flat_map(|c|[c[0].clamp(0.,1.),c[1].clamp(0.,1.),c[2].clamp(0.,1.),1.]).flat_map(f32::to_le_bytes).collect())
    }
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn malformed_profiles_are_rejected(){assert!(lut(&[]).is_err());assert!(lut(&[0;128]).is_err());}
}
