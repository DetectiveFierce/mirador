use wgpu::{
    self, BindGroup, BindGroupLayout, BufferUsages, ColorTargetState, ColorWrites, Device,
    FragmentState, MultisampleState, PrimitiveState, RenderPass, RenderPipeline,
    SamplerBindingType, ShaderStages, Texture, TextureFormat, TextureView, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, util::DeviceExt,
};

use std::collections::HashMap;
use std::mem;
use std::path::Path;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct IconVertex {
    position: [f32; 2],
    uv: [f32; 2],
}

impl IconVertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: mem::size_of::<IconVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x2,
                },
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Icon {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub texture_id: String,
}

impl Icon {
    pub fn new(x: f32, y: f32, width: f32, height: f32, texture_id: String) -> Self {
        Self {
            x,
            y,
            width,
            height,
            texture_id,
        }
    }
}

pub struct IconRenderer {
    render_pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    icons: Vec<Icon>,
    textures: HashMap<String, (Texture, TextureView, BindGroup)>,
    window_width: f32,
    window_height: f32,
    cached_vertex_buffers: HashMap<String, wgpu::Buffer>,
    cached_index_buffers: HashMap<String, wgpu::Buffer>,
    cached_icon_counts: HashMap<String, usize>,
}

impl IconRenderer {
    pub fn new(device: &Device, surface_format: TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Icon Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/icon.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Icon Bind Group Layout"),
            entries: &[
                // Texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Icon Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Icon Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[IconVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            render_pipeline,
            bind_group_layout,
            icons: Vec::new(),
            textures: HashMap::new(),
            window_width: 1360.0,
            window_height: 768.0,
            cached_vertex_buffers: HashMap::new(),
            cached_index_buffers: HashMap::new(),
            cached_icon_counts: HashMap::new(),
        }
    }

    pub fn load_texture(
        &mut self,
        device: &Device,
        queue: &wgpu::Queue,
        path: &str,
        texture_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let img = image::open(Path::new(path))?;
        let rgba = img.to_rgba8();
        let dimensions = rgba.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("Icon texture: {}", texture_id)),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("Icon bind group: {}", texture_id)),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        self.textures
            .insert(texture_id.to_string(), (texture, view, bind_group));

        Ok(())
    }

    pub fn add_icon(&mut self, icon: Icon) {
        self.icons.push(icon);
    }

    pub fn clear_icons(&mut self) {
        self.icons.clear();
        // Clear cached buffers when icons are cleared
        self.cached_vertex_buffers.clear();
        self.cached_index_buffers.clear();
        self.cached_icon_counts.clear();
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.window_width = width;
        self.window_height = height;
        // Clear cached buffers when window is resized
        self.cached_vertex_buffers.clear();
        self.cached_index_buffers.clear();
        self.cached_icon_counts.clear();
    }

    pub fn render(&mut self, device: &Device, render_pass: &mut RenderPass) {
        if self.icons.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.render_pipeline);

        // Group icons by texture to minimize bind group changes
        let mut icons_by_texture: HashMap<String, Vec<&Icon>> = HashMap::new();
        for icon in &self.icons {
            icons_by_texture
                .entry(icon.texture_id.clone())
                .or_insert_with(Vec::new)
                .push(icon);
        }

        for (texture_id, icons) in icons_by_texture {
            if let Some((_, _, bind_group)) = self.textures.get(&texture_id) {
                render_pass.set_bind_group(0, bind_group, &[]);

                // Check if we can reuse cached buffers for this texture
                let cached_count = self.cached_icon_counts.get(&texture_id).unwrap_or(&0);
                let need_new_buffers = *cached_count != icons.len();

                if need_new_buffers {
                    // Create vertices for all icons using this texture
                    let mut all_vertices = Vec::new();
                    let mut all_indices = Vec::new();

                    for (icon_index, icon) in icons.iter().enumerate() {
                        // Convert screen coordinates to normalized device coordinates
                        let x = (icon.x / self.window_width) * 2.0 - 1.0;
                        let y = (icon.y / self.window_height) * 2.0 - 1.0;
                        let width = (icon.width / self.window_width) * 2.0;
                        let height = (icon.height / self.window_height) * 2.0;

                        // Create vertices for this icon
                        let vertices = [
                            // Top-left
                            IconVertex {
                                position: [x, y],
                                uv: [0.0, 1.0], // Flip V coordinate
                            },
                            // Top-right
                            IconVertex {
                                position: [x + width, y],
                                uv: [1.0, 1.0], // Flip V coordinate
                            },
                            // Bottom-right
                            IconVertex {
                                position: [x + width, y + height],
                                uv: [1.0, 0.0], // Flip V coordinate
                            },
                            // Bottom-left
                            IconVertex {
                                position: [x, y + height],
                                uv: [0.0, 0.0], // Flip V coordinate
                            },
                        ];

                        // Add vertices to the batch
                        all_vertices.extend_from_slice(&vertices);

                        // Create indices for this icon (offset by the current vertex count)
                        let base_index = (icon_index * 4) as u16;
                        let indices = [
                            base_index,
                            base_index + 1,
                            base_index + 2,
                            base_index,
                            base_index + 2,
                            base_index + 3,
                        ];
                        all_indices.extend_from_slice(&indices);
                    }

                    // Create new vertex buffer for all icons using this texture
                    let vertex_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Icon Vertex Buffer"),
                            contents: bytemuck::cast_slice(&all_vertices),
                            usage: BufferUsages::VERTEX,
                        });

                    // Create new index buffer for all icons using this texture
                    let index_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Icon Index Buffer"),
                            contents: bytemuck::cast_slice(&all_indices),
                            usage: BufferUsages::INDEX,
                        });

                    // Cache the new buffers
                    self.cached_vertex_buffers
                        .insert(texture_id.clone(), vertex_buffer);
                    self.cached_index_buffers
                        .insert(texture_id.clone(), index_buffer);
                    self.cached_icon_counts
                        .insert(texture_id.clone(), icons.len());
                }

                // Use cached buffers
                if let (Some(vertex_buffer), Some(index_buffer)) = (
                    self.cached_vertex_buffers.get(&texture_id),
                    self.cached_index_buffers.get(&texture_id),
                ) {
                    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    render_pass.draw_indexed(0..(icons.len() * 6) as u32, 0, 0..1);
                }
            }
        }
    }
}
