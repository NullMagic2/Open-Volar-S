//! Direct3D 11 compute application of a Windows-CMM-generated ICC 3D LUT.
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::{Direct3D::Fxc::*, Direct3D::*, Direct3D11::*, Dxgi::Common::*},
    },
};
const SHADER: &str = r#"
Texture2D<float> Y : register(t0);
Texture2D<float2> UV : register(t1);
Texture3D<float4> LUT : register(t2);
SamplerState LinearClamp : register(s0);
RWByteAddressBuffer Output : register(u0);
cbuffer Parameters : register(b0) {uint width;uint height;float kr;float kb;float4 adjust;float effect;float use_lut;float2 padding;};
float3 color(uint x,uint y,float2 uv) {
    float l=(Y.Load(int3(x,y,0))*255.0-16.0)/219.0;
    float kg=1.0-kr-kb;
    float3 rgb=float3(l+2.0*(1.0-kr)*uv.y,l-2.0*kb*(1.0-kb)/kg*uv.x-2.0*kr*(1.0-kr)/kg*uv.y,l+2.0*(1.0-kb)*uv.x);
    rgb=saturate(rgb);
    float luminance=dot(rgb,float3(.2126,.7152,.0722));
    rgb=lerp(luminance.xxx,rgb,adjust.x);
    rgb=saturate((rgb-.5)*adjust.z+.5+adjust.y);
    rgb=saturate(rgb+2.*rgb*(1.-rgb)*adjust.w*float3(-.06,-.02,.06));
    if(effect>0.) {
        float luma=dot(rgb,float3(.2126,.7152,.0722));
        float mapped=luma+effect*luma*(1.-luma)*(2.*luma-1.);
        float low=min(rgb.r,min(rgb.g,rgb.b)),high=max(rgb.r,max(rgb.g,rgb.b));
        float scale=min(1.,min(mapped/max(luma-low,.000001),(1.-mapped)/max(high-luma,.000001)));
        rgb=saturate(mapped.xxx+(rgb-luma.xxx)*scale);
    }
    if(use_lut<.5) return rgb;
    return LUT.SampleLevel(LinearClamp,(saturate(rgb)*32.0+0.5)/33.0,0).rgb;
}
[numthreads(8,8,1)] void main(uint3 id:SV_DispatchThreadID) {
    uint x=id.x*4,y=id.y*2;if(x+3>=width||y+1>=height)return;
    uint row0=0,row1=0,uvpack=0;
    [unroll]for(uint pair=0;pair<2;pair++) {
        float2 uv=(UV.Load(int3(x/2+pair,y/2,0))*255.0-128.0)/224.0;
        float2 average=0;
        [unroll]for(uint row=0;row<2;row++) {
            [unroll]for(uint column=0;column<2;column++) {
                uint offset=pair*2+column;float3 rgb=color(x+offset,y+row,uv);
                float yy=dot(rgb,float3(kr,1.0-kr-kb,kb));
                uint outy=(uint)round(clamp(16.0+219.0*yy,16.0,235.0));
                if(row==0)row0|=outy<<(offset*8);else row1|=outy<<(offset*8);
                average+=float2((rgb.b-yy)/(2.0*(1.0-kb)),(rgb.r-yy)/(2.0*(1.0-kr)));
            }
        }
        uint2 outuv=(uint2)round(clamp(128.0+56.0*average,16.0,240.0));
        uvpack|=(outuv.x|(outuv.y<<8))<<(pair*16);
    }
    Output.Store(y*width+x,row0);Output.Store((y+1)*width+x,row1);
    Output.Store(width*height+(y/2)*width+x,uvpack);
}
"#;
pub struct D3d11 {
    pub width: usize,
    pub height: usize,
    context: ID3D11DeviceContext,
    constant: ID3D11Buffer,
    parameters: [u32;12],
    y: ID3D11Texture2D,
    uv: ID3D11Texture2D,
    output: ID3D11Buffer,
    readback: ID3D11Buffer,
}
// Access is serialized by Transform's GPU mutex, including the immediate context.
unsafe impl Send for D3d11 {}
impl D3d11 {
    pub unsafe fn new(lut: &[[f32; 3]], width: usize, height: usize, bt709: bool) -> Result<Self> {
        if width % 4 != 0 {
            return Err(Error::new(
                E_INVALIDARG,
                "GPU NV12 requires a pitch divisible by four",
            ));
        }
        let mut device = None;
        let mut context = None;
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            None,
            D3D11_CREATE_DEVICE_FLAG(0),
            Some(&[D3D_FEATURE_LEVEL_11_0]),
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )?;
        let device = device.unwrap();
        let context = context.unwrap();
        let texture = |w: u32, h: u32, format: DXGI_FORMAT| -> Result<ID3D11Texture2D> {
            let mut t = None;
            device.CreateTexture2D(
                &D3D11_TEXTURE2D_DESC {
                    Width: w,
                    Height: h,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: format,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                    ..Default::default()
                },
                None,
                Some(&mut t),
            )?;
            Ok(t.unwrap())
        };
        let y = texture(width as u32, height as u32, DXGI_FORMAT_R8_UNORM)?;
        let uv = texture(width as u32 / 2, height as u32 / 2, DXGI_FORMAT_R8G8_UNORM)?;
        let mut sy = None;
        let mut suv = None;
        device.CreateShaderResourceView(&y, None, Some(&mut sy))?;
        device.CreateShaderResourceView(&uv, None, Some(&mut suv))?;
        let table: Vec<[f32; 4]> = lut.iter().map(|v| [v[0], v[1], v[2], 1.]).collect();
        let mut t = None;
        device.CreateTexture3D(
            &D3D11_TEXTURE3D_DESC {
                Width: 33,
                Height: 33,
                Depth: 33,
                MipLevels: 1,
                Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
                Usage: D3D11_USAGE_IMMUTABLE,
                BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                ..Default::default()
            },
            Some(&D3D11_SUBRESOURCE_DATA {
                pSysMem: table.as_ptr().cast(),
                SysMemPitch: 33 * 16,
                SysMemSlicePitch: 33 * 33 * 16,
            }),
            Some(&mut t),
        )?;
        let mut sl = None;
        device.CreateShaderResourceView(t.as_ref().unwrap(), None, Some(&mut sl))?;
        let length = (width * height * 3 / 2) as u32;
        let desc = D3D11_BUFFER_DESC {
            ByteWidth: length,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_UNORDERED_ACCESS.0 as u32,
            MiscFlags: D3D11_RESOURCE_MISC_BUFFER_ALLOW_RAW_VIEWS.0 as u32,
            ..Default::default()
        };
        let mut output = None;
        device.CreateBuffer(&desc, None, Some(&mut output))?;
        let output = output.unwrap();
        let mut readback = None;
        device.CreateBuffer(
            &D3D11_BUFFER_DESC {
                ByteWidth: length,
                Usage: D3D11_USAGE_STAGING,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                ..Default::default()
            },
            None,
            Some(&mut readback),
        )?;
        let mut uav = None;
        device.CreateUnorderedAccessView(
            &output,
            Some(&D3D11_UNORDERED_ACCESS_VIEW_DESC {
                Format: DXGI_FORMAT_R32_TYPELESS,
                ViewDimension: D3D11_UAV_DIMENSION_BUFFER,
                Anonymous: D3D11_UNORDERED_ACCESS_VIEW_DESC_0 {
                    Buffer: D3D11_BUFFER_UAV {
                        FirstElement: 0,
                        NumElements: length / 4,
                        Flags: D3D11_BUFFER_UAV_FLAG_RAW.0 as u32,
                    },
                },
            }),
            Some(&mut uav),
        )?;
        let mut code = None;
        let mut errors = None;
        let result = D3DCompile(
            SHADER.as_ptr().cast(),
            SHADER.len(),
            s!("icc.hlsl"),
            None,
            None,
            s!("main"),
            s!("cs_5_0"),
            D3DCOMPILE_OPTIMIZATION_LEVEL3,
            0,
            &mut code,
            Some(&mut errors),
        );
        if let Err(e) = result {
            let detail = errors
                .map(|e| {
                    String::from_utf8_lossy(std::slice::from_raw_parts(
                        e.GetBufferPointer().cast::<u8>(),
                        e.GetBufferSize(),
                    ))
                    .into_owned()
                })
                .unwrap_or_default();
            return Err(Error::new(e.code(), detail));
        }
        let code = code.unwrap();
        let mut shader = None;
        device.CreateComputeShader(
            std::slice::from_raw_parts(code.GetBufferPointer().cast::<u8>(), code.GetBufferSize()),
            None,
            Some(&mut shader),
        )?;
        let params: [u32; 12] = [
            width as u32,
            height as u32,
            if bt709 { 0.2126f32 } else { 0.299f32 }.to_bits(),
            if bt709 { 0.0722f32 } else { 0.114f32 }.to_bits(),
            1f32.to_bits(),0,1f32.to_bits(),0,0,0,0,0,
        ];
        let mut constant = None;
        device.CreateBuffer(
            &D3D11_BUFFER_DESC {
                ByteWidth: 48,
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                ..Default::default()
            },
            Some(&D3D11_SUBRESOURCE_DATA {
                pSysMem: params.as_ptr().cast(),
                ..Default::default()
            }),
            Some(&mut constant),
        )?;
        let mut sampler = None;
        device.CreateSamplerState(
            &D3D11_SAMPLER_DESC {
                Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
                AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
                AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
                AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
                MaxLOD: f32::MAX,
                ..Default::default()
            },
            Some(&mut sampler),
        )?;
        context.CSSetShader(shader.as_ref().unwrap(), None);
        context.CSSetShaderResources(0, Some(&[sy, suv, sl]));
        context.CSSetConstantBuffers(0, Some(&[constant.clone()]));
        context.CSSetSamplers(0, Some(&[sampler]));
        context.CSSetUnorderedAccessViews(0, 1, Some(&uav), None);
        Ok(Self {
            width,
            height,
            context,
            constant:constant.unwrap(),
            parameters:params,
            y,
            uv,
            output,
            readback: readback.unwrap(),
        })
    }
    pub unsafe fn apply(&self, bytes: &mut [u8], picture:crate::picture::Picture, use_lut:bool) -> Result<()> {
        let ylen = self.width * self.height;
        let total = ylen * 3 / 2;
        if bytes.len() < total {
            return Err(Error::new(E_INVALIDARG, "Short decoded frame"));
        }
        let mut params=self.parameters;
        for (slot,value) in params[4..8].iter_mut().zip(picture.uniform()) {*slot=value.to_bits();}
        params[8]=picture.effect_strength().to_bits();
        params[9]=if use_lut {1f32.to_bits()} else {0};
        self.context.UpdateSubresource(&self.constant,0,None,params.as_ptr().cast(),0,0);
        self.context.UpdateSubresource(
            &self.y,
            0,
            None,
            bytes.as_ptr().cast(),
            self.width as u32,
            0,
        );
        self.context.UpdateSubresource(
            &self.uv,
            0,
            None,
            bytes[ylen..].as_ptr().cast(),
            self.width as u32,
            0,
        );
        self.context.Dispatch(
            (self.width as u32).div_ceil(32),
            (self.height as u32).div_ceil(16),
            1,
        );
        self.context.CopyResource(&self.readback, &self.output);
        let mut map = D3D11_MAPPED_SUBRESOURCE::default();
        self.context
            .Map(&self.readback, 0, D3D11_MAP_READ, 0, Some(&mut map))?;
        std::ptr::copy_nonoverlapping(map.pData.cast::<u8>(), bytes.as_mut_ptr(), total);
        self.context.Unmap(&self.readback, 0);
        Ok(())
    }
}


