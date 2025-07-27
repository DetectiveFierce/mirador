use crate::game::audio::GameAudioManager;
use crate::renderer::ui::button::{
    Button, ButtonAnchor, ButtonManager, ButtonPosition, TextAlign, create_danger_button_style,
    create_primary_button_style, create_warning_button_style,
};
use glyphon::Resolution;
use wgpu::{Device, Queue, RenderPass, SurfaceConfiguration};
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::window::Window;

/// Actions that can be triggered from the pause menu
#[derive(Debug, Clone, PartialEq)]
pub enum PauseMenuAction {
    /// Resume the current game
    Resume,
    /// Restart the current run/game
    Restart,
    /// Quit to the main menu/lobby
    QuitToMenu,
    /// Quit the entire application
    QuitApp,
    /// Toggle test mode on/off
    ToggleTestMode,
    /// No action has been taken
    None,
}

/// A pause menu overlay that appears when the game is paused.
///
/// The pause menu provides several options to the player:
/// - Resume the game
/// - Restart the current run
/// - Toggle test mode
/// - Quit to lobby
/// - Quit the application
/// - Toggle debug panel visibility
///
/// The menu automatically scales its buttons and text based on the window size
/// to maintain consistent appearance across different resolutions.
pub struct PauseMenu {
    /// Manages all the buttons in the pause menu
    pub button_manager: ButtonManager,
    /// Whether the pause menu is currently visible
    pub visible: bool,
    /// The last action that was triggered by the menu
    pub last_action: PauseMenuAction,
    /// Whether the debug panel should be shown
    pub show_debug_panel: bool,
}

impl PauseMenu {
    /// Creates a new pause menu instance.
    ///
    /// # Arguments
    ///
    /// * `device` - The WGPU device for rendering
    /// * `queue` - The WGPU command queue
    /// * `surface_format` - The surface texture format
    /// * `window` - The window reference for sizing calculations
    ///
    /// # Returns
    ///
    /// A new `PauseMenu` instance with all buttons configured and positioned
    pub fn new(
        device: &Device,
        queue: &Queue,
        surface_format: wgpu::TextureFormat,
        window: &Window,
    ) -> Self {
        let mut button_manager = ButtonManager::new(device, queue, surface_format, window);

        // Create pause menu buttons with proper scaling and positioning
        Self::create_menu_buttons(&mut button_manager, window.inner_size());

        Self {
            button_manager,
            visible: false,
            last_action: PauseMenuAction::None,
            show_debug_panel: false,
        }
    }

    /// Creates a scaled text style based on the window height.
    ///
    /// This ensures consistent text sizing across different screen resolutions
    /// by scaling relative to a reference height of 1080p.
    ///
    /// # Arguments
    ///
    /// * `window_height` - The current window height in pixels
    ///
    /// # Returns
    ///
    /// A `TextStyle` with appropriately scaled font size and line height
    fn scaled_text_style(window_height: f32) -> crate::renderer::text::TextStyle {
        // Virtual DPI scaling based on reference height
        let reference_height = 1080.0;
        let scale = (window_height / reference_height).clamp(0.7, 2.0);
        let font_size = (32.0 * scale).clamp(16.0, 48.0); // 32px at 1080p, min 16, max 48
        let line_height = (40.0 * scale).clamp(24.0, 60.0); // 40px at 1080p, min 24, max 60

        crate::renderer::text::TextStyle {
            font_family: "Hanken Grotesk".to_string(),
            font_size,
            line_height,
            color: crate::renderer::ui::button::create_primary_button_style()
                .text_style
                .color,
            weight: glyphon::Weight::MEDIUM,
            style: glyphon::Style::Normal,
        }
    }

