//! DirectX 12 compute counterpart of the Vulkan picture pipeline.
use windows::{core::{Error,Result},Win32::Foundation::E_FAIL};
use std::{sync::mpsc,time::Duration};
fn error(e:impl std::fmt::Display)->Error {Error::new(E_FAIL,e.to_string())}
pub struct Processor {
    device:wgpu::Device,queue:wgpu::Queue,pipeline:wgpu::ComputePipeline,bind:wgpu::BindGroup,
    input:wgpu::Buffer,output:wgpu::Buffer,readback:wgpu::Buffer,uniform:wgpu::Buffer,
    width:usize,height:usize,bt709:bool,lost:std::sync::Arc<std::sync::Mutex<Option<String>>>,
}
fn bytes<T>(v:&[T])->&[u8] {unsafe {std::slice::from_raw_parts(v.as_ptr().cast(),std::mem::size_of_val(v))}}
impl Processor {
    pub fn new(lut:&[[f32;3]],width:usize,height:usize,bt709:bool)->Result<Self> {pollster::block_on(Self::create(lut,width,height,bt709))}
    async fn create(lut:&[[f32;3]],width:usize,height:usize,bt709:bool)->Result<Self> {
        if width==0 || height==0 || width%4!=0 || height%2!=0 {return Err(error("Unsupported NV12 layout for DirectX 12"));}
        let instance=wgpu::Instance::new(wgpu::InstanceDescriptor{backends:wgpu::Backends::DX12,..wgpu::InstanceDescriptor::new_without_display_handle()});
        let adapter=instance.request_adapter(&wgpu::RequestAdapterOptions{power_preference:wgpu::PowerPreference::HighPerformance,force_fallback_adapter:false,compatible_surface:None}).await.map_err(error)?;
        if adapter.get_info().device_type==wgpu::DeviceType::Cpu {return Err(error("No hardware DirectX 12 adapter"));}
        let (device,queue)=adapter.request_device(&wgpu::DeviceDescriptor{label:Some("Live TV picture effects"),..Default::default()}).await.map_err(error)?;
        let lost=std::sync::Arc::new(std::sync::Mutex::new(None));let report=lost.clone();
        device.set_device_lost_callback(move|reason,message|{if reason!=wgpu::DeviceLostReason::Destroyed {*report.lock().unwrap()=Some(format!("Graphics device lost: {message}"));}});
        let scope=device.push_error_scope(wgpu::ErrorFilter::Validation);
        let size=(width*height*3/2) as u64;
        let buffer=|label,sz,usage|device.create_buffer(&wgpu::BufferDescriptor{label:Some(label),size:sz,usage,mapped_at_creation:false});
        let input=buffer("NV12 source",size,wgpu::BufferUsages::STORAGE|wgpu::BufferUsages::COPY_DST);
        let output=buffer("NV12 processed",size,wgpu::BufferUsages::STORAGE|wgpu::BufferUsages::COPY_SRC);
        let readback=buffer("NV12 readback",size,wgpu::BufferUsages::MAP_READ|wgpu::BufferUsages::COPY_DST);
        let uniform=buffer("Picture parameters",48,wgpu::BufferUsages::UNIFORM|wgpu::BufferUsages::COPY_DST);
        let rgba:Vec<[f32;4]>=lut.iter().map(|v|[v[0],v[1],v[2],1.]).collect();
        let lut_buffer=buffer("ICC lookup",std::mem::size_of_val(rgba.as_slice())as u64,wgpu::BufferUsages::STORAGE|wgpu::BufferUsages::COPY_DST);
        queue.write_buffer(&lut_buffer,0,bytes(&rgba));
        let shader=device.create_shader_module(wgpu::ShaderModuleDescriptor{label:Some("Picture and ICC"),source:wgpu::ShaderSource::Wgsl(include_str!("picture_compute.wgsl").into())});
        let pipeline=device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{label:Some("Picture and ICC"),layout:None,module:&shader,entry_point:Some("main"),compilation_options:Default::default(),cache:None});
        let bind=device.create_bind_group(&wgpu::BindGroupDescriptor{label:Some("Picture buffers"),layout:&pipeline.get_bind_group_layout(0),entries:&[
            wgpu::BindGroupEntry{binding:0,resource:input.as_entire_binding()},wgpu::BindGroupEntry{binding:1,resource:output.as_entire_binding()},wgpu::BindGroupEntry{binding:2,resource:uniform.as_entire_binding()},wgpu::BindGroupEntry{binding:3,resource:lut_buffer.as_entire_binding()}]});
        if let Some(e)=scope.pop().await {return Err(error(e));}
        Ok(Self{device,queue,pipeline,bind,input,output,readback,uniform,width,height,bt709,lost})
    }
    pub fn apply(&self,data:&mut[u8],picture:crate::picture::Picture,use_lut:bool)->Result<()> {
        if let Some(e)=self.lost.lock().unwrap().clone(){return Err(error(e));}
        let len=self.width*self.height*3/2;if data.len()<len {return Err(error("Short NV12 input"));}
        let (kr,kb)=if self.bt709{(0.2126f32,0.0722f32)}else{(0.299,0.114)};
        let mut params=[0u32;12];params[0]=self.width as u32;params[1]=self.height as u32;params[2]=kr.to_bits();params[3]=kb.to_bits();
        for (out,v) in params[4..8].iter_mut().zip(picture.uniform()){*out=v.to_bits();}params[8]=picture.effect_strength().to_bits();params[9]=(if use_lut{1f32}else{0.}).to_bits();
        self.queue.write_buffer(&self.input,0,&data[..len]);self.queue.write_buffer(&self.uniform,0,bytes(&params));
        let mut encoder=self.device.create_command_encoder(&Default::default());
        {let mut pass=encoder.begin_compute_pass(&Default::default());pass.set_pipeline(&self.pipeline);pass.set_bind_group(0,&self.bind,&[]);pass.dispatch_workgroups((self.width as u32).div_ceil(32),(self.height as u32).div_ceil(16),1);}
        encoder.copy_buffer_to_buffer(&self.output,0,&self.readback,0,len as u64);
        let index=self.queue.submit([encoder.finish()]);
        let (tx,rx)=mpsc::sync_channel(1);self.readback.slice(..).map_async(wgpu::MapMode::Read,move|r|{let _=tx.send(r);});
        self.device.poll(wgpu::PollType::Wait{submission_index:Some(index),timeout:Some(Duration::from_secs(2))}).map_err(error)?;
        rx.recv_timeout(Duration::from_secs(2)).map_err(error)?.map_err(error)?;
        {let view=self.readback.slice(..).get_mapped_range();data[..len].copy_from_slice(&view[..len]);}self.readback.unmap();Ok(())
    }
}
