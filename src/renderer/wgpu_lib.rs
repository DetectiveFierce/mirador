//! WGPU-based renderer for the Mirador game.
//!
//! This module provides [`WgpuRenderer`], which manages all GPU resources, pipelines, and rendering
//! logic for the main game scene, background, and loading screen. It handles initialization of WGPU,
//! creation of vertex/uniform buffers, pipelines, and orchestrates the rendering of the maze, player,
//! animated background, and UI overlays.
//!
//! # Features
//! - Loads maze geometry and floor/wall vertices
//! - Renders a starfield background and animated loading screen
//! - Handles depth buffering and uniform updates for camera/player movement
//! - Integrates with egui for UI overlays
//!
//! # Usage
//! Create a [`WgpuRenderer`] via [`WgpuRenderer::new`] and call [`WgpuRenderer::update_canvas`]
//! each frame to render the current game state.
use crate::game::CurrentScreen;
use crate::game::GameState;
use crate::maze::maze_animation::LoadingRenderer;
use crate::renderer::render_components::GameRenderer;
use crate::renderer::text::TextRenderer;
use crate::ui::ui_panel::UiState;
use egui_wgpu::ScreenDescriptor;
use egui_wgpu::wgpu;
use egui_wgpu::wgpu::{SurfaceTexture, TextureView};

/// Main WGPU renderer for the Mirador game.
///
/// This struct manages all GPU resources, pipelines, and rendering logic for the game scene,
/// including the maze, player, animated background, and loading screen.
///
/// # Fields
/// - `surface`: The WGPU surface for presenting rendered frames.
/// - `surface_config`: The surface configuration (format, size, etc.).
/// - `device`: The WGPU device for resource creation.
/// - `queue`: The WGPU queue for submitting commands.
/// - `game_renderer`: Main render pipeline for the maze and floor.
/// - `loading_screen_renderer`: Renderer for the loading screen maze and loading bar.
pub struct WgpuRenderer {
    /// The WGPU surface for presenting rendered frames.
    pub surface: wgpu::Surface<'static>,
    /// The surface configuration (format, size, etc.).
    pub surface_config: wgpu::SurfaceConfiguration,
    /// The WGPU device for resource creation.
    pub device: wgpu::Device,
    /// The WGPU queue for submitting commands.
    pub queue: wgpu::Queue,
    /// Main render pipeline for the maze and floor.
    pub game_renderer: GameRenderer,
    /// Renderer for the loading screen maze and loading bar.
    pub loading_screen_renderer: LoadingRenderer,
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

        let game_renderer = GameRenderer::new(&device, &surface_config);
        let loading_screen_renderer = LoadingRenderer::new(&device, &surface_config);

        Self {
            surface,
            surface_config,
            device,
            queue,
            game_renderer,
            loading_screen_renderer,
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
    /// - `loading`: If true, renders the loading screen; otherwise renders the main game scene.
    ///
    /// # Returns
    /// - `Ok((TextureView, ScreenDescriptor, SurfaceTexture))` on success.
    /// - `Err(String)` if the surface is outdated or unavailable.
    pub fn update_canvas(
        &mut self,
        window: &winit::window::Window,
        encoder: &mut wgpu::CommandEncoder,
        ui_state: &UiState,
        game_state: &GameState,
        text_renderer: &mut TextRenderer,
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
        let aspect = width as f32 / height as f32;
        let depth_texture_view =
            self.game_renderer
                .update_depth_texture(&self.device, width, height);

        if game_state.current_screen == CurrentScreen::Loading {
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

                // Render loading screen using new component architecture
                self.loading_screen_renderer
                    .render(&mut render_pass, window);
            }

            return Ok((surface_view, screen_descriptor, surface_texture)); // <- This return was already there
        }
        if game_state.current_screen == CurrentScreen::Game {
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
            let elapsed_time = ui_state.start_time.elapsed().as_secs_f32();
            self.game_renderer
                .star_renderer
                .update_background_color(&self.queue, [ui_state.r, ui_state.g, ui_state.b, 1.0]);
            self.game_renderer
                .star_renderer
                .update_star_time(&self.queue, elapsed_time);
            self.queue.write_buffer(
                &self.game_renderer.star_renderer.time_buffer,
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
            star_pass.set_pipeline(&self.game_renderer.star_renderer.pipeline);
            star_pass.set_bind_group(0, &self.game_renderer.star_renderer.uniform_bind_group, &[]);
            star_pass
                .set_vertex_buffer(0, self.game_renderer.star_renderer.vertex_buffer.slice(..));
            star_pass.set_index_buffer(
                self.game_renderer.star_renderer.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            star_pass.draw_indexed(0..self.game_renderer.star_renderer.num_indices, 0, 0..1);
            drop(star_pass);
        };

        {
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
            self.game_renderer
                .render_game(&self.queue, &game_state.player, &mut main_pass, aspect);
        }

        {
            text_renderer.resize(
                &self.queue,
                glyphon::Resolution {
                    width: self.surface_config.width,
                    height: self.surface_config.height,
                },
            );
            match text_renderer.prepare(&self.device, &self.queue, &self.surface_config) {
                Ok(_) => {}
                Err(e) => {
                    println!("Failed to prepare Glyphon: {:?}", e);
                }
            }

            let mut text_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            match text_renderer.render(&mut text_pass) {
                Ok(_) => {}
                Err(e) => println!("Glyphon render failed: {:?}", e),
            }
        }

        Ok((surface_view, screen_descriptor, surface_texture))
    }
}