    /// Creates and configures all the buttons for the pause menu.
    ///
    /// This method sets up the layout with proper DPI scaling, centering all main
    /// buttons vertically and placing a debug button in the bottom-left corner.
    ///
    /// # Arguments
    ///
    /// * `button_manager` - The button manager to add buttons to
    /// * `window_size` - The current window size for positioning calculations
    fn create_menu_buttons(button_manager: &mut ButtonManager, window_size: PhysicalSize<u32>) {
        let reference_height = 1080.0;
        let scale = (window_size.height as f32 / reference_height).clamp(0.7, 2.0);

        // Button sizing with DPI scaling
        let button_width = (window_size.width as f32 * 0.38 * scale).clamp(180.0, 600.0);
        let button_height = (window_size.height as f32 * 0.09 * scale).clamp(32.0, 140.0);
        let button_spacing = (window_size.height as f32 * 0.015 * scale).clamp(2.0, 24.0);
        let total_height = button_height * 5.0 + button_spacing * 4.0;
        let center_x = window_size.width as f32 / 2.0;
        let start_y = (window_size.height as f32 - total_height) / 2.0;
        let text_style = Self::scaled_text_style(window_size.height as f32);

        // Helper function to calculate y position for button at index i
        let y =
            |i: usize| start_y + button_height / 2.0 + i as f32 * (button_height + button_spacing);

        // Resume button - Primary action to continue the game
        let mut resume_style = create_primary_button_style();
        resume_style.text_style = text_style.clone();
        let resume_button = Button::new("resume", "Resume Game")
            .with_style(resume_style)
            .with_text_align(TextAlign::Center)
            .with_position(
                ButtonPosition::new(center_x, y(0), button_width, button_height)
                    .with_anchor(ButtonAnchor::Center),
            );

        // Restart Run button - Restarts the current game session
        let mut restart_run_style = create_warning_button_style();
        restart_run_style.text_style = text_style.clone();
        let restart_run_button = Button::new("restart_run", "Restart Run")
            .with_style(restart_run_style)
            .with_text_align(TextAlign::Center)
            .with_position(
                ButtonPosition::new(center_x, y(1), button_width, button_height)
                    .with_anchor(ButtonAnchor::Center),
            );

        // Toggle Test Mode button - Switches between normal and test modes
        let mut test_mode_style = create_warning_button_style();
        test_mode_style.text_style = text_style.clone();
        let test_mode_button = Button::new("toggle_test_mode", "Toggle Test Mode")
            .with_style(test_mode_style)
            .with_text_align(TextAlign::Center)
            .with_position(
                ButtonPosition::new(center_x, y(2), button_width, button_height)
                    .with_anchor(ButtonAnchor::Center),
            );

        // Quit to Lobby button - Returns to the main lobby/menu
        let mut quit_lobby_style = create_danger_button_style();
        quit_lobby_style.text_style = text_style.clone();
        let quit_lobby_button = Button::new("quit_lobby", "Quit to Lobby")
            .with_style(quit_lobby_style)
            .with_text_align(TextAlign::Center)
            .with_position(
                ButtonPosition::new(center_x, y(3), button_width, button_height)
                    .with_anchor(ButtonAnchor::Center),
            );

        // Quit App button - Exits the entire application
        let mut quit_style = create_danger_button_style();
        quit_style.text_style = text_style.clone();
        let quit_menu_button = Button::new("quit_menu", "Quit App")
            .with_style(quit_style)
            .with_text_align(TextAlign::Center)
            .with_position(
                ButtonPosition::new(center_x, y(4), button_width, button_height)
                    .with_anchor(ButtonAnchor::Center),
            );

        // Debug button - Small button in bottom-left corner to toggle debug info
        let mut debug_style = create_warning_button_style();
        debug_style.text_style.font_size = text_style.font_size * 0.5;
        debug_style.text_style.line_height = text_style.line_height * 0.5;
        debug_style.padding = (2.0 * scale, 6.0 * scale); // minimal horizontal, some vertical padding
        debug_style.spacing = crate::renderer::ui::button::ButtonSpacing::Wrap;

        // Measure the text width for three lines to make button square
        let (_min_x, text_width, text_height) = button_manager
            .text_renderer
            .measure_text(" Show\nDebug\n  Info", &debug_style.text_style);
        let debug_button_side = text_width.max(text_height) + 2.0 * debug_style.padding.1;
        let debug_button = Button::new("debug", " Show\nDebug\n  Info")
            .with_style(debug_style)
            .with_text_align(TextAlign::Center)
            .with_position(ButtonPosition {
                x: 60.0,
                y: window_size.height as f32 - debug_button_side - 16.0, // 16px from bottom
                width: debug_button_side,
                height: debug_button_side,
                anchor: ButtonAnchor::TopLeft,
            });

        // Add all buttons to the button manager
        button_manager.add_button(resume_button);
        button_manager.add_button(restart_run_button);
        button_manager.add_button(test_mode_button);
        button_manager.add_button(quit_lobby_button);
        button_manager.add_button(quit_menu_button);
        button_manager.add_button(debug_button);

        // Update button positions to ensure text is properly centered
        button_manager.update_button_positions();
    }

    /// Shows the pause menu and makes all buttons visible.
    ///
    /// # Arguments
    ///
    /// * `is_test_mode` - Whether the game is currently in test mode,
    ///   used to update the test mode button text appropriately
    pub fn show(&mut self, is_test_mode: bool) {
        self.visible = true;
        self.last_action = PauseMenuAction::None;

        // Make all buttons visible
        for button in self.button_manager.buttons.values_mut() {
            button.set_visible(true);
        }

        // Ensure button text is made visible and styled immediately
        self.button_manager.update_button_states();

        // Update the test mode button text based on current state
        self.update_test_mode_button_text(is_test_mode);
    }

