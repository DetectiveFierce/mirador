//! WGPU-based renderer for the Mirador game.
//!
//! This module provides [`WgpuRenderer`], which manages all GPU resources, pipelines, and rendering
//! logic for the main game scene, background, and title screen. It handles initialization of WGPU,
//! creation of vertex/uniform buffers, pipelines, and orchestrates the rendering of the maze, player,
//! animated background, and UI overlays.
//!
//! # Features
//! - Loads maze geometry and floor/wall vertices
//! - Renders a starfield background and animated title screen
//! - Handles depth buffering and uniform updates for camera/player movement
//! - Integrates with egui for UI overlays
//!
//! # Usage
//! Create a [`WgpuRenderer`] via [`WgpuRenderer::new`] and call [`WgpuRenderer::update_canvas`]
//! each frame to render the current game state.
use crate::background::stars::{self, StarRenderer};
use crate::game::collision::CollisionSystem;
use crate::game::player::Player;
use crate::math::{deg_to_rad, mat::Mat4};
use crate::maze::title_screen::TitleScreenRenderer;
use crate::maze::{parse_maze_file, title_screen};
use crate::renderer::debug_renderer::collect_wall_face_debug_vertices;
use crate::renderer::pipeline_builder::PipelineBuilder;
use crate::renderer::uniform::Uniforms;
use crate::renderer::vertex::Vertex;
use crate::ui::ui_panel::UiState;
use egui_wgpu::ScreenDescriptor;
use egui_wgpu::wgpu;
use egui_wgpu::wgpu::util::DeviceExt;
use egui_wgpu::wgpu::{SurfaceTexture, TextureView};

/// Main WGPU renderer for the Mirador game.
///
/// This struct manages all GPU resources, pipelines, and rendering logic for the game scene,
/// including the maze, player, animated background, and title screen.
///
/// # Fields
/// - `surface`: The WGPU surface for presenting rendered frames.
/// - `device`: The WGPU device for resource creation.
/// - `queue`: The WGPU queue for submitting commands.
/// - `surface_config`: The surface configuration (format, size, etc.).
/// - `pipeline`: Main render pipeline for the maze and floor.
/// - `vertex_buffer`: Combined vertex buffer for floor and wall geometry.
/// - `num_vertices`: Number of vertices to draw.
/// - `uniform_buffer`: Uniform buffer for camera/view/projection matrices.
/// - `uniform_bind_group`: Bind group for the uniform buffer.
/// - `depth_texture`: Optional depth texture for depth testing.
/// - `background`: StarRenderer for animated starfield background.
/// - `title_screen_renderer`: Renderer for the title screen maze and loading bar.
pub struct WgpuRenderer {
    /// The WGPU surface for presenting rendered frames.
    pub surface: wgpu::Surface<'static>,
    /// The WGPU device for resource creation.
    pub device: wgpu::Device,
    /// The WGPU queue for submitting commands.
    pub queue: wgpu::Queue,
    /// The surface configuration (format, size, etc.).
    pub surface_config: wgpu::SurfaceConfiguration,
    /// Main render pipeline for the maze and floor.
    pub pipeline: wgpu::RenderPipeline,
    /// Combined vertex buffer for floor and wall geometry.
    pub vertex_buffer: wgpu::Buffer,
    /// Number of vertices to draw.
    pub num_vertices: u32,
    /// Uniform buffer for camera/view/projection matrices.
    pub uniform_buffer: wgpu::Buffer,
    /// Bind group for the uniform buffer.
    pub uniform_bind_group: wgpu::BindGroup,
    /// Optional depth texture for depth testing.
    pub depth_texture: Option<wgpu::Texture>,
    /// StarRenderer for animated starfield background.
    pub background: StarRenderer,
    /// Renderer for the title screen maze and loading bar.
    pub title_screen_renderer: TitleScreenRenderer,
    /// Renderer for debug information.
    /// Whether to render bounding boxes for debugging.
    pub debug_render_bounding_boxes: bool,
    /// Vertex buffer for the debug renderer
    pub debug_vertex_buffer: Option<wgpu::Buffer>,
    pub debug_vertex_count: usize,
}

