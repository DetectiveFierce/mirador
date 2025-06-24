use egui_wgpu::wgpu::util::DeviceExt;
use egui_wgpu::wgpu::{self, Buffer};
use rand::Rng;

#[derive(Debug, Clone, Copy)]
struct Star {
    position: [f32; 2], // Changed to 2D for screen space
    size: f32,
    brightness: f32,
}

pub struct StarRenderer {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub num_indices: u32,
    pub pipeline: wgpu::RenderPipeline,
    pub time_buffer: Buffer,
    pub background_color_buffer: Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
}

pub fn create_star_renderer(
    device: &wgpu::Device,
    surface_config: &wgpu::SurfaceConfiguration,
    num_stars: usize,
) -> StarRenderer {
    // Generate random stars in screen space (-1 to 1)
    let mut stars = Vec::new();
    let mut rng = rand::thread_rng();

    for _ in 0..num_stars {
        stars.push(Star {
            position: [rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0)],
            size: rng.gen_range(0.015..0.04), // Much smaller stars for sharp points
            brightness: rng.gen_range(0.3..1.0),
        });
    }

    // Create vertices and indices for instanced quads
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for (star_idx, star) in stars.iter().enumerate() {
        let base_vertex = (star_idx * 4) as u16;

        // Create quad vertices (position, size, brightness)
        let half_size = star.size;

        // Bottom-left
        vertices.extend_from_slice(&[
            star.position[0] - half_size,
            star.position[1] - half_size, // position
            star.size,
            star.brightness,
            0.0,
            0.0, // size, brightness, tex_coords
        ]);

        // Bottom-right
        vertices.extend_from_slice(&[
            star.position[0] + half_size,
            star.position[1] - half_size,
            star.size,
            star.brightness,
            1.0,
            0.0,
        ]);

        // Top-right
        vertices.extend_from_slice(&[
            star.position[0] + half_size,
            star.position[1] + half_size,
            star.size,
            star.brightness,
            1.0,
            1.0,
        ]);

        // Top-left
        vertices.extend_from_slice(&[
            star.position[0] - half_size,
            star.position[1] + half_size,
            star.size,
            star.brightness,
            0.0,
            1.0,
        ]);

        // Create indices for two triangles
        indices.extend_from_slice(&[
            base_vertex,
            base_vertex + 1,
            base_vertex + 2,
            base_vertex,
            base_vertex + 2,
            base_vertex + 3,
        ]);
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Star Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Star Index Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    // Create time uniform buffer
    let time_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Star Time Buffer"),
        contents: bytemuck::cast_slice(&[0.0f32]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // Create background color uniform buffer (default to black)
    let background_color_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Background Color Buffer"),
        contents: bytemuck::cast_slice(&[0.0f32, 0.0f32, 0.0f32, 1.0f32]), // RGBA
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let (pipeline, uniform_bind_group) = create_star_pipeline(
        device,
        surface_config,
        &time_buffer,
        &background_color_buffer,
    );

    StarRenderer {
        vertex_buffer,
        index_buffer,
        num_indices: indices.len() as u32,
        pipeline,
        time_buffer,
        background_color_buffer,
        uniform_bind_group,
    }
}

pub fn create_star_pipeline(
    device: &wgpu::Device,
    surface_config: &wgpu::SurfaceConfiguration,
    time_buffer: &Buffer,
    background_color_buffer: &Buffer,
) -> (wgpu::RenderPipeline, wgpu::BindGroup) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Star Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("star_shader.wgsl").into()),
    });

    // Create bind group layout for uniforms
    let uniform_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("uniform_bind_group_layout"),
        });

    // Create bind group for uniforms
    let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &uniform_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: time_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: background_color_buffer.as_entire_binding(),
            },
        ],
        label: Some("uniform_bind_group"),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Star Pipeline Layout"),
        bind_group_layouts: &[&uniform_bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Star Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 6 * std::mem::size_of::<f32>() as u64, // 6 floats per vertex
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    // position (2 floats)
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    },
                    // size, brightness, tex_coords (4 floats)
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 2 * std::mem::size_of::<f32>() as u64,
                        shader_location: 1,
                    },
                ],
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_config.format,
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
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
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
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    });

    (pipeline, uniform_bind_group)
}

// Helper function to update background color
impl StarRenderer {
    pub fn update_background_color(&self, queue: &wgpu::Queue, color: [f32; 4]) {
        queue.write_buffer(
            &self.background_color_buffer,
            0,
            bytemuck::cast_slice(&color),
        );
    }

    pub fn update_time(&self, queue: &wgpu::Queue, time: f32) {
        queue.write_buffer(&self.time_buffer, 0, bytemuck::cast_slice(&[time]));
    }
}
