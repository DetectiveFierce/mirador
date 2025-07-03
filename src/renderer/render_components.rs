//! # Maze Generation Animation Rendering System
//!
//! This module provides specialized renderers for different aspects of a maze visualization
//! application. It includes renderers for the maze itself, loading progress indicators,
//! and animated exit effects. As well as the main 3D maze renderer.
//!
//! ## Architecture Overview
//!
//! The rendering system is built on top of the pipeline builder utilities and provides
//! three main renderer types:
//!
//! - [`MazeRenderer`] - Renders the maze texture to screen
//! - [`LoadingBarRenderer`] - Shows loading progress with a progress bar
//! - [`ExitShaderRenderer`] - Creates animated effects for the maze exit
//! - [`GameRenderer`] - Renders the maze in 3D with the background defined in stars.rs
//!
//! ## Coordinate System
//!
//! The maze uses a grid-based coordinate system where (0,0) is typically the top-left
//! corner. The renderers handle conversion between grid coordinates and screen space.
//!
//! ## Usage Pattern
//!
//! 1. Create renderers during initialization with device and surface configuration
//! 2. Update renderer state each frame (progress, time, etc.)
//! 3. Render during the render pass
//!
//! ## Example Setup
//!
//! ```rust,no_run
//! use crate::renderer::maze_renderer::{MazeRenderer, LoadingBarRenderer, MazeRenderConfig};
//! # let device: egui_wgpu::wgpu::Device = unimplemented!();
//! # let surface_config: egui_wgpu::wgpu::SurfaceConfiguration = unimplemented!();
//!
//! // Create maze texture
//! let config = MazeRenderConfig::new(50, 50); // 50x50 maze
//! let (texture, texture_view, sampler) = config.create_maze_texture(&device);
//!
//! // Create renderers
//! let maze_renderer = MazeRenderer::new(&device, &surface_config, &texture_view, &sampler);
//! let loading_renderer = LoadingBarRenderer::new(&device, &surface_config);
//! ```
use std::sync::{Arc, Mutex};
use std::time::Instant;

use winit::window::Window;

use egui_wgpu::wgpu::{self, util::DeviceExt};

use crate::{
    game::player::Player,
    math::{deg_to_rad, mat::Mat4},
    maze::{
        generator::{Maze, MazeGenerator},
        parse_maze_file,
    },
    renderer::{
        background::stars::{self, StarRenderer},
        debug_renderer::DebugRenderer,
        pipeline_builder::{
            BindGroupLayoutBuilder, PipelineBuilder, create_fullscreen_vertices,
            create_uniform_buffer, create_vertex_2d_layout,
        },
        uniform::Uniforms,
        vertex::Vertex,
    },
};

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
pub struct GameRenderer {
    pub pipeline: wgpu::RenderPipeline,
    /// Combined vertex buffer containing both floor and wall geometry data.
    pub vertex_buffer: wgpu::Buffer,
    /// Total number of vertices to render from the combined buffer.
    pub vertex_count: u32,
    /// GPU buffer storing model-view-projection matrix for vertex transformations.
    pub uniform_buffer: wgpu::Buffer,
    /// WebGPU bind group linking uniform buffer to shader binding point 0.
    pub uniform_bind_group: wgpu::BindGroup,
    /// Optional depth buffer for proper 3D occlusion (recreated on resize).
    pub depth_texture: Option<wgpu::Texture>,
    /// Background renderer for animated starfield effects.
    pub star_renderer: StarRenderer,
    /// Development tools for rendering bounding boxes and debug overlays.
    pub debug_renderer: DebugRenderer,
    /// Renderer for compass
    pub compass_renderer: CompassRenderer,
    pub exit_position: Option<(f32, f32)>,
}

