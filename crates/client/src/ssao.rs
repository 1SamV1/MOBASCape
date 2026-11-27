//! Screen-space ambient occlusion: soft contact shade where geometry meets,
//! under trees, at wall feet, round statues and units (shader `ssao.wgsl`).
//!
//! Three half-resolution passes after the scene: linear depth into
//! `linear`, the occlusion test into `raw` (AO in red, view depth in green
//! for the blur), then a depth-aware 4x4 blur into `out`, which holds the
//! final multiplier
//! (`mix(1, ao, strength)`). The post pass multiplies the lit scene by it
//! before tone mapping. Strength 0 skips both passes and clears `out` to 1.

use glam::Mat4;

/// AO target format: occlusion plus the view depth the blur weighs by.
const RAW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;
/// Half-resolution linear view distance.
const LINEAR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;
/// The multiplier the post pass samples.
pub const OUT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SsaoUniform {
    proj: [[f32; 4]; 4],
    inv_proj: [[f32; 4]; 4],
    params: [f32; 4],
    size: [f32; 4],
}

/// Tuning from `data/lighting.json`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SsaoSettings {
    /// How far the hemisphere reaches, in world units (a tile is 128).
    pub radius: f32,
    /// 0 = off, 1 = full occlusion.
    pub strength: f32,
    /// Contrast: values above 1 deepen the creases.
    pub power: f32,
}

pub struct Ssao {
    layout: wgpu::BindGroupLayout,
    depth_pipeline: wgpu::RenderPipeline,
    ao_pipeline: wgpu::RenderPipeline,
    blur_pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    targets: Targets,
    depth_size: (u32, u32),
    out_size: (u32, u32),
    settings: SsaoSettings,
    proj: Mat4,
}

impl Ssao {
    pub fn new(
        device: &wgpu::Device,
        depth_view: &wgpu::TextureView,
        multisampled: bool,
        size: (u32, u32),
        settings: SsaoSettings,
    ) -> Self {
        let depth_type = if multisampled {
            "texture_depth_multisampled_2d"
        } else {
            "texture_depth_2d"
        };
        let source = include_str!("ssao.wgsl").replace("DEPTH_TYPE", depth_type);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rift ssao shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssao layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ssao pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |entry: &str, format: wgpu::TextureFormat, label: &str| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let depth_pipeline = pipeline("fs_depth", LINEAR_FORMAT, "ssao depth pipeline");
        let ao_pipeline = pipeline("fs_ao", RAW_FORMAT, "ssao pipeline");
        let blur_pipeline = pipeline("fs_blur", OUT_FORMAT, "ssao blur pipeline");
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ssao uniform"),
            size: std::mem::size_of::<SsaoUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let targets = Targets::new(device, &layout, &uniform, depth_view, size);
        let out_size = targets.size;
        Self {
            layout,
            depth_pipeline,
            ao_pipeline,
            blur_pipeline,
            uniform,
            targets,
            depth_size: size,
            out_size,
            settings,
            proj: Mat4::IDENTITY,
        }
    }

