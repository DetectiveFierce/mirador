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
use crate::game::player::Player;
use crate::game::{CurrentScreen, TimerConfig};
use crate::maze::maze_animation::LoadingRenderer;
use crate::maze::parse_maze_file;
use crate::renderer::text::TextRenderer;
use crate::renderer::vertex::Vertex;
use crate::{
    game::{
        GameState,
        keys::{GameKey, KeyState, winit_key_to_game_key},
    },
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
        text_renderer.initialize_game_ui(&game_state.game_ui, width, height);

        Self {
            wgpu_renderer,
            egui_renderer,
            ui: UiState::new(),
            game_state,
            key_state: KeyState::new(),
            text_renderer,
            start_time: Instant::now(),
            elapsed_time: Duration::ZERO,
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
            .update_uniforms(&self.wgpu_renderer.queue, [0.75, 0.75], [1.75, 1.75]);
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
            .update_loading_bar(&self.wgpu_renderer.queue, progress);

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
    pub fn update_game_ui(&mut self) {
        // Start timer when game begins (not on title screen)
        if self.game_state.current_screen == CurrentScreen::Game
            && self.game_state.game_ui.timer.is_none()
        {
            // Configure timer with custom settings
            let timer_config = TimerConfig {
                duration: Duration::from_secs(60),
                warning_threshold: Duration::from_secs(30),
                critical_threshold: Duration::from_secs(15),
                normal_color: Color::rgb(100, 255, 100),
                warning_color: Color::rgb(255, 255, 100),
                critical_color: Color::rgb(255, 100, 100),
            };
            self.game_state.start_game_timer(Some(timer_config));
        }

        // Update UI and check if timer expired
        let timer_expired = self
            .text_renderer
            .update_game_ui(&mut self.game_state.game_ui);

        if timer_expired {
            // Handle timer expiration - you can add game over logic here
            println!("Timer expired! Game over.");
            self.game_state.current_screen = CurrentScreen::GameOver;
            // Example: self.game_state.game_over = true;
        }

        // Update level display if needed (example usage)
        // You can call this when the level changes:
        // self.text_renderer.set_level(new_level);

        // Update score display if needed (example usage)
        // You can call this when the score changes:
        // self.text_renderer.set_score(new_score);
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
            state.handle_loading_screen(window);
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
        state.update_game_ui(); // Updated to use the new method
        state.update_ui(window);

        // Prepare rendering commands
        let mut encoder = state
            .wgpu_renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        // Update canvas surface
        let (surface_view, screen_descriptor, surface_texture) =
            match state.wgpu_renderer.update_canvas(
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

        // Render UI
        state.egui_renderer.end_frame_and_draw(
            &state.wgpu_renderer.device,
            &state.wgpu_renderer.queue,
            &mut encoder,
            window,
            &surface_view,
            screen_descriptor,
        );

        window.request_redraw();

        // Submit commands and present
        state.wgpu_renderer.queue.submit(Some(encoder.finish()));
        surface_texture.present();

        // Handle title screen animation if needed
        if state.game_state.current_screen == CurrentScreen::Loading {
            state.game_state.game_ui.stop_timer();
            self.handle_maze_generation();
        } else if state.game_state.current_screen == CurrentScreen::NewGame {
            state.text_renderer.hide_game_over_display();
            self.new_level(true);
        } else if state.game_state.current_screen == CurrentScreen::Game
            && state.game_state.player.current_cell == state.game_state.exit_cell
        {
            self.new_level(false);
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
        state.game_state.player = Player::new();

        if game_over {
            state.game_state.game_ui.level = 0;
            state.game_state.game_ui.timer = Some(GameTimer::new(TimerConfig::default()));
        } else {
            state
                .game_state
                .set_level(&mut state.text_renderer, state.game_state.game_ui.level + 1);
            if let Some(timer) = &mut state.game_state.game_ui.timer {
                let mut time_back = Duration::ZERO;
                if timer.prev_time > timer.get_remaining_time() {
                    time_back = (timer.prev_time - timer.get_remaining_time()) / 2;
                } else if timer.prev_time < timer.get_remaining_time() {
                    time_back = (timer.get_remaining_time() - timer.prev_time) / 2;
                }

                timer.prev_time = timer.get_remaining_time();
                timer.add_time(time_back);
            }
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
            let steps = if renderer.generator.fast_mode { 30 } else { 10 };
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
                            state.game_state.exit_cell = exit_cell_position;
                        }

                        state
                            .game_state
                            .collision_system
                            .build_from_maze(&maze_grid);
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
                                    state.game_state.capture_mouse =
                                        !state.game_state.capture_mouse;
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

            WindowEvent::MouseInput { state, button, .. } => match state {
                ElementState::Pressed => {
                    if let Some(app_state) = self.state.as_mut() {
                        match button {
                            MouseButton::Left => {
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

            _ => (),
        }
    }
}
