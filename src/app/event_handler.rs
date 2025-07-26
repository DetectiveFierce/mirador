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

/// Main application struct that manages the game lifecycle and event handling.
///
/// This struct implements the [`ApplicationHandler`] trait to handle all window and device events.
/// It manages the WGPU instance, application state, and window lifecycle.
///
/// # Fields
/// - `instance`: The WGPU instance for graphics operations
/// - `state`: Optional application state (None until window is created)
/// - `window`: Optional window reference (None until window is created)
///
/// # Lifecycle
/// 1. Created with `App::new()` - initializes WGPU instance
/// 2. Window is set via `set_window()` - creates surface and application state
/// 3. Events are handled via `ApplicationHandler` trait methods
/// 4. Application runs until window is closed
#[derive(Default)]
pub struct App {
    /// The WGPU instance for graphics operations.
    pub instance: wgpu::Instance,
    /// The current application state, None until initialized.
    pub state: Option<AppState>,
    /// The application window, None until set.
    pub window: Option<Arc<Window>>,
}

impl App {
    /// Creates a new [`App`] instance with default WGPU configuration.
    ///
    /// This initializes the WGPU instance with default settings. The application
    /// state and window will be None until `set_window()` is called.
    ///
    /// # Returns
    /// A new [`App`] instance ready for window creation.
    ///
    /// # Example
    /// ```
    /// let app = App::new();
    /// ```
    pub fn new() -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        Self {
            instance,
            state: None,
            window: None,
        }
    }

    /// Asynchronously sets up the application window and initializes all game systems.
    ///
    /// This method creates the window, WGPU surface, and initializes all application
    /// state including renderers, audio systems, and game state. This is typically
    /// called when the application is resumed or when the window is first created.
    ///
    /// # Arguments
    /// - `window`: The window to associate with this application
    ///
    /// # Initialization Steps
    /// 1. Sets window size to 1360x768
    /// 2. Creates WGPU surface from the window
    /// 3. Initializes [`AppState`] with all renderers and game systems
    /// 4. Stores window and state references
    ///
    /// # Panics
    /// - If surface creation fails
    /// - If [`AppState`] initialization fails
    ///
    /// # Example
    /// ```ignore
    /// let window = event_loop.create_window(Window::default_attributes())?;
    /// app.set_window(window).await;
    /// ```
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

    /// Handles window resize events and updates all rendering systems.
    ///
    /// This method is called when the window is resized. It updates the WGPU surface
    /// configuration and resizes all UI components to match the new window dimensions.
    ///
    /// # Arguments
    /// - `width`: New window width in pixels
    /// - `height`: New window height in pixels
    ///
    /// # Behavior
    /// - Only processes resize if both dimensions are greater than 0
    /// - Updates WGPU surface configuration
    /// - Resizes pause menu and upgrade menu UI components
    /// - Logs error and backtrace if state is not initialized
    ///
    /// # Safety
    /// This method safely handles cases where the application state hasn't been
    /// initialized yet, logging errors instead of panicking.
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
            // Ensure pause menu and upgrade menu resize with the window
            use glyphon::Resolution;
            let resolution = Resolution { width, height };
            state
                .pause_menu
                .resize(&state.wgpu_renderer.queue, resolution);
            state
                .upgrade_menu
                .resize(&state.wgpu_renderer.queue, resolution);
        }
    }
}

