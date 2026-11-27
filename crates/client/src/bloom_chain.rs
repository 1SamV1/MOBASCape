//! Mip-chain bloom (shader `bloom_chain.wgsl`).
//!
//! The bright pass leaves the parts of the HDR scene above 1.0 in a small
//! target. This chain halves that image five times with a 13-tap filter,
//! then walks back up with a tent filter, adding each level onto the one
//! above. A spell's glow therefore gets a tight core plus wide, soft rings
//! instead of a single small blur, and the Karis average on the first step
//! keeps lone bright pixels from flickering. The result lands in the bloom
//! target the post pass already reads.

use wgpu::util::DeviceExt;

/// Halvings below the bright-pass image.
const LEVELS: usize = 5;
/// What the summed chain and the sharp bright image each add to the final
/// glow. The five-level sum carries about five times the energy of one
/// blur, so it is scaled down to keep the overall glow near the old one.
const CHAIN_WEIGHT: f32 = 0.17;
const CORE_WEIGHT: f32 = 0.2;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ChainUniform {
    params: [f32; 4],
}

enum Target {
    Level(usize),
    Final,
}

struct Pass {
    bind_group: wgpu::BindGroup,
    _uniform: wgpu::Buffer,
    target: Target,
    up: bool,
    clear: bool,
}

pub struct BloomChain {
    down_pipeline: wgpu::RenderPipeline,
    up_pipeline: wgpu::RenderPipeline,
    _levels: Vec<wgpu::Texture>,
    level_views: Vec<wgpu::TextureView>,
    passes: Vec<Pass>,
}

impl BloomChain {
    /// `bright` is the bright-pass target of `bright_size`.
    pub fn new(
        device: &wgpu::Device,
        bright: &wgpu::TextureView,
        bright_size: (u32, u32),
        sampler: &wgpu::Sampler,
    ) -> Self {
        let format = wgpu::TextureFormat::Rgba16Float;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rift bloom chain shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("bloom_chain.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom chain layout"),
            entries: &[
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom chain pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |entry: &str, blend: Option<wgpu::BlendState>, label: &str| {
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
                        blend,
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
        let add = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        let down_pipeline = pipeline("fs_down", None, "bloom down pipeline");
        let up_pipeline = pipeline(
            "fs_up",
            Some(wgpu::BlendState {
                color: add,
                alpha: add,
            }),
            "bloom up pipeline",
        );

        let mut sizes = Vec::with_capacity(LEVELS);
        let mut size = bright_size;
        for _ in 0..LEVELS {
            size = (size.0.div_ceil(2).max(1), size.1.div_ceil(2).max(1));
            sizes.push(size);
        }
        let levels: Vec<wgpu::Texture> = sizes
            .iter()
            .enumerate()
            .map(|(i, &(width, height))| {
                device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(&format!("bloom level {}", i + 1)),
                    size: wgpu::Extent3d {
                        width,
                        height,
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
            })
            .collect();
        let level_views: Vec<wgpu::TextureView> = levels
            .iter()
            .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()))
            .collect();

        let make = |source: &wgpu::TextureView,
                    source_size: (u32, u32),
                    weight: f32,
                    karis: bool,
                    target: Target,
                    up: bool,
                    clear: bool| {
            let uniform = ChainUniform {
                params: [
                    1.0 / source_size.0 as f32,
                    1.0 / source_size.1 as f32,
                    weight,
                    if karis { 1.0 } else { 0.0 },
                ],
            };
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bloom chain uniform"),
                contents: bytemuck::bytes_of(&uniform),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bloom chain bind group"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: buffer.as_entire_binding(),
                    },
                ],
            });
            Pass {
                bind_group,
                _uniform: buffer,
                target,
                up,
                clear,
            }
        };

        let mut passes = Vec::new();
        // Down: bright -> level 1 (Karis) -> ... -> level LEVELS.
        passes.push(make(
            bright,
            bright_size,
            1.0,
            true,
            Target::Level(0),
            false,
            true,
        ));
        for i in 1..LEVELS {
            passes.push(make(
                &level_views[i - 1],
                sizes[i - 1],
                1.0,
                false,
                Target::Level(i),
                false,
                true,
            ));
        }
        // Up: add each level onto the one above it.
        for i in (1..LEVELS).rev() {
            passes.push(make(
                &level_views[i],
                sizes[i],
                1.0,
                false,
                Target::Level(i - 1),
                true,
                false,
            ));
        }
        // Final: the summed chain plus the sharp core.
        passes.push(make(
            &level_views[0],
            sizes[0],
            CHAIN_WEIGHT,
            false,
            Target::Final,
            true,
            true,
        ));
        passes.push(make(
            bright,
            bright_size,
            CORE_WEIGHT,
            false,
            Target::Final,
            true,
            false,
        ));

        Self {
            down_pipeline,
            up_pipeline,
            _levels: levels,
            level_views,
            passes,
        }
    }

    /// Record the chain into `final_view` (the post pass's bloom input).
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        final_view: &wgpu::TextureView,
        down_timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
        up_timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        let last_down = self.passes.iter().rposition(|p| !p.up);
        let first_up = self.passes.iter().position(|p| p.up);
        let last = self.passes.len().checked_sub(1);
        for (index, pass) in self.passes.iter().enumerate() {
            let view = match pass.target {
                Target::Level(i) => &self.level_views[i],
                Target::Final => final_view,
            };
            // Each timer slot spans its whole run of passes: the begin
            // stamp on the first, the end stamp on the last.
            fn span<'a>(
                ts: &Option<wgpu::RenderPassTimestampWrites<'a>>,
                begin: bool,
                end: bool,
            ) -> Option<wgpu::RenderPassTimestampWrites<'a>> {
                ts.as_ref().map(|t| wgpu::RenderPassTimestampWrites {
                    query_set: t.query_set,
                    beginning_of_pass_write_index: t
                        .beginning_of_pass_write_index
                        .filter(|_| begin),
                    end_of_pass_write_index: t.end_of_pass_write_index.filter(|_| end),
                })
            }
            let timestamps = if pass.up {
                span(&up_timestamps, Some(index) == first_up, Some(index) == last)
            } else {
                span(&down_timestamps, index == 0, Some(index) == last_down)
            }
            .filter(|t| {
                t.beginning_of_pass_write_index.is_some() || t.end_of_pass_write_index.is_some()
            });
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(if pass.up {
                    "bloom up pass"
                } else {
                    "bloom down pass"
                }),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: if pass.clear {
                            wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                        } else {
                            wgpu::LoadOp::Load
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: timestamps,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(if pass.up {
                &self.up_pipeline
            } else {
                &self.down_pipeline
            });
            rp.set_bind_group(0, &pass.bind_group, &[]);
            rp.draw(0..3, 0..1);
        }
    }
}