pub fn available()->bool {unsafe {
    D3D11CreateDevice(None,D3D_DRIVER_TYPE_HARDWARE,None,D3D11_CREATE_DEVICE_FLAG(0),Some(&[D3D_FEATURE_LEVEL_11_0]),D3D11_SDK_VERSION,None,None,None).is_ok()
}}
pub enum Processor {Dx12(crate::icc_dx12::Processor),Dx11(D3d11)}
impl Processor {
    #[cfg(test)]
    pub unsafe fn new(lut:&[[f32;3]],width:usize,height:usize,bt709:bool)->Result<Self> {Self::selected(lut,width,height,bt709,crate::backend::Shader::Dx11)}
    pub unsafe fn selected(lut:&[[f32;3]],width:usize,height:usize,bt709:bool,shader:crate::backend::Shader)->Result<Self> {
        use crate::backend::Shader;
        if matches!(shader,Shader::Auto|Shader::Dx12) {
            match crate::icc_dx12::Processor::new(lut,width,height,bt709) {
                Ok(p)=>return Ok(Self::Dx12(p)),
                Err(e) if shader==Shader::Dx12 || crate::native::graphics_lost(&serde_json::json!({"error":e.to_string()}))=>return Err(e),
                Err(_)=>{}
            }
        }
        if matches!(shader,Shader::Auto|Shader::Dx11) {return D3d11::new(lut,width,height,bt709).map(Self::Dx11);}
        Err(Error::new(E_FAIL,"Shader acceleration is disabled"))
    }
    pub fn name(&self)->&'static str {match self {Self::Dx12(_)=>"DirectX 12 picture processing / ICC",Self::Dx11(_)=>"Direct3D 11 picture processing / ICC"}}
    pub unsafe fn apply(&self,bytes:&mut[u8],picture:crate::picture::Picture,use_lut:bool)->Result<()> {match self {Self::Dx12(p)=>p.apply(bytes,picture,use_lut),Self::Dx11(p)=>p.apply(bytes,picture,use_lut)}}
}
