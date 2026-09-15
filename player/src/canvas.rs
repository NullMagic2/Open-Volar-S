//! Processing resolution is independent of the size of the viewing area.
pub fn geometry(source: (u32, u32), window: (u32, u32), cap: (u32, u32)) -> ([u32; 4], (u32, u32)) {
    let fit = (window.0 as f64 / source.0 as f64).min(window.1 as f64 / source.1 as f64);
    let width = (source.0 as f64 * fit).round().max(1.) as u32;
    let height = (source.1 as f64 * fit).round().max(1.) as u32;
    let processing = fit
        .min(cap.0 as f64 / source.0 as f64)
        .min(cap.1 as f64 / source.1 as f64);
    (
        [
            (window.0 - width) / 2,
            (window.1 - height) / 2,
            width,
            height,
        ],
        (
            (source.0 as f64 * processing).round().max(1.) as u32,
            (source.1 as f64 * processing).round().max(1.) as u32,
        ),
    )
}
const SHADER: &str = r#"
@group(0) @binding(0) var picture:texture_2d<f32>;
@group(0) @binding(1) var linear:sampler;
@group(0) @binding(2) var<uniform> output_white:vec4<f32>;
struct Vertex { @builtin(position) position:vec4<f32>, @location(0) uv:vec2<f32> };
@vertex fn vs(@builtin(vertex_index) i:u32)->Vertex {
    var positions=array<vec2<f32>,3>(vec2(-1.,-1.),vec2(3.,-1.),vec2(-1.,3.));
    var out:Vertex;out.position=vec4(positions[i],0.,1.);out.uv=vec2((positions[i].x+1.)*.5,(1.-positions[i].y)*.5);return out;
}
@fragment fn fs(in:Vertex)->@location(0) vec4<f32>{return textureSampleLevel(picture,linear,in.uv,0.);}
@fragment fn fs_hdr(in:Vertex)->@location(0) vec4<f32>{
    let rgb=max(textureSampleLevel(picture,linear,in.uv,0.).rgb,vec3(0.));
    let decoded=select(pow((rgb+vec3(0.055))/1.055,vec3(2.4)),rgb/12.92,rgb<=vec3(0.04045));
    return vec4(decoded*output_white.x,1.);
}
"#;
pub struct Canvas {
    pipeline: wgpu::RenderPipeline,
    hdr_pipeline:Option<wgpu::RenderPipeline>,
    white:wgpu::Buffer,
    layout: wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
    target: Option<(wgpu::Texture, wgpu::TextureView, wgpu::BindGroup)>,
    pub size: (u32, u32),
}
impl Canvas {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, hdr:bool) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Canvas"),
            entries: &[
                wgpu::BindGroupLayoutEntry{binding:2,visibility:wgpu::ShaderStages::FRAGMENT,ty:wgpu::BindingType::Buffer{ty:wgpu::BufferBindingType::Uniform,has_dynamic_offset:false,min_binding_size:None},count:None},
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fit video to canvas"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let make_pipeline=|format,entry|device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Canvas presentation"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let pipeline=make_pipeline(format,"fs");
        let hdr_pipeline=hdr.then(||make_pipeline(wgpu::TextureFormat::Rgba16Float,"fs_hdr"));
        let white=device.create_buffer(&wgpu::BufferDescriptor{label:Some("Video reference white"),size:16,usage:wgpu::BufferUsages::UNIFORM|wgpu::BufferUsages::COPY_DST,mapped_at_creation:false});
        Self {
            pipeline,hdr_pipeline,white,
            layout,
            format,
            target: None,
            size: (0, 0),
        }
    }
    /// A pool slot shares immutable pipeline objects but owns its target image.
    pub fn empty_slot(&self) -> Self {
        Self { pipeline:self.pipeline.clone(), hdr_pipeline:self.hdr_pipeline.clone(), white:self.white.clone(), layout:self.layout.clone(), format:self.format,
            target:None, size:(0,0) }
    }
    pub fn ensure(&mut self, device: &wgpu::Device, sampler: &wgpu::Sampler, size: (u32, u32)) {
        if self.size == size {
            return;
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Processed video"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry{binding:2,resource:self.white.as_entire_binding()},
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        self.target = Some((texture, view, bind));
        self.size = size;
    }
    pub fn view(&self) -> wgpu::TextureView {
        self.target
            .as_ref()
            .unwrap()
            .1
            .clone()
    }
    pub fn set_white(&self,queue:&wgpu::Queue,scale:f32){queue.write_buffer(&self.white,0,&scale.to_le_bytes());}
    pub fn present(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: [u32; 4],
        hdr:bool,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Fit processed video"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        pass.set_pipeline(if hdr{self.hdr_pipeline.as_ref().expect("HDR capability checked before configuration")}else{&self.pipeline});
        pass.set_bind_group(0, &self.target.as_ref().unwrap().2, &[]);
        pass.set_viewport(
            viewport[0] as f32,
            viewport[1] as f32,
            viewport[2] as f32,
            viewport[3] as f32,
            0.,
            1.,
        );
        pass.draw(0..3, 0..1);
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn presentation_shader_validates(){let m=naga::front::wgsl::parse_str(super::SHADER).unwrap();naga::valid::Validator::new(naga::valid::ValidationFlags::all(),naga::valid::Capabilities::all()).validate(&m).unwrap();}
    #[test]
    fn all_processing_resolutions_fill_fullscreen() {
        for cap in [(1920, 1080), (2560, 1440), (3840, 2160)] {
            let (viewport, processing) = super::geometry((1920, 1080), (3840, 2160), cap);
            assert_eq!(viewport, [0, 0, 3840, 2160]);
            assert_eq!(processing, cap);
        }
        assert_eq!(
            super::geometry((1920, 1080), (1280, 1024), (3840, 2160)).0,
            [0, 152, 1280, 720]
        );
    }
}
