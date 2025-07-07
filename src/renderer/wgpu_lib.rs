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
use crate::renderer::game_renderer::GameRenderer;
use crate::renderer::game_renderer::game_over::GameOverRenderer;
use crate::renderer::loading_renderer::LoadingRenderer;
use crate::renderer::text::TextRenderer;
use crate::ui::ui_panel::UiState;
use egui_wgpu::ScreenDescriptor;
use egui_wgpu::wgpu;
use egui_wgpu::wgpu::{SurfaceTexture, TextureView};

/// Main WGPU renderer for the Mirador game.
///
/// This struct manages all GPU resources, pipelines, and rendering logic for the game scene,
/// including the maze, player, animated background, and loading screen.
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
    /// Renderer for the game over screen.
    pub game_over_renderer: GameOverRenderer,
}

impl WgpuRenderer {
    /// Initializes a new [`WgpuRenderer`] and all associated GPU resources.
    pub async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
    ) -> Self {
        let adapter = Self::create_adapter(instance, &surface).await;
        let (device, queue) = Self::create_device(&adapter).await;
        let surface_config = Self::create_surface_config(&surface, &adapter, width, height);

        surface.configure(&device, &surface_config);

        let game_renderer = GameRenderer::new(&device, &queue, &surface_config);
        let loading_screen_renderer = LoadingRenderer::new(&device, &surface_config);
        let game_over_renderer = GameOverRenderer::new(&device, &surface_config);

        Self {
            surface,
            surface_config,
            device,
            queue,
            game_renderer,
            loading_screen_renderer,
            game_over_renderer,
        }
    }

    /// Renders the current frame to the surface.
    pub fn update_canvas(
        &mut self,
        window: &winit::window::Window,
        encoder: &mut wgpu::CommandEncoder,
        ui_state: &UiState,
        game_state: &GameState,
        text_renderer: &mut TextRenderer,
    ) -> Result<(TextureView, ScreenDescriptor, SurfaceTexture), String> {
        let (surface_texture, surface_view) = self.get_surface_texture_and_view()?;
        let screen_descriptor = self.create_screen_descriptor(window);
        let depth_texture_view = self.update_depth_texture();

        match game_state.current_screen {
            CurrentScreen::Loading => {
                self.render_loading_screen(encoder, &surface_view, window);
            }
            CurrentScreen::GameOver => {
                self.render_game_over_screen(
                    encoder,
                    &surface_view,
                    &depth_texture_view,
                    ui_state,
                    game_state,
                    text_renderer,
                    window,
                );
            }
            CurrentScreen::Game => {
                self.render_game_screen(
                    encoder,
                    &surface_view,
                    &depth_texture_view,
                    ui_state,
                    game_state,
                    text_renderer,
                    window,
                );
            }
            _ => {}
        }

        Ok((surface_view, screen_descriptor, surface_texture))
    }

    // Private helper methods

    async fn create_adapter(
        instance: &wgpu::Instance,
        surface: &wgpu::Surface<'static>,
    ) -> wgpu::Adapter {
        instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(surface),
            })
            .await
            .expect("Failed to find an appropriate adapter")
    }

    async fn create_device(adapter: &wgpu::Adapter) -> (wgpu::Device, wgpu::Queue) {
        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: Default::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device")
    }

    fn create_surface_config(
        surface: &wgpu::Surface<'static>,
        adapter: &wgpu::Adapter,
        width: u32,
        height: u32,
    ) -> wgpu::SurfaceConfiguration {
        let capabilities = surface.get_capabilities(adapter);
        let format = capabilities
            .formats
            .iter()
            .find(|&&f| f == wgpu::TextureFormat::Bgra8UnormSrgb)
            .copied()
            .expect("Failed to select proper surface texture format");

        wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 0,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
        }
    }

    fn get_surface_texture_and_view(&self) -> Result<(SurfaceTexture, TextureView), String> {
        let surface_texture = match self.surface.get_current_texture() {
            Ok(texture) => texture,
            Err(wgpu::SurfaceError::Outdated) => {
                return Err("WGPU surface outdated".to_string());
            }
            Err(_) => {
                return Err("Failed to acquire next swap chain texture".to_string());
            }
        };

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        Ok((surface_texture, surface_view))
    }

    fn create_screen_descriptor(&self, window: &winit::window::Window) -> ScreenDescriptor {
        ScreenDescriptor {
            size_in_pixels: [self.surface_config.width, self.surface_config.height],
            pixels_per_point: window.scale_factor() as f32,
        }
    }

    fn update_depth_texture(&mut self) -> TextureView {
        let (width, height) = (self.surface_config.width, self.surface_config.height);
        self.game_renderer
            .update_depth_texture(&self.device, width, height)
    }

    fn render_loading_screen(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &TextureView,
        window: &winit::window::Window,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Loading Screen Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_view,
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

        self.loading_screen_renderer
            .render(&mut render_pass, window);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_game_over_screen(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &TextureView,
        depth_texture_view: &TextureView,
        ui_state: &UiState,
        game_state: &GameState,
        text_renderer: &mut TextRenderer,
        window: &winit::window::Window,
    ) {
        let aspect = self.surface_config.width as f32 / self.surface_config.height as f32;
        let background_color = [ui_state.r, ui_state.g, ui_state.b, 1.0];

        // Clear pass
        self.clear_render_target(encoder, surface_view, depth_texture_view, background_color);

        // Render stars
        self.render_stars(encoder, surface_view, ui_state, background_color);

        // Render game objects (frozen state)
        self.render_game_objects(
            encoder,
            surface_view,
            depth_texture_view,
            game_state,
            aspect,
        );

        // Render game over overlay
        self.render_game_over_overlay(encoder, surface_view, window);

        // Render text
        self.render_game_over_text(encoder, surface_view, text_renderer);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_game_screen(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &TextureView,
        depth_texture_view: &TextureView,
        ui_state: &UiState,
        game_state: &GameState,
        text_renderer: &mut TextRenderer,
        window: &winit::window::Window,
    ) {
        let aspect = self.surface_config.width as f32 / self.surface_config.height as f32;
        let background_color = [ui_state.r, ui_state.g, ui_state.b, 1.0];

        // Clear pass
        self.clear_render_target(encoder, surface_view, depth_texture_view, background_color);

        // Render stars
        self.render_stars(encoder, surface_view, ui_state, background_color);

        // Render game objects
        self.render_game_objects(
            encoder,
            surface_view,
            depth_texture_view,
            game_state,
            aspect,
        );

        // Render compass
        self.render_compass(encoder, surface_view, game_state, window);

        // Render text
        self.render_text(encoder, surface_view, text_renderer);
    }

    fn clear_render_target(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &TextureView,
        depth_texture_view: &TextureView,
        background_color: [f32; 4],
    ) {
        let _clear_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Clear Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: background_color[0] as f64,
                        g: background_color[1] as f64,
                        b: background_color[2] as f64,
                        a: background_color[3] as f64,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
    }

    fn render_stars(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &TextureView,
        ui_state: &UiState,
        background_color: [f32; 4],
    ) {
        let elapsed_time = ui_state.start_time.elapsed().as_secs_f32();

        // Update star renderer state
        self.game_renderer
            .star_renderer
            .update_background_color(&self.queue, background_color);
        self.game_renderer
            .star_renderer
            .update_star_time(&self.queue, elapsed_time);

        let mut star_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Star Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        star_pass.set_pipeline(&self.game_renderer.star_renderer.pipeline);
        star_pass.set_bind_group(0, &self.game_renderer.star_renderer.uniform_bind_group, &[]);
        star_pass.set_vertex_buffer(0, self.game_renderer.star_renderer.vertex_buffer.slice(..));
        star_pass.set_index_buffer(
            self.game_renderer.star_renderer.index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );
        star_pass.draw_indexed(0..self.game_renderer.star_renderer.num_indices, 0, 0..1);
    }

    fn render_game_objects(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &TextureView,
        depth_texture_view: &TextureView,
        game_state: &GameState,
        aspect: f32,
    ) {
        let mut main_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Main Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        self.game_renderer
            .render_game(&self.queue, game_state, &mut main_pass, aspect);
    }

    fn render_compass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &TextureView,
        game_state: &GameState,
        window: &winit::window::Window,
    ) {
        if let Some(exit_position) = self.game_renderer.exit_position {
            self.game_renderer.compass_renderer.update_compass_with_yaw(
                (game_state.player.position[0], game_state.player.position[2]),
                exit_position,
                game_state.player.yaw,
            );

            let mut compass_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Compass Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            self.game_renderer
                .compass_renderer
                .render(&mut compass_pass, window);
        }
    }

    fn render_game_over_overlay(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &TextureView,
        window: &winit::window::Window,
    ) {
        let mut game_over_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Game Over Overlay Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        self.game_over_renderer.render(&mut game_over_pass, window);
    }

    fn render_text(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &TextureView,
        text_renderer: &mut TextRenderer,
    ) {
        self.prepare_text_renderer(text_renderer);

        let mut text_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Text Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_view,
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

        if let Err(e) = text_renderer.render(&mut text_pass) {
            println!("Text render failed: {:?}", e);
        }
    }

    fn render_game_over_text(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &TextureView,
        text_renderer: &mut TextRenderer,
    ) {
        self.prepare_text_renderer(text_renderer);
        text_renderer.show_game_over_display();

        let mut text_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Game Over Text Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_view,
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

        if let Err(e) = text_renderer.render(&mut text_pass) {
            println!("Game over text render failed: {:?}", e);
        }
    }

    fn prepare_text_renderer(&self, text_renderer: &mut TextRenderer) {
        text_renderer.resize(
            &self.queue,
            glyphon::Resolution {
                width: self.surface_config.width,
                height: self.surface_config.height,
            },
        );

        if let Err(e) = text_renderer.prepare(&self.device, &self.queue, &self.surface_config) {
            println!("Failed to prepare text renderer: {:?}", e);
        }
    }
}
