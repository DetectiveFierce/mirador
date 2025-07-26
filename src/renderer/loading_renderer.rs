use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::{
    game::maze::generator::{Maze, MazeGenerator},
    renderer::pipeline_builder::{
        BindGroupLayoutBuilder, PipelineBuilder, create_fullscreen_vertices, create_uniform_buffer,
        create_vertex_2d_layout,
    },
};
use wgpu;
use winit::window::Window;

/// Main loading screen renderer that orchestrates maze generation visualization.
///
/// This renderer manages three visual components during maze generation:
/// - The maze itself being generated in real-time
/// - An animated loading bar showing generation progress
/// - A special effect on the maze exit cell when generation completes
pub struct LoadingRenderer {
    /// The maze generator that runs in a separate thread
    pub generator: MazeGenerator,
    /// Thread-safe reference to the maze being generated
    pub maze: Arc<Mutex<Maze>>,

    // Rendering components
    /// Renders the maze texture to the screen
    pub maze_renderer: MazeRenderer,
    /// Renders an animated progress bar
    pub loading_bar_renderer: LoadingBarRenderer,
    /// Renders special effects on the exit cell
    pub exit_shader_renderer: ExitShaderRenderer,

    /// GPU texture containing the maze visualization data
    pub texture: wgpu::Texture,
    /// Timestamp of the last frame update for timing calculations
    pub last_update: Instant,
}

impl LoadingRenderer {
    /// Creates a new loading renderer with all necessary GPU resources.
    ///
    /// # Arguments
    /// * `device` - The WGPU device for creating GPU resources
    /// * `surface_config` - Surface configuration for render target format
    ///
    /// # Returns
    /// A fully initialized LoadingRenderer ready to render maze generation
    pub fn new(device: &wgpu::Device, surface_config: &wgpu::SurfaceConfiguration) -> Self {
        // Initialize maze generation with fixed dimensions
        let maze_width = 25;
        let maze_height = 25;
        let (generator, maze) = MazeGenerator::new(maze_width, maze_height);

        // Calculate render dimensions and create GPU texture
        let config = MazeRenderConfig::new(maze_width as u32, maze_height as u32);
        let (texture, texture_view, sampler) = config.create_maze_texture(device);

        // Initialize all rendering components with their respective pipelines
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

    /// Updates the maze texture on the GPU with new generation data.
    ///
    /// This is called whenever the maze generator produces new visual data
    /// to keep the rendered maze in sync with the generation progress.
    ///
    /// # Arguments
    /// * `queue` - WGPU command queue for GPU operations
    /// * `maze_data` - Raw RGBA pixel data representing the current maze state
    /// * `width` - Width of the maze data in pixels
    /// * `height` - Height of the maze data in pixels
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
                bytes_per_row: Some(4 * width as u32), // 4 bytes per pixel (RGBA)
                rows_per_image: Some(height as u32),
            },
            wgpu::Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Updates the loading bar with current progress and animation state.
    ///
    /// # Arguments
    /// * `queue` - WGPU command queue for updating uniform buffers
    /// * `progress` - Generation progress from 0.0 to 1.0
    /// * `window` - Window reference for getting current screen dimensions
    pub fn update_loading_bar(&self, queue: &wgpu::Queue, progress: f32, window: &Window) {
        // Get current window dimensions for proper scaling
        let window_size = window.inner_size();
        let resolution = [window_size.width as f32, window_size.height as f32];
        let time = self.loading_bar_renderer.start_time.elapsed().as_secs_f32();

        self.loading_bar_renderer
            .update_uniforms(queue, progress, resolution, time);
    }

    /// Updates the exit cell shader effect with current animation state.
    ///
    /// # Arguments
    /// * `queue` - WGPU command queue for updating uniform buffers
    /// * `window` - Window reference for getting current screen dimensions
    pub fn update_exit_shader(&self, queue: &wgpu::Queue, window: &Window) {
        let window_size = window.inner_size();
        let resolution = [window_size.width as f32, window_size.height as f32];
        let time = self.exit_shader_renderer.start_time.elapsed().as_secs_f32();
        self.exit_shader_renderer
            .update_uniforms(queue, resolution, time);
    }