    /// Hides the pause menu and makes all buttons invisible.
    pub fn hide(&mut self) {
        self.visible = false;
        self.last_action = PauseMenuAction::None;

        // Hide all buttons
        for button in self.button_manager.buttons.values_mut() {
            button.set_visible(false);
        }
    }

    /// Toggles the pause menu visibility.
    ///
    /// # Arguments
    ///
    /// * `is_test_mode` - Whether the game is currently in test mode,
    ///   used when showing the menu to update button text
    pub fn toggle(&mut self, is_test_mode: bool) {
        if self.visible {
            self.hide();
        } else {
            self.show(is_test_mode);
        }
    }

    /// Returns whether the pause menu is currently visible.
    ///
    /// # Returns
    ///
    /// `true` if the menu is visible, `false` otherwise
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Handles input events for the pause menu.
    ///
    /// This method processes window events and checks for button clicks,
    /// setting the appropriate action and playing audio feedback.
    ///
    /// # Arguments
    ///
    /// * `event` - The window event to handle
    /// * `audio_manager` - The audio manager for playing button click sounds
    pub fn handle_input(&mut self, event: &WindowEvent, audio_manager: &mut GameAudioManager) {
        if !self.visible {
            return;
        }

        self.button_manager.handle_input(event);

        // Check for button clicks and play select sound for each action
        if self.button_manager.is_button_clicked("resume") {
            self.last_action = PauseMenuAction::Resume;
            let _ = audio_manager.play_select();
        }

        if self.button_manager.is_button_clicked("restart_run") {
            self.last_action = PauseMenuAction::Restart;
            let _ = audio_manager.play_select();
        }

        if self.button_manager.is_button_clicked("quit_lobby") {
            self.last_action = PauseMenuAction::QuitToMenu;
            let _ = audio_manager.play_select();
        }

        if self.button_manager.is_button_clicked("toggle_test_mode") {
            self.last_action = PauseMenuAction::ToggleTestMode;
            let _ = audio_manager.play_select();
        }

        if self.button_manager.is_button_clicked("quit_menu") {
            self.last_action = PauseMenuAction::QuitApp;
            let _ = audio_manager.play_select();
        }

        if self.button_manager.is_button_clicked("debug") {
            self.show_debug_panel = !self.show_debug_panel;
            let _ = audio_manager.play_select();
        }
    }

    /// Gets the last action that was triggered and resets it to `None`.
    ///
    /// This method should be called each frame to check what action
    /// the user has requested from the pause menu.
    ///
    /// # Returns
    ///
    /// The last `PauseMenuAction` that was triggered, or `None` if no action occurred
    pub fn get_last_action(&mut self) -> PauseMenuAction {
        let action = self.last_action.clone();
        self.last_action = PauseMenuAction::None;
        action
    }

    /// Handles window resize events by updating button positions and text rendering.
    ///
    /// # Arguments
    ///
    /// * `queue` - The WGPU command queue
    /// * `resolution` - The new window resolution
    pub fn resize(&mut self, queue: &Queue, resolution: Resolution) {
        self.button_manager.resize(queue, resolution);

        // Update window_size for correct centering calculations
        self.button_manager.window_size = winit::dpi::PhysicalSize {
            width: resolution.width,
            height: resolution.height,
        };

        // Recreate buttons with new positions based on the new window size
        self.recreate_buttons_for_new_size();
    }

