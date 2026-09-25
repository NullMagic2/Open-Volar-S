struct Parameters { source:vec4<f32>, process:vec4<f32>, color:vec4<f32>, aperture:vec4<f32>, adjust:vec4<f32> };
@group(0) @binding(0) var ytex:texture_2d<f32>;
@group(0) @binding(1) var uvtex:texture_2d<f32>;
@group(0) @binding(2) var previous:texture_2d<f32>;
@group(0) @binding(3) var lut:texture_3d<f32>;
@group(0) @binding(4) var linear:sampler;
@group(0) @binding(5) var<uniform> p:Parameters;
@group(0) @binding(6) var next:texture_2d<f32>;
@group(1) @binding(0) var reconstructed:texture_2d<f32>;
struct Vertex { @builtin(position) position:vec4<f32>, @location(0) uv:vec2<f32> };
@vertex fn vs(@builtin(vertex_index) i:u32)->Vertex {
    var positions=array<vec2<f32>,3>(vec2(-1.,-1.),vec2(3.,-1.),vec2(-1.,3.));
    var out:Vertex;out.position=vec4(positions[i],0.,1.);out.uv=vec2((positions[i].x+1.)*.5,(1.-positions[i].y)*.5);return out;
}
fn luma(x:i32,y:i32)->f32 {
    let pos=vec2(clamp(x,0,i32(p.source.x)-1),clamp(y,0,i32(p.source.y)-1));
    let original=textureLoad(ytex,pos,0).r;
    if p.process.z<.5||((pos.y&1)==i32(p.process.w)){return original;}
    // Weave stationary areas; interpolate the current field only in motion.
    // Comparing the same row in the preceding frame avoids spatial bob shimmer.
    let parity=i32(p.process.w);
    let last=i32(p.source.y)-1-((i32(p.source.y)-1-parity)&1);
    let above=clamp(pos.y-1,parity,last);
    let below=clamp(pos.y+1,parity,last);
    let a=textureLoad(ytex,vec2(pos.x,above),0).r;
    let b=textureLoad(ytex,vec2(pos.x,below),0).r;
    var interpolated=.5*(a+b);
    var difference=abs(a-b);
    // Reconstruct along a diagonal edge when it matches better than vertical bob.
    for(var direction=-1;direction<=1;direction+=2){
        let left=textureLoad(ytex,vec2(clamp(pos.x+direction,0,i32(p.source.x)-1),above),0).r;
        let right=textureLoad(ytex,vec2(clamp(pos.x-direction,0,i32(p.source.x)-1),below),0).r;
        let score=abs(left-right)+1./255.;
        if score<difference {difference=score;interpolated=.5*(left+right);}
    }
    // Fine stationary detail must not be classified as motion just because
    // adjacent scanlines differ. Doing that bobs sharp edges on alternate fields.
    // Inspect both temporal neighbors, including the current field's rows, so
    // repeated film frames cannot hide motion merely by repeating one comb.
    var motion=0.;
    for(var dy=-1;dy<=1;dy++){
        let q=vec2(pos.x,clamp(pos.y+dy,0,i32(p.source.y)-1));
        let current=textureLoad(ytex,q,0).r;
        motion=max(motion,abs(current-textureLoad(previous,q,0).r));
        if p.color.w>.5 {motion=max(motion,abs(current-textureLoad(next,q,0).r));}
    }
    let stable=1.-smoothstep(2./255.,8./255.,motion);
    let weave=select(0.,stable,p.color.y>.5);
    return mix(interpolated,original,weave);
}
// Reconstruct once at source resolution. The filterable R16Float target keeps
// more precision than the 8-bit broadcast and lets the sampler scale it.
@fragment fn fs_luma(in:Vertex)->@location(0) f32 {
    return luma(i32(in.position.x),i32(in.position.y));
}
fn filtered_y(uv:vec2<f32>)->f32{
    if p.process.z>.5 {return textureSampleLevel(reconstructed,linear,uv,0.).r;}
    // Clamp to visible texel centers before normalizing against decoder padding.
    let pixel=clamp(uv*p.source.xy,vec2(.5),p.source.xy-.5);
    return textureSampleLevel(ytex,linear,pixel/vec2(p.source.z,p.color.z),0.).r;
}
// Interlaced 4:2:0 stores alternating chroma rows from different fields.
// Filter within the selected field, never across those two capture times.
fn filtered_uv(uv:vec2<f32>)->vec2<f32> {
    if p.process.z<.5 {
        let pixel=clamp(uv*p.source.xy*.5,vec2(.5),p.source.xy*.5-.5);
        return textureSampleLevel(uvtex,linear,pixel/vec2<f32>(textureDimensions(uvtex)),0.).rg;
    }
    let size=vec2<i32>(textureDimensions(uvtex));
    let parity=i32(p.process.w);
    let height=min(size.y,i32(p.source.y)/2);
    let last=max(parity,height-1-((height-1-parity)&1));
    let field_y=(uv.y*p.source.y-.5-f32(parity+1))*.25;
    let row=i32(floor(field_y));
    let y0=clamp(parity+row*2,parity,last);
    let y1=clamp(parity+(row+1)*2,parity,last);
    let x=clamp(uv.x*p.source.x*.5,.5,p.source.x*.5-.5)/f32(size.x);
    // Sample horizontally at exact field-row centers, then blend explicitly
    // so hardware filtering never combines opposite fields.
    let a=textureSampleLevel(uvtex,linear,vec2(x,(f32(y0)+.5)/f32(size.y)),0.).rg;
    let b=textureSampleLevel(uvtex,linear,vec2(x,(f32(y1)+.5)/f32(size.y)),0.).rg;
    return mix(a,b,fract(field_y));
}
fn profile(rgb:vec3<f32>)->vec3<f32>{
    let at=clamp(rgb,vec3(0.),vec3(1.))*32.;let lo=vec3<i32>(floor(at));let hi=min(lo+vec3(1),vec3(32));let f=fract(at);
    let a=mix(textureLoad(lut,lo,0).rgb,textureLoad(lut,vec3(hi.x,lo.y,lo.z),0).rgb,f.x);
    let b=mix(textureLoad(lut,vec3(lo.x,hi.y,lo.z),0).rgb,textureLoad(lut,vec3(hi.x,hi.y,lo.z),0).rgb,f.x);
    let c=mix(textureLoad(lut,vec3(lo.x,lo.y,hi.z),0).rgb,textureLoad(lut,vec3(hi.x,lo.y,hi.z),0).rgb,f.x);
    let d=mix(textureLoad(lut,vec3(lo.x,hi.y,hi.z),0).rgb,textureLoad(lut,hi,0).rgb,f.x);
    return mix(mix(a,b,f.y),mix(c,d,f.y),f.z);
}
// SDR-only contrast shaping. Fixed across frames to avoid exposure pumping;
// no neighborhood sampling, so the effect introduces no halos or extra passes.
// aperture.z is the effect strength; x/y retain the crop coordinates.
fn subtle_hdr_effect(rgb:vec3<f32>)->vec3<f32>{
    let y=dot(rgb,vec3(.2126,.7152,.0722));
    let mapped_luma=y+p.aperture.z*y*(1.-y)*(2.*y-1.);
    let low=min(rgb.r,min(rgb.g,rgb.b));
    let high=max(rgb.r,max(rgb.g,rgb.b));
    // Preserve chroma direction, shrinking only when needed to stay inside SDR.
    let chroma_scale=min(1.,min(mapped_luma/max(y-low,0.000001),(1.-mapped_luma)/max(high-y,0.000001)));
    return clamp(vec3(mapped_luma)+(rgb-vec3(y))*chroma_scale,vec3(0.),vec3(1.));
}
@fragment fn fs(in:Vertex)->@location(0) vec4<f32>{
    let picture_uv=vec2(p.aperture.x+in.uv.x*p.aperture.y,in.uv.y);
    let encoded=filtered_y(picture_uv)*255.;let uv=filtered_uv(picture_uv)*255.-128.;
    let full=p.source.w>.5;let y=select((encoded-16.)/219.,encoded/255.,full);let chroma=uv/select(224.,255.,full);
    let kr=p.process.x;let kb=p.process.y;let kg=1.-kr-kb;
    var rgb=clamp(vec3(y+2.*(1.-kr)*chroma.y,y-2.*kb*(1.-kb)/kg*chroma.x-2.*kr*(1.-kr)/kg*chroma.y,y+2.*(1.-kb)*chroma.x),vec3(0.),vec3(1.));
    let luminance=dot(rgb,vec3(.2126,.7152,.0722));
    rgb=mix(vec3(luminance),rgb,p.adjust.x);
    rgb=(rgb-.5)*p.adjust.z+.5+p.adjust.y;
    // Signed temperature: +1 Cold, -1 Warm, 0 Neutral.
    // Preserve black/white and highlight differences instead of clipping a channel.
    rgb=clamp(rgb,vec3(0.),vec3(1.));
    rgb+=2.*rgb*(vec3(1.)-rgb)*p.adjust.w*vec3(-.06,-.02,.06);
    rgb=clamp(rgb,vec3(0.),vec3(1.));
    if p.aperture.z>0.{rgb=subtle_hdr_effect(rgb);}
    if p.color.x>.5{rgb=profile(rgb);}
    return vec4(rgb,1.);
}