    /// Renders the complete loading screen with all visual effects.
    ///
    /// The rendering order is:
    /// 1. Maze background (the generating maze)
    /// 2. Loading bar overlay (progress indicator)
    /// 3. Exit cell effect (if maze generation is complete)
    ///
    /// # Arguments
    /// * `render_pass` - Active WGPU render pass to draw into
    /// * `window` - Window reference for screen dimensions and positioning
    pub fn render(&self, render_pass: &mut wgpu::RenderPass, window: &Window) {
        // Render maze background - shows the current generation state
        self.maze_renderer.render(render_pass);

        // Render loading bar overlay with animated effect at the top of screen
        let window_size = window.inner_size();
        let bar_width = window_size.width;
        let bar_height = (window_size.height as f32 * 0.0125).ceil() as u32; // 1.25% of screen height
        let bar_x = 0u32;
        let bar_y = 0u32; // Top of screen
        self.loading_bar_renderer.render_with_scissor(
            render_pass,
            bar_x,
            bar_y,
            bar_width,
            bar_height,
        );

        // Render exit cell effect if maze generation is complete and has an exit
        if let Ok(maze_guard) = self.maze.lock() {
            if let Some(exit_cell) = maze_guard.exit_cell {
                self.exit_shader_renderer.render_to_cell(
                    render_pass,
                    window,
                    (exit_cell.col, exit_cell.row),
                    maze_guard.width,
                    maze_guard.height,
                );
            }
        }
    }

    /// Gets the current maze generation progress as a ratio.
    ///
    /// # Returns
    /// Progress value from 0.0 (just started) to 1.0 (complete)
    pub fn get_generation_progress(&self) -> f32 {
        self.generator.get_progress_ratio()
    }

    /// Checks if maze generation has completed.
    ///
    /// # Returns
    /// `true` if generation is finished, `false` if still in progress
    pub fn is_generation_complete(&self) -> bool {
        self.generator.is_complete()
    }

    /// Gets the dimensions of the maze being generated.
    ///
    /// # Returns
    /// Tuple of (width, height) in maze cells, or (126, 126) as fallback
    pub fn get_maze_dimensions(&self) -> (u32, u32) {
        if let Ok(maze_guard) = self.maze.lock() {
            let (width, height) = maze_guard.get_dimensions();
            (width as u32, height as u32)
        } else {
            (126, 126) // Default fallback if lock fails
        }
    }
}

/// Uniform buffer data for the animated loading bar shader.
///
/// Contains all data needed to render the progress bar with visual effects.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LoadingBarUniforms {
    /// Current loading progress from 0.0 to 1.0
    pub progress: f32,
    /// Animation time in seconds since creation
    pub time: f32,
    /// Screen resolution [width, height] for proper scaling
    pub resolution: [f32; 2],
    /// Padding to ensure proper GPU alignment
    pub _padding: [f32; 2],
}

/// Uniform buffer data for the exit cell shader effect.
///
/// Contains timing and resolution data for the exit cell animation.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ExitShaderUniforms {
    /// Animation time in seconds since creation
    pub time: f32,
    /// Screen resolution [width, height] for proper scaling
    pub resolution: [f32; 2],
    /// Padding to ensure proper GPU alignment
    pub _padding: [f32; 3],
}

/// Renderer responsible for displaying the maze texture as a background.
///
/// This renderer takes the maze texture data and displays it full-screen,
/// providing the visual backdrop for the loading screen.
pub struct MazeRenderer {
    /// GPU render pipeline for maze rendering
    pub pipeline: wgpu::RenderPipeline,
    /// Vertex buffer containing fullscreen quad vertices
    pub vertex_buffer: wgpu::Buffer,
    /// Bind group containing texture and sampler resources
    pub bind_group: wgpu::BindGroup,
}

