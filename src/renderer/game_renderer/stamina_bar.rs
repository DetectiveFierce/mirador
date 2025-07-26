//! Stamina Bar Renderer Module
//!
//! This module provides a GPU-accelerated stamina bar renderer using WebGPU.
//! The stamina bar is rendered as a full-screen quad using a vertex shader that
//! generates geometry procedurally, eliminating the need for vertex buffers.

use crate::renderer::pipeline_builder::{
    BindGroupLayoutBuilder, PipelineBuilder, create_uniform_buffer,
};
use std::time::Instant;
use wgpu;

/// Uniform data structure passed to the stamina bar shader.
///
/// This struct is laid out in memory according to WebGPU's uniform buffer
/// alignment requirements. The `_padding` field ensures proper 16-byte alignment.
///
/// # Memory Layout
/// - `progress`: Current stamina level (0.0 = empty, 1.0 = full)
/// - `time`: Elapsed time in seconds for animations
/// - `resolution`: Screen resolution [width, height] for aspect ratio correction
/// - `_padding`: Padding bytes to maintain 16-byte alignment
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct StaminaBarUniforms {
    /// Current stamina progress from 0.0 (empty) to 1.0 (full)
    pub progress: f32,
    /// Elapsed time in seconds since renderer creation, used for animations
    pub time: f32,
    /// Screen resolution as [width, height] for proper aspect ratio
    pub resolution: [f32; 2],
    /// Padding to ensure proper GPU memory alignment (unused)
    pub _padding: [f32; 2],
}

/// GPU-accelerated stamina bar renderer.
///
/// This renderer displays a stamina bar using a custom shader pipeline.
/// It uses a full-screen triangle rendering technique where the vertex shader
/// generates geometry procedurally, requiring no vertex buffers.
///
/// # Features
/// - Hardware-accelerated rendering
/// - Alpha blending support for transparency effects
/// - Real-time uniform updates for smooth animations
/// - Automatic aspect ratio correction
///
/// # Shader Requirements
/// The renderer expects a shader file at "../shaders/loading-bar.wgsl" that
/// implements the stamina bar visual effects.
pub struct StaminaBarRenderer {
    /// The WebGPU render pipeline for stamina bar rendering
    pub pipeline: wgpu::RenderPipeline,
    /// GPU buffer containing uniform data (progress, time, resolution)
    pub uniform_buffer: wgpu::Buffer,
    /// Bind group that associates the uniform buffer with shader bindings
    pub bind_group: wgpu::BindGroup,
    /// Creation timestamp for calculating elapsed time
    pub start_time: Instant,
}

impl StaminaBarRenderer {
    /// Creates a new stamina bar renderer.
    ///
    /// Initializes the complete rendering pipeline including shaders, uniforms,
    /// and GPU resources. The renderer starts with a full stamina bar (progress = 1.0).
    ///
    /// # Arguments
    /// * `device` - WebGPU device for creating GPU resources
    /// * `surface_config` - Surface configuration containing the target pixel format
    ///
    /// # Returns
    /// A fully initialized `StaminaBarRenderer` ready for rendering
    ///
    /// # Example
    /// ```rust
    /// let renderer = StaminaBarRenderer::new(&device, &surface_config);
    /// ```
    pub fn new(device: &wgpu::Device, surface_config: &wgpu::SurfaceConfiguration) -> Self {
        // Initialize uniform data with default values
        let uniforms = StaminaBarUniforms {
            progress: 1.0,              // Start with full stamina
            time: 0.0,                  // Start time at zero
            resolution: [800.0, 600.0], // Default resolution
            _padding: [0.0; 2],         // Padding for alignment
        };

        // Create GPU uniform buffer
        let uniform_buffer = create_uniform_buffer(device, &uniforms, "Stamina Bar Uniform Buffer");

        // Build bind group layout for shader uniform access
        let bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label("Stamina Bar Bind Group Layout")
            .with_uniform_buffer(0, wgpu::ShaderStages::VERTEX_FRAGMENT)
            .build();

        // Create bind group linking uniform buffer to shader binding point 0
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("Stamina Bar Bind Group"),
        });

        // Build render pipeline with shader and alpha blending
        let pipeline = PipelineBuilder::new(device, surface_config.format)
            .with_label("Stamina Bar Pipeline")
            .with_shader(include_str!("../shaders/loading-bar.wgsl"))
            .with_bind_group_layout(&bind_group_layout)
            .with_alpha_blending() // Enable transparency support
            .build();

        Self {
            pipeline,
            uniform_buffer,
            bind_group,
            start_time: Instant::now(),
        }
    }

    /// Updates the stamina bar's uniform data on the GPU.
    ///
    /// This method should be called each frame to update the stamina bar's
    /// visual state. The progress value is automatically clamped to ensure
    /// it stays within valid bounds.
    ///
    /// # Arguments
    /// * `queue` - WebGPU command queue for GPU operations
    /// * `progress` - Stamina level from 0.0 (empty) to 1.0 (full)
    /// * `resolution` - Current screen resolution as [width, height]
    /// * `time` - Current time in seconds for shader animations
    ///
    /// # Note
    /// The progress value is clamped to [0.0, 1.0] range and is NOT inverted,
    /// meaning the bar shrinks as stamina decreases (0.0 = empty bar).
    ///
    /// # Example
    /// ```rust
    /// renderer.update_uniforms(&queue, 0.75, [1920.0, 1080.0], elapsed_time);
    /// ```
    pub fn update_uniforms(
        &self,
        queue: &wgpu::Queue,
        progress: f32,
        resolution: [f32; 2],
        time: f32,
    ) {
        let uniforms = StaminaBarUniforms {
            progress: progress.clamp(0.0, 1.0), // Ensure valid range
            time,
            resolution,
            _padding: [0.0; 2],
        };

        // Upload uniform data to GPU buffer
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Renders the stamina bar to the current render pass.
    ///
    /// This method draws the stamina bar using a procedural full-screen triangle
    /// technique. The vertex shader generates 3 vertices that cover the entire
    /// screen, and the fragment shader handles the stamina bar visualization.
    ///
    /// # Arguments
    /// * `render_pass` - Active WebGPU render pass to draw into
    ///
    /// # Rendering Details
    /// - Uses 3 vertices (full-screen triangle) with no vertex buffer
    /// - Single instance rendering (0..1)
    /// - Relies on vertex shader to generate screen-covering geometry
    ///
    /// # Example
    /// ```rust
    /// let mut render_pass = encoder.begin_render_pass(&render_pass_desc);
    /// renderer.render(&mut render_pass);
    /// ```
    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        // Set the stamina bar rendering pipeline
        render_pass.set_pipeline(&self.pipeline);

        // Bind uniform data at binding point 0
        render_pass.set_bind_group(0, &self.bind_group, &[]);

        // Draw full-screen triangle (3 vertices, 1 instance)
        // The vertex shader generates screen-covering geometry procedurally
        render_pass.draw(0..3, 0..1);
    }
}
