use crate::app::AppState;
use crate::renderer::pipeline_builder::{
    BindGroupLayoutBuilder, PipelineBuilder, create_uniform_buffer,
};
use std::time::Instant;
use wgpu::{self, util::DeviceExt};
use winit::window::Window;

#[repr(C)]
/// Uniform data for title screen rendering.
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TitleUniforms {
    /// View-projection matrix for title screen transformations.
    view_proj_matrix: [[f32; 4]; 4],
}

/// Renderer for the title screen with texture and shader support.
pub struct TitleRenderer {
    /// The render pipeline for title screen rendering.
    pub pipeline: wgpu::RenderPipeline,
    /// Vertex buffer containing the fullscreen quad geometry.
    pub vertex_buffer: wgpu::Buffer,
    /// Uniform buffer for transformation data.
    pub uniform_buffer: wgpu::Buffer,
    /// Bind group containing texture and sampler bindings.
    pub bind_group: wgpu::BindGroup,
}

impl TitleRenderer {
    /// Creates a new TitleRenderer with initialized pipeline and resources.
    ///
    /// # Arguments
    /// * `device` - The WGPU device
    /// * `queue` - The WGPU queue for texture loading
    /// * `surface_config` - The surface configuration for pipeline creation
    ///
    /// # Returns
    /// A new TitleRenderer instance
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        // Load title texture
        let title_texture = Self::load_title_texture(device, queue);

        let uniforms = TitleUniforms {
            view_proj_matrix: [[0.0; 4]; 4],
        };

        let uniform_buffer = create_uniform_buffer(device, &uniforms, "Title Uniform Buffer");

        // Create bind group layout for texture + sampler (no uniforms needed)
        let bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label("Title Bind Group Layout")
            .with_texture(0, wgpu::ShaderStages::FRAGMENT)
            .with_sampler(1, wgpu::ShaderStages::FRAGMENT)
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

        // Create bind group for title texture
        let title_texture_view = title_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&title_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("Title Bind Group"),
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
            .with_label("Title Pipeline")
            .with_shader(include_str!("shaders/title.wgsl"))
            .with_vertex_buffer(vertex_buffer_layout)
            .with_bind_group_layout(&bind_group_layout)
            .with_alpha_blending()
            .build();

        let vertex_buffer = Self::create_fullscreen_quad_vertices(device);

