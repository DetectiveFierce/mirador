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

pub struct LoadingRenderer {
    pub generator: MazeGenerator,
    pub maze: Arc<Mutex<Maze>>,

    // Rendering components
    pub maze_renderer: MazeRenderer,
    pub loading_bar_renderer: LoadingBarRenderer,
    pub exit_shader_renderer: ExitShaderRenderer,

    pub texture: wgpu::Texture,
    pub last_update: Instant,
}

impl LoadingRenderer {
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

    pub fn update_loading_bar(&self, queue: &wgpu::Queue, progress: f32, window: &Window) {
        // Update both progress and time/resolution for the animated effect
        let window_size = window.inner_size();
        let resolution = [window_size.width as f32, window_size.height as f32];
        let time = self.loading_bar_renderer.start_time.elapsed().as_secs_f32();

        self.loading_bar_renderer
            .update_uniforms(queue, progress, resolution, time);
    }

    pub fn update_exit_shader(&self, queue: &wgpu::Queue, window: &Window) {
        let window_size = window.inner_size();
        let resolution = [window_size.width as f32, window_size.height as f32];
        let time = self.exit_shader_renderer.start_time.elapsed().as_secs_f32();
        self.exit_shader_renderer
            .update_uniforms(queue, resolution, time);
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass, window: &Window) {
        // Render maze background
        self.maze_renderer.render(render_pass);

        // Render loading bar overlay with animated effect
        let window_size = window.inner_size();
        let bar_width = window_size.width;
        let bar_height = (window_size.height as f32 * 0.0125).ceil() as u32; // Match stamina bar thickness
        let bar_x = 0u32;
        let bar_y = 0u32;
        self.loading_bar_renderer.render_with_scissor(
            render_pass,
            bar_x,
            bar_y,
            bar_width,
            bar_height,
        );

        // Render exit cell effect if maze has an exit
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

    pub fn get_generation_progress(&self) -> f32 {
        self.generator.get_progress_ratio()
    }

    pub fn is_generation_complete(&self) -> bool {
        self.generator.is_complete()
    }

    pub fn get_maze_dimensions(&self) -> (u32, u32) {
        if let Ok(maze_guard) = self.maze.lock() {
            let (width, height) = maze_guard.get_dimensions();
            (width as u32, height as u32)
        } else {
            (126, 126) // Default fallback
        }
    }
}

// Updated uniforms structure to include animation data
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LoadingBarUniforms {
    pub progress: f32,
    pub time: f32,
    pub resolution: [f32; 2],
    pub _padding: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ExitShaderUniforms {
    pub time: f32,
    pub resolution: [f32; 2],
    pub _padding: [f32; 3],
}

pub struct MazeRenderer {
    pub pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl MazeRenderer {
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

    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..6, 0..1);
    }
}

pub struct LoadingBarRenderer {
    pub pipeline: wgpu::RenderPipeline,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub start_time: Instant, // Added to track animation time
}

impl LoadingBarRenderer {
    pub fn new(device: &wgpu::Device, surface_config: &wgpu::SurfaceConfiguration) -> Self {
        let uniforms = LoadingBarUniforms {
            progress: 0.0,
            time: 0.0,
            resolution: [800.0, 600.0], // Default resolution
            _padding: [0.0; 2],
        };

        let uniform_buffer = create_uniform_buffer(device, &uniforms, "Loading Bar Uniform Buffer");

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

        // Use the same exit shader for animated effect
        let pipeline = PipelineBuilder::new(device, surface_config.format)
            .with_label("Loading Bar Pipeline")
            .with_shader(include_str!("./shaders/loading-bar.wgsl")) // New shader file
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

    pub fn update_uniforms(
        &self,
        queue: &wgpu::Queue,
        progress: f32,
        resolution: [f32; 2],
        time: f32,
    ) {
        // Remap progress so the second half fills twice as fast
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

    pub fn render(&self, render_pass: &mut wgpu::RenderPass, window: &Window) {
        // Calculate loading bar position (thin bar across very top of screen)
        let window_size = window.inner_size();
        let bar_width = window_size.width; // Full width of screen
        let bar_height = 8u32; // Thin bar height

        let bar_x = 0u32;
        let bar_y = 0u32; // Very top of screen

        // Set scissor rect to constrain the animated effect to the loading bar area
        render_pass.set_scissor_rect(bar_x, bar_y, bar_width, bar_height);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1); // Full-screen triangle, but clipped to loading bar
    }

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
        render_pass.draw(0..3, 0..1);
    }
}

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
        maze_width: usize,
        maze_height: usize,
    ) {
        // Use the same pixel math as Maze::get_render_data
        let cell_px = 4.0;
        let wall_px = 1.0;
        let render_width = maze_width as f32 * cell_px + (maze_width as f32 + 1.0) * wall_px;
        let render_height = maze_height as f32 * cell_px + (maze_height as f32 + 1.0) * wall_px;

        let window_size = window.inner_size();
        let win_w = window_size.width as f32;
        let win_h = window_size.height as f32;

        // Compute the exit cell's pixel rectangle in the texture
        let col = exit_cell.0 as f32;
        let row = exit_cell.1 as f32;
        let x = col * (cell_px + wall_px) + wall_px;
        let y = row * (cell_px + wall_px) + wall_px;
        let w = cell_px;
        let h = cell_px;

        // Scale to window coordinates (since the texture is stretched to fill the window)
        let scissor_x = ((x / render_width) * win_w).round().max(0.0) as u32;
        let scissor_y = ((y / render_height) * win_h).round().max(0.0) as u32;
        let scissor_width = ((w / render_width) * win_w).round().max(1.0) as u32;
        let scissor_height = ((h / render_height) * win_h).round().max(1.0) as u32;

        render_pass.set_scissor_rect(scissor_x, scissor_y, scissor_width, scissor_height);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

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