impl GameRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        let uniforms = Uniforms::new();
        let uniform_buffer = uniforms.create_buffer(device);
        let (uniform_bind_group, uniform_bind_group_layout) =
            uniforms.create_bind_group(&uniform_buffer, device);

        let pipeline = PipelineBuilder::new(device, surface_config.format)
            .with_label("Main Pipeline")
            .with_shader(include_str!("./shaders/main-shader.wgsl"))
            .with_vertex_buffer(Vertex::desc())
            .with_bind_group_layout(&uniform_bind_group_layout)
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

        // Load wall grid from file
        let (maze_grid, exit_cell) = parse_maze_file("src/maze/saved-mazes/test.mz");

        let (mut floor_vertices, _exit_position) =
            Vertex::create_floor_vertices(&maze_grid, exit_cell);

        // Generate wall geometry
        let mut wall_vertices = Vertex::create_wall_vertices(&maze_grid);

        // Append wall vertices to floor
        floor_vertices.append(&mut wall_vertices);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Combined Vertex Buffer"),
            contents: bytemuck::cast_slice(&floor_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let star_renderer = stars::create_star_renderer(device, surface_config, 100);

        let debug_renderer = DebugRenderer {
            debug_render_bounding_boxes: false,
            debug_vertex_buffer: None,
            debug_vertex_count: 0,
        };

        let compass_renderer = CompassRenderer::new(device, queue, surface_config);

        Self {
            pipeline,
            vertex_buffer,
            vertex_count: floor_vertices.len() as u32,
            uniform_buffer,
            uniform_bind_group,
            depth_texture: None,
            star_renderer,
            debug_renderer,
            compass_renderer,
            exit_position: None,
        }
    }

    pub fn update_depth_texture(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> wgpu::TextureView {
        if self.depth_texture.is_none()
            || self.depth_texture.as_ref().unwrap().width() != width
            || self.depth_texture.as_ref().unwrap().height() != height
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
            .unwrap()
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn render_game(
        &mut self,
        queue: &wgpu::Queue,
        player: &Player,
        pass: &mut wgpu::RenderPass,
        aspect: f32,
    ) {
        // Step 1: Model Matrix - Just identity since the floor is at world origin
        let model_matrix = Mat4::identity();

        // Step 2: View Matrix - Based on player's camera position and orientation
        let view_matrix = player.get_view_matrix();

        // Step 3: Projection Matrix - Using FOV from UI state
        let projection_matrix = Mat4::perspective(
            deg_to_rad(player.fov),
            aspect,
            0.1,    // zNear
            2000.0, // zFar
        );

        // Step 4: Combine matrices: Projection * View * Model
        let final_mvp_matrix = projection_matrix
            .multiply(&view_matrix)
            .multiply(&model_matrix);

        let uniforms = Uniforms {
            matrix: final_mvp_matrix.into(), // Access the inner `[[f32; 4]; 4]` array
        };
        // upload the uniform values to the uniform buffer
        queue.write_buffer(&self.uniform_buffer, 0, uniforms.as_bytes());

        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        pass.draw(0..self.vertex_count, 0..1);

        // Inside your render method:
        if self.debug_renderer.debug_render_bounding_boxes
            && self.debug_renderer.debug_vertex_count > 0
        {
            if let Some(debug_buffer) = &self.debug_renderer.debug_vertex_buffer {
                pass.set_vertex_buffer(0, debug_buffer.slice(..));
                pass.draw(0..self.debug_renderer.debug_vertex_count as u32, 0..1);
            }
        }
    }
}

/// Handles rendering of the maze and loading bar on the maze generation animation screen.
///
/// This struct manages GPU resources for the maze texture, pipelines, vertex buffers, and loading bar.
/// It also holds a [`MazeGenerator`] and a shared [`Maze`] instance for generating and displaying the maze.
///
/// # Fields
/// - `generator`: Maze generator for producing new mazes.
/// - `maze`: Shared, thread-safe reference to the current maze.
/// - `maze_renderer`: Maze renderer for displaying the maze.
/// - `loading_bar_renderer`: Loading bar renderer for displaying the loading progress.
/// - `exit_shader_renderer`: Exit shader renderer for displaying the exit shader.
/// - `texture`: Texture containing the maze image.
/// - `last_update`: Timestamp of the last update (for animation/timing).
pub struct LoadingRenderer {
    /// Maze generator for producing new mazes.
    pub generator: MazeGenerator,
    /// Shared, thread-safe reference to the current maze.
    pub maze: Arc<Mutex<Maze>>,

    // Rendering components
    pub maze_renderer: MazeRenderer,
    pub loading_bar_renderer: LoadingBarRenderer,
    pub exit_shader_renderer: ExitShaderRenderer,

    /// Texture containing the maze image.
    pub texture: wgpu::Texture,
    /// Timestamp of the last update (for animation/timing).
    pub last_update: Instant,
}

impl LoadingRenderer {
    /// Creates a new simplified maze generation animation screen renderer.
    pub fn new(device: &wgpu::Device, surface_config: &wgpu::SurfaceConfiguration) -> Self {
        // Initialize maze generation
        let maze_width = 25;
        let maze_height = 25;
        let (generator, maze) = MazeGenerator::new(maze_width, maze_height);

        // Get render dimensions
        let config = MazeRenderConfig::new(maze_width as u32, maze_height as u32);
        let (texture, texture_view, sampler) = config.create_maze_texture(device);

        // Create rendering components
        let maze_renderer = MazeRenderer::new(device, surface_config, &texture_view, &sampler);
        let loading_bar_renderer = LoadingBarRenderer::new(device, surface_config);
        let exit_shader_renderer = ExitShaderRenderer::new(device, surface_config);

        Self {
            generator,
            maze,
            maze_renderer,
            loading_bar_renderer,
            exit_shader_renderer,
            texture,
            last_update: Instant::now(),
        }
    }

    /// Updates the maze texture with new pixel data.
    ///
    /// # Arguments
    /// - `queue`: The wgpu queue to write to the texture.
    /// - `maze_data`: The new maze image data (RGBA, row-major).
    /// - `width`: Width of the maze image.
    /// - `height`: Height of the maze image.
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

    /// Updates the loading bar progress.
    pub fn update_loading_bar(&self, queue: &wgpu::Queue, progress: f32) {
        self.loading_bar_renderer.update_progress(queue, progress);
    }

    /// Updates the exit shader animation.
    pub fn update_exit_shader(&self, queue: &wgpu::Queue, window: &Window) {
        let window_size = window.inner_size();
        let resolution = [window_size.width as f32, window_size.height as f32];
        let time = self.exit_shader_renderer.start_time.elapsed().as_secs_f32();
        self.exit_shader_renderer
            .update_uniforms(queue, resolution, time);
    }

    /// Renders the exit cell with the special shader effect.
    ///
    /// # Arguments
    /// - `render_pass`: The render pass to draw into.
    /// - `exit_cell`: The exit cell coordinates.
    /// - `cell_size`: Size of each cell in pixels.
    /// - `screen_size`: Screen dimensions.
    ///
    /// Renders all components for the maze generation animation.
    pub fn render(&self, render_pass: &mut wgpu::RenderPass, window: &Window) {
        // Render maze background
        self.maze_renderer.render(render_pass);

        // Render loading bar overlay
        self.loading_bar_renderer.render(render_pass);

        // Render exit cell effect if maze has an exit
        if let Ok(maze_guard) = self.maze.lock() {
            if let Some(exit_cell) = maze_guard.exit_cell {
                self.exit_shader_renderer.render_to_cell(
                    render_pass,
                    window,
                    (exit_cell.col, exit_cell.row),
                );
            }
        }
    }

    /// Convenience method to get maze progress for loading bar.
    pub fn get_generation_progress(&self) -> f32 {
        self.generator.get_progress_ratio()
    }

    /// Check if maze generation is complete.
    pub fn is_generation_complete(&self) -> bool {
        self.generator.is_complete()
    }

    /// Get maze dimensions for texture updates.
    pub fn get_maze_dimensions(&self) -> (u32, u32) {
        if let Ok(maze_guard) = self.maze.lock() {
            let (width, height) = maze_guard.get_dimensions();
            (width as u32, height as u32)
        } else {
            (126, 126) // Default fallback
        }
    }
}

/// Uniform data structure for the loading bar shader.
///
/// This structure is uploaded to the GPU and used by the loading bar fragment shader
/// to determine how much of the progress bar to fill.
///
/// ## Memory Layout
///
/// The structure uses `#[repr(C)]` to ensure consistent memory layout between
/// Rust and WGSL. The padding ensures 16-byte alignment as required by WGSL
/// uniform buffer rules.
///
/// ## Shader Usage
///
/// In WGSL, this corresponds to:
/// ```wgsl
/// struct LoadingBarUniforms {
///     progress: f32,
///     // Implicit padding in WGSL
/// }
/// @group(0) @binding(0) var<uniform> uniforms: LoadingBarUniforms;
/// ```
///
/// # Fields
///
/// - `progress` - Loading progress from 0.0 (0%) to 1.0 (100%)
/// - `_padding` - Ensures proper WGSL alignment (16 bytes total)
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LoadingBarUniforms {
    pub progress: f32,
    pub _padding: [f32; 3],
}

/// Uniform data structure for the exit shader effects.
///
/// This structure provides time and screen resolution information to the
/// exit shader for creating animated procedural effects.
///
/// ## Memory Layout
///
/// Uses `#[repr(C)]` for consistent layout. The resolution is stored as a 2-component
/// vector for easy use in shader calculations.
///
/// ## Shader Usage
///
/// In WGSL, this corresponds to:
/// ```wgsl
/// struct ExitShaderUniforms {
///     time: f32,
///     resolution: vec2<f32>,
///     // Implicit padding in WGSL
/// }
/// @group(0) @binding(0) var<uniform> uniforms: ExitShaderUniforms;
/// ```
///
/// # Fields
///
/// - `time` - Elapsed time in seconds since shader start (for animation)
/// - `resolution` - Screen/viewport resolution as [width, height]
/// - `_padding` - Ensures proper WGSL alignment
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ExitShaderUniforms {
    pub time: f32,
    pub resolution: [f32; 2],
    pub _padding: [f32; 3],
}

/// Renders a maze texture to the screen with proper scaling and filtering.
///
/// This renderer takes a pre-generated maze texture and displays it fullscreen
/// or in a specified viewport. It handles texture sampling and can apply
/// various filtering modes.
///
/// ## Rendering Pipeline
///
/// 1. Uses a fullscreen quad vertex buffer
/// 2. Samples the maze texture with the provided sampler
/// 3. Outputs directly to the framebuffer
///
/// ## Texture Requirements
///
/// - Format: `Rgba8UnormSrgb` (or compatible)
/// - Usage: Must include `TEXTURE_BINDING`
/// - The texture should contain the pre-rendered maze data
///
/// ## Performance Notes
///
/// This renderer is very efficient as it only performs texture sampling
/// without complex computations. It's suitable for real-time rendering
/// of static or infrequently updated maze textures.
///
/// ## Example Usage
///
/// ```rust,no_run
/// # use crate::renderer::maze_renderer::MazeRenderer;
/// # let device: egui_wgpu::wgpu::Device = unimplemented!();
/// # let surface_config: egui_wgpu::wgpu::SurfaceConfiguration = unimplemented!();
/// # let texture_view: egui_wgpu::wgpu::TextureView = unimplemented!();
/// # let sampler: egui_wgpu::wgpu::Sampler = unimplemented!();
/// # let mut render_pass: egui_wgpu::wgpu::RenderPass = unimplemented!();
///
/// let renderer = MazeRenderer::new(&device, &surface_config, &texture_view, &sampler);
///
/// // In render loop:
/// renderer.render(&mut render_pass);
/// ```
pub struct MazeRenderer {
    /// The render pipeline configured for maze texture rendering
    pub pipeline: wgpu::RenderPipeline,
    /// Vertex buffer containing fullscreen quad vertices
    pub vertex_buffer: wgpu::Buffer,
    /// Bind group containing the maze texture and sampler
    pub bind_group: wgpu::BindGroup,
}

impl MazeRenderer {
    /// Create a new maze renderer.
    ///
    /// This sets up the complete rendering pipeline for displaying a maze texture,
    /// including the render pipeline, vertex buffer, and bind group.
    ///
    /// ## Shader Requirements
    ///
    /// The maze shader should be located at `../maze/2D-maze-shader.wgsl` relative
    /// to this module and should expect:
    /// - Vertex input: `@location(0) position: vec2<f32>`
    /// - Texture binding: `@group(0) @binding(0) var maze_texture: texture_2d<f32>;`
    /// - Sampler binding: `@group(0) @binding(1) var maze_sampler: sampler;`
    ///
    /// # Parameters
    ///
    /// - `device` - WGPU device for creating GPU resources
    /// - `surface_config` - Surface configuration (provides target format)
    /// - `texture_view` - View of the maze texture to render
    /// - `sampler` - Sampler for texture filtering (typically nearest neighbor for pixel art)
    ///
    /// # Returns
    ///
    /// A configured `MazeRenderer` ready for rendering.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::maze_renderer::{MazeRenderer, MazeRenderConfig};
    /// # let device: egui_wgpu::wgpu::Device = unimplemented!();
    /// # let surface_config: egui_wgpu::wgpu::SurfaceConfiguration = unimplemented!();
    ///
    /// // Create maze texture
    /// let config = MazeRenderConfig::new(25, 25);
    /// let (texture, texture_view, sampler) = config.create_maze_texture(&device);
    ///
    /// let renderer = MazeRenderer::new(&device, &surface_config, &texture_view, &sampler);
    /// ```
    pub fn new(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        texture_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> Self {
        let bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label("Maze Texture Bind Group Layout")
            .with_texture(0, wgpu::ShaderStages::FRAGMENT)
            .with_sampler(1, wgpu::ShaderStages::FRAGMENT)
            .build();

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
            label: Some("Maze Texture Bind Group"),
        });

        let pipeline = PipelineBuilder::new(device, surface_config.format)
            .with_label("Maze Render Pipeline")
            .with_shader(include_str!("./shaders/2D-maze-shader.wgsl"))
            .with_vertex_buffer(create_vertex_2d_layout())
            .with_bind_group_layout(&bind_group_layout)
            .build();

        let vertex_buffer = create_fullscreen_vertices(device);

        Self {
            pipeline,
            vertex_buffer,
            bind_group,
        }
    }

    /// Render the maze to the current render pass.
    ///
    /// This method performs the actual rendering by setting up the pipeline,
    /// bind groups, and vertex buffer, then issuing a draw call for the
    /// fullscreen quad.
    ///
    /// ## Render State
    ///
    /// This method assumes:
    /// - A render pass is active
    /// - The render pass targets are compatible with the pipeline
    /// - No scissor test or viewport restrictions (renders fullscreen)
    ///
    /// ## Performance
    ///
    /// Very fast - only 6 vertices and simple texture sampling.
    /// Suitable for real-time rendering at high frame rates.
    ///
    /// # Parameters
    ///
    /// - `render_pass` - Active render pass to render into
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::maze_renderer::MazeRenderer;
    /// # let renderer: MazeRenderer = unimplemented!();
    /// # let mut render_pass: egui_wgpu::wgpu::RenderPass = unimplemented!();
    ///
    /// // In render loop:
    /// renderer.render(&mut render_pass);
    /// ```
    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..6, 0..1);
    }
}

