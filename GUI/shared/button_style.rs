// Windows Orbit button colors and translated-row sizing, shared by both frontends.
pub const RECORD_EDGE: (u8,u8,u8) = (190,81,64);
pub const CAPTION_EDGE: (u8,u8,u8) = (190,165,52);
/// Stationary Windows material grain and lighting, in native BGRA pixel order.
/// Metal uses a fixed 425 logical-pixel tile instead of shrinking a whole image
/// into each button. Both frontends call this to keep the same surface finish.
pub fn material_pixels(source:&[u8],sw:i32,sh:i32,stride:usize,
    material:usize,w:i32,h:i32,dpi:u32)->Vec<u8>{
    let mut pixels=vec![0u8;(w*h*4) as usize];
    for y in 0..h {for x in 0..w {
        let (sx,sy)=if material==0 {
            let scale=sw as f32/(425.*dpi.max(1) as f32/96.);
            (((x as f32*scale) as i32)%sw,((y as f32*scale) as i32)%sh)
        }else{(x*sw/w,y*sh/h)};
        let from=sy as usize*stride+sx as usize*4;
        let at=((y*w+x)*4) as usize;
        let lighting=if material==0{18.*(1.-x as f32/w as f32)-15.*y as f32/h as f32}else{0.};
        let glass=if material==1{0.09*(-((y as f32/h as f32-0.03)/0.24).powi(2)).exp()}else{0.};
        for channel in 0..3 {
            let base=source[from+channel] as f32+lighting;
            let warm=[176.,218.,244.][channel];
            pixels[at+channel]=(base*(1.-glass)+warm*glass).clamp(0.,255.) as u8;
        }
        pixels[at+3]=255;
    }}
    pixels
}
pub fn row_widths(required:&[f64], original:&[f64], available:f64)->Vec<f64>{
    let minimum:f64=required.iter().sum();
    let slack=(available-minimum).max(0.);
    let total:f64=original.iter().sum();
    required.iter().zip(original).map(|(need,base)|need+slack*base/total).collect()
}
