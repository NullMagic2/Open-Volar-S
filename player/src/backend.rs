//! Portable backend identifiers. Platform implementations own capability checks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Backend { #[default] Vulkan, Microsoft }
impl Backend {
    pub fn id(self) -> &'static str { match self {Self::Vulkan=>"vulkan",Self::Microsoft=>"microsoft"} }
    pub fn label(self) -> &'static str { match self {Self::Vulkan=>"Vulkan GPU",Self::Microsoft=>"Microsoft processing"} }
    pub fn choices(microsoft: bool) -> Vec<Self> {
        let mut choices=vec![Self::Vulkan]; if microsoft {choices.push(Self::Microsoft);} choices
    }
    pub fn load(id: Option<&str>, microsoft: bool) -> Self {
        if id==Some("microsoft") && microsoft {Self::Microsoft} else {Self::Vulkan}
    }
}
pub fn microsoft_available() -> bool {
    #[cfg(windows)] {crate::native::microsoft_available()}
    #[cfg(not(windows))] {false}
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn choices_and_saved_values_follow_platform_capabilities() {
        assert_eq!(Backend::choices(false),vec![Backend::Vulkan]);
        assert_eq!(Backend::choices(true),vec![Backend::Vulkan,Backend::Microsoft]);
        assert_eq!(Backend::load(Some("microsoft"),false),Backend::Vulkan);
        assert_eq!(Backend::load(Some("microsoft"),true),Backend::Microsoft);
        assert_eq!(Backend::load(None,true),Backend::Vulkan);
        assert_eq!(Backend::load(Some("unknown"),true),Backend::Vulkan);
        for b in Backend::choices(true) {assert_eq!(Backend::load(Some(b.id()),true),b);}
    }
}


pub const SHADER_CONTROL:u16=365;
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
pub enum Shader { #[default] Auto, Vulkan, Dx12, Dx11, Off }
#[derive(Clone,Copy,Debug,Default)]
pub struct Capabilities {pub vulkan:bool,pub dx12:bool,pub dx11:bool}
impl Shader {
    pub fn id(self)->&'static str {match self {Self::Auto=>"auto",Self::Vulkan=>"vulkan",Self::Dx12=>"dx12",Self::Dx11=>"dx11",Self::Off=>"off"}}
    pub fn label(self)->&'static str {match self {Self::Auto=>"Automatic",Self::Vulkan=>"Vulkan",Self::Dx12=>"DirectX 12",Self::Dx11=>"DirectX 11",Self::Off=>"Off (CPU)"}}
    pub fn load(id:Option<&str>)->Self {match id {Some("vulkan")=>Self::Vulkan,Some("dx12")=>Self::Dx12,Some("dx11")=>Self::Dx11,Some("off")=>Self::Off,_=>Self::Auto}}
    pub fn choices(backend:Backend,c:Capabilities)->Vec<Self> {
        // Cross-API Vulkan readback was measured and rejected for its frame cost.
        if backend==Backend::Vulkan {return vec![Self::Auto,Self::Vulkan];}
        let mut v=vec![Self::Auto];
        if c.vulkan {v.push(Self::Vulkan);} if c.dx12 {v.push(Self::Dx12);} if c.dx11 {v.push(Self::Dx11);} v.push(Self::Off);v
    }
    pub fn compatible(self,backend:Backend,c:Capabilities)->Self {if Self::choices(backend,c).contains(&self){self}else{Self::Auto}}
    pub fn effects(self,_backend:Backend,c:Capabilities)->bool {match self {Self::Auto=>c.vulkan||c.dx12||c.dx11,Self::Vulkan=>c.vulkan,Self::Dx12=>c.dx12,Self::Dx11=>c.dx11,Self::Off=>false}}
}
pub fn capabilities()->Capabilities {
    static C:std::sync::OnceLock<Capabilities>=std::sync::OnceLock::new();
    *C.get_or_init(|| {
        let instance=wgpu::Instance::new(wgpu::InstanceDescriptor{backends:wgpu::Backends::VULKAN|wgpu::Backends::DX12,..wgpu::InstanceDescriptor::new_without_display_handle()});
        let mut c=Capabilities::default();
        for a in pollster::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN|wgpu::Backends::DX12)) {
            let info=a.get_info();if info.device_type==wgpu::DeviceType::Cpu {continue;}
            match info.backend {wgpu::Backend::Vulkan=>c.vulkan=true,wgpu::Backend::Dx12=>c.dx12=true,_=>{}}
        }
        #[cfg(windows)] {c.dx11=crate::icc_gpu::available();}
        c
    })
}
#[cfg(test)] mod shader_tests {
    use super::*;
    #[test] fn manual_shader_choices_are_capability_filtered_and_saved() {
        let all=Capabilities{vulkan:true,dx12:true,dx11:true};
        let choices=Shader::choices(Backend::Microsoft,all);
        assert_eq!(choices,vec![Shader::Auto,Shader::Vulkan,Shader::Dx12,Shader::Dx11,Shader::Off]);
        for s in choices {assert_eq!(Shader::load(Some(s.id())),s);}
        assert_eq!(Shader::choices(Backend::Microsoft,Capabilities::default()),vec![Shader::Auto,Shader::Off]);
        assert_eq!(Shader::choices(Backend::Vulkan,all),vec![Shader::Auto,Shader::Vulkan]);
        assert_eq!(Shader::Off.compatible(Backend::Vulkan,all),Shader::Auto);
        assert!(!Shader::Off.effects(Backend::Microsoft,all));
        assert!(!Shader::Auto.effects(Backend::Microsoft,Capabilities::default()));
        assert!(Shader::Auto.effects(Backend::Microsoft,all));
    }
}
