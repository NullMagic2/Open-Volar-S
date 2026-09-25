//! X11 embedding for the existing GTK video widget and standalone diagnostics.
use libc::{c_char,c_int,c_long,c_uint,c_ulong,c_void};
use raw_window_handle::{RawDisplayHandle,RawWindowHandle,XlibDisplayHandle,XlibWindowHandle};
use std::{ptr::NonNull,sync::Arc};
#[link(name="X11")]
unsafe extern "C"{
    fn XOpenDisplay(name:*const c_char)->*mut c_void;
    fn XCloseDisplay(d:*mut c_void)->c_int;
    fn XDefaultScreen(d:*mut c_void)->c_int;
    fn XDefaultRootWindow(d:*mut c_void)->c_ulong;
    fn XCreateSimpleWindow(d:*mut c_void,parent:c_ulong,x:c_int,y:c_int,w:c_uint,h:c_uint,border:c_uint,border_color:c_ulong,background:c_ulong)->c_ulong;
    fn XStoreName(d:*mut c_void,w:c_ulong,name:*const c_char)->c_int;
    fn XMapWindow(d:*mut c_void,w:c_ulong)->c_int;
    fn XDestroyWindow(d:*mut c_void,w:c_ulong)->c_int;
    fn XFlush(d:*mut c_void)->c_int;
    fn XInternAtom(d:*mut c_void,name:*const c_char,only:c_int)->c_ulong;
    fn XSetWMProtocols(d:*mut c_void,w:c_ulong,atoms:*mut c_ulong,count:c_int)->c_int;
    fn XGetGeometry(d:*mut c_void,w:c_ulong,root:*mut c_ulong,x:*mut c_int,y:*mut c_int,width:*mut c_uint,height:*mut c_uint,border:*mut c_uint,depth:*mut c_uint)->c_int;
    fn XSelectInput(d:*mut c_void,w:c_ulong,mask:c_long)->c_int;
    fn XGetWindowProperty(d:*mut c_void,w:c_ulong,property:c_ulong,offset:c_long,length:c_long,delete:c_int,kind:c_ulong,actual_kind:*mut c_ulong,format:*mut c_int,nitems:*mut c_ulong,after:*mut c_ulong,data:*mut *mut u8)->c_int;
    fn XFree(data:*mut c_void)->c_int;
    fn XPending(d:*mut c_void)->c_int;
    fn XNextEvent(d:*mut c_void,event:*mut c_void)->c_int;
}
pub struct Window{
    pub surface:Option<wgpu::Surface<'static>>,display:NonNull<c_void>,id:c_ulong,owned:bool,delete:c_ulong,obscured:bool,
    overlays:std::collections::BTreeMap<u32,(u64,wgpu::Texture,crate::control::Overlay)>,
    pub canvas:Option<wgpu::Texture>,overlay_pipeline:Option<wgpu::RenderPipeline>,
    config:Option<wgpu::SurfaceConfiguration>,pipeline:Option<wgpu::RenderPipeline>,sampler:Option<wgpu::Sampler>,
}
impl Window{
    pub fn new(instance:&Arc<gpu_video::VulkanInstance>,id:Option<u64>)->Result<Self,Box<dyn std::error::Error>>{unsafe{
        let display=NonNull::new(XOpenDisplay(std::ptr::null())).ok_or("Cannot connect to the desktop X11 display")?;
        let mut window=Self{surface:None,display,id:0,owned:id.is_none(),delete:0,obscured:false,overlays:Default::default(),canvas:None,overlay_pipeline:None,config:None,pipeline:None,sampler:None};
        let d=display.as_ptr();let w=id.unwrap_or_else(||XCreateSimpleWindow(d,XDefaultRootWindow(d),30,30,960,540,0,0,0));window.id=w;
        if w==0{return Err("Cannot create video window".into());}
        XSelectInput(d,w,(1<<16)|(1<<17));
        if window.owned{
            XStoreName(d,w,c"Live TV! — Open Volar S".as_ptr());
            window.delete=XInternAtom(d,c"WM_DELETE_WINDOW".as_ptr(),0);XSetWMProtocols(d,w,&mut window.delete,1);
            XMapWindow(d,w);XFlush(d);
        }
        window.surface=Some(instance.wgpu_instance().create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle{
            raw_display_handle:Some(RawDisplayHandle::Xlib(XlibDisplayHandle::new(Some(display),XDefaultScreen(d)))),
            raw_window_handle:RawWindowHandle::Xlib(XlibWindowHandle::new(w)),
        })?);Ok(window)
    }}
    pub fn closed(&mut self)->bool{unsafe{
        while XPending(self.display.as_ptr())>0{let mut event=[0 as c_long;24];XNextEvent(self.display.as_ptr(),event.as_mut_ptr().cast());
            if event[0] as c_int==15{self.obscured=event[5] as c_int==2;}
            if event[0] as c_int==33&&event[7] as c_ulong==self.delete{return true;}
        }false
    }}
    pub fn monitor_profile(&self)->Option<Vec<u8>>{unsafe{
        let atom=XInternAtom(self.display.as_ptr(),c"_ICC_PROFILE".as_ptr(),1);if atom==0{return None;}
        let(mut kind,mut format,mut count,mut after,mut data)=(0,0,0,0,std::ptr::null_mut());
        let result=XGetWindowProperty(self.display.as_ptr(),XDefaultRootWindow(self.display.as_ptr()),atom,0,4*1024*1024,0,0,&mut kind,&mut format,&mut count,&mut after,&mut data);
        if data.is_null(){return None;}
        let profile=if result==0&&format==8&&after==0&&(128..=16*1024*1024).contains(&count){Some(std::slice::from_raw_parts(data,count as usize).to_vec())}else{None};
        XFree(data.cast());profile
    }}
    pub fn size(&self)->(u32,u32){unsafe{
        let(mut root,mut x,mut y,mut w,mut h,mut border,mut depth)=(0,0,0,0,0,0,0);
        XGetGeometry(self.display.as_ptr(),self.id,&mut root,&mut x,&mut y,&mut w,&mut h,&mut border,&mut depth);(w.max(1),h.max(1))
    }}
    pub fn configure(&mut self,device:&gpu_video::VulkanDevice){
        let d=device.wgpu_device();let surface=self.surface.as_ref().unwrap();let caps=surface.get_capabilities(&device.wgpu_adapter());let(w,h)=self.size();
        let format=caps.formats.iter().copied().find(|f|*f==wgpu::TextureFormat::Bgra8Unorm||*f==wgpu::TextureFormat::Rgba8Unorm).unwrap_or(caps.formats[0]);
        let config=wgpu::SurfaceConfiguration{usage:wgpu::TextureUsages::RENDER_ATTACHMENT,format,width:w,height:h,present_mode:if caps.present_modes.contains(&wgpu::PresentMode::Mailbox){wgpu::PresentMode::Mailbox}else{wgpu::PresentMode::Fifo},desired_maximum_frame_latency:2,alpha_mode:caps.alpha_modes[0],view_formats:vec![]};
        surface.configure(&d,&config);
        let shader=d.create_shader_module(wgpu::ShaderModuleDescriptor{label:Some("Native video canvas"),source:wgpu::ShaderSource::Wgsl(r#"
            @group(0) @binding(0) var picture:texture_2d<f32>;
            @group(0) @binding(1) var linear:sampler;
            struct Vertex{@builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32>};
            @vertex fn vs(@builtin(vertex_index) i:u32)->Vertex{
                var p=array<vec2<f32>,3>(vec2(-1.,-1.),vec2(3.,-1.),vec2(-1.,3.));
                var v:Vertex;v.position=vec4(p[i],0.,1.);v.uv=vec2((p[i].x+1.)*.5,(1.-p[i].y)*.5);return v;
            }
            @fragment fn fs(v:Vertex)->@location(0) vec4<f32>{return textureSample(picture,linear,v.uv);}
        "#.into())});
        self.pipeline=Some(d.create_render_pipeline(&wgpu::RenderPipelineDescriptor{label:Some("Native video canvas"),layout:None,
            vertex:wgpu::VertexState{module:&shader,entry_point:Some("vs"),compilation_options:Default::default(),buffers:&[]},
            fragment:Some(wgpu::FragmentState{module:&shader,entry_point:Some("fs"),compilation_options:Default::default(),targets:&[Some(wgpu::ColorTargetState{format,blend:None,write_mask:wgpu::ColorWrites::ALL})]}),
            primitive:Default::default(),depth_stencil:None,multisample:Default::default(),multiview_mask:None,cache:None}));
        self.overlay_pipeline=Some(d.create_render_pipeline(&wgpu::RenderPipelineDescriptor{label:Some("Native video canvas"),layout:None,
            vertex:wgpu::VertexState{module:&shader,entry_point:Some("vs"),compilation_options:Default::default(),buffers:&[]},
            fragment:Some(wgpu::FragmentState{module:&shader,entry_point:Some("fs"),compilation_options:Default::default(),targets:&[Some(wgpu::ColorTargetState{format,blend:Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),write_mask:wgpu::ColorWrites::ALL})]}),
            primitive:Default::default(),depth_stencil:None,multisample:Default::default(),multiview_mask:None,cache:None}));
        self.sampler=Some(d.create_sampler(&wgpu::SamplerDescriptor{min_filter:wgpu::FilterMode::Linear,mag_filter:wgpu::FilterMode::Linear,..Default::default()}));self.config=Some(config);
    }
    pub fn set_overlays(&mut self,renderer:&crate::video::Renderer,overlays:&std::collections::BTreeMap<u32,crate::control::Overlay>){
        self.overlays.retain(|id,_|overlays.contains_key(id));
        for (&id,overlay) in overlays{
            if self.overlays.get(&id).is_some_and(|(revision,_,_)|*revision==overlay.revision){continue;}
            let texture=renderer.device.create_texture(&wgpu::TextureDescriptor{label:Some("Native caption/OSD"),size:wgpu::Extent3d{width:overlay.width,height:overlay.height,depth_or_array_layers:1},mip_level_count:1,sample_count:1,dimension:wgpu::TextureDimension::D2,format:wgpu::TextureFormat::Bgra8Unorm,usage:wgpu::TextureUsages::TEXTURE_BINDING|wgpu::TextureUsages::COPY_DST,view_formats:&[]});
            renderer.queue.write_texture(texture.as_image_copy(),&overlay.data,wgpu::TexelCopyBufferLayout{offset:0,bytes_per_row:Some(overlay.width*4),rows_per_image:None},texture.size());
            self.overlays.insert(id,(overlay.revision,texture,overlay.clone()));
        }
    }
    pub fn dimensions(&self,aspect:(u32,u32))->serde_json::Value{
        let(w,h)=self.size();let [x,y,vw,vh]=crate::canvas::geometry(aspect,(w,h),(w,h)).0;
        serde_json::json!({"w":w,"h":h,"ml":x,"mr":w-x-vw,"mt":y,"mb":h-y-vh})
    }
    pub fn present(&mut self,renderer:&crate::video::Renderer,aspect:(u32,u32))->Result<bool,String>{
        let size=self.size();let config=self.config.as_mut().ok_or("Unconfigured video surface")?;
        let surface=self.surface.as_ref().unwrap();
        if size!=(config.width,config.height){config.width=size.0;config.height=size.1;surface.configure(&renderer.device,config);}
        if self.canvas.as_ref().is_none_or(|t|(t.width(),t.height())!=size){
            self.canvas=Some(renderer.device.create_texture(&wgpu::TextureDescriptor{label:Some("Video and captions canvas"),size:wgpu::Extent3d{width:size.0,height:size.1,depth_or_array_layers:1},mip_level_count:1,sample_count:1,dimension:wgpu::TextureDimension::D2,format:config.format,usage:wgpu::TextureUsages::TEXTURE_BINDING|wgpu::TextureUsages::RENDER_ATTACHMENT|wgpu::TextureUsages::COPY_SRC,view_formats:&[]}));
        }
        let canvas=self.canvas.as_ref().unwrap().create_view(&Default::default());
        let pipeline=self.pipeline.as_ref().unwrap();let overlay_pipeline=self.overlay_pipeline.as_ref().unwrap();
        let bind=|texture:&wgpu::Texture,pipeline:&wgpu::RenderPipeline|renderer.device.create_bind_group(&wgpu::BindGroupDescriptor{label:None,layout:&pipeline.get_bind_group_layout(0),entries:&[
            wgpu::BindGroupEntry{binding:0,resource:wgpu::BindingResource::TextureView(&texture.create_view(&Default::default()))},wgpu::BindGroupEntry{binding:1,resource:wgpu::BindingResource::Sampler(self.sampler.as_ref().unwrap())}]});
        let source=bind(renderer.output.as_ref().ok_or("No video")?,pipeline);
        let overlays:Vec<_>=self.overlays.values().map(|(_,t,o)|(bind(t,overlay_pipeline),o)).collect();
        let mut encoder=renderer.device.create_command_encoder(&Default::default());
        let [x,y,width,height]=crate::canvas::geometry(aspect,size,size).0;
        {let attachments=[Some(wgpu::RenderPassColorAttachment{view:&canvas,resolve_target:None,depth_slice:None,ops:wgpu::Operations{load:wgpu::LoadOp::Clear(wgpu::Color::BLACK),store:wgpu::StoreOp::Store}})];
            let mut pass=encoder.begin_render_pass(&wgpu::RenderPassDescriptor{label:Some("Video and native captions"),color_attachments:&attachments,depth_stencil_attachment:None,occlusion_query_set:None,timestamp_writes:None,multiview_mask:None});
            pass.set_pipeline(pipeline);pass.set_bind_group(0,&source,&[]);pass.set_viewport(x as f32,y as f32,width as f32,height as f32,0.,1.);pass.draw(0..3,0..1);
            pass.set_pipeline(overlay_pipeline);
            for (bind,overlay) in &overlays{
                if overlay.x.saturating_add(overlay.width)>size.0||overlay.y.saturating_add(overlay.height)>size.1{continue;}
                pass.set_bind_group(0,bind,&[]);pass.set_viewport(overlay.x as f32,overlay.y as f32,overlay.width as f32,overlay.height as f32,0.,1.);pass.draw(0..3,0..1);
            }
        }
        renderer.queue.submit(Some(encoder.finish()));
        if self.obscured{return Ok(false);}
        let target=match surface.get_current_texture(){
            wgpu::CurrentSurfaceTexture::Success(t)|wgpu::CurrentSurfaceTexture::Suboptimal(t)=>t,
            wgpu::CurrentSurfaceTexture::Outdated=>{surface.configure(&renderer.device,config);return Ok(false);},
            wgpu::CurrentSurfaceTexture::Occluded|wgpu::CurrentSurfaceTexture::Timeout=>return Ok(false),
            other=>return Err(format!("Native video surface: {other:?}")),
        };
        let view=target.texture.create_view(&Default::default());let output=bind(self.canvas.as_ref().unwrap(),pipeline);
        let mut encoder=renderer.device.create_command_encoder(&Default::default());
        {let attachments=[Some(wgpu::RenderPassColorAttachment{view:&view,resolve_target:None,depth_slice:None,ops:wgpu::Operations{load:wgpu::LoadOp::Clear(wgpu::Color::BLACK),store:wgpu::StoreOp::Store}})];
            let mut pass=encoder.begin_render_pass(&wgpu::RenderPassDescriptor{label:Some("Native canvas presentation"),color_attachments:&attachments,depth_stencil_attachment:None,occlusion_query_set:None,timestamp_writes:None,multiview_mask:None});
            pass.set_pipeline(pipeline);pass.set_bind_group(0,&output,&[]);pass.draw(0..3,0..1);
        }
        renderer.queue.submit(Some(encoder.finish()));target.present();Ok(true)
    }

}
impl Drop for Window{fn drop(&mut self){self.surface.take();unsafe{if self.owned&&self.id!=0{XDestroyWindow(self.display.as_ptr(),self.id);}XCloseDisplay(self.display.as_ptr());}}}
