use crate::game::GameState;
use crate::game::enemy::Enemy;
use crate::renderer::pipeline_builder::{
    BindGroupLayoutBuilder, PipelineBuilder, create_uniform_buffer,
};
use egui_wgpu::wgpu::{self, util::DeviceExt};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct EnemyUniforms {
    view_proj_matrix: [[f32; 4]; 4],
    enemy_position: [f32; 3],
    enemy_size: f32,
    player_position: [f32; 3],
    _padding: f32,
}

pub struct EnemyRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,

    // For smooth rotation
    smoothed_rotation: f32,
    smoothing_factor: f32,
}

impl EnemyRenderer {
    pub fn new(
        enemy: Enemy,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        // Load jeffree texture
        let jeffree_texture = Self::load_jeffree_texture(device, queue);

        let uniforms = EnemyUniforms {
            view_proj_matrix: [[0.0; 4]; 4],
            enemy_position: enemy.pathfinder.position,
            enemy_size: enemy.size,
            player_position: [0.0; 3],
            _padding: 0.0,
        };

        let uniform_buffer = create_uniform_buffer(device, &uniforms, "Enemy Uniform Buffer");

        // Create bind group layout for texture + sampler + uniforms
        let bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label("Enemy Bind Group Layout")
            .with_uniform_buffer(0, wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT)
            .with_texture(1, wgpu::ShaderStages::FRAGMENT)
            .with_sampler(2, wgpu::ShaderStages::FRAGMENT)
            .build();

        // Create sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create bind group for jeffree texture
        let jeffree_texture_view =
            jeffree_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&jeffree_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("Enemy Bind Group"),
        });

        // Create vertex buffer layout for position + tex_coords
        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: 5 * 4, // 5 floats * 4 bytes each
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3, // position (x, y, z)
                },
                wgpu::VertexAttribute {
                    offset: 3 * 4, // 3 floats * 4 bytes
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2, // tex_coords
                },
            ],
        };

        let pipeline = PipelineBuilder::new(device, surface_config.format)
            .with_label("Enemy Pipeline")
            .with_shader(include_str!("../shaders/enemy.wgsl"))
            .with_vertex_buffer(vertex_buffer_layout)
            .with_bind_group_layout(&bind_group_layout)
            .with_alpha_blending()
            .with_depth_stencil(wgpu::DepthStencilState {
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                format: wgpu::TextureFormat::Depth24Plus,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
            .build();

        let vertex_buffer = Self::create_billboard_vertices(device);

        Self {
            pipeline,
            vertex_buffer,
            uniform_buffer,
            bind_group,
            smoothed_rotation: 0.0,
            smoothing_factor: 0.85, // Smooth rotation
        }
    }

    fn load_jeffree_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
        let path = "assets/jeffree.png";

        // Load image using image crate
        let img = match image::open(path) {
            Ok(img) => img.to_rgba8(),
            Err(e) => {
                eprintln!("Failed to load jeffree texture {}: {}", path, e);
                // Create a fallback texture (solid red square)
                let mut fallback = image::RgbaImage::new(64, 64);
                for pixel in fallback.pixels_mut() {
                    *pixel = image::Rgba([255, 0, 0, 255]); // Red
                }
                fallback
            }
        };

        let dimensions = img.dimensions();
        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("jeffree Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
            &img,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        texture
    }

    fn create_billboard_vertices(device: &wgpu::Device) -> wgpu::Buffer {
        // Create a quad centered at origin that will be positioned and rotated by the shader
        // The quad is in local space and will be transformed to world space
        let vertices: &[f32] = &[
            // Position (x, y, z)    // Texture coords (u, v)
            // Triangle 1
            -0.5, -0.5, 0.0, 0.0, 1.0, // Bottom-left
            0.5, -0.5, 0.0, 1.0, 1.0, // Bottom-right
            -0.5, 0.5, 0.0, 0.0, 0.0, // Top-left
            // Triangle 2
            0.5, -0.5, 0.0, 1.0, 1.0, // Bottom-right
            0.5, 0.5, 0.0, 1.0, 0.0, // Top-right
            -0.5, 0.5, 0.0, 0.0, 0.0, // Top-left
        ];

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Enemy Billboard Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        })
    }

    /// Update enemy position and rotation to face player
    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
        game_state: &GameState,
        view_proj_matrix: [[f32; 4]; 4],
    ) {
        // Calculate rotation to face player
        let dx = game_state.player.position[0] - game_state.enemy.pathfinder.position[0];
        let dz = game_state.player.position[2] - game_state.enemy.pathfinder.position[2];

        // Calculate target rotation using the same coordinate system as your compass
        // Your compass uses dx.atan2(dz) pattern, so use that here
        let target_rotation = dx.atan2(dz);

        // Smooth rotation interpolation
        let mut rotation_diff = target_rotation - self.smoothed_rotation;

        // Wrap to shortest path
        if rotation_diff > std::f32::consts::PI {
            rotation_diff -= 2.0 * std::f32::consts::PI;
        } else if rotation_diff < -std::f32::consts::PI {
            rotation_diff += 2.0 * std::f32::consts::PI;
        }

        self.smoothed_rotation += rotation_diff * self.smoothing_factor;

        // Update uniform buffer
        let uniforms = EnemyUniforms {
            view_proj_matrix,
            enemy_position: game_state.enemy.pathfinder.position,
            enemy_size: game_state.enemy.size,
            player_position: game_state.player.position,
            _padding: 0.0,
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }
    /// Render the enemy
    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }

    /// Get current rotation angle (in radians)
    pub fn get_rotation(&self) -> f32 {
        self.smoothed_rotation
    }

    /// Set smoothing factor for rotation (0.0 = very smooth, 1.0 = instant)
    pub fn set_smoothing_factor(&mut self, factor: f32) {
        self.smoothing_factor = factor.clamp(0.01, 1.0);
    }
}
