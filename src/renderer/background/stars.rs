//! Animated starfield background renderer.
//!
//! This module provides [`StarRenderer`], which renders a field of animated stars using `wgpu`.
//! Stars are randomly generated in screen space and rendered as glowing points/quads.
//!
//! The renderer supports updating the background color and animating stars over time via uniform buffers.

use egui_wgpu::wgpu::util::DeviceExt;
use egui_wgpu::wgpu::{self, Buffer};
use rand::Rng;

/// Represents a single star in the starfield.
///
/// Each star has a 2D position (in normalized device coordinates), a size, and a brightness value.
#[derive(Debug, Clone, Copy)]
struct Star {
    /// 2D position in screen space, range [-1.0, 1.0].
    position: [f32; 2],
    /// Size of the star (radius in NDC units).
    size: f32,
    /// Brightness multiplier (0.0 = dim, 1.0 = bright).
    brightness: f32,
}

/// Handles GPU resources and rendering pipeline for the animated starfield.
///
/// Contains vertex/index buffers, uniform buffers for time and background color,
/// the render pipeline, and the bind group for uniforms.
pub struct StarRenderer {
    /// Vertex buffer containing star quad vertices.
    pub vertex_buffer: Buffer,
    /// Index buffer for drawing star quads as triangles.
    pub index_buffer: Buffer,
    /// Number of indices to draw.
    pub num_indices: u32,
    /// Render pipeline for the starfield.
    pub pipeline: wgpu::RenderPipeline,
    /// Uniform buffer for animation time.
    pub time_buffer: Buffer,
    /// Uniform buffer for background color (RGBA).
    pub background_color_buffer: Buffer,
    /// Bind group for uniforms.
    pub uniform_bind_group: wgpu::BindGroup,
}

/// Creates a [`StarRenderer`] with randomly generated stars and all necessary GPU resources.
///
/// # Arguments
/// - `device`: The wgpu device to create buffers and pipeline.
/// - `surface_config`: The surface configuration (for color format).
/// - `num_stars`: Number of stars to generate.
///
/// # Returns
/// A fully initialized [`StarRenderer`] ready for rendering.
///
/// # Implementation Notes
/// - Stars are randomly placed in NDC space ([-1, 1]).
/// - Each star is rendered as a quad (two triangles).
/// - Uniform buffers are created for animation time and background color.
/// - The render pipeline and bind group are created using [`create_star_pipeline`].
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

/// Creates the render pipeline and uniform bind group for the starfield.
///
/// # Arguments
/// - `device`: The wgpu device.
/// - `surface_config`: The surface configuration (for color format).
/// - `time_buffer`: Uniform buffer for animation time.
/// - `background_color_buffer`: Uniform buffer for background color.
///
/// # Returns
/// A tuple of (`wgpu::RenderPipeline`, `wgpu::BindGroup`).
///
/// # Implementation Notes
/// - Loads the WGSL shader from `star_shader.wgsl`.
/// - Sets up vertex attributes for position, size, brightness, and tex coords.
/// - Configures blending for alpha transparency.
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
    /// Updates the background color used by the starfield shader.
    ///
    /// # Arguments
    /// - `queue`: The wgpu queue to write to the buffer.
    /// - `color`: The new background color as `[r, g, b, a]` (all floats 0.0..1.0).
    pub fn update_background_color(&self, queue: &wgpu::Queue, color: [f32; 4]) {
        queue.write_buffer(
            &self.background_color_buffer,
            0,
            bytemuck::cast_slice(&color),
        );
    }

    /// Updates the animation time uniform for the starfield shader.
    ///
    /// # Arguments
    /// - `queue`: The wgpu queue to write to the buffer.
    /// - `time`: The new time value (in seconds).
    pub fn update_star_time(&self, queue: &wgpu::Queue, time: f32) {
        queue.write_buffer(&self.time_buffer, 0, bytemuck::cast_slice(&[time]));
    }
}