impl MazeRenderer {
    /// Creates a new maze renderer with the specified texture resources.
    ///
    /// # Arguments
    /// * `device` - WGPU device for creating GPU resources
    /// * `surface_config` - Surface configuration for render target format
    /// * `texture_view` - View of the maze texture to render
    /// * `sampler` - Texture sampler for filtering
    ///
    /// # Returns
    /// A fully initialized MazeRenderer ready to display maze textures
    pub fn new(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        texture_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> Self {
        // Create bind group layout for texture and sampler
        let bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label("Maze Texture Bind Group Layout")
            .with_texture(0, wgpu::ShaderStages::FRAGMENT)
            .with_sampler(1, wgpu::ShaderStages::FRAGMENT)
            .build();

        // Create bind group with actual texture and sampler
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

        // Create render pipeline with maze shader
        let pipeline = PipelineBuilder::new(device, surface_config.format)
            .with_label("Maze Render Pipeline")
            .with_shader(include_str!("./shaders/2D-maze-shader.wgsl"))
            .with_vertex_buffer(create_vertex_2d_layout())
            .with_bind_group_layout(&bind_group_layout)
            .build();

        // Create fullscreen quad vertices
        let vertex_buffer = create_fullscreen_vertices(device);

        Self {
            pipeline,
            vertex_buffer,
            bind_group,
        }
    }

    /// Renders the maze texture to the current render pass.
    ///
    /// # Arguments
    /// * `render_pass` - Active render pass to draw into
    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..6, 0..1); // Draw fullscreen quad (2 triangles)
    }
}

/// Renderer for the animated loading progress bar.
///
/// Creates a thin animated bar at the top of the screen that fills up
/// as maze generation progresses, with visual effects and accelerated
/// filling in the second half for better user experience.
pub struct LoadingBarRenderer {
    /// GPU render pipeline for loading bar effects
    pub pipeline: wgpu::RenderPipeline,
    /// Uniform buffer containing progress and animation data
    pub uniform_buffer: wgpu::Buffer,
    /// Bind group for accessing uniform data in shaders
    pub bind_group: wgpu::BindGroup,
    /// Start time for calculating animation progress
    pub start_time: Instant,
}

impl LoadingBarRenderer {
    /// Creates a new loading bar renderer with animated effects.
    ///
    /// # Arguments
    /// * `device` - WGPU device for creating GPU resources
    /// * `surface_config` - Surface configuration for render target format
    ///
    /// # Returns
    /// A fully initialized LoadingBarRenderer ready to display progress
    pub fn new(device: &wgpu::Device, surface_config: &wgpu::SurfaceConfiguration) -> Self {
        // Initialize uniforms with default values
        let uniforms = LoadingBarUniforms {
            progress: 0.0,
            time: 0.0,
            resolution: [800.0, 600.0], // Default resolution, updated per frame
            _padding: [0.0; 2],
        };

        let uniform_buffer = create_uniform_buffer(device, &uniforms, "Loading Bar Uniform Buffer");

        // Create bind group layout for uniform buffer access
        let bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label("Loading Bar Bind Group Layout")
            .with_uniform_buffer(0, wgpu::ShaderStages::VERTEX_FRAGMENT)
            .build();

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("Loading Bar Bind Group"),
        });

        // Create render pipeline with loading bar shader and alpha blending
        let pipeline = PipelineBuilder::new(device, surface_config.format)
            .with_label("Loading Bar Pipeline")
            .with_shader(include_str!("./shaders/loading-bar.wgsl"))
            .with_bind_group_layout(&bind_group_layout)
            .with_alpha_blending()
            .build();

        Self {
            pipeline,
            uniform_buffer,
            bind_group,
            start_time: Instant::now(),
        }
    }

    /// Updates the loading bar's uniform buffer with current state.
    ///
    /// Includes a visual enhancement where the second half of the progress
    /// bar fills twice as fast for better perceived loading speed.
    ///
    /// # Arguments
    /// * `queue` - WGPU command queue for buffer updates
    /// * `progress` - Actual generation progress (0.0 to 1.0)
    /// * `resolution` - Current screen resolution [width, height]
    /// * `time` - Current animation time in seconds
    pub fn update_uniforms(
        &self,
        queue: &wgpu::Queue,
        progress: f32,
        resolution: [f32; 2],
        time: f32,
    ) {
        // Visual enhancement: make the second half fill twice as fast
        let speedup = 2.0;
        let visual_progress = if progress > 0.5 {
            (0.5 + (progress - 0.5) * speedup).min(1.0)
        } else {
            progress
        };

        let uniforms = LoadingBarUniforms {
            progress: visual_progress,
            time,
            resolution,
            _padding: [0.0; 2],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Renders the loading bar with default positioning (top of screen).
    ///
    /// # Arguments
    /// * `render_pass` - Active render pass to draw into
    /// * `window` - Window reference for getting screen dimensions
    pub fn render(&self, render_pass: &mut wgpu::RenderPass, window: &Window) {
        // Calculate default loading bar position (thin bar across top of screen)
        let window_size = window.inner_size();
        let bar_width = window_size.width; // Full width of screen
        let bar_height = 8u32; // Thin bar height

        let bar_x = 0u32;
        let bar_y = 0u32; // Top of screen

        // Use scissor test to constrain the animated effect to the loading bar area
        render_pass.set_scissor_rect(bar_x, bar_y, bar_width, bar_height);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1); // Full-screen triangle, clipped to loading bar
    }

    /// Renders the loading bar with custom positioning using scissor test.
    ///
    /// # Arguments
    /// * `render_pass` - Active render pass to draw into
    /// * `bar_x` - X position of the loading bar in pixels
    /// * `bar_y` - Y position of the loading bar in pixels
    /// * `bar_width` - Width of the loading bar in pixels
    /// * `bar_height` - Height of the loading bar in pixels
    pub fn render_with_scissor(
        &self,
        render_pass: &mut wgpu::RenderPass,
        bar_x: u32,
        bar_y: u32,
        bar_width: u32,
        bar_height: u32,
    ) {
        render_pass.set_scissor_rect(bar_x, bar_y, bar_width, bar_height);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1); // Full-screen triangle, clipped to specified area
    }
}

