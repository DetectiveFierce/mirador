//! Main application module for Mirador.
//!
//! This module defines the [`AppState`] and [`App`] structs, which together manage the
//! initialization, event handling, rendering, and game state for the Mirador application.
//!
//! # Overview
//! - [`AppState`] contains all state required for a running game session, including rendering,
//!   UI, game logic, and input state.
//! - [`App`] is the main application object, responsible for window and event loop integration,
//!   and implements [`winit::application::ApplicationHandler`] for cross-platform event handling.
//!
//! # Main Responsibilities
//! - Initialize WGPU and egui renderers
//! - Manage game state, UI, and input
//! - Handle window events, resizing, and redraws
//! - Orchestrate maze generation and title screen animation
//! - Integrate with the winit event loop
use crate::game::GameTimer;
use crate::game::enemy::{Enemy, place_enemy_standard};
use crate::game::player::Player;
use crate::game::{
    self, GameState,
    keys::{GameKey, KeyState, winit_key_to_game_key},
};
use crate::game::{CurrentScreen, TimerConfig};
use crate::math::coordinates::maze_to_world;
use crate::maze::parse_maze_file;
use crate::renderer::loading_renderer::LoadingRenderer;
use crate::renderer::primitives::Vertex;
use crate::renderer::text::TextRenderer;
use crate::renderer::title;
use crate::{
    renderer::wgpu_lib::WgpuRenderer,
    ui::{egui_lib::EguiRenderer, ui_panel::UiState},
};
use egui_wgpu::wgpu;
use egui_wgpu::wgpu::util::DeviceExt;
use glyphon::Color;
use std::time::Duration;
use std::{sync::Arc, time::Instant};
use winit::event::MouseButton;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, DeviceId, ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

/// Holds all state required for a running Mirador game session.
///
/// This includes rendering backends, UI state, game logic, and input state.
pub struct AppState {
    /// The WGPU renderer for the main game and background.
    pub wgpu_renderer: WgpuRenderer,
    /// The egui renderer for UI overlays.
    pub egui_renderer: EguiRenderer,
    /// The current UI state (sliders, colors, etc.).
    pub ui: UiState,
    /// The main game state (player, timing, maze, etc.).
    pub game_state: GameState,
    /// The current input state (pressed keys, etc.).
    pub key_state: KeyState,
    /// The text renderer for all game UI text elements.
    pub text_renderer: TextRenderer,
    pub start_time: Instant,
    pub elapsed_time: Duration,
    pub pause_menu: crate::ui::pause_menu::PauseMenu,
}