/// Renders an animated progress bar for loading operations.
///
/// This renderer displays a customizable loading bar that shows progress
/// from 0% to 100%. It uses alpha blending to overlay on top of other
/// content and can be styled through the shader.
///
/// ## Visual Design
///
/// The loading bar appearance is defined in the shader and typically includes:
/// - Background bar (usually semi-transparent)
/// - Filled portion indicating progress (usually opaque, colored)
/// - Smooth progress animation
/// - Optional decorative elements (borders, gradients, etc.)
///
/// ## Blending
///
/// Uses alpha blending to overlay on existing content. This allows the
/// loading bar to appear on top of other UI elements or the maze background.
///
/// ## Performance
///
/// Very lightweight - only updates uniform buffer when progress changes
/// and renders a simple fullscreen quad.
///
/// ## Example Usage
///
/// ```rust,no_run
/// # use crate::renderer::maze_renderer::LoadingBarRenderer;
/// # let device: egui_wgpu::wgpu::Device = unimplemented!();
/// # let surface_config: egui_wgpu::wgpu::SurfaceConfiguration = unimplemented!();
/// # let queue: egui_wgpu::wgpu::Queue = unimplemented!();
/// # let mut render_pass: egui_wgpu::wgpu::RenderPass = unimplemented!();
///
/// let mut renderer = LoadingBarRenderer::new(&device, &surface_config);
///
/// // Update progress (0.0 to 1.0)
/// renderer.update_progress(&queue, 0.5); // 50% complete
///
/// // Render the loading bar
/// renderer.render(&mut render_pass);
/// ```
pub struct LoadingBarRenderer {
    /// The render pipeline configured for loading bar rendering with alpha blending
    pub pipeline: wgpu::RenderPipeline,
    /// Vertex buffer containing fullscreen quad vertices
    pub vertex_buffer: wgpu::Buffer,
    /// Uniform buffer storing the current progress value
    pub uniform_buffer: wgpu::Buffer,
    /// Bind group containing the uniform buffer
    pub bind_group: wgpu::BindGroup,
}

