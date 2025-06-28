//! Maze generation animation renderer module.
//!
//! This module provides [`AnimationRenderer`], which handles rendering a maze and a loading bar
//! for the maze's animation screen using `wgpu`. It manages GPU resources for the maze texture, pipelines,
//! and loading bar, and provides methods to update the maze texture and loading bar progress.

use crate::maze::generator::Maze;
use crate::maze::generator::MazeGenerator;
use crate::renderer::render_components::{
    ExitShaderRenderer, LoadingBarRenderer, MazeRenderConfig, MazeRenderer,
};
use egui_wgpu::wgpu;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use winit::window::Window;

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
pub struct AnimationRenderer {
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

impl AnimationRenderer {
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