impl AppState {
    /// Asynchronously creates a new [`AppState`] with initialized renderers and game state.
    ///
    /// # Arguments
    /// - `instance`: The WGPU instance.
    /// - `surface`: The WGPU surface for rendering.
    /// - `window`: The application window.
    /// - `width`: Initial window width.
    /// - `height`: Initial window height.
    pub async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        window: &Window,
        width: u32,
        height: u32,
    ) -> Self {
        window.set_cursor_visible(false);
        let wgpu_renderer = WgpuRenderer::new(instance, surface, width, height).await;
        let egui_renderer = EguiRenderer::new(
            &wgpu_renderer.device,
            wgpu_renderer.surface_config.format,
            None,
            1,
            window,
        );

        let mut text_renderer = TextRenderer::new(
            &wgpu_renderer.device,
            &wgpu_renderer.queue,
            wgpu_renderer.surface_config.format,
            window,
        );

        // Check if font loading was successful
        if text_renderer.loaded_fonts.is_empty() {
            println!("WARNING: No fonts loaded! Text may not render properly.");
        } else {
            println!("Loaded fonts: {:?}", text_renderer.loaded_fonts);
        }

        let game_state = GameState::new();
        // Initialize all game UI elements
        game::initialize_game_ui(&mut text_renderer, &game_state.game_ui, window);

        // Create game over display
        text_renderer.create_game_over_display(width, height);

        let pause_menu = crate::ui::pause_menu::PauseMenu::new(
            &wgpu_renderer.device,
            &wgpu_renderer.queue,
            wgpu_renderer.surface_config.format,
            window,
        );

        // Add big bold 'Mirador' text in the top right for the title screen
        let width = wgpu_renderer.surface_config.width as f32;
        let height = wgpu_renderer.surface_config.height as f32;
        let mirador_style = crate::renderer::text::TextStyle {
            font_family: "HankenGrotesk".to_string(),
            font_size: 125.0,
            line_height: 150.0,
            color: Color::rgb(58, 53, 70), // #3A3546
            weight: glyphon::Weight::BOLD,
            style: glyphon::Style::Normal,
        };
        // Estimate text width for right alignment
        let text_width = 620.0; // Conservative estimate for large text
        let text_height = 150.0;
        let mirador_position = crate::renderer::text::TextPosition {
            x: width - text_width - 20.0, // 20px margin from right
            y: 100.0,                     // 100px margin from top
            max_width: Some(text_width),
            max_height: Some(text_height),
        };
        text_renderer.create_text_buffer(
            "title_mirador_overlay",
            "Mirador",
            Some(mirador_style),
            Some(mirador_position.clone()),
        );
        // Add subtitle text at the same x, 60px from the bottom
        let subtitle_style = crate::renderer::text::TextStyle {
            font_family: "HankenGrotesk".to_string(),
            font_size: 48.0,
            line_height: 72.0,
            color: Color::rgb(58, 53, 70), // #3A3546
            weight: glyphon::Weight::MEDIUM,
            style: glyphon::Style::Normal,
        };
        let subtitle_text = "Click anywhere to get lost.";
        let subtitle_text_height = 72.0;
        let subtitle_position = crate::renderer::text::TextPosition {
            x: mirador_position.x - 120.0,
            y: height + 100.0,
            max_width: Some(text_width),
            max_height: Some(4.0 * subtitle_text_height),
        };
        text_renderer.create_text_buffer(
            "title_subtitle_overlay",
            subtitle_text,
            Some(subtitle_style),
            Some(subtitle_position),
        );

        Self {
            wgpu_renderer,
            egui_renderer,
            ui: UiState::new(),
            game_state,
            key_state: KeyState::new(),
            text_renderer,
            start_time: Instant::now(),
            elapsed_time: Duration::ZERO,
            pause_menu,
        }
    }

    /// Resizes the WGPU surface and updates the configuration.
    ///
    /// # Arguments
    /// - `width`: New width of the surface.
    /// - `height`: New height of the surface.
    pub fn resize_surface(&mut self, width: u32, height: u32) {
        self.wgpu_renderer.surface_config.width = width;
        self.wgpu_renderer.surface_config.height = height;
        self.wgpu_renderer.surface.configure(
            &self.wgpu_renderer.device,
            &self.wgpu_renderer.surface_config,
        );

        self.wgpu_renderer
            .game_renderer
            .compass_renderer
            .update_uniforms(&self.wgpu_renderer.queue, [0.75, 0.75], [4.75, 4.75]);

        // Update game over display position for new window size
        if let Err(e) = self.text_renderer.update_game_over_position(width, height) {
            println!("Failed to update game over position: {}", e);
        }
    }

    /// Updates the title screen maze and loading bar, and uploads new texture data.
    pub fn handle_loading_screen(&mut self, window: &winit::window::Window) {
        let progress = self
            .wgpu_renderer
            .loading_screen_renderer
            .generator
            .get_progress_ratio();

        let (maze_width, maze_height) = match self.wgpu_renderer.loading_screen_renderer.maze.lock()
        {
            Ok(maze_lock) => maze_lock.get_dimensions(),

            Err(err) => {
                eprintln!("Failed to acquire maze lock for dimensions: {}", err);
                return;
            }
        };

        self.wgpu_renderer
            .loading_screen_renderer
            .update_loading_bar(&self.wgpu_renderer.queue, progress, window);

        self.wgpu_renderer
            .loading_screen_renderer
            .update_exit_shader(&self.wgpu_renderer.queue, window);

        let maze_data = match self.wgpu_renderer.loading_screen_renderer.maze.lock() {
            Ok(maze_lock) => maze_lock.get_render_data(
                &self
                    .wgpu_renderer
                    .loading_screen_renderer
                    .generator
                    .connected_cells,
            ),

            Err(err) => {
                eprintln!("Failed to acquire maze lock: {}", err);
                return;
            }
        };

        self.wgpu_renderer.loading_screen_renderer.update_texture(
            &self.wgpu_renderer.queue,
            &maze_data,
            maze_width,
            maze_height,
        );
        self.wgpu_renderer.loading_screen_renderer.last_update = Instant::now();
    }

    /// Handles mouse capture and cursor visibility based on game state.
    ///
    /// Locks/unlocks the cursor and centers it if mouse capture is enabled.
    pub fn triage_mouse(&mut self, window: &Window) {
        if self.game_state.capture_mouse {
            if let Err(e) = window.set_cursor_grab(winit::window::CursorGrabMode::Locked) {
                eprintln!("Failed to lock cursor: {}", e);
            }
            window.set_cursor_visible(false);
            let window_size = window.inner_size().to_logical::<f64>(window.scale_factor());

            let center_x = window_size.width / 2.0;
            let center_y = window_size.height / 2.0;
            if let Err(e) =
                window.set_cursor_position(winit::dpi::LogicalPosition::new(center_x, center_y))
            {
                eprintln!("Failed to center cursor: {}", e);
            }
        } else if !self.game_state.capture_mouse {
            if let Err(e) = window.set_cursor_grab(winit::window::CursorGrabMode::None) {
                eprintln!("Failed to unlock cursor: {}", e);
            }
            window.set_cursor_visible(true);
        }
    }

    /// Updates all game UI elements including timer, level, and score displays.
    pub fn update_game_ui(&mut self, window: &winit::window::Window) {
        // Start timer when game begins (not on title screen)
        if self.game_state.current_screen == CurrentScreen::Game
            && self.game_state.game_ui.timer.is_none()
        {
            // Configure timer with custom settings
            let timer_config = TimerConfig {
                duration: Duration::from_secs(30),
                warning_threshold: Duration::from_secs(20),
                critical_threshold: Duration::from_secs(10),
                normal_color: Color::rgb(100, 255, 100),
                warning_color: Color::rgb(255, 255, 100),
                critical_color: Color::rgb(255, 100, 100),
            };
            self.game_state.start_game_timer(Some(timer_config));
        }

        // Hide game UI elements during loading screen or title screen
        if self.game_state.current_screen == CurrentScreen::Loading
            || self.game_state.current_screen == CurrentScreen::Title
        {
            // Hide timer, level, and score displays
            if let Some(buffer) = self.text_renderer.text_buffers.get_mut("main_timer") {
                buffer.visible = false;
            }
            if let Some(buffer) = self.text_renderer.text_buffers.get_mut("level") {
                buffer.visible = false;
            }
            if let Some(buffer) = self.text_renderer.text_buffers.get_mut("score") {
                buffer.visible = false;
            }
        } else {
            // Show game UI elements when not loading
            if let Some(buffer) = self.text_renderer.text_buffers.get_mut("main_timer") {
                buffer.visible = true;
            }
            if let Some(buffer) = self.text_renderer.text_buffers.get_mut("level") {
                buffer.visible = true;
            }
            if let Some(buffer) = self.text_renderer.text_buffers.get_mut("score") {
                buffer.visible = true;
            }
        }

        // Always update the text UI, but only update the timer if in Game
        let timer_expired = game::update_game_ui(
            &mut self.text_renderer,
            &mut self.game_state.game_ui,
            &self.game_state.current_screen,
            window,
        );

        if timer_expired {
            // Handle timer expiration - you can add game over logic here
            println!("Timer expired! Game over.");
            self.game_state.current_screen = CurrentScreen::GameOver;
        }

        if self.game_state.enemy.pathfinder.reached_player {
            self.game_state.current_screen = CurrentScreen::GameOver;
            self.game_state.enemy = Enemy::new([-0.5, 30.0, 0.0], 150.0);
            self.game_state.enemy.pathfinder.reached_player = false
        }

        // Show/hide game over display based on current screen
        if self.game_state.current_screen == CurrentScreen::GameOver {
            self.text_renderer.show_game_over_display();
        } else {
            self.text_renderer.hide_game_over_display();
        }
    }
}

