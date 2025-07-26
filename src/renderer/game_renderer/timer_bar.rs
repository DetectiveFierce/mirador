//! Timer Bar Renderer Module
//!
//! This module provides a GPU-accelerated timer bar renderer using wgpu.
//! The timer bar can display progress with animated effects and is designed
//! for real-time rendering applications.

use crate::renderer::pipeline_builder::{
    BindGroupLayoutBuilder, PipelineBuilder, create_uniform_buffer,
};
use std::time::Instant;
use wgpu;

/// Uniform buffer data structure for the timer bar shader.
///
/// This structure is passed to the GPU shader to control the appearance
/// and animation of the timer bar. The layout uses explicit padding to
/// ensure proper GPU memory alignment.
///
/// # Memory Layout
/// The struct uses `#[repr(C)]` to ensure consistent memory layout across
/// platforms, which is required for GPU buffer uploads.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TimerBarUniforms {
    /// Progress value from 0.0 to 1.0 representing the completion percentage
    pub progress: f32,

    /// Current time in seconds since the timer started, used for animations
    pub time: f32,

    /// Screen resolution as [width, height] in pixels for proper scaling
    pub resolution: [f32; 2],

    /// Padding to ensure proper GPU memory alignment (16-byte alignment)
    pub _padding: [f32; 2],
}

/// GPU-accelerated timer bar renderer.
///
/// This renderer creates and manages a fullscreen timer bar that can display
/// progress with smooth animations. It uses a dedicated render pipeline with
/// custom shaders for efficient GPU rendering.
///
/// # Usage
/// ```rust,no_run
/// # use wgpu;
/// # let device: wgpu::Device = todo!();
/// # let surface_config: wgpu::SurfaceConfiguration = todo!();
/// let timer_bar = TimerBarRenderer::new(&device, &surface_config);
///
/// // In your render loop:
/// # let queue: wgpu::Queue = todo!();
/// # let mut render_pass: wgpu::RenderPass = todo!();
/// timer_bar.update_uniforms(&queue, 0.5, [800.0, 600.0], 1.0);
/// timer_bar.render(&mut render_pass);
/// ```
pub struct TimerBarRenderer {
    /// The GPU render pipeline configured for timer bar rendering
    pub pipeline: wgpu::RenderPipeline,

    /// GPU buffer containing the uniform data (TimerBarUniforms)
    pub uniform_buffer: wgpu::Buffer,

    /// Bind group that associates the uniform buffer with the shader
    pub bind_group: wgpu::BindGroup,

    /// Start time for calculating elapsed time in animations
    pub start_time: Instant,
}

impl TimerBarRenderer {
    /// Creates a new timer bar renderer.
    ///
    /// This function initializes all GPU resources needed for timer bar rendering,
    /// including the render pipeline, uniform buffer, and bind groups.
    ///
    /// # Arguments
    /// * `device` - The wgpu device for creating GPU resources
    /// * `surface_config` - Surface configuration containing the target format
    ///
    /// # Returns
    /// A new `TimerBarRenderer` instance ready for rendering
    ///
    /// # Example
    /// ```rust,no_run
    /// # use wgpu;
    /// # let device: wgpu::Device = todo!();
    /// # let surface_config: wgpu::SurfaceConfiguration = todo!();
    /// let timer_bar = TimerBarRenderer::new(&device, &surface_config);
    /// ```
    pub fn new(device: &wgpu::Device, surface_config: &wgpu::SurfaceConfiguration) -> Self {
        // Initialize uniform data with default values
        let uniforms = TimerBarUniforms {
            progress: 1.0,              // Start at full progress
            time: 0.0,                  // Start time at zero
            resolution: [800.0, 600.0], // Default resolution
            _padding: [0.0; 2],         // Zero padding
        };

        // Create the uniform buffer on the GPU
        let uniform_buffer = create_uniform_buffer(device, &uniforms, "Timer Bar Uniform Buffer");

        // Create bind group layout defining how uniforms are accessed in shaders
        let bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label("Timer Bar Bind Group Layout")
            .with_uniform_buffer(0, wgpu::ShaderStages::VERTEX_FRAGMENT)
            .build();

        // Create bind group that connects the uniform buffer to the layout
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("Timer Bar Bind Group"),
        });

        // Create the render pipeline with the timer bar shader
        let pipeline = PipelineBuilder::new(device, surface_config.format)
            .with_label("Timer Bar Pipeline")
            .with_shader(include_str!("../shaders/timer-bar.wgsl"))
            .with_bind_group_layout(&bind_group_layout)
            .with_alpha_blending() // Enable transparency for smooth edges
            .build();

        Self {
            pipeline,
            uniform_buffer,
            bind_group,
            start_time: Instant::now(),
        }
    }

    /// Updates the uniform buffer with new timer bar parameters.
    ///
    /// This method should be called each frame to update the timer bar's
    /// appearance and animation state. The progress value is automatically
    /// clamped to the valid range [0.0, 1.0].
    ///
    /// # Arguments
    /// * `queue` - The wgpu command queue for buffer uploads
    /// * `progress` - Progress value (0.0 = empty, 1.0 = full), will be clamped
    /// * `resolution` - Current screen resolution as [width, height]
    /// * `time` - Current time in seconds for animations
    ///
    /// # Example
    /// ```rust,no_run
    /// # use wgpu;
    /// # let timer_bar: TimerBarRenderer = todo!();
    /// # let queue: wgpu::Queue = todo!();
    /// // Update with 75% progress at 1920x1080 resolution
    /// timer_bar.update_uniforms(&queue, 0.75, [1920.0, 1080.0], 2.5);
    /// ```
    pub fn update_uniforms(
        &self,
        queue: &wgpu::Queue,
        progress: f32,
        resolution: [f32; 2],
        time: f32,
    ) {
        let uniforms = TimerBarUniforms {
            progress: progress.clamp(0.0, 1.0), // Ensure progress stays in valid range
            time,
            resolution,
            _padding: [0.0; 2],
        };

        // Upload the new uniform data to the GPU
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Renders the timer bar to the current render pass.
    ///
    /// This method draws the timer bar using a fullscreen triangle technique
    /// (3 vertices) which is more efficient than a quad for fullscreen effects.
    /// The actual timer bar shape and positioning is handled in the shader.
    ///
    /// # Arguments
    /// * `render_pass` - Active render pass to draw into
    ///
    /// # Note
    /// Make sure to call `update_uniforms()` before rendering to ensure
    /// the timer bar displays the current state.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use wgpu;
    /// # let timer_bar: TimerBarRenderer = todo!();
    /// # let mut render_pass: wgpu::RenderPass = todo!();
    /// timer_bar.render(&mut render_pass);
    /// ```
    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        // Set the timer bar pipeline as active
        render_pass.set_pipeline(&self.pipeline);

        // Bind the uniform buffer to the shader
        render_pass.set_bind_group(0, &self.bind_group, &[]);

        // Draw using fullscreen triangle technique (3 vertices, 1 instance)
        render_pass.draw(0..3, 0..1);
    }
}
