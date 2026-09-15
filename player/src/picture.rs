//! User picture controls, applied before the monitor ICC transform.
use serde_json::{json, Value};
pub const PRESET:u16=340;
pub const SATURATION:u16=341;
pub const BRIGHTNESS:u16=342;
pub const CONTRAST:u16=343;
pub const RESET:u16=346;
pub const EFFECT:u16=348;
pub const SLIDERS:[u16;3]=[SATURATION,BRIGHTNESS,CONTRAST];
pub const PRESETS:[&str;5]=["Neutral","Cold","Warm","Vivid","Custom"];
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Picture {pub saturation:i32,pub brightness:i32,pub contrast:i32,pub cold:bool,pub warm:bool,pub hdr_effect:bool}
impl Default for Picture {fn default()->Self{Self{saturation:100,brightness:0,contrast:100,cold:false,warm:false,hdr_effect:false}}}
impl Picture {
    pub fn preset(index:usize)->Self {match index {1=>Self{cold:true,..Self::default()},2=>Self{warm:true,..Self::default()},3=>Self{saturation:120,contrast:110,..Self::default()},_=>Self::default()}}
    pub fn index(self)->usize {(0..PRESETS.len()-1).find(|&i|Self::preset(i)==Self{hdr_effect:false,..self}).unwrap_or(PRESETS.len()-1)}
    pub fn load(v:&Value)->Self {Self{saturation:v["saturation"].as_i64().unwrap_or(100).clamp(0,200) as i32,brightness:v["brightness"].as_i64().unwrap_or(0).clamp(-100,100) as i32,contrast:v["contrast"].as_i64().unwrap_or(100).clamp(0,200) as i32,cold:v["cold"].as_bool().unwrap_or(false) && !v["warm"].as_bool().unwrap_or(false),warm:v["warm"].as_bool().unwrap_or(false),hdr_effect:v["hdr_effect"].as_bool().unwrap_or(false)}}
    pub fn json(self)->Value {json!({"saturation":self.saturation,"brightness":self.brightness,"contrast":self.contrast,"cold":self.cold,"warm":self.warm,"hdr_effect":self.hdr_effect})}
    /// A monotonic SDR curve, bounded to a 5.30% signal shift. Not HDR reconstruction.
    pub fn effect_strength(self)->f32 {if self.hdr_effect{0.55}else{0.}}
    /// Scalar counterpart of the Vulkan and Direct3D picture shaders.
    pub fn apply_rgb(self, rgb:[f32;3])->[f32;3] {
        let [saturation,brightness,contrast,temperature]=self.uniform();
        let rgb=rgb.map(|v|v.clamp(0.,1.));
        let luma=rgb[0]*0.2126+rgb[1]*0.7152+rgb[2]*0.0722;
        let mut rgb=rgb.map(|v|((luma+(v-luma)*saturation-0.5)*contrast+0.5+brightness).clamp(0.,1.));
        for (v,tint) in rgb.iter_mut().zip([-0.06,-0.02,0.06]) {*v=(*v+2.**v*(1.-*v)*temperature*tint).clamp(0.,1.);}
        if self.hdr_effect {
            let y=rgb[0]*0.2126+rgb[1]*0.7152+rgb[2]*0.0722;
            let mapped=y+self.effect_strength()*y*(1.-y)*(2.*y-1.);
            let low=rgb.iter().copied().fold(f32::INFINITY,f32::min);
            let high=rgb.iter().copied().fold(f32::NEG_INFINITY,f32::max);
            let scale=1f32.min(mapped/(y-low).max(0.000001)).min((1.-mapped)/(high-y).max(0.000001));
            rgb=rgb.map(|v|(mapped+(v-y)*scale).clamp(0.,1.));
        }
        rgb
    }
    pub fn uniform(self)->[f32;4] {[self.saturation as f32/100.,self.brightness as f32/200.,self.contrast as f32/100.,if self.cold{1.}else if self.warm{-1.}else{0.}]}
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn effect_is_independent_and_backwards_compatible(){
        assert!(!Picture::load(&json!({})).hdr_effect);
        for i in 0..PRESETS.len()-1 {let p=Picture{hdr_effect:true,..Picture::preset(i)};assert_eq!(p.index(),i);assert_eq!(Picture::load(&p.json()),p);}
        assert_eq!(Picture::default().effect_strength(),0.);
    }
    #[test] fn subtle_curve_preserves_endpoints_and_orders_tones(){
        let strength=Picture{hdr_effect:true,..Default::default()}.effect_strength();
        let curve=|y:f32|y+strength*y*(1.-y)*(2.*y-1.);
        assert_eq!(curve(0.),0.);assert_eq!(curve(0.5),0.5);assert_eq!(curve(1.),1.);
        let mut previous=-1.;
        for i in 0..=10000 {let y=i as f32/10000.;let v=curve(y);assert!(v>previous);assert!((v-y).abs()<0.053);assert!((0. ..=1.).contains(&v));previous=v;}
    }
    #[test] fn neutral_is_identity(){assert_eq!(Picture::default().uniform(),[1.,0.,1.,0.]);}
    #[test] fn presets_and_custom_roundtrip(){for i in 0..PRESETS.len()-1{let p=Picture::preset(i);assert_eq!(p.index(),i);assert_eq!(Picture::load(&p.json()),p);}let p=Picture{brightness:17,..Picture::preset(1)};assert_eq!(p.index(),PRESETS.len()-1);assert_eq!(Picture::load(&p.json()),p);}
    #[test] fn invalid_saved_values_are_bounded(){let p=Picture::load(&json!({"saturation":-4,"brightness":900,"contrast":999}));assert_eq!(p.uniform(),[0.,0.5,2.,0.]);}
}

