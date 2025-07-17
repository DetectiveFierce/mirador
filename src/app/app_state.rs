//! AppState module for Mirador.
//!
//! This module defines the [`AppState`] struct, which holds all state required for a running
//! game session, including rendering backends, UI state, game logic, and input state.

use crate::game::enemy::Enemy;
use crate::game::{self, CurrentScreen, GameState, TimerConfig, keys::KeyState};
use crate::renderer::text::TextRenderer;
use crate::renderer::wgpu_lib::WgpuRenderer;
use glyphon::Color;
use std::time::Duration;
use std::time::Instant;
use wgpu;
use winit::window::Window;

/// Holds all state required for a running Mirador game session.
///
/// This includes rendering backends, UI state, game logic, and input state.
pub struct AppState {
    /// The WGPU renderer for the main game and background.
    pub wgpu_renderer: WgpuRenderer,
    /// The main game state (player, timing, maze, etc.).
    pub game_state: GameState,
    /// The current input state (pressed keys, etc.).
    pub key_state: KeyState,
    /// The text renderer for all game UI text elements.
    pub text_renderer: TextRenderer,
    pub start_time: Instant,
    pub elapsed_time: Duration,
    pub pause_menu: crate::renderer::ui::pause_menu::PauseMenu,
    pub game_over_start_time: Option<Instant>,
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

        let pause_menu = crate::renderer::ui::pause_menu::PauseMenu::new(
            &wgpu_renderer.device,
            &wgpu_renderer.queue,
            wgpu_renderer.surface_config.format,
            window,
        );

        // Add big boldMirador' text in the top right for the title screen
        let width = wgpu_renderer.surface_config.width as f32;
        let height = wgpu_renderer.surface_config.height as f32;
        let mirador_style = crate::renderer::text::TextStyle {
            font_family: "HankenGrotesk".to_string(),
            font_size: 125.0,
            line_height: 150.0,
            color: Color::rgb(58, 53, 70), // #3
            weight: glyphon::Weight::BOLD,
            style: glyphon::Style::Normal,
        };
        // Estimate text width for right alignment
        let text_width = 620.0; // Conservative estimate for large text
        let text_height = 1500.0; // Let's assume a large height for the title
        let mirador_position = crate::renderer::text::TextPosition {
            x: width - text_width - 200.0, // 20px margin from right
            y: 100.0,                      // 100px margin from top
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
            color: Color::rgb(58, 53, 70), // #3
            weight: glyphon::Weight::MEDIUM,
            style: glyphon::Style::Normal,
        };
        let subtitle_text = "Click anywhere to get lost.";
        let subtitle_text_height = 72.0;
        let subtitle_position = crate::renderer::text::TextPosition {
            x: mirador_position.x - 1200.0,
            y: height + 1000.0,
            max_width: Some(text_width),
            max_height: Some(subtitle_text_height),
        };
        text_renderer.create_text_buffer(
            "title_subtitle_overlay",
            subtitle_text,
            Some(subtitle_style),
            Some(subtitle_position),
        );

        Self {
            wgpu_renderer,
            game_state,
            key_state: KeyState::default(),
            text_renderer,
            start_time: Instant::now(),
            elapsed_time: Duration::default(),
            pause_menu,
            game_over_start_time: None,
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
                eprintln!("Failed to lock cursor: {}, {}", e, e);
            }
            window.set_cursor_visible(false);
            let window_size = window.inner_size().to_logical::<f64>(window.scale_factor());

            let center_x = window_size.width / 2.0;
            let center_y = window_size.height / 2.0;
            if let Err(e) =
                window.set_cursor_position(winit::dpi::LogicalPosition::new(center_x, center_y))
            {
                eprintln!("Failed to center cursor: {}, {}", e, e);
            }
        } else if !self.game_state.capture_mouse {
            if let Err(e) = window.set_cursor_grab(winit::window::CursorGrabMode::None) {
                eprintln!("Failed to unlock cursor: {}, {}", e, e);
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
            // HIDE the timer text buffer always (replaced by bar)
            if let Some(buffer) = self.text_renderer.text_buffers.get_mut("main_timer") {
                buffer.visible = false;
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
            self.game_state.stop_game_timer();
            self.game_state.current_screen = CurrentScreen::GameOver;
            self.game_over_start_time = Some(Instant::now());
        }

        if self.game_state.enemy.pathfinder.reached_player {
            self.game_state.stop_game_timer();
            self.game_state.current_screen = CurrentScreen::GameOver;
            self.game_state.enemy = Enemy::new([-0.5, 30.0, 0.0], 150.0);
            self.game_state.enemy.pathfinder.reached_player = false;
        }

        // Show/hide game over display based on current screen
        if self.game_state.current_screen == CurrentScreen::GameOver {
            self.text_renderer.show_game_over_display();
        } else {
            self.text_renderer.hide_game_over_display();
        }
    }
}
