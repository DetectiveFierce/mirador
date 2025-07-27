//! Game renderer module for the Mirador 3D maze game.
//!
//! This module provides the main rendering system for the 3D maze game, handling
//! all visual elements including the maze geometry, enemies, UI elements, and
//! special effects. The renderer uses WebGPU for hardware-accelerated graphics
//! and implements a multi-pass rendering pipeline for optimal performance.
//!
//! # Overview
//!
//! The game renderer consists of several specialized renderers:
//! - `GameRenderer`: Main renderer coordinating all visual elements
//! - `CompassRenderer`: Renders the directional compass overlay
//! - `EnemyRenderer`: Handles enemy visualization and animation
//! - `StarRenderer`: Creates animated starfield background effects
//! - `TimerBarRenderer`: Renders the time remaining indicator
//! - `StaminaBarRenderer`: Displays player stamina levels
//! - `DebugRenderer`: Development tools for debugging and visualization
//!
//! # Rendering Pipeline
//!
//! The renderer uses a structured multi-pass approach:
//! 1. **Background Pass**: Animated starfield using `StarRenderer`
//! 2. **Geometry Pass**: Maze floors and walls with depth testing
//! 3. **Entity Pass**: Enemies and interactive elements
//! 4. **UI Pass**: Compass, timer, stamina bars, and overlays
//! 5. **Debug Pass**: Optional development overlays
//!
//! # Coordinate System
//!
//! Uses a right-handed coordinate system:
//! - X-axis: Right direction
//! - Y-axis: Up direction  
//! - Z-axis: Toward viewer (negative Z is forward)
//! - Maze positioned at world origin (0, 0, 0)
//!
//! # Usage
//!
//! ```rust
//! use mirador::renderer::game_renderer::GameRenderer;
//! use wgpu::{Device, Queue, SurfaceConfiguration};
//!
//! // Create renderer
//! let mut renderer = GameRenderer::new(&device, &queue, &surface_config);
//!
//! // Load textures
//! renderer.load_ceiling_texture(&device, &queue)?;
//!
//! // Update depth buffer on resize
//! let depth_view = renderer.update_depth_texture(&device, width, height);
//!
//! // Render frame
//! renderer.render_game(&queue, &game_state, &mut pass, aspect_ratio);
//! ```

pub mod compass;
pub mod debug;
pub mod enemy;
pub mod game_over;
pub mod stamina_bar;
pub mod stars;
pub mod timer_bar;

use crate::game::GameState;
use crate::game::enemy::Enemy;
use crate::math::deg_to_rad;
use crate::math::mat::Mat4;
use crate::renderer::game_renderer::compass::CompassRenderer;
use crate::renderer::game_renderer::debug::DebugRenderer;
use crate::renderer::game_renderer::enemy::EnemyRenderer;
use crate::renderer::game_renderer::stars::StarRenderer;
use crate::renderer::pipeline_builder::PipelineBuilder;
use crate::renderer::primitives::{Uniforms, Vertex};
use crate::assets;
use image;
use stamina_bar::StaminaBarRenderer;
use std::time::Instant;
use timer_bar::TimerBarRenderer;
use wgpu;
use wgpu::util::DeviceExt;