    /// Recreates all button positions and styles for the current window size.
    ///
    /// This method is called after a window resize to ensure all buttons
    /// are properly positioned and scaled for the new dimensions.
    fn recreate_buttons_for_new_size(&mut self) {
        let window_size = self.button_manager.window_size;
        let reference_height = 1080.0;
        let scale = (window_size.height as f32 / reference_height).clamp(0.7, 2.0);

        // Recalculate button dimensions and positioning
        let button_width = (window_size.width as f32 * 0.38 * scale).clamp(180.0, 600.0);
        let button_height = (window_size.height as f32 * 0.09 * scale).clamp(32.0, 140.0);
        let button_spacing = (window_size.height as f32 * 0.015 * scale).clamp(2.0, 24.0);
        let total_height = button_height * 5.0 + button_spacing * 4.0;
        let center_x = window_size.width as f32 / 2.0;
        let start_y = (window_size.height as f32 - total_height) / 2.0;
        let text_style = Self::scaled_text_style(window_size.height as f32);
        let y =
            |i: usize| start_y + button_height / 2.0 + i as f32 * (button_height + button_spacing);

        // Update each button's position and text style
        if let Some(resume_button) = self.button_manager.get_button_mut("resume") {
            resume_button.position.x = center_x;
            resume_button.position.y = y(0);
            resume_button.position.width = button_width;
            resume_button.position.height = button_height;
            resume_button.position.anchor = ButtonAnchor::Center;
            resume_button.style.text_style = text_style.clone();
        }

        if let Some(restart_run_button) = self.button_manager.get_button_mut("restart_run") {
            restart_run_button.text = "Restart Run".to_string();
            restart_run_button.style = create_warning_button_style();
            restart_run_button.style.text_style = text_style.clone();
            restart_run_button.position.x = center_x;
            restart_run_button.position.y = y(1);
            restart_run_button.position.width = button_width;
            restart_run_button.position.height = button_height;
            restart_run_button.position.anchor = ButtonAnchor::Center;
        }

        if let Some(test_mode_button) = self.button_manager.get_button_mut("toggle_test_mode") {
            test_mode_button.text = "Toggle Test Mode".to_string();
            test_mode_button.style = create_warning_button_style();
            test_mode_button.style.text_style = text_style.clone();
            test_mode_button.position.x = center_x;
            test_mode_button.position.y = y(2);
            test_mode_button.position.width = button_width;
            test_mode_button.position.height = button_height;
            test_mode_button.position.anchor = ButtonAnchor::Center;
        }

        if let Some(quit_lobby_button) = self.button_manager.get_button_mut("quit_lobby") {
            quit_lobby_button.text = "Quit to Lobby".to_string();
            quit_lobby_button.style = create_danger_button_style();
            quit_lobby_button.style.text_style = text_style.clone();
            quit_lobby_button.position.x = center_x;
            quit_lobby_button.position.y = y(3);
            quit_lobby_button.position.width = button_width;
            quit_lobby_button.position.height = button_height;
            quit_lobby_button.position.anchor = ButtonAnchor::Center;
        }

        if let Some(quit_menu_button) = self.button_manager.get_button_mut("quit_menu") {
            quit_menu_button.style = create_danger_button_style();
            quit_menu_button.style.text_style = text_style.clone();
            quit_menu_button.position.x = center_x;
            quit_menu_button.position.y = y(4);
            quit_menu_button.position.width = button_width;
            quit_menu_button.position.height = button_height;
            quit_menu_button.position.anchor = ButtonAnchor::Center;
        }

        // Update debug button position for new window size
        let (style, padding) =
            if let Some(debug_button) = self.button_manager.get_button_mut("debug") {
                debug_button.style.spacing = crate::renderer::ui::button::ButtonSpacing::Wrap;
                (
                    debug_button.style.text_style.clone(),
                    debug_button.style.padding,
                )
            } else {
                (create_warning_button_style().text_style, (2.0, 6.0))
            };
        let (_min_x, text_width, text_height) = self
            .button_manager
            .text_renderer
            .measure_text("Show\nDebug\nInfo", &style);
        let side = text_width.max(text_height) + 2.0 * padding.1;
        if let Some(debug_button) = self.button_manager.get_button_mut("debug") {
            debug_button.position.x = 60.0;
            debug_button.position.y = window_size.height as f32 - side - 16.0;
            debug_button.position.width = side;
            debug_button.position.height = side;
            debug_button.position.anchor = ButtonAnchor::TopLeft;
        }

        // Update text positions after all changes
        self.button_manager.update_button_positions();
    }

    /// Prepares the pause menu for rendering by updating text layout.
    ///
    /// This should be called each frame before rendering.
    ///
    /// # Arguments
    ///
    /// * `device` - The WGPU device
    /// * `queue` - The WGPU command queue
    /// * `surface_config` - The surface configuration
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or a `PrepareError` if text preparation fails
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        surface_config: &SurfaceConfiguration,
    ) -> Result<(), glyphon::PrepareError> {
        self.button_manager.prepare(device, queue, surface_config)
    }

    /// Renders the pause menu to the screen.
    ///
    /// # Arguments
    ///
    /// * `device` - The WGPU device
    /// * `render_pass` - The render pass to draw into
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or a `RenderError` if rendering fails
    pub fn render(
        &mut self,
        device: &Device,
        render_pass: &mut RenderPass,
    ) -> Result<(), glyphon::RenderError> {
        self.button_manager.render(device, render_pass)
    }

    /// Returns whether the debug panel should be visible.
    ///
    /// # Returns
    ///
    /// `true` if the debug panel should be shown, `false` otherwise
    pub fn is_debug_panel_visible(&self) -> bool {
        self.show_debug_panel
    }

    /// Updates the test mode button text based on the current test mode state.
    ///
    /// # Arguments
    ///
    /// * `is_test_mode` - Whether the game is currently in test mode
    pub fn update_test_mode_button_text(&mut self, is_test_mode: bool) {
        if let Some(button) = self.button_manager.get_button_mut("toggle_test_mode") {
            if is_test_mode {
                button.text = "Exit Test Mode".to_string();
            } else {
                button.text = "Enter Test Mode".to_string();
            }
        }
    }
}
