//! Event handler module for Mirador.
//!
//! Contains the App struct and its event handling logic.

use crate::app::app_state::AppState;
use crate::renderer::loading_renderer::LoadingRenderer;
use std::{sync::Arc, time::Instant};
use wgpu;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, DeviceId, ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

#[derive(Default)]
pub struct App {
    pub instance: wgpu::Instance,
    pub state: Option<AppState>,
    pub window: Option<Arc<Window>>,
}

impl App {
    pub fn new() -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        Self {
            instance,
            state: None,
            window: None,
        }
    }

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
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = match event_loop.create_window(Window::default_attributes()) {
            Ok(window) => window,
            Err(err) => {
                panic!("Failed to create window: {}", err);
            }
        };
        pollster::block_on(self.set_window(window));
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if let Some(state) = self.state.as_mut() {
                if let Some(window) = &mut self.window {
                    if state.game_state.current_screen == crate::game::CurrentScreen::Game
                        && state.game_state.capture_mouse
                    {
                        state.game_state.player.mouse_movement(delta.0, delta.1);
                    }
                    state.triage_mouse(window);
                }
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let state = match self.state.as_mut() {
            Some(state) => state,
            None => {
                panic!("State not initialized");
            }
        };

        // If in pause menu, pass all input events to the pause menu first
        let pause_action = if state.game_state.current_screen == crate::game::CurrentScreen::Pause
            && state.pause_menu.is_visible()
        {
            state.pause_menu.handle_input(&event);
            state.pause_menu.get_last_action()
        } else {
            crate::renderer::ui::pause_menu::PauseMenuAction::None
        };

        // Handle pause menu actions
        match pause_action {
            crate::renderer::ui::pause_menu::PauseMenuAction::Resume => {
                // Return to previous screen or default to Game
                if let Some(previous_screen) = state.game_state.previous_screen {
                    state.game_state.current_screen = previous_screen;
                    state.game_state.previous_screen = None;

                    match previous_screen {
                        crate::game::CurrentScreen::Game => {
                            // Resume game
                            state.game_state.game_ui.resume_timer();
                            // Unlock enemy movement
                            state.game_state.enemy.pathfinder.locked = false;
                            // Lock cursor
                            state.game_state.capture_mouse = true;
                        }
                        crate::game::CurrentScreen::Title => {
                            // Return to title screen - cursor should be unlocked
                            state.game_state.capture_mouse = false;
                        }
                        _ => {
                            // For other screens, just unlock cursor
                            state.game_state.capture_mouse = false;
                        }
                    }
                } else {
                    // Fallback: return to game
                    state.game_state.current_screen = crate::game::CurrentScreen::Game;
                    state.game_state.game_ui.resume_timer();
                    state.game_state.enemy.pathfinder.locked = false;
                    state.game_state.capture_mouse = true;
                }
                state.pause_menu.hide();
            }
            crate::renderer::ui::pause_menu::PauseMenuAction::Settings => {
                // Restart current run - handle this after the match to avoid borrow issues
                state.game_state.current_screen = crate::game::CurrentScreen::NewGame;
                state.game_state.previous_screen = None; // Clear previous screen
                state.pause_menu.hide();

                // Hide title screen elements when transitioning away from title
                if let Some(buf) = state
                    .text_renderer
                    .text_buffers
                    .get_mut("title_mirador_overlay")
                {
                    buf.visible = false;
                }
                if let Some(buf) = state
                    .text_renderer
                    .text_buffers
                    .get_mut("title_subtitle_overlay")
                {
                    buf.visible = false;
                }
            }
            crate::renderer::ui::pause_menu::PauseMenuAction::ToggleTestMode => {
                // Toggle between test mode and normal mode
                if state.game_state.is_test_mode {
                    // Currently in test mode, switch to normal mode (loading screen)
                    state.game_state.is_test_mode = false;
                    state.game_state.current_screen = crate::game::CurrentScreen::Loading;
                    state.game_state.previous_screen = None; // Clear previous screen
                    state.pause_menu.hide();

                    // Recapture mouse when exiting test mode
                    state.game_state.capture_mouse = true;

                    // Reset to normal game state
                    state.game_state.maze_path = None;
                    state.wgpu_renderer.loading_screen_renderer = LoadingRenderer::new(
                        &state.wgpu_renderer.device,
                        &state.wgpu_renderer.surface_config,
                    );
                    // Clear previous level state
                    state.game_state.player = crate::game::player::Player::new();
                    state.game_state.enemy.pathfinder.position = [0.0, 30.0, 0.0];
                    state.game_state.enemy.pathfinder.locked = true;
                    state.game_state.exit_cell = None;

                    // Reset score and level to starting values
                    state.game_state.set_score(0);
                    state.game_state.set_level(1);

                    // Stop and reset timer with normal game configuration
                    state.game_state.game_ui.timer = None; // Clear the test timer
                    // The timer will be properly initialized when the game starts (in update_game_ui)

                    // Hide title screen elements when transitioning away from title
                    if let Some(buf) = state
                        .text_renderer
                        .text_buffers
                        .get_mut("title_mirador_overlay")
                    {
                        buf.visible = false;
                    }
                    if let Some(buf) = state
                        .text_renderer
                        .text_buffers
                        .get_mut("title_subtitle_overlay")
                    {
                        buf.visible = false;
                    }
                } else {
                    // Currently in normal mode, switch to test mode
                    state.game_state.is_test_mode = true;
                    state.game_state.current_screen = crate::game::CurrentScreen::Game;
                    state.game_state.previous_screen = None; // Clear previous screen
                    state.pause_menu.hide();

                    // Recapture mouse for test mode
                    state.game_state.capture_mouse = true;

                    // Set up test environment
                    crate::test_mode::setup_test_environment(
                        &mut state.game_state,
                        &mut state.wgpu_renderer,
                    );
                    // Set a dummy maze path to prevent re-entry
                    state.game_state.maze_path = Some(std::path::PathBuf::from("test_mode"));
                }

                // Hide title screen elements when transitioning away from title
                if let Some(buf) = state
                    .text_renderer
                    .text_buffers
                    .get_mut("title_mirador_overlay")
                {
                    buf.visible = false;
                }
                if let Some(buf) = state
                    .text_renderer
                    .text_buffers
                    .get_mut("title_subtitle_overlay")
                {
                    buf.visible = false;
                }
            }
            crate::renderer::ui::pause_menu::PauseMenuAction::Restart => {
                // Quit to lobby (title screen)
                state.game_state.current_screen = crate::game::CurrentScreen::Title;
                state.game_state.previous_screen = None; // Clear previous screen
                state.pause_menu.hide();
                // Reset game state
                state.game_state = crate::game::GameState::new();
                // Show title screen elements
                if let Some(buf) = state
                    .text_renderer
                    .text_buffers
                    .get_mut("title_mirador_overlay")
                {
                    buf.visible = true;
                }
                if let Some(buf) = state
                    .text_renderer
                    .text_buffers
                    .get_mut("title_subtitle_overlay")
                {
                    buf.visible = true;
                }
            }
            crate::renderer::ui::pause_menu::PauseMenuAction::QuitToMenu => {
                // Quit the application
                std::process::exit(0);
            }
            crate::renderer::ui::pause_menu::PauseMenuAction::None => {}
        }

        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");

                // Ensure all GPU operations are complete before shutting down
                if let Some(state) = &mut self.state {
                    state.wgpu_renderer.cleanup();
                }

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
                if let Some(game_key) = crate::game::keys::winit_key_to_game_key(&key) {
                    match key_state {
                        ElementState::Pressed => {
                            state.key_state.press_key(game_key);

                            // Handle non-movement keys immediately on press
                            match game_key {
                                crate::game::keys::GameKey::Quit => event_loop.exit(),
                                crate::game::keys::GameKey::ToggleBoundingBoxes => {
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
                                crate::game::keys::GameKey::Escape => {
                                    match state.game_state.current_screen {
                                        crate::game::CurrentScreen::Game => {
                                            // Enter pause menu
                                            state.game_state.previous_screen =
                                                Some(crate::game::CurrentScreen::Game);
                                            state.game_state.current_screen =
                                                crate::game::CurrentScreen::Pause;
                                            // Pause timer
                                            state.game_state.game_ui.pause_timer();
                                            // Lock enemy movement
                                            state.game_state.enemy.pathfinder.locked = true;
                                            // Unlock cursor
                                            state.game_state.capture_mouse = false;
                                            // Show pause menu with current test mode state
                                            state.pause_menu.show(state.game_state.is_test_mode);
                                        }
                                        crate::game::CurrentScreen::Pause => {
                                            // Return to previous screen
                                            if let Some(previous_screen) =
                                                state.game_state.previous_screen
                                            {
                                                state.game_state.current_screen = previous_screen;
                                                state.game_state.previous_screen = None;

                                                match previous_screen {
                                                    crate::game::CurrentScreen::Game => {
                                                        // Resume game
                                                        state.game_state.game_ui.resume_timer();
                                                        // Unlock enemy movement
                                                        state.game_state.enemy.pathfinder.locked =
                                                            false;
                                                        // Lock cursor
                                                        state.game_state.capture_mouse = true;
                                                    }
                                                    crate::game::CurrentScreen::Title => {
                                                        // Return to title screen - cursor should be unlocked
                                                        state.game_state.capture_mouse = false;
                                                    }
                                                    _ => {
                                                        // For other screens, just unlock cursor
                                                        state.game_state.capture_mouse = false;
                                                    }
                                                }
                                            } else {
                                                // Fallback: return to title screen
                                                state.game_state.current_screen =
                                                    crate::game::CurrentScreen::Title;
                                                state.game_state.capture_mouse = false;
                                            }
                                        }
                                        crate::game::CurrentScreen::Title => {
                                            // Enter pause menu from title screen
                                            state.game_state.previous_screen =
                                                Some(crate::game::CurrentScreen::Title);
                                            state.game_state.current_screen =
                                                crate::game::CurrentScreen::Pause;
                                            // Unlock cursor to allow menu interaction
                                            state.game_state.capture_mouse = false;
                                            // Show pause menu with current test mode state
                                            state.pause_menu.show(state.game_state.is_test_mode);
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
                                if app_state.game_state.current_screen
                                    == crate::game::CurrentScreen::Title
                                {
                                    app_state.game_state.current_screen =
                                        crate::game::CurrentScreen::Loading;
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
                                app_state
                                    .key_state
                                    .press_key(crate::game::keys::GameKey::MouseButtonLeft);
                            }
                            MouseButton::Right => {
                                app_state
                                    .key_state
                                    .press_key(crate::game::keys::GameKey::MouseButtonRight);
                            }
                            _ => {}
                        }
                    }
                }
                ElementState::Released => {
                    if let Some(app_state) = self.state.as_mut() {
                        match button {
                            MouseButton::Left => {
                                app_state
                                    .key_state
                                    .release_key(crate::game::keys::GameKey::MouseButtonLeft);
                            }
                            MouseButton::Right => {
                                app_state
                                    .key_state
                                    .release_key(crate::game::keys::GameKey::MouseButtonRight);
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