/// Main renderer for the 3D maze game.
///
/// The `GameRenderer` is responsible for rendering the complete 3D maze environment,
/// including floors, walls, starfield backgrounds, and debug overlays. It manages
/// all rendering pipelines, vertex data, and GPU resources required for the game's
/// visual presentation.
///
/// ## Rendering Pipeline
///
/// The renderer uses a multi-pass approach:
/// 1. **Background Pass**: Renders animated starfield using `StarRenderer`
/// 2. **Main Pass**: Renders maze geometry (floors and walls) with depth testing
/// 3. **Debug Pass**: Optional overlay rendering for development tools
///
/// ## Memory Management
///
/// Vertex data for floors and walls is combined into a single buffer for optimal
/// GPU performance. The depth texture is dynamically resized based on the surface
/// dimensions and automatically managed to prevent memory leaks.
///
/// ## Coordinate System
///
/// The renderer uses a right-handed coordinate system with:
/// - X-axis pointing right
/// - Y-axis pointing up
/// - Z-axis pointing toward the viewer
/// - Maze positioned at world origin
///
/// ## Shader Integration
///
/// Works with `shader.wgsl` which expects:
/// ```wgsl
/// struct Uniforms {
///     matrix: mat4x4<f32>,
/// }
/// @group(0) @binding(0) var<uniform> uniforms: Uniforms;
/// ```
///
/// # Fields
///
/// - `pipeline` - Main render pipeline for maze geometry with depth testing and alpha blending
/// - `vertex_buffer` - Combined vertex buffer containing both floor and wall geometry data
/// - `vertex_count` - Total number of vertices to render from the combined buffer
/// - `uniform_buffer` - GPU buffer storing model-view-projection matrix for vertex transformations
/// - `uniform_bind_group` - WebGPU bind group linking uniform buffer to shader binding point 0
/// - `depth_texture` - Optional depth buffer for proper 3D occlusion (recreated on resize)
/// - `star_renderer` - Background renderer for animated starfield effects
/// - `debug_renderer` - Development tools for rendering bounding boxes and debug overlays
/// - `compass_renderer` - Renders the directional compass overlay
/// - `exit_position` - Optional coordinates of the maze exit for special rendering
/// - `enemy_renderer` - Handles enemy visualization and animation
/// - `start_time` - Tracks animation start time for time-based effects
/// - `timer_bar_renderer` - Renders the time remaining indicator
/// - `stamina_bar_renderer` - Displays player stamina levels
/// - `ceiling_texture` - Optional texture for ceiling rendering
/// - `ceiling_texture_view` - Texture view for ceiling rendering
/// - `ceiling_sampler` - Sampler for ceiling texture filtering
/// - `ceiling_bind_group` - Bind group for ceiling texture resources
pub struct GameRenderer {
    /// Main render pipeline for maze geometry with depth testing and alpha blending
    pub pipeline: wgpu::RenderPipeline,
    /// Combined vertex buffer containing both floor and wall geometry data
    pub vertex_buffer: wgpu::Buffer,
    /// Total number of vertices to render from the combined buffer
    pub vertex_count: u32,
    /// GPU buffer storing model-view-projection matrix for vertex transformations
    pub uniform_buffer: wgpu::Buffer,
    /// WebGPU bind group linking uniform buffer to shader binding point 0
    pub uniform_bind_group: wgpu::BindGroup,
    /// Optional depth buffer for proper 3D occlusion (recreated on resize)
    pub depth_texture: Option<wgpu::Texture>,
    /// Background renderer for animated starfield effects
    pub star_renderer: StarRenderer,
    /// Development tools for rendering bounding boxes and debug overlays
    pub debug_renderer: DebugRenderer,
    /// Renders the directional compass overlay
    pub compass_renderer: CompassRenderer,
    /// Optional coordinates of the maze exit for special rendering
    pub exit_position: Option<(f32, f32)>,
    /// Handles enemy visualization and animation
    pub enemy_renderer: EnemyRenderer,
    /// Tracks animation start time for time-based effects
    pub start_time: Instant,
    /// Renders the time remaining indicator
    pub timer_bar_renderer: TimerBarRenderer,
    /// Displays player stamina levels
    pub stamina_bar_renderer: StaminaBarRenderer,
    /// Optional texture for ceiling rendering
    pub ceiling_texture: Option<wgpu::Texture>,
    /// Texture view for ceiling rendering
    pub ceiling_texture_view: Option<wgpu::TextureView>,
    /// Sampler for ceiling texture filtering
    pub ceiling_sampler: Option<wgpu::Sampler>,
    /// Bind group for ceiling texture resources
    pub ceiling_bind_group: Option<wgpu::BindGroup>,
}