/// Main application object for Mirador.
///
/// Manages the WGPU instance, window, and application state, and implements
/// [`winit::application::ApplicationHandler`] for cross-platform event handling.
#[derive(Default)]
pub struct App {
    /// The WGPU instance for the application.
    instance: wgpu::Instance,
    /// The current application state (renderers, game, UI, etc.).
    state: Option<AppState>,
    /// The application window.
    window: Option<Arc<Window>>,
}

impl App {
    /// Creates a new [`App`] with a fresh WGPU instance and no window or state.
    pub fn new() -> Self {
        let instance = egui_wgpu::wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        Self {
            instance,
            state: None,
            window: None,
        }
    }

    /// Asynchronously sets up the application window and initializes [`AppState`].
    ///
    /// # Arguments
    /// - `window`: The application window to use.
    pub async fn set_window(&mut self, window: Window) {
        let window = Arc::new(window);
        let initial_width = 1360;
        let initial_height = 768;

        let _ = window.request_inner_size(PhysicalSize::new(initial_width, initial_height));

        let surface = self
            .instance
            .create_surface(window.clone())
            .expect("Failed to create surface!");

        let state = AppState::new(
            &self.instance,
            surface,
            &window,
            initial_width,
            initial_height,
        )
        .await;

        self.window.get_or_insert(window);
        self.state.get_or_insert(state);
    }