impl LoadingBarRenderer {
    /// Create a new loading bar renderer.
    ///
    /// This sets up the complete rendering pipeline for displaying a progress bar,
    /// including alpha blending support for overlay rendering.
    ///
    /// ## Shader Requirements
    ///
    /// The loading bar shader should be located at `../maze/loading-bar-shader.wgsl`
    /// and should expect:
    /// - Vertex input: `@location(0) position: vec2<f32>`
    /// - Uniform binding: `@group(0) @binding(0) var<uniform> uniforms: LoadingBarUniforms;`
    ///
    /// ## Initial State
    ///
    /// The renderer starts with 0% progress. Use [`update_progress()`](LoadingBarRenderer::update_progress)
    /// to change the progress value.
    ///
    /// # Parameters
    ///
    /// - `device` - WGPU device for creating GPU resources
    /// - `surface_config` - Surface configuration (provides target format)
    ///
    /// # Returns
    ///
    /// A configured `LoadingBarRenderer` ready for rendering.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::maze_renderer::LoadingBarRenderer;
    /// # let device: egui_wgpu::wgpu::Device = unimplemented!();
    /// # let surface_config: egui_wgpu::wgpu::SurfaceConfiguration = unimplemented!();
    ///
    /// let renderer = LoadingBarRenderer::new(&device, &surface_config);
    /// ```
    pub fn new(device: &wgpu::Device, surface_config: &wgpu::SurfaceConfiguration) -> Self {
        let uniforms = LoadingBarUniforms {
            progress: 0.0,
            _padding: [0.0; 3],
        };

        let uniform_buffer = create_uniform_buffer(device, &uniforms, "Loading Bar Uniform Buffer");

        let bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label("Loading Bar Bind Group Layout")
            .with_uniform_buffer(0, wgpu::ShaderStages::FRAGMENT)
            .build();

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("Loading Bar Bind Group"),
        });

        let pipeline = PipelineBuilder::new(device, surface_config.format)
            .with_label("Loading Bar Pipeline")
            .with_shader(include_str!("./shaders/loading-bar-shader.wgsl"))
            .with_vertex_buffer(create_vertex_2d_layout())
            .with_bind_group_layout(&bind_group_layout)
            .with_alpha_blending()
            .build();

        let vertex_buffer = create_fullscreen_vertices(device);

        Self {
            pipeline,
            vertex_buffer,
            uniform_buffer,
            bind_group,
        }
    }

    /// Update the loading progress.
    ///
    /// This method uploads new progress data to the GPU uniform buffer.
    /// The progress value is automatically clamped to the valid range [0.0, 1.0].
    ///
    /// ## Performance Notes
    ///
    /// This operation involves a GPU buffer write, so avoid calling it
    /// unnecessarily frequently (e.g., every frame with the same value).
    /// It's designed for periodic updates as loading progresses.
    ///
    /// # Parameters
    ///
    /// - `queue` - WGPU queue for buffer uploads
    /// - `progress` - Progress value from 0.0 (0%) to 1.0 (100%)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::maze_renderer::LoadingBarRenderer;
    /// # let renderer: LoadingBarRenderer = unimplemented!();
    /// # let queue: egui_wgpu::wgpu::Queue = unimplemented!();
    ///
    /// // Update progress to 75%
    /// renderer.update_progress(&queue, 0.75);
    ///
    /// // Values outside [0,1] are automatically clamped
    /// renderer.update_progress(&queue, 1.5); // Becomes 1.0
    /// renderer.update_progress(&queue, -0.1); // Becomes 0.0
    /// ```
    pub fn update_progress(&self, queue: &wgpu::Queue, progress: f32) {
        let uniforms = LoadingBarUniforms {
            progress: progress.clamp(0.0, 1.0),
            _padding: [0.0; 3],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Render the loading bar to the current render pass.
    ///
    /// This method renders the loading bar as an overlay using alpha blending.
    /// The bar will appear on top of any previously rendered content.
    ///
    /// ## Render State
    ///
    /// This method assumes:
    /// - A render pass is active
    /// - Alpha blending is desired (the pipeline enables it)
    /// - The loading bar should cover the full viewport
    ///
    /// # Parameters
    ///
    /// - `render_pass` - Active render pass to render into
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::maze_renderer::LoadingBarRenderer;
    /// # let renderer: LoadingBarRenderer = unimplemented!();
    /// # let mut render_pass: egui_wgpu::wgpu::RenderPass = unimplemented!();
    ///
    /// // Render background content first
    /// // ... other rendering ...
    ///
    /// // Render loading bar on top
    /// renderer.render(&mut render_pass);
    /// ```
    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..6, 0..1);
    }
}

/// Renders animated procedural effects for the maze exit cell.
///
/// This renderer creates dynamic visual effects specifically for the exit cell
/// of the maze. It uses a full-screen triangle technique for efficiency and
/// applies scissor testing to limit rendering to only the exit cell area.
///
/// ## Rendering Technique
///
/// Uses the "full-screen triangle" technique instead of a quad:
/// - Single triangle with vertices outside the viewport
/// - GPU clips to viewport automatically
/// - More efficient than quad (fewer vertices, no diagonal edge)
/// - Combined with scissor test for precise cell targeting
///
/// ## Coordinate System
///
/// The renderer expects maze coordinates where:
/// - (0,0) is the top-left cell
/// - Coordinates increase right (x) and down (y)
/// - Grid size is assumed to be 25x25 cells
/// - Cell rendering includes shrinking for visual borders
///
/// ## Animation
///
/// The shader receives time and resolution uniforms for creating
/// time-based animations. The internal timer starts when the renderer
/// is created.
///
/// ## Example Usage
///
/// ```rust,no_run
/// # use crate::renderer::maze_renderer::ExitShaderRenderer;
/// # let device: egui_wgpu::wgpu::Device = unimplemented!();
/// # let surface_config: egui_wgpu::wgpu::SurfaceConfiguration = unimplemented!();
/// # let queue: egui_wgpu::wgpu::Queue = unimplemented!();
/// # let window: &winit::window::Window = unimplemented!();
/// # let mut render_pass: egui_wgpu::wgpu::RenderPass = unimplemented!();
///
/// let mut renderer = ExitShaderRenderer::new(&device, &surface_config);
///
/// // In render loop:
/// let elapsed = renderer.start_time.elapsed().as_secs_f32();
/// renderer.update_uniforms(&queue, [800.0, 600.0], elapsed);
/// renderer.render_to_cell(&mut render_pass, window, (24, 24)); // Bottom-right exit
/// ```
pub struct ExitShaderRenderer {
    pub pipeline: wgpu::RenderPipeline,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub start_time: Instant,
}