impl WgpuRenderer {
    /// Initializes a new [`WgpuRenderer`] and all associated GPU resources.
    ///
    /// # Arguments
    /// - `instance`: The WGPU instance.
    /// - `surface`: The WGPU surface for presentation.
    /// - `width`: Initial width of the surface.
    /// - `height`: Initial height of the surface.
    ///
    /// # Returns
    /// A fully initialized [`WgpuRenderer`] ready for rendering.
    pub async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
    ) -> Self {
        let power_pref = wgpu::PowerPreference::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: power_pref,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let features = wgpu::Features::empty();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: features,
                    required_limits: Default::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let selected_format = wgpu::TextureFormat::Bgra8UnormSrgb;
        let swapchain_format = swapchain_capabilities
            .formats
            .iter()
            .find(|d| **d == selected_format)
            .expect("failed to select proper surface texture format!");

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: *swapchain_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 0,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &surface_config);

        let uniforms = Uniforms::new();
        let uniform_buffer = uniforms.create_buffer(&device);
        let (uniform_bind_group, uniform_bind_group_layout) =
            uniforms.create_bind_group(&uniform_buffer, &device);

        let pipeline = PipelineBuilder::new(&device, surface_config.format)
            .with_label("Main Pipeline")
            .with_shader(include_str!("shader.wgsl"))
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

        let mut floor_vertices = Vertex::create_floor_vertices(&maze_grid, exit_cell);

        // Generate wall geometry
        let mut wall_vertices = Vertex::create_wall_vertices(&maze_grid);

        // Append wall vertices to floor
        floor_vertices.append(&mut wall_vertices);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Combined Vertex Buffer"),
            contents: bytemuck::cast_slice(&floor_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let star_renderer = stars::create_star_renderer(&device, &surface_config, 100);
        let title_screen_renderer =
            title_screen::TitleScreenRenderer::new(&device, &surface_config);

        let debug_render_bounding_boxes = false;
        let debug_vertex_buffer = None;
        let debug_vertex_count = 0;
        Self {
            surface,
            device,
            queue,
            surface_config,
            uniform_buffer,
            uniform_bind_group,
            pipeline,
            vertex_buffer,
            num_vertices: floor_vertices.len() as u32,
            depth_texture: None,
            background: star_renderer,
            title_screen_renderer,
            debug_render_bounding_boxes,
            debug_vertex_buffer,
            debug_vertex_count,
        }
    }

    /// Renders the current frame to the surface, including the maze, player, background, and UI.
    ///
    /// # Arguments
    /// - `window`: The window for retrieving DPI scaling.
    /// - `ui_state`: Current UI state (colors, etc.).
    /// - `encoder`: Command encoder for submitting draw commands.
    /// - `start_time`: Start time for animation timing.
    /// - `player`: Reference to the current player state.
    /// - `title`: If true, renders the title screen; otherwise renders the main game scene.
    ///
    /// # Returns
    /// - `Ok((TextureView, ScreenDescriptor, SurfaceTexture))` on success.
    /// - `Err(String)` if the surface is outdated or unavailable.
    pub fn update_canvas(
        &mut self,
        window: &winit::window::Window,
        ui_state: &UiState,
        encoder: &mut wgpu::CommandEncoder,
        start_time: std::time::Instant,
        player: &Player,
        title: bool,
    ) -> Result<(TextureView, ScreenDescriptor, SurfaceTexture), String> {
        let surface_texture_obj = self.surface.get_current_texture();

        let surface_texture = match surface_texture_obj {
            Err(wgpu::SurfaceError::Outdated) => {
                // Ignoring outdated to allow resizing and minimization
                println!("wgpu surface outdated");
                return Err("wgpu surface outdated".to_string());
            }
            Err(_) => {
                surface_texture_obj.expect("Failed to acquire next swap chain texture");
                return Err("Failed to acquire next swap chain texture".to_string());
            }
            Ok(surface_texture) => surface_texture,
        };

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.surface_config.width, self.surface_config.height],
            pixels_per_point: window.scale_factor() as f32,
        };
        // If we don't have a depth texture OR if its size is different
        // from the canvasTexture when make a new depth texture
        let (width, height) = (self.surface_config.width, self.surface_config.height);

        let depth_texture_view = {
            if self.depth_texture.is_none()
                || self.depth_texture.as_ref().unwrap().width() != width
                || self.depth_texture.as_ref().unwrap().height() != height
            {
                if let Some(depth_texture) = self.depth_texture.take() {
                    // Manually drop the texture to free up resources
                    drop(depth_texture);
                }

                self.depth_texture = Some(self.device.create_texture(&wgpu::TextureDescriptor {
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
        };

        if title {
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &surface_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                // Render title screen using new component architecture
                self.title_screen_renderer.render(&mut render_pass, window);
            }

            return Ok((surface_view, screen_descriptor, surface_texture)); // <- This return was already there
        }
        if !title {
            let clear_pass_desc = wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: ui_state.r as f64,
                            g: ui_state.g as f64,
                            b: ui_state.b as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            };
            let clear_pass = encoder.begin_render_pass(&clear_pass_desc);
            // No draw calls needed - just clears

            drop(clear_pass)
        }

        {
            let elapsed_time = start_time.elapsed().as_secs_f32();
            self.background
                .update_background_color(&self.queue, [ui_state.r, ui_state.g, ui_state.b, 1.0]);
            self.background.update_star_time(&self.queue, elapsed_time);
            self.queue.write_buffer(
                &self.background.time_buffer,
                0,
                bytemuck::cast_slice(&[elapsed_time]),
            );

            let star_pass_desc = wgpu::RenderPassDescriptor {
                label: Some("Star Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Preserve cleared background
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None, // No depth testing for stars
                occlusion_query_set: None,
                timestamp_writes: None,
            };
            let mut star_pass = encoder.begin_render_pass(&star_pass_desc);
            star_pass.set_pipeline(&self.background.pipeline);
            star_pass.set_bind_group(0, &self.background.uniform_bind_group, &[]);
            star_pass.set_vertex_buffer(0, self.background.vertex_buffer.slice(..));
            star_pass.set_index_buffer(
                self.background.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            star_pass.draw_indexed(0..self.background.num_indices, 0, 0..1);
            drop(star_pass);
        };

        let aspect = width as f32 / height as f32;

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
        self.queue
            .write_buffer(&self.uniform_buffer, 0, uniforms.as_bytes());

        let main_pass_desc = wgpu::RenderPassDescriptor {
            label: Some("Main Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        };
        let mut main_pass = encoder.begin_render_pass(&main_pass_desc);
        main_pass.set_pipeline(&self.pipeline);
        main_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        main_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        main_pass.draw(0..self.num_vertices, 0..1);

        // Inside your render method:
        if self.debug_render_bounding_boxes && self.debug_vertex_count > 0 {
            if let Some(debug_buffer) = &self.debug_vertex_buffer {
                main_pass.set_vertex_buffer(0, debug_buffer.slice(..));
                main_pass.draw(0..self.debug_vertex_count as u32, 0..1);
            }
        }

        Ok((surface_view, screen_descriptor, surface_texture))
    }

    pub fn update_debug_vertices(&mut self, collision_system: &CollisionSystem) {
        // Skip if debug rendering is disabled
        if !self.debug_render_bounding_boxes {
            self.debug_vertex_count = 0;
            return;
        }

        // Collect only wall face AABBs, not the entire BVH hierarchy
        let debug_vertices = collect_wall_face_debug_vertices(&collision_system.bvh);

        // Create or update the debug vertex buffer
        self.debug_vertex_count = debug_vertices.len();
        if self.debug_vertex_count > 0 {
            let debug_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Debug Vertex Buffer"),
                    contents: bytemuck::cast_slice(&debug_vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            self.debug_vertex_buffer = Some(debug_buffer);
        }
    }
}
