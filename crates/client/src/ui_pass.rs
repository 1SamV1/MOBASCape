//! The interface's own drawing pass (docs/INTERFACE.md, stage 2): after the
//! scene, over it, with no depth. It is given the atlas once and a list of
//! quads each frame, both from `rift-ui`, and knows nothing of what they
//! show.

use crate::renderer::create_dynamic_buffer;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct UiVertex {
    /// Clip space: -1 to 1 across and up the window.
    position: [f32; 2],
    /// Atlas pixels.
    uv: [f32; 2],
    colour: [f32; 4],
}

pub struct UiPass {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    index_capacity: usize,
    index_count: u32,
    /// Whether the window's colours are stored the way a screen shows them,
    /// in which case the colours given here have to be turned into light.
    srgb: bool,
}

impl UiPass {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        atlas: &rift_ui::Atlas,
    ) -> Self {
        let srgb = format.is_srgb();
        let size = wgpu::Extent3d {
            width: atlas.width,
            height: atlas.height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("interface atlas"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: if srgb {
                wgpu::TextureFormat::Rgba8UnormSrgb
            } else {
                wgpu::TextureFormat::Rgba8Unorm
            },
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * atlas.width),
                rows_per_image: Some(atlas.height),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("interface layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("interface bind group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("interface pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("interface shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("ui_overlay.wgsl").into()),
        });

        const ATTRS: [wgpu::VertexAttribute; 3] =
            wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4];

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("interface pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<UiVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &ATTRS,
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let vertex_capacity = 4096;
        let index_capacity = 6144;
        Self {
            pipeline,
            bind_group,
            vertex_buffer: create_dynamic_buffer(
                device,
                "interface vertices",
                vertex_capacity * std::mem::size_of::<UiVertex>(),
                wgpu::BufferUsages::VERTEX,
            ),
            index_buffer: create_dynamic_buffer(
                device,
                "interface indices",
                index_capacity * std::mem::size_of::<u32>(),
                wgpu::BufferUsages::INDEX,
            ),
            vertex_capacity,
            index_capacity,
            index_count: 0,
            srgb,
        }
    }

    /// This frame's quads, placed in window pixels, for a window this size.
    pub fn set_mesh(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        window: (u32, u32),
        vertices: &[rift_ui::Vertex],
        indices: &[u32],
    ) {
        let (w, h) = (window.0.max(1) as f32, window.1.max(1) as f32);
        let srgb = self.srgb;
        let converted: Vec<UiVertex> = vertices
            .iter()
            .map(|vertex| {
                let c = vertex.colour;
                UiVertex {
                    position: [vertex.pos[0] / w * 2.0 - 1.0, 1.0 - vertex.pos[1] / h * 2.0],
                    uv: vertex.uv,
                    colour: if srgb {
                        [to_light(c[0]), to_light(c[1]), to_light(c[2]), c[3]]
                    } else {
                        c
                    },
                }
            })
            .collect();

        if converted.len() > self.vertex_capacity {
            self.vertex_capacity = converted.len().next_power_of_two();
            self.vertex_buffer = create_dynamic_buffer(
                device,
                "interface vertices",
                self.vertex_capacity * std::mem::size_of::<UiVertex>(),
                wgpu::BufferUsages::VERTEX,
            );
        }
        if indices.len() > self.index_capacity {
            self.index_capacity = indices.len().next_power_of_two();
            self.index_buffer = create_dynamic_buffer(
                device,
                "interface indices",
                self.index_capacity * std::mem::size_of::<u32>(),
                wgpu::BufferUsages::INDEX,
            );
        }
        if !converted.is_empty() {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&converted));
        }
        if !indices.is_empty() {
            queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(indices));
        }
        self.index_count = indices.len() as u32;
    }

    /// Draw over what is already in `view`. Returns whether a pass was
    /// recorded, so the GPU profiler knows if its timestamps are valid.
    pub fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        timestamp_writes: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) -> bool {
        if self.index_count == 0 {
            return false;
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("interface pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.index_count, 0, 0..1);
        true
    }
}

/// A colour as a screen stores it, turned into the amount of light it
/// stands for.
fn to_light(stored: f32) -> f32 {
    if stored <= 0.04045 {
        stored / 12.92
    } else {
        ((stored + 0.055) / 1.055).powf(2.4)
    }
}