    /// Handles window resize events and updates the rendering surface.
    ///
    /// # Arguments
    /// - `width`: New width of the window.
    /// - `height`: New height of the window.
    pub fn handle_resized(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            let state = match &mut self.state {
                Some(state) => state,
                None => {
                    eprintln!("Cannot resize surface without state initialized!");
                    #[cfg(debug_assertions)]
                    eprintln!("Backtrace: {:?}", std::backtrace::Backtrace::capture());
                    return;
                }
            };
            state.resize_surface(width, height);
            // Ensure pause menu resizes with the window
            use glyphon::Resolution;
            let resolution = Resolution { width, height };
            state
                .pause_menu
                .resize(&state.wgpu_renderer.queue, resolution);
        }
    }

    /// Handles redraw requests, updates game state, renders the frame, and manages the title screen.
    pub fn handle_redraw(&mut self) {
        let window = self
            .window
            .as_ref()
            .expect("Window must be initialized before use");
        if window.is_minimized().unwrap_or(false) {
            println!("Window is minimized");
            return;
        }

        let state = self
            .state
            .as_mut()
            .expect("State must be initialized before use");

        if state.game_state.current_screen == CurrentScreen::Loading {
            state
                .game_state
                .audio_manager
                .pause_enemy_audio("enemy")
                .expect("Failed to pause enemy audio");
            state.handle_loading_screen(window);
        } else if state.game_state.current_screen == CurrentScreen::Title {
            title::handle_title(state, window);
            return;
        } else {
            state.game_state.player.update_cell(
                &state
                    .wgpu_renderer
                    .loading_screen_renderer
                    .maze
                    .lock()
                    .unwrap()
                    .walls,
            );
        }

        // Update game state and UI
        state.key_state.update(&mut state.game_state);
        state.update_game_ui(window);
        state.update_ui(window);
        state
            .game_state
            .audio_manager
            .set_listener_position(state.game_state.player.position)
            .expect("Failed to set listener position");
        state
            .game_state
            .audio_manager
            .update_enemy_position("enemy", state.game_state.enemy.pathfinder.position)
            .expect("Failed to update enemy position");

        // Prepare rendering commands
        let mut encoder = state
            .wgpu_renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        // Update canvas surface
        let (surface_view, surface_texture) = match state.wgpu_renderer.update_canvas(
            window,
            &mut encoder,
            &state.ui,
            &state.game_state,
            &mut state.text_renderer,
        ) {
            Ok(result) => result,
            Err(err) => {
                eprintln!("Failed to update canvas: {}", err);
                #[cfg(debug_assertions)]
                eprintln!("Backtrace: {:?}", std::backtrace::Backtrace::capture());
                return;
            }
        };

        // --- Debug Info Panel ---
        if state.pause_menu.is_debug_panel_visible() {
            let window_size = &state.wgpu_renderer.surface_config;
            let debug_text = format!(
                "Window Size: {} x {}",
                window_size.width, window_size.height
            );
            let style = crate::renderer::text::TextStyle {
                font_family: "HankenGrotesk".to_string(),
                font_size: 22.0,
                line_height: 26.0,
                color: Color::rgb(220, 40, 40),
                weight: glyphon::Weight::BOLD,
                style: glyphon::Style::Normal,
            };
            let pos = crate::renderer::text::TextPosition {
                x: window_size.width as f32 - 320.0,
                y: 20.0,
                max_width: Some(300.0),
                max_height: Some(40.0),
            };
            state.text_renderer.create_text_buffer(
                "debug_info",
                &debug_text,
                Some(style),
                Some(pos),
            );
        } else {
            // Hide debug info by making it transparent if it exists
            if let Some(buf) = state.text_renderer.text_buffers.get_mut("debug_info") {
                buf.visible = false;
            }
        }
        // Prepare and render text BEFORE pause menu overlay
        if let Err(e) = state.text_renderer.prepare(
            &state.wgpu_renderer.device,
            &state.wgpu_renderer.queue,
            &state.wgpu_renderer.surface_config,
        ) {
            println!("Failed to prepare text renderer: {}", e);
        }
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                label: Some("text render pass"),
                occlusion_query_set: None,
            });
            if let Err(e) = state.text_renderer.render(&mut render_pass) {
                println!("Failed to render text: {}", e);
            }
        }
        // --- End Game UI ---

        // If paused, render the pause menu on top
        if state.game_state.current_screen == CurrentScreen::Pause {
            if !state.pause_menu.is_visible() {
                state.pause_menu.show();
            }

            // Create a render pass for the pause menu
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                label: Some("pause menu render pass"),
                occlusion_query_set: None,
            });

            // --- Add semi-transparent grey overlay ---
            let overlay_color = [0.08, 0.09, 0.11, 0.88]; // darker, neutral semi-transparent grey
            let (w, h) = (
                state.wgpu_renderer.surface_config.width as f32,
                state.wgpu_renderer.surface_config.height as f32,
            );
            state
                .pause_menu
                .button_manager
                .rectangle_renderer
                .add_rectangle(crate::renderer::rectangle::Rectangle::new(
                    0.0,
                    0.0,
                    w,
                    h,
                    overlay_color,
                ));
            state
                .pause_menu
                .button_manager
                .rectangle_renderer
                .render(&state.wgpu_renderer.device, &mut render_pass);
            // --- End overlay ---

            // Prepare pause menu for rendering (text)
            if let Err(e) = state.pause_menu.prepare(
                &state.wgpu_renderer.device,
                &state.wgpu_renderer.queue,
                &state.wgpu_renderer.surface_config,
            ) {
                println!("Failed to prepare pause menu: {}", e);
            }

            // Render the pause menu (rectangles + text)
            if let Err(e) = state
                .pause_menu
                .render(&state.wgpu_renderer.device, &mut render_pass)
            {
                println!("Failed to render pause menu: {}", e);
            }
        } else {
            if state.pause_menu.is_visible() {
                state.pause_menu.hide();
            }
            // Explicitly clear rectangles if menu is not visible
            state
                .pause_menu
                .button_manager
                .rectangle_renderer
                .clear_rectangles();
        }

        window.request_redraw();

        // Submit commands and present
        state.wgpu_renderer.queue.submit(Some(encoder.finish()));
        surface_texture.present();

        // Update enemy pathfinding
        state.game_state.enemy.update(
            state.game_state.player.position,
            state.game_state.delta_time,
            state.game_state.game_ui.level as u32,
            |from, to| {
                state
                    .game_state
                    .collision_system
                    .cylinder_intersects_geometry(from, to, 5.0)
            },
        );

        // Handle title screen animation if needed
        if state.game_state.current_screen == CurrentScreen::Loading {
            state.game_state.game_ui.stop_timer();
            self.handle_maze_generation();
        } else if state.game_state.current_screen == CurrentScreen::NewGame {
            state.text_renderer.hide_game_over_display();
            self.new_level(true);
        } else if state.game_state.current_screen == CurrentScreen::Game
            && Some(state.game_state.player.current_cell) == state.game_state.exit_cell
        {
            state.game_state.enemy.pathfinder.position = [0.0, 30.0, 0.0];
            state.game_state.enemy.pathfinder.locked = true;
            self.new_level(false);
        } else if state.game_state.current_screen == CurrentScreen::Game {
            state
                .game_state
                .audio_manager
                .resume_enemy_audio("enemy")
                .expect("Failed to resume enemy audio");
            state.game_state.enemy.pathfinder.locked = false;
        }
    }

    pub fn new_level(&mut self, game_over: bool) {
        let state = self
            .state
            .as_mut()
            .expect("State must be initialized before use");
        state.game_state.current_screen = CurrentScreen::Loading;
        state.game_state.maze_path = None;
        state.wgpu_renderer.loading_screen_renderer = LoadingRenderer::new(
            &state.wgpu_renderer.device,
            &state.wgpu_renderer.surface_config,
        );

        // Clear previous level state
        state.game_state.player = Player::new();
        state.game_state.enemy.pathfinder.position = [0.0, 30.0, 0.0];
        state.game_state.enemy.pathfinder.locked = true;
        state.game_state.exit_cell = None; // Clear exit cell to prevent accidental win condition

        // Stop and reset timer
        if let Some(timer) = &mut state.game_state.game_ui.timer {
            timer.stop();
            timer.reset();
        }

        if game_over {
            state.game_state.set_level(1);
            state.game_state.set_score(0);
            state.game_state.game_ui.timer = Some(GameTimer::new(TimerConfig::default()));
            game::update_game_ui(
                &mut state.text_renderer,
                &mut state.game_state.game_ui,
                &state.game_state.current_screen,
                self.window
                    .as_ref()
                    .expect("Window must be initialized before use"),
            );
            // Ensure clean state for new game
            state.game_state.exit_cell = None;
        } else {
            let current_level = state.game_state.game_ui.level;

            // Calculate completion time and performance metrics
            let (completion_time, _) = if let Some(timer) = &state.game_state.game_ui.timer {
                let remaining_time = timer.get_remaining_time().as_secs_f32();
                let total_time = timer.config.duration.as_secs_f32();
                let completion_time = total_time - remaining_time;

                // Performance-based time bonus calculation
                // Optimal time: 15 seconds, Average time: 25 seconds, Slow time: 35+ seconds
                let time_bonus = if completion_time <= 15.0 {
                    // Exceptional performance: 15-25 seconds added
                    let performance_ratio = (15.0 - completion_time).max(0.0) / 15.0;
                    15.0 + (performance_ratio * 10.0)
                } else if completion_time <= 25.0 {
                    // Good performance: 8-15 seconds added
                    let performance_ratio = (25.0 - completion_time) / 10.0;
                    8.0 + (performance_ratio * 7.0)
                } else if completion_time <= 35.0 {
                    // Average performance: 3-8 seconds added
                    let performance_ratio = (35.0 - completion_time) / 10.0;
                    3.0 + (performance_ratio * 5.0)
                } else {
                    // Slow completion: 1-3 seconds added
                    let performance_ratio = (45.0 - completion_time).max(0.0) / 10.0;
                    1.0 + (performance_ratio * 2.0)
                };

                (completion_time, time_bonus)
            } else {
                (30.0, 3.0) // Fallback values
            };

            // Enhanced scoring system
            let base_score = 150 * current_level as u32; // Increased base score

            // Speed bonus calculation
            let speed_bonus = if completion_time <= 15.0 {
                // Exceptional: 3x to 5x multiplier
                let multiplier = 3.0 + ((15.0 - completion_time) / 15.0) * 2.0;
                (base_score as f32 * multiplier) as u32
            } else if completion_time <= 25.0 {
                // Good: 1.5x to 3x multiplier
                let multiplier = 1.5 + ((25.0 - completion_time) / 10.0) * 1.5;
                (base_score as f32 * multiplier) as u32
            } else if completion_time <= 35.0 {
                // Average: 0.5x to 1.5x multiplier
                let multiplier = 0.5 + ((35.0 - completion_time) / 10.0) * 1.0;
                (base_score as f32 * multiplier) as u32
            } else {
                // Slow: 0.1x to 0.5x multiplier
                let multiplier = 0.1 + ((45.0 - completion_time).max(0.0) / 10.0) * 0.4;
                (base_score as f32 * multiplier) as u32
            };

            // Level progression bonus (small bonus for reaching higher levels)
            let level_bonus = if current_level > 5 {
                (current_level - 5) as u32 * 50
            } else {
                0
            };

            // Consecutive level bonus (reward for sustained performance)
            let consecutive_bonus = if completion_time <= 20.0 {
                // Only give consecutive bonus for good performance
                current_level as u32 * 25
            } else {
                0
            };

            let total_score = base_score + speed_bonus + level_bonus + consecutive_bonus;

            // Update score and level
            state
                .game_state
                .set_score(state.game_state.game_ui.score + total_score);
            state.game_state.set_level(current_level + 1);

            // Enhanced time management: Not supported in new timer, so skip add_time/subtract_time/prev_time
        }
    }
    /// Updates frame timing, FPS, and delta time in the game state.
    ///
    /// # Arguments
    /// - `current_time`: The current time (typically from `Instant::now()`).
    pub fn handle_frame_timing(&mut self, current_time: Instant) {
        if let Some(state) = self.state.as_mut() {
            let duration = current_time.duration_since(state.game_state.last_fps_time);

            state.elapsed_time = current_time.duration_since(state.start_time);
            state.ui.elapsed_time += 1.0;
            state.game_state.frame_count += 1;

            if duration.as_secs_f32() >= 1.0 {
                state.game_state.current_fps = state.game_state.frame_count;
                state.game_state.frame_count = 0;
                state.game_state.last_fps_time = current_time;
            }

            let delta_time = current_time
                .duration_since(state.game_state.last_frame_time)
                .as_secs_f32();

            state.game_state.delta_time = delta_time;
            state.game_state.last_frame_time = current_time;

            if state
                .wgpu_renderer
                .game_renderer
                .debug_renderer
                .debug_render_bounding_boxes
            {
                state
                    .wgpu_renderer
                    .game_renderer
                    .debug_renderer
                    .update_debug_vertices(
                        &state.wgpu_renderer.device,
                        &state.game_state.collision_system,
                    );
            }
        }
    }

    /// Advances the maze generation animation and uploads new geometry when complete.
    pub fn handle_maze_generation(&mut self) {
        if let Some(state) = self.state.as_mut() {
            let renderer = &mut state.wgpu_renderer.loading_screen_renderer;

            // Calculate update timing
            let speed = if renderer.generator.fast_mode {
                Duration::from_millis(10) / 20
            } else {
                Duration::from_millis(10)
            };

            // Skip if not time to update or already complete
            if renderer.last_update.elapsed() < speed || renderer.generator.is_complete() {
                return;
            }

            // Process animation steps
            let steps = if renderer.generator.fast_mode {
                300
            } else {
                100
            };
            for _ in 0..steps {
                if !renderer.generator.step() {
                    break;
                }
            }

            // Report progress
            let (current, total) = renderer.generator.get_progress();

            if current % 50 == 0 || renderer.generator.is_complete() {
                println!(
                    "Progress: {}/{} ({:.1}%)",
                    current,
                    total,
                    (current as f32 * 100.0 / total.max(1) as f32)
                );
            }
            if renderer.generator.is_complete() && state.game_state.maze_path.is_none() {
                println!("Maze generation complete! Saving to file...");

                // Play completion sound
                state
                    .game_state
                    .audio_manager
                    .complete()
                    .expect("Failed to play complete sound!");

                // Handle completion
                if renderer.generator.is_complete() {
                    println!("Maze generation complete! Saving to file...");
                    let maze_lock = renderer.maze.lock().unwrap();
                    state.game_state.maze_path = maze_lock.save_to_file().map_or_else(
                        |err| {
                            eprintln!("Failed to save maze: {}", err);
                            std::process::exit(1);
                        },
                        Some,
                    );

                    // Generate geometry if maze was saved successfully
                    if let Some(maze_path) = &state.game_state.maze_path {
                        let (maze_grid, exit_cell) = parse_maze_file(maze_path.to_str().unwrap());
                        let (mut floor_vertices, exit_position) =
                            Vertex::create_floor_vertices(&maze_grid, exit_cell);

                        state.wgpu_renderer.game_renderer.exit_position = Some(exit_position);

                        floor_vertices.append(&mut Vertex::create_wall_vertices(&maze_grid));

                        state.wgpu_renderer.game_renderer.vertex_buffer = state
                            .wgpu_renderer
                            .device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("Combined Vertex Buffer"),
                                contents: bytemuck::cast_slice(&floor_vertices),
                                usage: wgpu::BufferUsages::VERTEX,
                            });

                        if let Some(exit_cell_position) = exit_cell {
                            state.game_state.exit_cell = Some(exit_cell_position);
                            state.game_state.enemy = place_enemy_standard(
                                maze_to_world(
                                    &exit_cell_position,
                                    maze_lock.get_dimensions(),
                                    30.0,
                                ),
                                state.game_state.player.position,
                                state.game_state.game_ui.level,
                                |from, to| {
                                    state
                                        .game_state
                                        .collision_system
                                        .cylinder_intersects_geometry(from, to, 5.0)
                                },
                            );
                        }

                        state
                            .game_state
                            .collision_system
                            .build_from_maze(&maze_grid);

                        // Spawn the player at the bottom-left corner of the maze
                        state.game_state.player.spawn_at_maze_entrance(&maze_grid);
                    }
                }
            }
        }
    }
}

