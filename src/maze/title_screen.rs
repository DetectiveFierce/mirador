use crate::maze::generator::Maze;
use crate::maze::generator::MazeGenerator;
use egui_wgpu::wgpu;
use egui_wgpu::wgpu::util::DeviceExt;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct TitleScreenRenderer {
    pub generator: MazeGenerator,
    pub maze: Arc<Mutex<Maze>>,
    pub vertex_buffer: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
    pub texture: wgpu::Texture,
    pub bind_group: wgpu::BindGroup,
    // Loading bar resources
    pub loading_bar_pipeline: wgpu::RenderPipeline,
    pub loading_bar_vertex_buffer: wgpu::Buffer,
    pub loading_bar_uniform_buffer: wgpu::Buffer,
    pub loading_bar_bind_group: wgpu::BindGroup,
    pub last_update: Instant,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LoadingBarUniforms {
    progress: f32,
    _padding: [f32; 3],
}

impl TitleScreenRenderer {
    pub fn new(device: &wgpu::Device, surface_config: &wgpu::SurfaceConfiguration) -> Self {
        let maze_width = 25;
        let maze_height = 25;

        let (generator, maze) = MazeGenerator::new(maze_width, maze_height);

        // Get the actual render dimensions instead of using maze dimensions directly
        let (render_width, render_height) = {
            let maze_lock = maze.lock().unwrap();
            maze_lock.get_dimensions()
        };

        // Create texture for maze with correct render dimensions
        let texture_size = wgpu::Extent3d {
            width: render_width as u32,   // Now uses actual render width (126)
            height: render_height as u32, // Now uses actual render height (126)
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Maze Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
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
            label: Some("texture_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("2D Maze Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("2D-maze-shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x2,
                    }],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let vertices: &[f32] = &[
            -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0,
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Create loading bar resources
        let loading_bar_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Loading Bar Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("loading-bar-shader.wgsl").into()),
        });

        let loading_bar_uniform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Loading Bar Uniform Buffer"),
                contents: bytemuck::cast_slice(&[LoadingBarUniforms {
                    progress: 0.0,
                    _padding: [0.0; 3],
                }]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let loading_bar_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("loading_bar_bind_group_layout"),
            });

        let loading_bar_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &loading_bar_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: loading_bar_uniform_buffer.as_entire_binding(),
            }],
            label: Some("loading_bar_bind_group"),
        });

        let loading_bar_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Loading Bar Pipeline Layout"),
                bind_group_layouts: &[&loading_bar_bind_group_layout],
                push_constant_ranges: &[],
            });

        let loading_bar_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Loading Bar Pipeline"),
            layout: Some(&loading_bar_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &loading_bar_shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x2,
                    }],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &loading_bar_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let loading_bar_vertex_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Loading Bar Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        Self {
            generator,
            maze,
            texture,
            bind_group,
            pipeline,
            vertex_buffer,
            loading_bar_pipeline,
            loading_bar_vertex_buffer,
            loading_bar_uniform_buffer,
            loading_bar_bind_group,
            last_update: Instant::now(),
        }
    }

    pub fn update_texture(
        &self,
        queue: &wgpu::Queue,
        maze_data: &[u8],
        width: usize,
        height: usize,
    ) {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            maze_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width as u32),
                rows_per_image: Some(height as u32),
            },
            wgpu::Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn update_loading_bar(&self, queue: &wgpu::Queue, progress: f32) {
        let uniforms = LoadingBarUniforms {
            progress: progress.clamp(0.0, 1.0),
            _padding: [0.0; 3],
        };
        queue.write_buffer(
            &self.loading_bar_uniform_buffer,
            0,
            bytemuck::cast_slice(&[uniforms]),
        );
    }
}
