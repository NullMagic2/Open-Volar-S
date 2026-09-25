struct Parameters {width:u32,height:u32,kr:f32,kb:f32,adjust:vec4<f32>,effect:f32,use_lut:f32,padding:vec2<f32>}
@group(0) @binding(0) var<storage,read> source:array<u32>;
@group(0) @binding(1) var<storage,read_write> output:array<u32>;
@group(0) @binding(2) var<uniform> p:Parameters;
@group(0) @binding(3) var<storage,read> lut:array<vec4<f32>>;
fn byte_at(i:u32)->f32{return f32((source[i/4]>>((i%4)*8))&255u);}
fn lookup(c:vec3<u32>)->vec3<f32>{return lut[c.x+33u*(c.y+33u*c.z)].rgb;}
fn color(x:u32,y:u32,uv:vec2<f32>)->vec3<f32>{
    let l=(byte_at(y*p.width+x)-16.)/219.;let kg=1.-p.kr-p.kb;
    var rgb=clamp(vec3(l+2.*(1.-p.kr)*uv.y,l-2.*p.kb*(1.-p.kb)/kg*uv.x-2.*p.kr*(1.-p.kr)/kg*uv.y,l+2.*(1.-p.kb)*uv.x),vec3(0.),vec3(1.));
    let luminance=dot(rgb,vec3(.2126,.7152,.0722));
    rgb=mix(vec3(luminance),rgb,p.adjust.x);
    rgb=clamp((rgb-.5)*p.adjust.z+.5+p.adjust.y,vec3(0.),vec3(1.));
    rgb=clamp(rgb+2.*rgb*(1.-rgb)*p.adjust.w*vec3(-.06,-.02,.06),vec3(0.),vec3(1.));
    if p.effect>0. {
        let luma=dot(rgb,vec3(.2126,.7152,.0722));let mapped=luma+p.effect*luma*(1.-luma)*(2.*luma-1.);
        let low=min(rgb.r,min(rgb.g,rgb.b));let high=max(rgb.r,max(rgb.g,rgb.b));
        let scale=min(1.,min(mapped/max(luma-low,.000001),(1.-mapped)/max(high-luma,.000001)));
        rgb=clamp(vec3(mapped)+(rgb-vec3(luma))*scale,vec3(0.),vec3(1.));
    }
    if p.use_lut<.5{return rgb;}
    let c=rgb*32.;let lo=vec3<u32>(floor(c));let hi=min(lo+vec3(1u),vec3(32u));let f=fract(c);
    return mix(mix(mix(lookup(lo),lookup(vec3(hi.x,lo.y,lo.z)),f.x),mix(lookup(vec3(lo.x,hi.y,lo.z)),lookup(vec3(hi.x,hi.y,lo.z)),f.x),f.y),mix(mix(lookup(vec3(lo.x,lo.y,hi.z)),lookup(vec3(hi.x,lo.y,hi.z)),f.x),mix(lookup(vec3(lo.x,hi.y,hi.z)),lookup(hi),f.x),f.y),f.z);
}
@compute @workgroup_size(8,8,1) fn main(@builtin(global_invocation_id) id:vec3<u32>){
    let x=id.x*4u;let y=id.y*2u;if x+3u>=p.width || y+1u>=p.height{return;}
    var row0=0u;var row1=0u;var uvpack=0u;
    for(var pair=0u;pair<2u;pair++){
        let base=p.width*p.height+(y/2u)*p.width+x+pair*2u;
        let uv=(vec2(byte_at(base),byte_at(base+1u))-128.)/224.;var average=vec2(0.);
        for(var row=0u;row<2u;row++){for(var col=0u;col<2u;col++){
            let offset=pair*2u+col;let rgb=color(x+offset,y+row,uv);let yy=dot(rgb,vec3(p.kr,1.-p.kr-p.kb,p.kb));
            let outy=u32(round(clamp(16.+219.*yy,16.,235.)));
            if row==0u{row0|=outy<<(offset*8u);}else{row1|=outy<<(offset*8u);}
            average+=vec2((rgb.b-yy)/(2.*(1.-p.kb)),(rgb.r-yy)/(2.*(1.-p.kr)));
        }}
        let outuv=vec2<u32>(round(clamp(128.+56.*average,vec2(16.),vec2(240.))));uvpack|=(outuv.x|(outuv.y<<8u))<<(pair*16u);
    }
    output[(y*p.width+x)/4u]=row0;output[((y+1u)*p.width+x)/4u]=row1;output[(p.width*p.height+(y/2u)*p.width+x)/4u]=uvpack;
}