    /// New render size: the depth buffer was recreated.
    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        depth_view: &wgpu::TextureView,
        size: (u32, u32),
    ) {
        self.targets = Targets::new(device, &self.layout, &self.uniform, depth_view, size);
        self.depth_size = size;
        self.out_size = self.targets.size;
    }

    pub fn set_settings(&mut self, settings: SsaoSettings) {
        self.settings = settings;
    }

    pub fn enabled(&self) -> bool {
        self.settings.strength > 0.0
    }

    /// The multiplier texture the post pass samples.
    pub fn output_view(&self) -> &wgpu::TextureView {
        &self.targets.out_view
    }

    /// Upload this frame's projection and tuning.
    pub fn update(&mut self, queue: &wgpu::Queue, proj: Mat4) {
        self.proj = proj;
        let s = self.settings;
        let uniform = SsaoUniform {
            proj: proj.to_cols_array_2d(),
            inv_proj: proj.inverse().to_cols_array_2d(),
            params: [s.radius, s.strength, s.radius * 0.04, s.power],
            size: [
                self.depth_size.0 as f32,
                self.depth_size.1 as f32,
                self.out_size.0 as f32,
                self.out_size.1 as f32,
            ],
        };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&uniform));
    }

    /// Record the occlusion and blur passes (or a clear to 1 when off).
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        fn attachment(
            view: &wgpu::TextureView,
            clear: wgpu::Color,
        ) -> Option<wgpu::RenderPassColorAttachment<'_>> {
            Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    store: wgpu::StoreOp::Store,
                },
            })
        }
        if !self.enabled() {
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ssao off"),
                color_attachments: &[attachment(&self.targets.out_view, wgpu::Color::WHITE)],
                depth_stencil_attachment: None,
                timestamp_writes: timestamps,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            return;
        }
        // The timer slot spans both passes: begin on the first, end on the
        // blur.
        let begin = timestamps
            .as_ref()
            .map(|t| wgpu::RenderPassTimestampWrites {
                query_set: t.query_set,
                beginning_of_pass_write_index: t.beginning_of_pass_write_index,
                end_of_pass_write_index: None,
            });
        let end = timestamps
            .as_ref()
            .map(|t| wgpu::RenderPassTimestampWrites {
                query_set: t.query_set,
                beginning_of_pass_write_index: None,
                end_of_pass_write_index: t.end_of_pass_write_index,
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ssao depth pass"),
                color_attachments: &[attachment(&self.targets.linear_view, wgpu::Color::WHITE)],
                depth_stencil_attachment: None,
                timestamp_writes: begin,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.depth_pipeline);
            pass.set_bind_group(0, &self.targets.depth_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ssao pass"),
                color_attachments: &[attachment(&self.targets.raw_view, wgpu::Color::WHITE)],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.ao_pipeline);
            pass.set_bind_group(0, &self.targets.ao_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ssao blur pass"),
            color_attachments: &[attachment(&self.targets.out_view, wgpu::Color::WHITE)],
            depth_stencil_attachment: None,
            timestamp_writes: end,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.blur_pipeline);
        pass.set_bind_group(0, &self.targets.blur_bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// The half-resolution targets and the bind groups that chain them.
struct Targets {
    linear_view: wgpu::TextureView,
    raw_view: wgpu::TextureView,
    out_view: wgpu::TextureView,
    depth_bind_group: wgpu::BindGroup,
    ao_bind_group: wgpu::BindGroup,
    blur_bind_group: wgpu::BindGroup,
    size: (u32, u32),
}

impl Targets {
    fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        uniform: &wgpu::Buffer,
        depth_view: &wgpu::TextureView,
        size: (u32, u32),
    ) -> Self {
        let out_size = (size.0.div_ceil(2).max(1), size.1.div_ceil(2).max(1));
        let target = |format: wgpu::TextureFormat, label: &str| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: out_size.0,
                        height: out_size.1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        let linear_view = target(LINEAR_FORMAT, "ssao linear depth");
        let raw_view = target(RAW_FORMAT, "ssao raw");
        let out_view = target(OUT_FORMAT, "ssao out");
        let group = |previous: &wgpu::TextureView, label: &str| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(depth_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(previous),
                    },
                ],
            })
        };
        // The depth pass never reads binding 2; bind a texture it doesn't
        // write so no pass reads and writes the same target.
        let depth_bind_group = group(&out_view, "ssao depth bind group");
        let ao_bind_group = group(&linear_view, "ssao bind group");
        let blur_bind_group = group(&raw_view, "ssao blur bind group");
        Self {
            linear_view,
            raw_view,
            out_view,
            depth_bind_group,
            ao_bind_group,
            blur_bind_group,
            size: out_size,
        }
    }
}