impl GameRenderer {
    /// Creates a new `GameRenderer` instance with all necessary GPU resources.
    ///
    /// This constructor initializes all rendering components including the main
    /// pipeline, vertex buffers, uniform buffers, and specialized renderers for
    /// various game elements.
    ///
    /// # Arguments
    ///
    /// * `device` - WebGPU device for creating GPU resources
    /// * `queue` - WebGPU queue for command submission
    /// * `surface_config` - Surface configuration for format and size information
    ///
    /// # Returns
    ///
    /// A fully initialized `GameRenderer` instance ready for rendering.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mirador::renderer::game_renderer::GameRenderer;
    /// use wgpu::{Device, Queue, SurfaceConfiguration};
    ///
    /// let renderer = GameRenderer::new(&device, &queue, &surface_config);
    /// ```
    ///
    /// # GPU Resource Creation
    ///
    /// The constructor creates several GPU resources:
    /// - Render pipeline with depth testing and alpha blending
    /// - Uniform buffer for transformation matrices
    /// - Bind group for shader resource binding
    /// - Specialized renderers for stars, compass, enemies, and UI elements
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        use crate::benchmarks::{BenchmarkConfig, Profiler};

        // Initialize profiler for GameRenderer initialization benchmarking
        let mut init_profiler = Profiler::new(BenchmarkConfig {
            enabled: true,
            print_results: false, // Respect user's console output preference
            write_to_file: false,
            min_duration_threshold: std::time::Duration::from_micros(1),
            max_samples: 1000,
        });

        // Benchmark uniform buffer creation
        init_profiler.start_section("uniform_buffer_creation");
        let uniforms = Uniforms::new();
        let uniform_buffer = uniforms.create_buffer(device);
        let (uniform_bind_group, _uniform_bind_group_layout) =
            uniforms.create_bind_group(&uniform_buffer, device);
        init_profiler.end_section("uniform_buffer_creation");