        Self {
            pipeline,
            vertex_buffer,
            uniform_buffer, // still created, but not used in bind group
            bind_group,
        }
    }

    fn load_title_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
        let path = "assets/Mirador-title.png";

        // Load image using image crate
        let img = match image::open(path) {
            Ok(img) => img.to_rgba8(),
            Err(e) => {
                eprintln!("Failed to load title texture {}: {}", path, e);
                // Create a fallback texture (solid white square with black text-like pattern)
                let mut fallback = image::RgbaImage::new(512, 256);
                for (x, y, pixel) in fallback.enumerate_pixels_mut() {
                    // Create a simple "TITLE" pattern as fallback
                    let is_text = (x > 100 && x < 400 && y > 100 && y < 150)
                        && ((x / 20) % 2 == 0 || (y / 20) % 2 == 0);
                    if is_text {
                        *pixel = image::Rgba([0, 0, 0, 255]); // Black text
                    } else {
                        *pixel = image::Rgba([255, 255, 255, 255]); // White background
                    }
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
            label: Some("Title Texture"),
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

    fn create_fullscreen_quad_vertices(device: &wgpu::Device) -> wgpu::Buffer {
        // Create a fullscreen quad in normalized device coordinates (-1 to 1)
        // This will cover the entire screen
        let vertices: &[f32] = &[
            // Position (x, y, z)    // Texture coords (u, v)
            // Triangle 1
            -1.0, -1.0, 0.0, 0.0, 1.0, // Bottom-left
            1.0, -1.0, 0.0, 1.0, 1.0, // Bottom-right
            -1.0, 1.0, 0.0, 0.0, 0.0, // Top-left
            // Triangle 2
            1.0, -1.0, 0.0, 1.0, 1.0, // Bottom-right
            1.0, 1.0, 0.0, 1.0, 0.0, // Top-right
            -1.0, 1.0, 0.0, 0.0, 0.0, // Top-left
        ];

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Title Fullscreen Quad Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        })
    }

    /// Update the title renderer with the current view projection matrix
    pub fn update(&mut self, queue: &wgpu::Queue, view_proj_matrix: [[f32; 4]; 4]) {
        let uniforms = TitleUniforms { view_proj_matrix };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Render the title
    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}

/// Handles the title screen rendering and animation logic.
pub fn handle_title(state: &mut AppState, window: &Window) {
    // Explicitly hide game UI overlays on the title screen
    if let Some(buffer) = state.text_renderer.text_buffers.get_mut("main_timer") {
        buffer.visible = false;
    }
    if let Some(buffer) = state.text_renderer.text_buffers.get_mut("level") {
        buffer.visible = false;
    }
    if let Some(buffer) = state.text_renderer.text_buffers.get_mut("score") {
        buffer.visible = false;
    }

    // --- Dynamic placement for title and subtitle overlays ---
    let width = state.wgpu_renderer.surface_config.width as f32;
    let height = state.wgpu_renderer.surface_config.height as f32;

    // Apply DPI scaling based on height (consistent with other UI elements)
    let reference_height = 1080.0;
    let scale = (height / reference_height).clamp(0.7, 2.0);

    // Dynamically scale font sizes with DPI scaling
    let title_font_size = (width * 0.09 * scale).clamp(48.0, 220.0); // 9% of width, min 48, max 220
    let title_line_height = (title_font_size * 1.2).clamp(60.0, 260.0);
    let subtitle_font_size = (width * 0.018 * scale).clamp(14.0, 96.0); // 1.8% of width, min 14, max 96 (increased max)
    let subtitle_line_height = (subtitle_font_size * 1.3).clamp(18.0, 128.0); // increased max

    // Gather info for both overlays first to avoid borrow checker issues
    let title_overlay_info = state
        .text_renderer
        .text_buffers
        .get("title_mirador_overlay")
        .map(|buf| {
            let mut style = buf.style.clone();
            style.font_size = title_font_size;
            style.line_height = title_line_height;
            let text = buf.text_content.clone();
            (style, text)
        });
    let subtitle_overlay_info = state
        .text_renderer
        .text_buffers
        .get("title_subtitle_overlay")
        .map(|buf| {
            let mut style = buf.style.clone();
            style.font_size = subtitle_font_size;
            style.line_height = subtitle_line_height;
            let text = buf.text_content.clone();
            (style, text)
        });
    // Now update positions and styles mutably
    if let Some((style, text)) = title_overlay_info {
        let _ = state
            .text_renderer
            .update_style("title_mirador_overlay", style.clone());
        let (_min_x, text_width, text_height) = state.text_renderer.measure_text(&text, &style);
        let margin_right = width * 0.045; // 4.5% of width
        let margin_top = height * 0.10; // 10% of height
        let pos = crate::renderer::text::TextPosition {
            x: width - text_width - margin_right,
            y: margin_top,
            max_width: Some(text_width + 20.0 * scale), // Add padding to prevent clipping
            max_height: Some(text_height + 10.0 * scale),
        };
        let _ = state
            .text_renderer
            .update_position("title_mirador_overlay", pos);
    }
    if let Some((style, text)) = subtitle_overlay_info {
        let _ = state
            .text_renderer
            .update_style("title_subtitle_overlay", style.clone());
        let (_min_x, text_width, text_height) = state.text_renderer.measure_text(&text, &style);
        let margin_right = width * 0.06; // 6% of width
        let margin_bottom = height * 0.10; // 10% of height
        let pos = crate::renderer::text::TextPosition {
            x: width - text_width - margin_right,
            y: height - text_height - margin_bottom,
            max_width: Some(text_width + 40.0 * scale), // Add more padding for subtitle to prevent clipping
            max_height: Some(text_height + 20.0 * scale),
        };
        let _ = state
            .text_renderer
            .update_position("title_subtitle_overlay", pos);
    }
    // --- End dynamic placement ---

    // Animate the subtitle text color (fade from black to white and back)
    let subtitle_color = {
        if let Some(buf) = state
            .text_renderer
            .text_buffers
            .get_mut("title_subtitle_overlay")
        {
            let now = Instant::now();
            let elapsed = now.duration_since(state.start_time).as_secs_f32();
            // Fade: t goes 0..1..0 over a slower period (e.g., 6 seconds for a full cycle)
            let t = 0.5 * (1.0 + ((elapsed / 3.0) * std::f32::consts::PI).sin());
            let c = (t * 255.0).round() as u8;
            buf.style.color = glyphon::Color::rgb(c, c, c);
            Some(buf.style.clone())
        } else {
            None
        }
    };
    if let Some(style) = subtitle_color {
        let _ = state
            .text_renderer
            .update_style("title_subtitle_overlay", style);
    }
    // Render the title screen
    let mut encoder = state
        .wgpu_renderer
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    let (surface_view, surface_texture) = match state.wgpu_renderer.get_surface_texture_and_view() {
        Ok((surface_texture, surface_view)) => (surface_view, surface_texture),
        Err(e) => {
            eprintln!("Failed to get surface texture: {}", e);
            return;
        }
    };
    state
        .wgpu_renderer
        .render_title_screen(&mut encoder, &surface_view, window);
    // Render overlay text
    state
        .text_renderer
        .prepare(
            &state.wgpu_renderer.device,
            &state.wgpu_renderer.queue,
            &state.wgpu_renderer.surface_config,
        )
        .ok();
    state
        .wgpu_renderer
        .render_text(&mut encoder, &surface_view, &mut state.text_renderer);
    window.request_redraw();
    state.wgpu_renderer.queue.submit(Some(encoder.finish()));
    surface_texture.present();

    // Poll the device to process any pending operations
    // This helps ensure resources are properly cleaned up
    state.wgpu_renderer.device.poll(wgpu::Maintain::Poll);
}