impl ExitShaderRenderer {
    pub fn new(device: &wgpu::Device, surface_config: &wgpu::SurfaceConfiguration) -> Self {
        let uniforms = ExitShaderUniforms {
            time: 0.0,
            resolution: [800.0, 600.0], // Default resolution, will be updated
            _padding: [0.0; 3],
        };

        let uniform_buffer = create_uniform_buffer(device, &uniforms, "Exit Shader Uniform Buffer");

        let bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label("Exit Shader Bind Group Layout")
            .with_uniform_buffer(0, wgpu::ShaderStages::VERTEX_FRAGMENT)
            .build();

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("Exit Shader Bind Group"),
        });

        // Exit shader uses a full-screen triangle trick, so no vertex buffer needed
        let pipeline = PipelineBuilder::new(device, surface_config.format)
            .with_label("Exit Shader Pipeline")
            .with_shader(include_str!("./shaders/exit_shader.wgsl"))
            .with_bind_group_layout(&bind_group_layout)
            .build();

        Self {
            pipeline,
            uniform_buffer,
            bind_group,
            start_time: Instant::now(),
        }
    }

    pub fn update_uniforms(&self, queue: &wgpu::Queue, resolution: [f32; 2], time: f32) {
        let uniforms = ExitShaderUniforms {
            time,
            resolution,
            _padding: [0.0; 3],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    pub fn render_to_cell(
        &self,
        render_pass: &mut wgpu::RenderPass,
        window: &winit::window::Window,
        exit_cell: (usize, usize),
    ) {
        // Calculate scissor rectangle for the exit cell
        let grid_size = 25.0;
        let shrink_factor = 0.845;
        let border = 3.0;

        let window_size = window.inner_size();
        let total_width = window_size.width as f32;
        let total_height = window_size.height as f32;

        let usable_width = total_width - 2.0 * border;
        let usable_height = total_height - 2.0 * border;

        let cell_width = usable_width / grid_size;
        let cell_height = usable_height / grid_size;

        let full_x = border + exit_cell.0 as f32 * cell_width;
        let full_y = border + exit_cell.1 as f32 * cell_height;

        let shrunk_width = cell_width * shrink_factor;
        let shrunk_height = cell_height * shrink_factor;

        let offset_x = (cell_width - shrunk_width) / 2.0;
        let offset_y = (cell_height - shrunk_height) / 2.0;

        let scissor_x = (full_x + offset_x).round() as u32;
        let scissor_y = (full_y + offset_y).round() as u32;
        let scissor_width = shrunk_width.round() as u32;
        let scissor_height = shrunk_height.round() as u32;

        // Set scissor rect and render
        render_pass.set_scissor_rect(scissor_x, scissor_y, scissor_width, scissor_height);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1); // Full-screen triangle
    }
}

/// Configuration for maze rendering setup
pub struct MazeRenderConfig {
    pub maze_width: u32,
    pub maze_height: u32,
    pub render_width: u32,
    pub render_height: u32,
}

impl MazeRenderConfig {
    pub fn new(maze_width: u32, maze_height: u32) -> Self {
        // Calculate render dimensions (assuming 5x scale factor + 1 for borders)
        let render_width = maze_width * 5 + 1;
        let render_height = maze_height * 5 + 1;

        Self {
            maze_width,
            maze_height,
            render_width,
            render_height,
        }
    }