/// Renderer for special visual effects on the maze exit cell.
///
/// Provides animated visual effects specifically targeted at the exit cell
/// of the maze once generation is complete, helping draw player attention
/// to the goal location.
pub struct ExitShaderRenderer {
    /// GPU render pipeline for exit cell effects
    pub pipeline: wgpu::RenderPipeline,
    /// Uniform buffer containing animation timing data
    pub uniform_buffer: wgpu::Buffer,
    /// Bind group for accessing uniform data in shaders
    pub bind_group: wgpu::BindGroup,
    /// Start time for calculating animation progress
    pub start_time: Instant,
}

impl ExitShaderRenderer {
    /// Creates a new exit shader renderer for animating the maze exit.
    ///
    /// # Arguments
    /// * `device` - WGPU device for creating GPU resources
    /// * `surface_config` - Surface configuration for render target format
    ///
    /// # Returns
    /// A fully initialized ExitShaderRenderer ready to animate exit effects
    pub fn new(device: &wgpu::Device, surface_config: &wgpu::SurfaceConfiguration) -> Self {
        // Initialize uniforms with default values
        let uniforms = ExitShaderUniforms {
            time: 0.0,
            resolution: [800.0, 600.0], // Default resolution, updated per frame
            _padding: [0.0; 3],
        };

        let uniform_buffer = create_uniform_buffer(device, &uniforms, "Exit Shader Uniform Buffer");

        // Create bind group layout for uniform buffer access
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

        // Create render pipeline with exit shader (uses full-screen triangle technique)
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

    /// Updates the exit shader's uniform buffer with current animation state.
    ///
    /// # Arguments
    /// * `queue` - WGPU command queue for buffer updates
    /// * `resolution` - Current screen resolution [width, height]
    /// * `time` - Current animation time in seconds
    pub fn update_uniforms(&self, queue: &wgpu::Queue, resolution: [f32; 2], time: f32) {
        let uniforms = ExitShaderUniforms {
            time,
            resolution,
            _padding: [0.0; 3],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Renders the exit effect precisely positioned over a specific maze cell.
    ///
    /// Uses scissor testing to ensure the effect only appears within the bounds
    /// of the target exit cell, with pixel-perfect positioning calculations.
    ///
    /// # Arguments
    /// * `render_pass` - Active render pass to draw into
    /// * `window` - Window reference for screen coordinate conversion
    /// * `exit_cell` - (column, row) coordinates of the exit cell in the maze
    /// * `maze_width` - Width of the maze in cells
    /// * `maze_height` - Height of the maze in cells
    pub fn render_to_cell(
        &self,
        render_pass: &mut wgpu::RenderPass,
        window: &winit::window::Window,
        exit_cell: (usize, usize),
        maze_width: usize,
        maze_height: usize,
    ) {
        // Use the same pixel scaling as the maze rendering system
        let cell_px = 4.0; // Pixels per cell
        let wall_px = 1.0; // Pixels per wall
        let render_width = maze_width as f32 * cell_px + (maze_width as f32 + 1.0) * wall_px;
        let render_height = maze_height as f32 * cell_px + (maze_height as f32 + 1.0) * wall_px;

        let window_size = window.inner_size();
        let win_w = window_size.width as f32;
        let win_h = window_size.height as f32;

        // Calculate the exit cell's pixel rectangle in the maze texture
        let col = exit_cell.0 as f32;
        let row = exit_cell.1 as f32;
        let x = col * (cell_px + wall_px) + wall_px; // Account for wall spacing
        let y = row * (cell_px + wall_px) + wall_px;
        let w = cell_px; // Cell width
        let h = cell_px; // Cell height

        // Convert texture coordinates to screen coordinates (texture is stretched to fill window)
        let scissor_x = ((x / render_width) * win_w).round().max(0.0) as u32;
        let scissor_y = ((y / render_height) * win_h).round().max(0.0) as u32;
        let scissor_width = ((w / render_width) * win_w).round().max(1.0) as u32;
        let scissor_height = ((h / render_height) * win_h).round().max(1.0) as u32;

        // Render effect only within the calculated scissor rectangle
        render_pass.set_scissor_rect(scissor_x, scissor_y, scissor_width, scissor_height);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1); // Full-screen triangle, clipped to exit cell
    }
}

/// Configuration helper for calculating maze rendering dimensions.
///
/// Handles the math for converting maze logical dimensions (in cells)
/// to render dimensions (in pixels) with proper scaling and borders.
pub struct MazeRenderConfig {
    /// Width of the maze in logical cells
    pub maze_width: u32,
    /// Height of the maze in logical cells
    pub maze_height: u32,
    /// Width of the rendered texture in pixels
    pub render_width: u32,
    /// Height of the rendered texture in pixels
    pub render_height: u32,
}

impl MazeRenderConfig {
    /// Creates a new maze render configuration with calculated dimensions.
    ///
    /// Uses a 5x scale factor plus 1 pixel border to convert from logical
    /// maze dimensions to pixel dimensions for rendering.
    ///
    /// # Arguments
    /// * `maze_width` - Width of the maze in cells
    /// * `maze_height` - Height of the maze in cells
    ///
    /// # Returns
    /// Configuration with calculated render dimensions
    pub fn new(maze_width: u32, maze_height: u32) -> Self {
        // Calculate render dimensions using 5x scale factor + 1 for borders
        let render_width = maze_width * 5 + 1;
        let render_height = maze_height * 5 + 1;

        Self {
            maze_width,
            maze_height,
            render_width,
            render_height,
        }
    }

    /// Creates a GPU texture and associated resources for maze rendering.
    ///
    /// # Arguments
    /// * `device` - WGPU device for creating GPU resources
    ///
    /// # Returns
    /// Tuple of (texture, texture_view, sampler) ready for use in rendering
    pub fn create_maze_texture(
        &self,
        device: &wgpu::Device,
    ) -> (wgpu::Texture, wgpu::TextureView, wgpu::Sampler) {
        let texture_size = wgpu::Extent3d {
            width: self.render_width,
            height: self.render_height,
            depth_or_array_layers: 1,
        };

        // Create texture with RGBA format for color maze data
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

        // Create sampler with nearest-neighbor filtering for crisp pixel art
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest, // Preserve sharp edges
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        (texture, texture_view, sampler)
    }
}
