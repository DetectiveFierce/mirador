use crate::renderer::pipeline_builder::{
    BindGroupLayoutBuilder, PipelineBuilder, create_fullscreen_vertices, create_uniform_buffer,
    create_vertex_2d_layout,
};
use wgpu;

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
    /// # let device: wgpu::Device = unimplemented!();
    /// # let surface_config: wgpu::SurfaceConfiguration = unimplemented!();
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
            .with_shader(include_str!("../shaders/game-over.wgsl"))
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
    /// # let queue: wgpu::Queue = unimplemented!();
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
    /// # let mut render_pass: wgpu::RenderPass = unimplemented!();
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