impl ApplicationHandler for App {
    /// Called when the application is resumed; creates the window and initializes state.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = match event_loop.create_window(Window::default_attributes()) {
            Ok(window) => window,
            Err(err) => {
                panic!("Failed to create window: {}", err);
            }
        };
        pollster::block_on(self.set_window(window));
    }

    /// Handles device-level events, such as mouse motion for camera control.
    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if let Some(state) = self.state.as_mut() {
                if let Some(window) = &mut self.window {
                    if state.game_state.current_screen == CurrentScreen::Game
                        && state.game_state.capture_mouse
                    {
                        state.game_state.player.mouse_movement(delta.0, delta.1);
                    }
                    state.triage_mouse(window);
                }
            }
        }
    }

    /// Handles all window-level events, including input, resizing, redraws, and close requests.
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let state = match self.state.as_mut() {
            Some(state) => state,
            None => {
                panic!("State not initialized");
            }
        };
        let window = match self.window.as_ref() {
            Some(window) => window,
            None => {
                panic!("Window not initialized");
            }
        };

        state.egui_renderer.handle_input(window, &event);

        // If in pause menu, pass all input events to the pause menu first
        let pause_action = if state.game_state.current_screen == CurrentScreen::Pause
            && state.pause_menu.is_visible()
        {
            state.pause_menu.handle_input(&event);
            state.pause_menu.get_last_action()
        } else {
            crate::ui::pause_menu::PauseMenuAction::None
        };

        // Handle pause menu actions
        match pause_action {
            crate::ui::pause_menu::PauseMenuAction::Resume => {
                state.game_state.current_screen = CurrentScreen::Game;
                state.game_state.game_ui.resume_timer();
                state.game_state.enemy.pathfinder.locked = false;
                state.game_state.capture_mouse = true;
                // Explicitly hide the pause menu
                state.pause_menu.hide();
            }
            crate::ui::pause_menu::PauseMenuAction::QuitToMenu => {
                event_loop.exit();
            }
            crate::ui::pause_menu::PauseMenuAction::Settings => {
                // "Restart Run" button - trigger the same sequence as game over restart
                state.game_state.current_screen = CurrentScreen::NewGame;
                state.game_state.capture_mouse = true;
            }
            crate::ui::pause_menu::PauseMenuAction::Restart => {
                // "Quit to Lobby" button - for now, do nothing (or could exit to menu)
                // This could be changed to exit the game or go to a lobby screen
            }
            _ => {}
        }

        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }

            WindowEvent::Resized(new_size) => {
                self.handle_resized(new_size.width, new_size.height);
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: key,
                        state: key_state,
                        repeat: false,
                        ..
                    },
                ..
            } => {
                if let Some(game_key) = winit_key_to_game_key(&key) {
                    match key_state {
                        ElementState::Pressed => {
                            state.key_state.press_key(game_key);

                            // Handle non-movement keys immediately on press
                            match game_key {
                                GameKey::Quit => event_loop.exit(),
                                GameKey::ToggleSliders => {
                                    state.ui.show_sliders = !state.ui.show_sliders
                                }
                                GameKey::ToggleBoundingBoxes => {
                                    state
                                        .wgpu_renderer
                                        .game_renderer
                                        .debug_renderer
                                        .debug_render_bounding_boxes = !state
                                        .wgpu_renderer
                                        .game_renderer
                                        .debug_renderer
                                        .debug_render_bounding_boxes;
                                }
                                GameKey::Escape => {
                                    match state.game_state.current_screen {
                                        CurrentScreen::Game => {
                                            // Enter pause menu
                                            state.game_state.current_screen = CurrentScreen::Pause;
                                            // Pause timer
                                            state.game_state.game_ui.pause_timer();
                                            // Lock enemy movement
                                            state.game_state.enemy.pathfinder.locked = true;
                                            // Unlock cursor
                                            state.game_state.capture_mouse = false;
                                        }
                                        CurrentScreen::Pause => {
                                            // Resume game
                                            state.game_state.current_screen = CurrentScreen::Game;
                                            state.game_state.game_ui.resume_timer();
                                            // Unlock enemy movement
                                            state.game_state.enemy.pathfinder.locked = false;
                                            // Lock cursor
                                            state.game_state.capture_mouse = true;
                                        }
                                        _ => {
                                            // For all other screens, just toggle cursor lock
                                            state.game_state.capture_mouse =
                                                !state.game_state.capture_mouse;
                                        }
                                    }
                                }

                                _ => {} // Movement keys are handled in process_movement
                            }
                        }
                        ElementState::Released => {
                            state.key_state.release_key(game_key);
                        }
                    }
                }
            }

            WindowEvent::MouseInput {
                state: mouse_state,
                button,
                ..
            } => match mouse_state {
                ElementState::Pressed => {
                    if let Some(app_state) = self.state.as_mut() {
                        match button {
                            MouseButton::Left => {
                                // If on title screen, transition to loading
                                if app_state.game_state.current_screen == CurrentScreen::Title {
                                    app_state.game_state.current_screen = CurrentScreen::Loading;
                                    // Optionally, lock mouse here if needed
                                    app_state.game_state.capture_mouse = true;
                                    // Hide the overlay text
                                    if let Some(buf) = app_state
                                        .text_renderer
                                        .text_buffers
                                        .get_mut("title_mirador_overlay")
                                    {
                                        buf.visible = false;
                                    }
                                    // Hide the subtitle overlay
                                    if let Some(buf) = app_state
                                        .text_renderer
                                        .text_buffers
                                        .get_mut("title_subtitle_overlay")
                                    {
                                        buf.visible = false;
                                    }
                                    return;
                                }
                                app_state.key_state.press_key(GameKey::MouseButtonLeft);
                            }
                            MouseButton::Right => {
                                app_state.key_state.press_key(GameKey::MouseButtonRight);
                            }
                            _ => {}
                        }
                    }
                }
                ElementState::Released => {
                    if let Some(app_state) = self.state.as_mut() {
                        match button {
                            MouseButton::Left => {
                                app_state.key_state.release_key(GameKey::MouseButtonLeft);
                            }
                            MouseButton::Right => {
                                app_state.key_state.release_key(GameKey::MouseButtonRight);
                            }
                            _ => {}
                        }
                    }
                }
            },

            WindowEvent::RedrawRequested => {
                let current_time = Instant::now();
                self.handle_frame_timing(current_time);
                self.handle_redraw();
            }

            _ => {
                // Handle any other events that weren't caught above
            }
        }
    }
}
