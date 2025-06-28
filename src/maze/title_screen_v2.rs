use crate::maze::generator::MazeGenerator;
use crate::renderer::render_components::{
    ExitShaderRenderer, LoadingBarRenderer, MazeRenderConfig, MazeRenderer,
};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use winit::window::Window;

/// Simplified title screen renderer using component-based architecture.
/// This reduces complexity by separating concerns and eliminating code duplication.
pub struct TitleScreenRendererV2 {
    // Maze generation
    pub generator: MazeGenerator,
    pub maze: Arc<Mutex<crate::maze::generator::Maze>>,

    // Rendering components
    pub maze_renderer: MazeRenderer,
    pub loading_bar_renderer: LoadingBarRenderer,
    pub exit_shader_renderer: ExitShaderRenderer,

    // Maze texture resources
    pub texture: wgpu::Texture,

    // Timing
    pub last_update: Instant,
}

impl TitleScreenRendererV2 {
    /// Creates a new simplified title screen renderer.
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

    /// Updates the maze texture with new generation data.
    pub fn update_texture(&self, queue: &wgpu::Queue, maze_data: &[u8], width: u32, height: u32) {
        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            maze_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Updates the loading bar progress.
    pub fn update_loading_bar(&self, device: &wgpu::Device, queue: &wgpu::Queue, progress: f32) {
        self.loading_bar_renderer
            .update_progress(device, queue, progress);
    }

    /// Updates the exit shader animation.
    pub fn update_exit_shader(&self, queue: &wgpu::Queue, window: &Window) {
        let window_size = window.inner_size();
        let resolution = [window_size.width as f32, window_size.height as f32];
        self.exit_shader_renderer.update_uniforms(queue, resolution);
    }

    /// Renders all components for the title screen.
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
        if let Ok(maze_guard) = self.maze.lock() {
            maze_guard.get_generation_progress()
        } else {
            0.0
        }
    }

    /// Check if maze generation is complete.
    pub fn is_generation_complete(&self) -> bool {
        if let Ok(maze_guard) = self.maze.lock() {
            maze_guard.is_complete()
        } else {
            false
        }
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