#[cfg(test)] mod temperature_tests {
    use super::*;
    #[test] fn warm_complements_cold_and_preserves_existing_settings() {
        let warm=Picture::preset(2);let cold=Picture::preset(1);
        assert_eq!(warm.uniform()[3],-cold.uniform()[3]);
        assert!(warm.warm && !warm.cold);
        assert_eq!(Picture::load(&warm.json()),warm);
        assert_eq!(Picture::load(&json!({"cold":true})),cold);
        assert_eq!(Picture::load(&json!({})),Picture::default());
        assert_eq!(Picture::preset(3).saturation,120);
    }
}

#[cfg(test)] mod highlight_tests {
    #[test] fn temperature_preserves_white_black_and_highlight_order(){
        for temperature in [-1f32,0.,1.] {for tint in [-0.06f32,-0.02,0.06] {
            let adjust=|v:f32|v+2.*v*(1.-v)*temperature*tint;
            assert_eq!(adjust(0.),0.);assert_eq!(adjust(1.),1.);
            let mut previous=-1.;for n in 0..=10000{let y=adjust(n as f32/10000.);assert!(y>previous && y<=1.);previous=y;}
            assert!(adjust(0.96)<adjust(0.98));
        }}
    }
}

#[cfg(test)] mod scalar_tests {
    use super::*;
    #[test] fn controls_change_the_expected_color_properties() {
        let input=[0.6,0.4,0.2];
        let mono=Picture{saturation:0,..Default::default()}.apply_rgb(input);
        assert!((mono[0]-mono[1]).abs()<0.00001 && (mono[1]-mono[2]).abs()<0.00001);
        let cold=Picture::preset(1).apply_rgb(input);let warm=Picture::preset(2).apply_rgb(input);
        assert!(cold[0]<input[0] && cold[2]>input[2]);assert!(warm[0]>input[0] && warm[2]<input[2]);
        let brighter=Picture{brightness:20,..Default::default()}.apply_rgb(input);assert!((brighter[1]-0.5).abs()<0.00001);
        for preset in [1,2] {assert_eq!(Picture::preset(preset).apply_rgb([0.;3]),[0.;3]);assert_eq!(Picture::preset(preset).apply_rgb([1.;3]),[1.;3]);}
        let hdr=Picture{hdr_effect:true,..Default::default()};assert!(hdr.apply_rgb([0.25;3])[0]<0.25);assert!(hdr.apply_rgb([0.75;3])[0]>0.75);
    }
}