    pub fn create_maze_texture(
        &self,
        device: &wgpu::Device,
    ) -> (wgpu::Texture, wgpu::TextureView, wgpu::Sampler) {
        let texture_size = wgpu::Extent3d {
            width: self.render_width,
            height: self.render_height,
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

        (texture, texture_view, sampler)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GameOverUniforms {
    time: f32,
    _padding: [f32; 3],
}

pub struct GameOverRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl GameOverRenderer {
    /// Create a new game over renderer.
    ///
    /// This sets up the complete rendering pipeline for displaying a game over overlay,
    /// including alpha blending support for semitransparent rendering over the game scene.
    ///
    /// ## Shader Requirements
    ///
    /// The game over shader should be located at `../maze/game-over.wgsl`
    /// and should expect:
    /// - Vertex input: `@location(0) position: vec2<f32>`
    /// - Uniform binding: `@group(0) @binding(0) var<uniform> uniforms: GameOverUniforms;`
    ///
    /// ## Initial State
    ///
    /// The renderer starts with time = 0.0. Use [`update_time()`](GameOverRenderer::update_time)
    /// to animate the overlay effect.
    ///
    /// # Parameters
    ///
    /// - `device` - WGPU device for creating GPU resources
    /// - `surface_config` - Surface configuration (provides target format)
    ///
    /// # Returns
    ///
    /// A configured `GameOverRenderer` ready for rendering.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::maze_renderer::GameOverRenderer;
    /// # let device: egui_wgpu::wgpu::Device = unimplemented!();
    /// # let surface_config: egui_wgpu::wgpu::SurfaceConfiguration = unimplemented!();
    ///
    /// let renderer = GameOverRenderer::new(&device, &surface_config);
    /// ```
    pub fn new(device: &wgpu::Device, surface_config: &wgpu::SurfaceConfiguration) -> Self {
        let uniforms = GameOverUniforms {
            time: 0.0,
            _padding: [0.0; 3],
        };

        let uniform_buffer = create_uniform_buffer(device, &uniforms, "Game Over Uniform Buffer");

        let bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label("Game Over Bind Group Layout")
            .with_uniform_buffer(0, wgpu::ShaderStages::FRAGMENT)
            .build();

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("Game Over Bind Group"),
        });

        let pipeline = PipelineBuilder::new(device, surface_config.format)
            .with_label("Game Over Pipeline")
            .with_shader(include_str!("./shaders/game-over.wgsl"))
            .with_vertex_buffer(create_vertex_2d_layout())
            .with_bind_group_layout(&bind_group_layout)
            .with_alpha_blending()
            .build();

        let vertex_buffer = create_fullscreen_vertices(device);

        Self {
            pipeline,
            vertex_buffer,
            uniform_buffer,
            bind_group,
        }
    }

    /// Update the animation time for the game over overlay.
    ///
    /// This method uploads new time data to the GPU uniform buffer to enable
    /// animated effects like pulsing, fading, or other time-based animations
    /// in the game over shader.
    ///
    /// ## Performance Notes
    ///
    /// This operation involves a GPU buffer write. It's typically called once
    /// per frame during the game over state to maintain smooth animations.
    ///
    /// # Parameters
    ///
    /// - `queue` - WGPU queue for buffer uploads
    /// - `time` - Time value in seconds (typically elapsed time since game over)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::maze_renderer::GameOverRenderer;
    /// # let renderer: GameOverRenderer = unimplemented!();
    /// # let queue: egui_wgpu::wgpu::Queue = unimplemented!();
    ///
    /// // Update with elapsed time for animations
    /// let elapsed = start_time.elapsed().as_secs_f32();
    /// renderer.update_time(&queue, elapsed);
    /// ```
    pub fn update_time(&self, queue: &wgpu::Queue, time: f32) {
        let uniforms = GameOverUniforms {
            time,
            _padding: [0.0; 3],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Render the game over overlay to the current render pass.
    ///
    /// This method renders a semitransparent red overlay that covers the entire
    /// screen using alpha blending. The overlay appears on top of the frozen
    /// game scene underneath.
    ///
    /// ## Render State
    ///
    /// This method assumes:
    /// - A render pass is active
    /// - Alpha blending is desired (the pipeline enables it)
    /// - The game scene has already been rendered
    /// - The overlay should cover the full viewport
    ///
    /// # Parameters
    ///
    /// - `render_pass` - Active render pass to render into
    /// - `window` - Window reference for potential window-specific adjustments
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::maze_renderer::GameOverRenderer;
    /// # let renderer: GameOverRenderer = unimplemented!();
    /// # let mut render_pass: egui_wgpu::wgpu::RenderPass = unimplemented!();
    /// # let window: &winit::window::Window = unimplemented!();
    ///
    /// // Render game scene first
    /// // ... render background, stars, game objects ...
    ///
    /// // Render game over overlay on top
    /// renderer.render(&mut render_pass, window);
    /// ```
    pub fn render(&self, render_pass: &mut wgpu::RenderPass, _window: &winit::window::Window) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..6, 0..1);
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CompassUniforms {
    screen_position: [f32; 2], // Bottom-right position
    compass_size: [f32; 2],    // Width and height
    _padding: [f32; 4],
}

pub struct CompassRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    base_bind_group: wgpu::BindGroup,
    needle_bind_groups: Vec<wgpu::BindGroup>,
    current_needle_index: usize,

    // Simple smoothing for compass direction
    smoothed_compass_angle: f32,
    smoothing_factor: f32,
}

impl CompassRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        // Load compass base texture
        let base_texture = Self::load_base_texture(device, queue);

        // Load all needle textures
        let needle_textures = Self::load_needle_textures(device, queue);

        let uniforms = CompassUniforms {
            screen_position: [0.85, 0.85], // Bottom-right corner (normalized coordinates)
            compass_size: [0.12, 0.12],    // 12% of screen size
            _padding: [0.0; 4],
        };

        let uniform_buffer = create_uniform_buffer(device, &uniforms, "Compass Uniform Buffer");

        // Create bind group layout for texture + sampler + uniforms
        let bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label("Compass Bind Group Layout")
            .with_uniform_buffer(0, wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT)
            .with_texture(1, wgpu::ShaderStages::FRAGMENT)
            .with_sampler(2, wgpu::ShaderStages::FRAGMENT)
            .build();

        // Create sampler for all textures
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create bind group for base texture
        let base_texture_view = base_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let base_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&base_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("Compass Base Bind Group"),
        });

        // Create bind groups for each needle texture
        let needle_bind_groups: Vec<wgpu::BindGroup> = needle_textures
            .iter()
            .enumerate()
            .map(|(i, texture)| {
                let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: uniform_buffer.as_entire_binding(),
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
                    label: Some(&format!("Compass Needle Bind Group {}", i)),
                })
            })
            .collect();

        // Create vertex buffer layout for position + tex_coords
        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: 4 * 4, // 4 floats * 4 bytes each
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2, // position
                },
                wgpu::VertexAttribute {
                    offset: 2 * 4, // 2 floats * 4 bytes
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2, // tex_coords
                },
            ],
        };

        let pipeline = PipelineBuilder::new(device, surface_config.format)
            .with_label("Compass Pipeline")
            .with_shader(include_str!("./shaders/compass.wgsl"))
            .with_vertex_buffer(vertex_buffer_layout)
            .with_bind_group_layout(&bind_group_layout)
            .with_alpha_blending()
            .build();

        let vertex_buffer = Self::create_compass_vertices(device);

        Self {
            pipeline,
            vertex_buffer,
            uniform_buffer,
            base_bind_group,
            needle_bind_groups,
            current_needle_index: 0,

            smoothed_compass_angle: 0.0,
            smoothing_factor: 0.8, // Higher = more responsive, lower = smoother
        }
    }

    fn load_base_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
        let path = "assets/compass/gold-compass.png";

        // Load image using image crate
        let img = match image::open(path) {
            Ok(img) => img.to_rgba8(),
            Err(e) => {
                eprintln!("Failed to load compass base texture {}: {}", path, e);
                // Create a fallback texture (solid color or default compass)
                image::RgbaImage::new(64, 64)
            }
        };

        let dimensions = img.dimensions();
        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Compass Base Texture"),
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

    fn load_needle_textures(device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<wgpu::Texture> {
        let mut textures = Vec::new();

        // Load needle textures (needle-1.png through needle-12.png)
        for i in 0..=11 {
            let path = format!("assets/compass/needle/needle-{}.png", i);

            // Load image using image crate
            let img = match image::open(&path) {
                Ok(img) => img.to_rgba8(),
                Err(e) => {
                    eprintln!("Failed to load needle texture {}: {}", path, e);
                    // Create a fallback texture (transparent or simple needle)
                    image::RgbaImage::new(64, 64)
                }
            };

            let dimensions = img.dimensions();
            let texture_size = wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            };

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("Compass Needle Texture {}", i)),
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

            textures.push(texture);
        }

        textures
    }

    fn create_compass_vertices(device: &wgpu::Device) -> wgpu::Buffer {
        // Create a quad for the compass (will be positioned via uniforms in shader)
        // Raw vertex data: [x, y, u, v] for each vertex
        let vertices: &[f32] = &[
            // Triangle 1
            -1.0, -1.0, 0.0, 1.0, // Bottom-left
            1.0, -1.0, 1.0, 1.0, // Bottom-right
            -1.0, 1.0, 0.0, 0.0, // Top-left
            // Triangle 2
            1.0, -1.0, 1.0, 1.0, // Bottom-right
            1.0, 1.0, 1.0, 0.0, // Top-right
            -1.0, 1.0, 0.0, 0.0, // Top-left
        ];

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Compass Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        })
    }

    /// Update compass position and size
    pub fn update_uniforms(
        &self,
        queue: &wgpu::Queue,
        screen_position: [f32; 2],
        compass_size: [f32; 2],
    ) {
        let uniforms = CompassUniforms {
            screen_position,
            compass_size,
            _padding: [0.0; 4],
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass, _window: &winit::window::Window) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        // First render the compass base
        render_pass.set_bind_group(0, &self.base_bind_group, &[]);
        render_pass.draw(0..6, 0..1);

        // Then render the needle on top
        render_pass.set_bind_group(0, &self.needle_bind_groups[self.current_needle_index], &[]);
        render_pass.draw(0..6, 0..1);
    }

    /// Calculate which needle image to show based on player and exit positions
    pub fn update_compass_direction(&mut self, player_pos: (f32, f32), exit_pos: (f32, f32)) {
        // Vector from player to exit
        let direction_vector = (player_pos.0 - exit_pos.0, player_pos.1 - exit_pos.1);

        // Skip if too close (avoid jitter when on top of exit)
        let distance_sq =
            direction_vector.0 * direction_vector.0 + direction_vector.1 * direction_vector.1;
        if distance_sq < 0.0001 {
            return;
        }

        // Calculate angle to exit in world space
        let mut target_angle = direction_vector.1.atan2(direction_vector.0); // [-π, π]

        // Normalize to [0, 2π]
        if target_angle < 0.0 {
            target_angle += 2.0 * std::f32::consts::PI;
        }

        // Smooth angle update (exponential smoothing)
        let alpha = self.smoothing_factor; // Lower = slower/smoother
        let mut delta = target_angle - self.smoothed_compass_angle;

        // Wrap to [-π, π] for shortest rotation
        if delta > std::f32::consts::PI {
            delta -= 2.0 * std::f32::consts::PI;
        } else if delta < -std::f32::consts::PI {
            delta += 2.0 * std::f32::consts::PI;
        }

        self.smoothed_compass_angle += alpha * delta;

        // Re-wrap smoothed angle to [0, 2π]
        while self.smoothed_compass_angle < 0.0 {
            self.smoothed_compass_angle += 2.0 * std::f32::consts::PI;
        }
        while self.smoothed_compass_angle >= 2.0 * std::f32::consts::PI {
            self.smoothed_compass_angle -= 2.0 * std::f32::consts::PI;
        }

        // Map to needle frame (0–11, since we have 12 needles indexed 1-12)
        let new_index = ((self.smoothed_compass_angle / (2.0 * std::f32::consts::PI)) * 12.0)
            .floor() as usize
            % 12;

        self.current_needle_index = new_index;
    }

    /// Updates the compass to point toward the exit from the player's current position.
    ///
    /// This function calculates the direction from the player to the exit cell and
    /// adjusts for the player's current orientation (yaw) so that the compass always
    /// indicates the direction the player should move to reach the exit.
    ///
    /// # Arguments
    ///
    /// * `player_pos` - The player's position as (x, z) coordinates
    /// * `exit_pos` - The exit's position as (x, z) coordinates
    /// * `player_yaw_degrees` - The player's current yaw angle in degrees
    pub fn update_compass_with_yaw(
        &mut self,
        player_pos: (f32, f32), // (x, z) coordinates
        exit_pos: (f32, f32),   // (x, z) coordinates
        player_yaw_degrees: f32,
    ) {
        // Calculate vector from player to exit
        let dx = exit_pos.0 - player_pos.0; // Change in X
        let dz = exit_pos.1 - player_pos.1; // Change in Z

        let distance_sq = dx * dx + dz * dz;

        // Skip if too close to exit
        if distance_sq < 0.0001 {
            return;
        }

        // Calculate direction to exit using the same trig approach as player movement
        // First, get forward vector based on player's yaw (same as in move_forward)
        let forward_x = player_yaw_degrees.to_radians().sin();
        let forward_z = player_yaw_degrees.to_radians().cos();

        // Get right vector (same as in move_right)
        let right_x = player_yaw_degrees.to_radians().cos();
        let right_z = player_yaw_degrees.to_radians().sin();

        // Normalize the direction vector to the exit
        let length = distance_sq.sqrt();
        let dir_x = dx / length;
        let dir_z = dz / length;

        // Calculate dot products to determine the angle
        let forward_dot = -forward_x * dir_x - forward_z * dir_z; // Dot product with forward vector
        let right_dot = right_x * dir_x - right_z * dir_z; // Dot product with right vector

        // Calculate angle using atan2
        let mut target_compass_angle = right_dot.atan2(forward_dot);

        // Normalize to [-π, π]
        target_compass_angle = self.normalize_angle(target_compass_angle);

        // Initialize smoothed angle on first update
        if self.smoothed_compass_angle.is_nan() {
            self.smoothed_compass_angle = target_compass_angle;
        }

        // Calculate the shortest angular distance for smooth interpolation
        let angle_diff =
            self.shortest_angle_diff(target_compass_angle, self.smoothed_compass_angle);

        // Apply smoothing
        self.smoothed_compass_angle += angle_diff * self.smoothing_factor;

        // Normalize the smoothed angle
        self.smoothed_compass_angle = self.normalize_angle(self.smoothed_compass_angle);

        // Convert to needle index (0-11 for 12 needle sprites)
        // Convert from [-π, π] to [0, 2π] for easier indexing
        let angle_for_index = if self.smoothed_compass_angle < 0.0 {
            self.smoothed_compass_angle + 2.0 * std::f32::consts::PI
        } else {
            self.smoothed_compass_angle
        };

        // Convert to 12-segment index (each segment is 30° = π/6 radians)
        // Add half a segment (π/12) for proper rounding to nearest segment
        let needle_index = ((angle_for_index + std::f32::consts::PI / 12.0)
            / (std::f32::consts::PI / 6.0))
            .floor() as usize
            % 12;

        self.current_needle_index = needle_index;
    }

    /// Normalize angle to [-π, π]
    fn normalize_angle(&self, mut angle: f32) -> f32 {
        while angle > std::f32::consts::PI {
            angle -= 2.0 * std::f32::consts::PI;
        }
        while angle < -std::f32::consts::PI {
            angle += 2.0 * std::f32::consts::PI;
        }
        angle
    }

    /// Calculate shortest angular difference between two angles
    fn shortest_angle_diff(&self, target: f32, current: f32) -> f32 {
        let mut diff = target - current;

        // Wrap to shortest path
        if diff > std::f32::consts::PI {
            diff -= 2.0 * std::f32::consts::PI;
        } else if diff < -std::f32::consts::PI {
            diff += 2.0 * std::f32::consts::PI;
        }

        diff
    }

    /// Alternative update with configurable smoothing
    pub fn update_compass_with_smoothing(
        &mut self,
        player_pos: (f32, f32),
        exit_pos: (f32, f32),
        player_yaw_degrees: f32,
        smoothing: f32, // 0.0 = very smooth, 1.0 = instant response
    ) {
        let old_smoothing = self.smoothing_factor;
        self.smoothing_factor = smoothing.clamp(0.01, 1.0);

        self.update_compass_with_yaw(player_pos, exit_pos, player_yaw_degrees);

        self.smoothing_factor = old_smoothing;
    }

    /// For debugging - get current compass angle in degrees
    pub fn get_compass_angle_degrees(&self) -> f32 {
        self.smoothed_compass_angle.to_degrees()
    }

    /// Set smoothing factor (0.0 = very smooth, 1.0 = instant)
    pub fn set_smoothing_factor(&mut self, factor: f32) {
        self.smoothing_factor = factor.clamp(0.01, 1.0);
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct EnemyUniforms {
    model_matrix: [[f32; 4]; 4],
    view_proj_matrix: [[f32; 4]; 4],
}

#[allow(dead_code)]
pub struct EnemyRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
}