impl ApplicationHandler for App {
    /// Handles application resume events by creating a new window.
    ///
    /// This method is called when the application is resumed (e.g., when switching
    /// back to the application on mobile devices). It creates a new window and
    /// initializes the application state.
    ///
    /// # Arguments
    /// - `event_loop`: The active event loop for creating the window
    ///
    /// # Panics
    /// - If window creation fails
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = match event_loop.create_window(Window::default_attributes()) {
            Ok(window) => window,
            Err(err) => {
                panic!("Failed to create window: {}", err);
            }
        };
        pollster::block_on(self.set_window(window));
    }

    /// Handles device events, primarily mouse movement for camera control.
    ///
    /// This method processes device events, with special handling for mouse movement
    /// to control the player's camera orientation. Mouse movement is only processed
    /// when the game is active and mouse capture is enabled.
    ///
    /// # Arguments
    /// - `_event_loop`: The active event loop (unused)
    /// - `_device_id`: The device ID (unused)
    /// - `event`: The device event to process
    ///
    /// # Mouse Movement Handling
    /// - Only processes mouse movement when in Game or ExitReached screens
    /// - Requires mouse capture to be enabled
    /// - Updates player camera orientation based on mouse delta
    /// - Calls `triage_mouse()` to handle cursor state
    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if let Some(state) = self.state.as_mut() {
                if let Some(window) = &mut self.window {
                    if (state.game_state.current_screen == crate::game::CurrentScreen::Game
                        || state.game_state.current_screen
                            == crate::game::CurrentScreen::ExitReached)
                        && state.game_state.capture_mouse
                    {
                        // Allow mouse movement in both Game and ExitReached screens
                        state.game_state.player.mouse_movement(delta.0, delta.1);
                    }
                    state.triage_mouse(window);
                }
            }
        }
    }

    /// Handles window events including input, resize, and close requests.
    ///
    /// This is the main event processing method that handles all window-related events.
    /// It processes keyboard input, mouse input, window resize, close requests, and
    /// redraw requests. The method also manages game state transitions and UI interactions.
    ///
    /// # Arguments
    /// - `event_loop`: The active event loop
    /// - `_`: Window ID (unused)
    /// - `event`: The window event to process
    ///
    /// # Event Types Handled
    /// - **CloseRequested**: Initiates application shutdown
    /// - **Resized**: Calls `handle_resized()` to update rendering
    /// - **KeyboardInput**: Processes game controls and UI navigation
    /// - **MouseInput**: Handles mouse button presses for UI interaction
    /// - **RedrawRequested**: Triggers frame rendering and game updates
    ///
    /// # Game State Management
    /// - Manages transitions between different game screens
    /// - Handles pause menu interactions and state
    /// - Processes upgrade menu visibility and interactions
    /// - Manages mouse capture state based on current screen
    ///
    /// # Panics
    /// - If application state is not initialized
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
            state
                .pause_menu
                .handle_input(&event, &mut state.game_state.audio_manager);
            state.pause_menu.get_last_action()
        } else {
            crate::renderer::ui::pause_menu::PauseMenuAction::None
        };

        // If in upgrade menu, pass all input events to the upgrade menu first
        if state.game_state.current_screen == crate::game::CurrentScreen::UpgradeMenu
            && state.upgrade_menu.is_visible()
        {
            state
                .upgrade_menu
                .handle_input(&event, &mut state.game_state);
        }

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
                            if !state.game_state.is_test_mode {
                                // Only resume timer in normal mode (enemy locking is handled in update loop)
                                state.game_state.game_ui.resume_timer();
                            }
                            // In test mode, keep timer paused (enemy locking is handled in update loop)
                            // Lock cursor
                            state.game_state.capture_mouse = true;
                            // Restore game audio volumes
                            state
                                .game_state
                                .audio_manager
                                .set_game_volumes()
                                .expect("Failed to set game volumes");
                        }
                        crate::game::CurrentScreen::Title => {
                            // Return to title screen - cursor should be unlocked
                            state.game_state.capture_mouse = false;
                            // Set title screen audio volumes
                            state
                                .game_state
                                .audio_manager
                                .set_title_screen_volumes()
                                .expect("Failed to set title screen volumes");
                        }
                        _ => {
                            // For other screens, just unlock cursor
                            state.game_state.capture_mouse = false;
                        }
                    }
                } else {
                    // Fallback: return to game
                    state.game_state.current_screen = crate::game::CurrentScreen::Game;
                    if !state.game_state.is_test_mode {
                        // Only resume timer in normal mode (enemy locking is handled in update loop)
                        state.game_state.game_ui.resume_timer();
                    }
                    // In test mode, keep timer paused (enemy locking is handled in update loop)
                    state.game_state.capture_mouse = true;
                    // Restore game audio volumes
                    state
                        .game_state
                        .audio_manager
                        .set_game_volumes()
                        .expect("Failed to set game volumes");
                }
                state.pause_menu.hide();
            }
            crate::renderer::ui::pause_menu::PauseMenuAction::Restart => {
                // Restart current run - handle this after the match to avoid borrow issues
                state.game_state.current_screen = crate::game::CurrentScreen::NewGame;
                state.game_state.previous_screen = None; // Clear previous screen
                state.pause_menu.hide();
                // Restore game audio volumes for new game
                state
                    .game_state
                    .audio_manager
                    .set_game_volumes()
                    .expect("Failed to set game volumes");
                // Ensure mouse is captured for the new game
                state.game_state.capture_mouse = true;
                // Apply mouse capture immediately
                if let Some(window) = self.window.as_ref() {
                    state.triage_mouse(window);
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
            crate::renderer::ui::pause_menu::PauseMenuAction::ToggleTestMode => {
                // Toggle between test mode and normal mode
                if state.game_state.is_test_mode {
                    // Currently in test mode, switch to normal mode (loading screen)
                    state.game_state.is_test_mode = false;
                    state.game_state.current_screen = crate::game::CurrentScreen::Loading;
                    state.game_state.previous_screen = None; // Clear previous screen
                    state.pause_menu.hide();
                    // Restore game audio volumes for normal mode
                    state
                        .game_state
                        .audio_manager
                        .set_game_volumes()
                        .expect("Failed to set game volumes");

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
            crate::renderer::ui::pause_menu::PauseMenuAction::QuitToMenu => {
                // Quit to lobby (title screen)
                state.game_state.current_screen = crate::game::CurrentScreen::Title;
                state.game_state.previous_screen = None; // Clear previous screen
                state.pause_menu.hide();
                // Reset game state
                state.game_state = crate::game::GameState::new();
                // Reset loading screen renderer to ensure new maze generation
                state.wgpu_renderer.loading_screen_renderer = LoadingRenderer::new(
                    &state.wgpu_renderer.device,
                    &state.wgpu_renderer.surface_config,
                );
                // Set title screen audio volumes
                state
                    .game_state
                    .audio_manager
                    .set_title_screen_volumes()
                    .expect("Failed to set title screen volumes");
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
            crate::renderer::ui::pause_menu::PauseMenuAction::QuitApp => {
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
                                crate::game::keys::GameKey::ToggleUpgradeMenu => {
                                    // Toggle upgrade menu visibility
                                    if state.upgrade_menu.is_visible() {
                                        state.upgrade_menu.hide();
                                        // Return to game if we were in upgrade menu
                                        if state.game_state.current_screen
                                            == crate::game::CurrentScreen::UpgradeMenu
                                        {
                                            state.game_state.current_screen =
                                                crate::game::CurrentScreen::Game;
                                            state.game_state.capture_mouse = true;
                                        }
                                    } else {
                                        // Show upgrade menu
                                        state.upgrade_menu.show();
                                        state.game_state.previous_screen =
                                            Some(state.game_state.current_screen);
                                        state.game_state.current_screen =
                                            crate::game::CurrentScreen::UpgradeMenu;
                                        state.game_state.capture_mouse = false;
                                    }
                                }
                                crate::game::keys::GameKey::Escape => {
                                    match state.game_state.current_screen {
                                        crate::game::CurrentScreen::Game => {
                                            // Enter pause menu
                                            state.game_state.previous_screen =
                                                Some(crate::game::CurrentScreen::Game);
                                            state.game_state.current_screen =
                                                crate::game::CurrentScreen::Pause;
                                            // Pause timer (enemy locking is handled in update loop)
                                            state.game_state.game_ui.pause_timer();
                                            // Unlock cursor
                                            state.game_state.capture_mouse = false;
                                            // Show pause menu with current test mode state
                                            state.pause_menu.show(state.game_state.is_test_mode);
                                            // Set pause menu audio volumes
                                            state
                                                .game_state
                                                .audio_manager
                                                .set_pause_menu_volumes()
                                                .expect("Failed to set pause menu volumes");
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
                                                        if !state.game_state.is_test_mode {
                                                            // Only resume timer in normal mode (enemy locking is handled in update loop)
                                                            state.game_state.game_ui.resume_timer();
                                                        }
                                                        // In test mode, keep timer paused (enemy locking is handled in update loop)
                                                        // Lock cursor
                                                        state.game_state.capture_mouse = true;
                                                        // Restore game audio volumes
                                                        state
                                                            .game_state
                                                            .audio_manager
                                                            .set_game_volumes()
                                                            .expect("Failed to set game volumes");
                                                    }
                                                    crate::game::CurrentScreen::Title => {
                                                        // Return to title screen - cursor should be unlocked
                                                        state.game_state.capture_mouse = false;
                                                        // Set title screen audio volumes
                                                        state.game_state.audio_manager.set_title_screen_volumes()
                                                            .expect("Failed to set title screen volumes");
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
                                                // Set title screen audio volumes
                                                state
                                                    .game_state
                                                    .audio_manager
                                                    .set_title_screen_volumes()
                                                    .expect("Failed to set title screen volumes");
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
                                            // Set pause menu audio volumes
                                            state
                                                .game_state
                                                .audio_manager
                                                .set_pause_menu_volumes()
                                                .expect("Failed to set pause menu volumes");
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
                                    // Set game audio volumes before leaving title screen
                                    app_state
                                        .game_state
                                        .audio_manager
                                        .set_game_volumes()
                                        .expect("Failed to set game volumes");
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