        // Benchmark bind group layout creation
        init_profiler.start_section("bind_group_layout_creation");
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Main Pipeline Bind Group Layout"),
            entries: &[
                // Uniform buffer (binding 0)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Texture (binding 1)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler (binding 2)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        init_profiler.end_section("bind_group_layout_creation");

        // Benchmark main pipeline creation
        init_profiler.start_section("main_pipeline_creation");
        let pipeline = PipelineBuilder::new(device, surface_config.format)
            .with_label("Main Pipeline")
            .with_shader(include_str!("../shaders/main-shader.wgsl"))
            .with_vertex_buffer(Vertex::desc())
            .with_bind_group_layout(&bind_group_layout)
            .with_blend_state(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::OVER,
            })
            .with_no_culling()
            .with_depth_stencil(wgpu::DepthStencilState {
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                format: wgpu::TextureFormat::Depth24Plus,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
            .build();
        init_profiler.end_section("main_pipeline_creation");

        // Benchmark vertex buffer creation
        init_profiler.start_section("vertex_buffer_creation");
        let empty_vertices: Vec<Vertex> = Vec::new();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Initial Vertex Buffer"),
            contents: bytemuck::cast_slice(&empty_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        init_profiler.end_section("vertex_buffer_creation");

        // Benchmark star renderer creation
        init_profiler.start_section("star_renderer_creation");
        let star_renderer = stars::create_star_renderer(device, surface_config, 100);
        init_profiler.end_section("star_renderer_creation");

        // Benchmark debug renderer creation
        init_profiler.start_section("debug_renderer_creation");
        let debug_renderer = DebugRenderer {
            debug_render_bounding_boxes: false,
            debug_vertex_buffer: None,
            debug_vertex_count: 0,
        };
        init_profiler.end_section("debug_renderer_creation");

        // Benchmark compass renderer creation
        init_profiler.start_section("compass_renderer_creation");
        let compass_renderer = CompassRenderer::new(device, queue, surface_config);
        init_profiler.end_section("compass_renderer_creation");

        // Benchmark enemy renderer creation
        init_profiler.start_section("enemy_renderer_creation");
        let enemy = Enemy::new([-1370.0, 50.0, 1370.0], 100.0);
        let enemy_renderer = EnemyRenderer::new(enemy, device, queue, surface_config);
        init_profiler.end_section("enemy_renderer_creation");

        // Benchmark timer bar renderer creation
        init_profiler.start_section("timer_bar_renderer_creation");
        let timer_bar_renderer = TimerBarRenderer::new(device, surface_config);
        init_profiler.end_section("timer_bar_renderer_creation");

        // Benchmark stamina bar renderer creation
        init_profiler.start_section("stamina_bar_renderer_creation");
        let stamina_bar_renderer = StaminaBarRenderer::new(device, surface_config);
        init_profiler.end_section("stamina_bar_renderer_creation");

        Self {
            pipeline,
            vertex_buffer,
            vertex_count: 0, // Will be set when maze is loaded
            uniform_buffer,
            uniform_bind_group,
            depth_texture: None,
            star_renderer,
            debug_renderer,
            compass_renderer,
            exit_position: None,
            enemy_renderer,
            start_time: Instant::now(), // Initialize start time
            timer_bar_renderer,
            stamina_bar_renderer,
            ceiling_texture: None,
            ceiling_texture_view: None,
            ceiling_sampler: None,
            ceiling_bind_group: None,
        }
    }

    /// Loads the ceiling texture and creates the bind group for texturing.
    ///
    /// This method loads the ceiling texture from the assets directory and sets up
    /// all necessary GPU resources for texture rendering, including the texture
    /// itself, texture view, sampler, and bind group.
    ///
    /// # Arguments
    ///
    /// * `device` - WebGPU device for creating GPU resources
    /// * `queue` - WebGPU queue for uploading texture data
    ///
    /// # Returns
    ///
    /// `Result<(), Box<dyn std::error::Error>>` - Success or error from texture loading
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mirador::renderer::game_renderer::GameRenderer;
    /// use wgpu::{Device, Queue};
    ///
    /// let mut renderer = GameRenderer::new(&device, &queue, &surface_config);
    /// renderer.load_ceiling_texture(&device, &queue)?;
    /// ```
    ///
    /// # Texture Details
    ///
    /// - Loads texture from `assets/tiles.jpg`
    /// - Creates RGBA8 texture with sRGB format
    /// - Uses repeat addressing for seamless tiling
    /// - Linear filtering for smooth texture interpolation
    /// - Creates bind group with uniform buffer, texture, and sampler
    pub fn load_ceiling_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Load the tiles texture from embedded assets
        let img = image::load_from_memory(assets::TILES_IMAGE)?;
        let rgba = img.to_rgba8();
        let dimensions = rgba.dimensions();

        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Ceiling Texture"),
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
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create sampler with repeat addressing for tiling
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create bind group layout for texture + sampler + uniforms
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Ceiling Texture Bind Group Layout"),
            entries: &[
                // Uniform buffer (binding 0)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Texture (binding 1)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler (binding 2)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Ceiling Texture Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        // Store the resources
        self.ceiling_texture = Some(texture);
        self.ceiling_texture_view = Some(texture_view);
        self.ceiling_sampler = Some(sampler);
        self.ceiling_bind_group = Some(bind_group);

        Ok(())
    }

    /// Updates or creates the depth texture for proper 3D occlusion.
    ///
    /// This method manages the depth buffer, creating a new one when the surface
    /// dimensions change or when no depth texture exists. The depth texture is
    /// essential for proper 3D rendering with depth testing.
    ///
    /// # Arguments
    ///
    /// * `device` - WebGPU device for creating GPU resources
    /// * `width` - New width of the surface
    /// * `height` - New height of the surface
    ///
    /// # Returns
    ///
    /// A `wgpu::TextureView` of the depth texture for use in render passes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mirador::renderer::game_renderer::GameRenderer;
    /// use wgpu::Device;
    ///
    /// let mut renderer = GameRenderer::new(&device, &queue, &surface_config);
    /// let depth_view = renderer.update_depth_texture(&device, 1920, 1080);
    /// ```
    ///
    /// # Memory Management
    ///
    /// - Automatically drops old depth texture when recreating
    /// - Only recreates when dimensions actually change
    /// - Uses Depth24Plus format for optimal precision and performance
    pub fn update_depth_texture(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> wgpu::TextureView {
        if self.depth_texture.is_none()
            || self
                .depth_texture
                .as_ref()
                .expect("Depth texture should exist")
                .width()
                != width
            || self
                .depth_texture
                .as_ref()
                .expect("Depth texture should exist")
                .height()
                != height
        {
            if let Some(depth_texture) = self.depth_texture.take() {
                // Manually drop the texture to free up resources
                drop(depth_texture);
            }

            self.depth_texture = Some(device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth24Plus,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }));
        }
        self.depth_texture
            .as_ref()
            .expect("Depth texture should exist")
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Renders the complete game scene including maze, enemies, and UI elements.
    ///
    /// This is the main rendering method that coordinates all visual elements
    /// in the correct order. It calculates view and projection matrices once
    /// and applies them to all rendered objects for consistency.
    ///
    /// # Arguments
    ///
    /// * `queue` - WebGPU queue for command submission
    /// * `game_state` - Current game state containing player and enemy information
    /// * `pass` - Render pass to record drawing commands
    /// * `aspect` - Aspect ratio of the surface for projection calculations
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mirador::renderer::game_renderer::GameRenderer;
    /// use mirador::game::GameState;
    /// use wgpu::{Queue, RenderPass};
    ///
    /// let mut renderer = GameRenderer::new(&device, &queue, &surface_config);
    /// renderer.render_game(&queue, &game_state, &mut pass, 16.0 / 9.0);
    /// ```
    ///
    /// # Rendering Order
    ///
    /// The method renders elements in this order:
    /// 1. **Maze/Floor**: Main geometry with depth testing
    /// 2. **Enemies**: Animated enemy entities
    /// 3. **UI Elements**: Compass, timer, stamina bars (handled separately)
    ///
    /// # Matrix Calculations
    ///
    /// - View matrix from player camera position and orientation
    /// - Projection matrix with configurable FOV and aspect ratio
    /// - Combined view-projection matrix for efficient rendering
    /// - Model matrix for floor (identity) and individual enemy transforms
    pub fn render_game(
        &mut self,
        queue: &wgpu::Queue,
        game_state: &GameState,
        pass: &mut wgpu::RenderPass,
        aspect: f32,
    ) {
        // Calculate view and projection matrices once
        let view_matrix = game_state.player.get_view_matrix();
        let projection_matrix = Mat4::perspective(
            deg_to_rad(game_state.player.fov),
            aspect,
            0.1,    // zNear
            2000.0, // zFar
        );
        let view_proj_matrix = view_matrix.multiply(&projection_matrix);

        // ==============================================
        // 1. RENDER MAZE/FLOOR FIRST
        // ==============================================
        {
            // Model Matrix for floor - identity since floor is at world origin
            let model_matrix = Mat4::identity();

            // Combine matrices: Projection * View * Model
            let final_mvp_matrix = model_matrix.multiply(&view_proj_matrix);

            // Calculate elapsed time for animation
            let elapsed = self.start_time.elapsed().as_secs_f32();

            let uniforms = Uniforms {
                matrix: final_mvp_matrix.into(),
                time: elapsed,
                _padding: [0.0; 7],
            };

            // Upload uniform values for the maze/floor
            queue.write_buffer(&self.uniform_buffer, 0, uniforms.as_bytes());

            // Render the maze/floor only if we have vertices to render
            if self.vertex_count > 0 {
                pass.set_pipeline(&self.pipeline);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

                // Use ceiling texture bind group if available, otherwise fall back to uniform bind group
                if let Some(ceiling_bind_group) = &self.ceiling_bind_group {
                    pass.set_bind_group(0, ceiling_bind_group, &[]);
                } else {
                    pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                }

                pass.draw(0..self.vertex_count, 0..1);
            }

            // Debug rendering for maze/floor
            if self.debug_renderer.debug_render_bounding_boxes
                && self.debug_renderer.debug_vertex_count > 0
            {
                if let Some(debug_buffer) = &self.debug_renderer.debug_vertex_buffer {
                    pass.set_vertex_buffer(0, debug_buffer.slice(..));
                    pass.draw(0..self.debug_renderer.debug_vertex_count as u32, 0..1);
                }
            }
        }

        // ==============================================
        // 2. RENDER ENEMIES
        // ==============================================
        {
            // Update enemy transform with the combined view-projection matrix
            self.enemy_renderer.update(
                queue,
                game_state,
                view_proj_matrix.0, // Pass the view-projection matrix
            );

            // Actually render the enemy
            self.enemy_renderer.render(pass);
        }
    }
}