impl EnemyRenderer {
    /// Create a new enemy renderer.
    ///
    /// This sets up the complete rendering pipeline for displaying enemy sprites,
    /// including texture loading, vertex/index buffers, and transform matrices
    /// for positioning enemies in the game world.
    ///
    /// ## Shader Requirements
    ///
    /// The enemy shader should be located at `../maze/enemy.wgsl`
    /// and should expect:
    /// - Vertex input: `@location(0) position: vec3<f32>`, `@location(1) tex_coords: vec2<f32>`
    /// - Uniform binding: `@group(0) @binding(0) var<uniform> uniforms: EnemyUniforms;`
    /// - Texture binding: `@group(0) @binding(1) var enemy_texture: texture_2d<f32>;`
    /// - Sampler binding: `@group(0) @binding(2) var enemy_sampler: sampler;`
    ///
    /// ## Asset Requirements
    ///
    /// The enemy sprite should be located at `assets/frankie.png`
    ///
    /// # Parameters
    ///
    /// - `device` - WGPU device for creating GPU resources
    /// - `queue` - WGPU queue for uploading texture data
    /// - `surface_config` - Surface configuration (provides target format)
    ///
    /// # Returns
    ///
    /// A configured `EnemyRenderer` ready for rendering enemy sprites.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::maze_renderer::EnemyRenderer;
    /// # let device: egui_wgpu::wgpu::Device = unimplemented!();
    /// # let queue: egui_wgpu::wgpu::Queue = unimplemented!();
    /// # let surface_config: egui_wgpu::wgpu::SurfaceConfiguration = unimplemented!();
    ///
    /// let renderer = EnemyRenderer::new(&device, &queue, &surface_config);
    /// ```
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        // Load enemy texture
        let enemy_image = image::load_from_memory(include_bytes!("../../assets/frankie.png"))
            .expect("Failed to load enemy texture")
            .to_rgba8();

