//! Native GPU presentation using the exact Windows reconstruction/picture shader.
use gpu_video::broadcast::Frame;
use wgpu::*;
use crate::picture::Picture;
pub struct Renderer {
    pub device:Device,pub queue:Queue,
    layout:BindGroupLayout,luma_layout:BindGroupLayout,
    pipeline:RenderPipeline,luma_pipeline:RenderPipeline,
    sampler:Sampler,uniform:Buffer,lut:TextureView,lut_texture:Texture,color:bool,
    luma:Option<Texture>,pub output:Option<Texture>,
    pub picture:Picture,pub deinterlace:bool,pub aperture:Option<(u32,u32)>,
}
impl Renderer {
    pub fn new(device:Device,queue:Queue)->Self{
        let mut entries:Vec<_>=(0..4).map(|binding|BindGroupLayoutEntry{binding,visibility:ShaderStages::FRAGMENT,
            ty:BindingType::Texture{sample_type:TextureSampleType::Float{filterable:binding!=3},view_dimension:if binding==3{TextureViewDimension::D3}else{TextureViewDimension::D2},multisampled:false},count:None}).collect();
        entries.extend([
            BindGroupLayoutEntry{binding:4,visibility:ShaderStages::FRAGMENT,ty:BindingType::Sampler(SamplerBindingType::Filtering),count:None},
            BindGroupLayoutEntry{binding:5,visibility:ShaderStages::FRAGMENT,ty:BindingType::Buffer{ty:BufferBindingType::Uniform,has_dynamic_offset:false,min_binding_size:None},count:None},
        ]);
        let mut next=entries[0];next.binding=6;entries.push(next);
        let layout=device.create_bind_group_layout(&BindGroupLayoutDescriptor{label:Some("Native video textures"),entries:&entries});
        let luma_layout=device.create_bind_group_layout(&BindGroupLayoutDescriptor{label:Some("Reconstructed source luma"),entries:&[entries[0]]});
        let pl=device.create_pipeline_layout(&PipelineLayoutDescriptor{label:None,bind_group_layouts:&[Some(&layout),Some(&luma_layout)],immediate_size:0});
        let ll=device.create_pipeline_layout(&PipelineLayoutDescriptor{label:None,bind_group_layouts:&[Some(&layout)],immediate_size:0});
        let shader=device.create_shader_module(ShaderModuleDescriptor{label:Some("Shared Windows/Linux video shader"),source:ShaderSource::Wgsl(include_str!("../../../GUI/shared/video.wgsl").into())});
        let make=|layout:&PipelineLayout,entry:&str,format:TextureFormat|device.create_render_pipeline(&RenderPipelineDescriptor{
            label:Some(entry),layout:Some(layout),vertex:VertexState{module:&shader,entry_point:Some("vs"),compilation_options:Default::default(),buffers:&[]},
            fragment:Some(FragmentState{module:&shader,entry_point:Some(entry),compilation_options:Default::default(),targets:&[Some(ColorTargetState{format,blend:None,write_mask:ColorWrites::ALL})]}),
            primitive:Default::default(),depth_stencil:None,multisample:Default::default(),multiview_mask:None,cache:None});
        let pipeline=make(&pl,"fs",TextureFormat::Rgba8Unorm);let luma_pipeline=make(&ll,"fs_luma",TextureFormat::R16Float);
        let lut=device.create_texture(&TextureDescriptor{label:Some("Monitor transform"),size:Extent3d{width:33,height:33,depth_or_array_layers:33},mip_level_count:1,sample_count:1,dimension:TextureDimension::D3,format:TextureFormat::Rgba32Float,usage:TextureUsages::TEXTURE_BINDING|TextureUsages::COPY_DST,view_formats:&[]});
        let sampler=device.create_sampler(&SamplerDescriptor{mag_filter:FilterMode::Linear,min_filter:FilterMode::Linear,..Default::default()});
        let uniform=device.create_buffer(&BufferDescriptor{label:Some("Shared video parameters"),size:80,usage:BufferUsages::UNIFORM|BufferUsages::COPY_DST,mapped_at_creation:false});
        Self{device,queue,layout,luma_layout,pipeline,luma_pipeline,sampler,uniform,lut:lut.create_view(&Default::default()),lut_texture:lut,color:false,luma:None,output:None,picture:Picture::default(),deinterlace:true,aperture:None}
    }
    pub fn set_color(&mut self,data:Option<&[u8]>)->Result<(),String>{
        if let Some(data)=data{
            let bytes=crate::color::lut(data)?;
            self.queue.write_texture(self.lut_texture.as_image_copy(),&bytes,TexelCopyBufferLayout{offset:0,bytes_per_row:Some(33*16),rows_per_image:Some(33)},self.lut_texture.size());
            self.color=true;
        }else{self.color=false;}Ok(())
    }
    fn target(&self,width:u32,height:u32,format:TextureFormat)->Texture{
        self.device.create_texture(&TextureDescriptor{label:Some("Native video processing"),size:Extent3d{width,height,depth_or_array_layers:1},mip_level_count:1,sample_count:1,dimension:TextureDimension::D2,format,usage:TextureUsages::TEXTURE_BINDING|TextureUsages::RENDER_ATTACHMENT|TextureUsages::COPY_SRC,view_formats:&[]})
    }
    pub fn render(&mut self,frame:&Frame,previous:Option<&Frame>,next:Option<&Frame>,field:u32,size:(u32,u32))->Result<(),String>{
        let w=frame.texture.width();let h=frame.texture.height();
        if w==0||h==0||w%2!=0||h%2!=0||size.0==0||size.1==0||size.0>8192||size.1>8192{return Err("Invalid video dimensions".into());}
        let same=|other:&&Frame|other.texture.size()==frame.texture.size()&&(other.pts-frame.pts).abs()<=frame.duration.max(1)*2;
        let previous=previous.filter(same);let next=next.filter(same);
        if self.luma.as_ref().is_none_or(|t|t.width()!=w||t.height()!=h){self.luma=Some(self.target(w,h,TextureFormat::R16Float));}
        if self.output.as_ref().is_none_or(|t|(t.width(),t.height())!=size){self.output=Some(self.target(size.0,size.1,TextureFormat::Rgba8Unorm));}
        let plane=|f:&Frame,aspect,format|f.texture.create_view(&TextureViewDescriptor{aspect,format:Some(format),..Default::default()});
        let y=plane(frame,TextureAspect::Plane0,TextureFormat::R8Unorm);
        let uv=plane(frame,TextureAspect::Plane1,TextureFormat::Rg8Unorm);
        let previous_view=plane(previous.unwrap_or(frame),TextureAspect::Plane0,TextureFormat::R8Unorm);
        let next_view=plane(next.unwrap_or(frame),TextureAspect::Plane0,TextureFormat::R8Unorm);
        let bind=self.device.create_bind_group(&BindGroupDescriptor{label:Some("Native decoded NV12"),layout:&self.layout,entries:&[
            BindGroupEntry{binding:0,resource:BindingResource::TextureView(&y)},BindGroupEntry{binding:1,resource:BindingResource::TextureView(&uv)},
            BindGroupEntry{binding:2,resource:BindingResource::TextureView(&previous_view)},BindGroupEntry{binding:3,resource:BindingResource::TextureView(&self.lut)},
            BindGroupEntry{binding:4,resource:BindingResource::Sampler(&self.sampler)},BindGroupEntry{binding:5,resource:self.uniform.as_entire_binding()},
            BindGroupEntry{binding:6,resource:BindingResource::TextureView(&next_view)}]});
        let luma=self.luma.as_ref().unwrap().create_view(&Default::default());
        let luma_bind=self.device.create_bind_group(&BindGroupDescriptor{label:None,layout:&self.luma_layout,entries:&[BindGroupEntry{binding:0,resource:BindingResource::TextureView(&luma)}]});
        let deinterlace=self.deinterlace&&frame.interlaced;let parity=if frame.top_first{field&1}else{1-(field&1)};
        let adjust=self.picture.uniform();
        let (crop_x,crop_width)=self.aperture.unwrap_or((0,w));
        let p:[f32;20]=[w as f32,h as f32,w as f32,if frame.full{1.}else{0.},
            if frame.bt709{0.2126}else{0.299},if frame.bt709{0.0722}else{0.114},if deinterlace{1.}else{0.},parity as f32,
            if self.color{1.}else{0.},if previous.is_some(){1.}else{0.},h as f32,if next.is_some(){1.}else{0.},crop_x as f32/w as f32,crop_width as f32/w as f32,self.picture.effect_strength(),0.,adjust[0],adjust[1],adjust[2],adjust[3]];
        self.queue.write_buffer(&self.uniform,0,&p.iter().flat_map(|f|f.to_le_bytes()).collect::<Vec<_>>());
        let mut encoder=self.device.create_command_encoder(&Default::default());
        let output=self.output.as_ref().unwrap().create_view(&Default::default());
        for (view,pipeline,luma_pass) in [(&luma,&self.luma_pipeline,true),(&output,&self.pipeline,false)]{
            if luma_pass&&!deinterlace{continue;}
            let attachments=[Some(RenderPassColorAttachment{view,resolve_target:None,depth_slice:None,ops:Operations{load:LoadOp::Clear(Color::BLACK),store:StoreOp::Store}})];
            let mut pass=encoder.begin_render_pass(&RenderPassDescriptor{label:Some("Native video field"),color_attachments:&attachments,depth_stencil_attachment:None,occlusion_query_set:None,timestamp_writes:None,multiview_mask:None});
            pass.set_pipeline(pipeline);pass.set_bind_group(0,&bind,&[]);if !luma_pass{pass.set_bind_group(1,&luma_bind,&[]);}pass.draw(0..3,0..1);
        }
        self.queue.submit(Some(encoder.finish()));Ok(())
    }
    pub fn screenshot(&self,path:&std::path::Path)->Result<(),Box<dyn std::error::Error>>{
        self.save_texture(self.output.as_ref().ok_or("No rendered video")?,path)
    }
    pub fn save_texture(&self,t:&Texture,path:&std::path::Path)->Result<(),Box<dyn std::error::Error>>{
        let stride=(t.width()*4).div_ceil(256)*256;
        let buffer=self.device.create_buffer(&BufferDescriptor{label:Some("Snapshot"),size:stride as u64*t.height() as u64,usage:BufferUsages::COPY_DST|BufferUsages::MAP_READ,mapped_at_creation:false});
        let mut encoder=self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(t.as_image_copy(),TexelCopyBufferInfo{buffer:&buffer,layout:TexelCopyBufferLayout{offset:0,bytes_per_row:Some(stride),rows_per_image:None}},t.size());
        self.queue.submit(Some(encoder.finish()));
        let (tx,rx)=std::sync::mpsc::channel();buffer.slice(..).map_async(MapMode::Read,move|r|{let _=tx.send(r);});
        self.device.poll(PollType::wait_indefinitely())?;rx.recv()??;
        let mapped=buffer.slice(..).get_mapped_range();let mut bytes=Vec::with_capacity((t.width()*t.height()*4) as usize);
        for row in mapped.chunks_exact(stride as usize){bytes.extend_from_slice(&row[..t.width() as usize*4]);}
        drop(mapped);buffer.unmap();
        if matches!(t.format(),TextureFormat::Bgra8Unorm|TextureFormat::Bgra8UnormSrgb){for p in bytes.chunks_exact_mut(4){p.swap(0,2);}}
        image::save_buffer(path,&bytes,t.width(),t.height(),image::ColorType::Rgba8)?;Ok(())
    }
}