        let texture_size = wgpu::Extent3d {
            width: enemy_image.width(),
            height: enemy_image.height(),
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Enemy Texture"),
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
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &enemy_image,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * enemy_image.width()),
                rows_per_image: Some(enemy_image.height()),
            },
            texture_size,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Enemy Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Initialize with identity matrices
        let uniforms = EnemyUniforms {
            model_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            view_proj_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        };

        let uniform_buffer = create_uniform_buffer(device, &uniforms, "Enemy Uniform Buffer");

        let bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label("Enemy Bind Group Layout")
            .with_uniform_buffer(0, wgpu::ShaderStages::VERTEX)
            .with_texture(1, wgpu::ShaderStages::FRAGMENT)
            .with_sampler(2, wgpu::ShaderStages::FRAGMENT)
            .build();

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
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
            label: Some("Enemy Bind Group"),
        });

        let pipeline = PipelineBuilder::new(device, surface_config.format)
            .with_label("Enemy Pipeline")
            .with_shader(include_str!("./shaders/enemy.wgsl"))
            .with_vertex_buffer(create_vertex_3d_layout())
            .with_bind_group_layout(&bind_group_layout)
            .with_alpha_blending()
            .build();

        let vertex_buffer = create_sprite_vertices(device);
        let index_buffer = create_sprite_indices(device);

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            texture,
            texture_view,
            sampler,
            bind_group,
        }
    }

    /// Update the enemy's transformation matrices.
    ///
    /// This method uploads new transformation data to the GPU uniform buffer
    /// to position, rotate, and scale the enemy sprite in the game world.
    ///
    /// ## Performance Notes
    ///
    /// This operation involves a GPU buffer write. It should be called once
    /// per frame per enemy to update their positions smoothly.
    ///
    /// # Parameters
    ///
    /// - `queue` - WGPU queue for buffer uploads
    /// - `position` - World position of the enemy (x, y)
    /// - `size` - Size of the enemy sprite (width, height)
    /// - `view_proj_matrix` - Combined view and projection matrix from camera
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::maze_renderer::EnemyRenderer;
    /// # let renderer: EnemyRenderer = unimplemented!();
    /// # let queue: egui_wgpu::wgpu::Queue = unimplemented!();
    /// # let view_proj: [[f32; 4]; 4] = unimplemented!();
    ///
    /// // Update enemy position and size
    /// renderer.update_transform(&queue, (100.0, 200.0), (32.0, 32.0), view_proj);
    /// ```
    pub fn update_transform(
        &self,
        queue: &wgpu::Queue,
        position: (f32, f32),
        size: (f32, f32),
        view_proj_matrix: [[f32; 4]; 4],
    ) {
        // Create model matrix with translation and scale
        let model_matrix = [
            [size.0, 0.0, 0.0, 0.0],
            [0.0, size.1, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [position.0, position.1, 0.0, 1.0],
        ];

        let uniforms = EnemyUniforms {
            model_matrix,
            view_proj_matrix,
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Render the enemy sprite to the current render pass.
    ///
    /// This method renders the enemy sprite using the previously uploaded
    /// transformation matrices. The sprite will be positioned, scaled, and
    /// rendered with alpha blending support.
    ///
    /// ## Render State
    ///
    /// This method assumes:
    /// - A render pass is active
    /// - Transform matrices have been updated via `update_transform()`
    /// - Alpha blending is desired (the pipeline enables it)
    /// - The background has already been rendered
    ///
    /// # Parameters
    ///
    /// - `render_pass` - Active render pass to render into
    /// - `window` - Window reference for potential window-specific adjustments
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::maze_renderer::EnemyRenderer;
    /// # let renderer: EnemyRenderer = unimplemented!();
    /// # let mut render_pass: egui_wgpu::wgpu::RenderPass = unimplemented!();
    /// # let window: &winit::window::Window = unimplemented!();
    ///
    /// // Render background first
    /// // ... render maze, player, etc ...
    ///
    /// // Update enemy transform
    /// renderer.update_transform(&queue, enemy_position, enemy_size, view_proj);
    ///
    /// // Render enemy sprite
    /// renderer.render(&mut render_pass, window);
    /// ```
    pub fn render(&self, render_pass: &mut wgpu::RenderPass, _window: &winit::window::Window) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
}

// Helper functions for creating sprite geometry
fn create_sprite_vertices(device: &wgpu::Device) -> wgpu::Buffer {
    // Sprite vertices for a unit quad centered at origin
    let vertices = [
        // Position (x, y, z), Texture coordinates (u, v)
        [-0.5, -0.5, 0.0, 0.0, 1.0], // Bottom-left
        [0.5, -0.5, 0.0, 1.0, 1.0],  // Bottom-right
        [0.5, 0.5, 0.0, 1.0, 0.0],   // Top-right
        [-0.5, 0.5, 0.0, 0.0, 0.0],  // Top-left
    ];

    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Enemy Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    })
}

fn create_sprite_indices(device: &wgpu::Device) -> wgpu::Buffer {
    // Indices for two triangles making a quad
    let indices: [u16; 6] = [
        0, 1, 2, // First triangle
        2, 3, 0, // Second triangle
    ];

    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Enemy Index Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    })
}

fn create_vertex_3d_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x2,
            },
        ],
    }
}
