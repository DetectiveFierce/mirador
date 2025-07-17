use crate::renderer::ui::buttons::{
    Button, ButtonAnchor, ButtonManager, ButtonPosition, TextAlign, create_danger_button_style,
    create_goldenrod_button_style, create_lobby_button_style, create_primary_button_style,
    create_warning_button_style,
};
use glyphon::Resolution;
use wgpu::{Device, Queue, RenderPass, SurfaceConfiguration};
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::window::Window;

#[derive(Debug, Clone, PartialEq)]
pub enum PauseMenuAction {
    Resume,
    Settings,
    Restart,
    QuitToMenu,
    ToggleTestMode,
    None,
}

pub struct PauseMenu {
    pub button_manager: ButtonManager,
    pub visible: bool,
    pub last_action: PauseMenuAction,
    pub show_debug_panel: bool, // Track debug panel visibility
}

impl PauseMenu {
    pub fn new(
        device: &Device,
        queue: &Queue,
        surface_format: wgpu::TextureFormat,
        window: &Window,
    ) -> Self {
        let mut button_manager = ButtonManager::new(device, queue, surface_format, window);

        // Create pause menu buttons
        Self::create_menu_buttons(&mut button_manager, window.inner_size());

        Self {
            button_manager,
            visible: false,
            last_action: PauseMenuAction::None,
            show_debug_panel: false,
        }
    }

    fn scaled_text_style(window_height: f32) -> crate::renderer::text::TextStyle {
        // 3.2% of window height for font size, 4% for line height (tweak as needed)
        let font_size = (window_height * 0.032).max(16.0); // minimum 16px
        let line_height = (window_height * 0.04).max(24.0);
        crate::renderer::text::TextStyle {
            font_family: "HankenGrotesk".to_string(),
            font_size,
            line_height,
            color: crate::renderer::ui::buttons::create_primary_button_style()
                .text_style
                .color,
            weight: glyphon::Weight::MEDIUM,
            style: glyphon::Style::Normal,
        }
    }

    fn create_menu_buttons(button_manager: &mut ButtonManager, window_size: PhysicalSize<u32>) {
        // Make button width proportional to window width, clamped to a reasonable range
        let button_width = (window_size.width as f32 * 0.38).clamp(180.0, 480.0); // 38% of width, min 180, max 480
        let button_height = (window_size.height as f32 * 0.09).clamp(32.0, 120.0); // 9% of height, min 32, max 120
        let button_spacing = (window_size.height as f32 * 0.015).clamp(2.0, 16.0); // 1.5% of height, min 2, max 16
        let total_height = button_height * 5.0 + button_spacing * 4.0; // 5 buttons now
        let center_x = window_size.width as f32 / 2.0;
        let start_y = (window_size.height as f32 - total_height) / 2.0;
        let text_style = Self::scaled_text_style(window_size.height as f32);

        // Helper for y position
        let y =
            |i: usize| start_y + button_height / 2.0 + i as f32 * (button_height + button_spacing);

        // Resume button
        let mut resume_style = create_primary_button_style();
        resume_style.text_style = text_style.clone();
        let resume_button = Button::new("resume", "Resume Game")
            .with_style(resume_style)
            .with_text_align(TextAlign::Center)
            .with_position(
                ButtonPosition::new(center_x, y(0), button_width, button_height)
                    .with_anchor(ButtonAnchor::Center),
            );

        // Settings button is now 'Restart Run' with goldenrod style
        let mut settings_style = create_goldenrod_button_style();
        settings_style.text_style = text_style.clone();
        let settings_button = Button::new("settings", "Restart Run")
            .with_style(settings_style)
            .with_text_align(TextAlign::Center)
            .with_position(
                ButtonPosition::new(center_x, y(1), button_width, button_height)
                    .with_anchor(ButtonAnchor::Center),
            );

        // Toggle Test Mode button with goldenrod style
        let mut test_mode_style = create_goldenrod_button_style();
        test_mode_style.text_style = text_style.clone();
        let test_mode_button = Button::new("toggle_test_mode", "Toggle Test Mode")
            .with_style(test_mode_style)
            .with_text_align(TextAlign::Center)
            .with_position(
                ButtonPosition::new(center_x, y(2), button_width, button_height)
                    .with_anchor(ButtonAnchor::Center),
            );

        // Restart button is now 'Quit to Lobby' with less saturated red style
        let mut restart_style = create_lobby_button_style();
        restart_style.text_style = text_style.clone();
        let restart_button = Button::new("restart", "Quit to Lobby")
            .with_style(restart_style)
            .with_text_align(TextAlign::Center)
            .with_position(
                ButtonPosition::new(center_x, y(3), button_width, button_height)
                    .with_anchor(ButtonAnchor::Center),
            );

        // Quit to Menu button - Center aligned for comparison, now darker red
        let mut quit_style = create_danger_button_style();
        quit_style.text_style = text_style.clone();
        let quit_menu_button = Button::new("quit_menu", "Quit App")
            .with_style(quit_style)
            .with_text_align(TextAlign::Center)
            .with_position(
                ButtonPosition::new(center_x, y(4), button_width, button_height)
                    .with_anchor(ButtonAnchor::Center),
            );

        // Add debug button in bottom left
        let mut debug_style = create_warning_button_style();
        debug_style.text_style.font_size = text_style.font_size * 0.5;
        debug_style.text_style.line_height = text_style.line_height * 0.5;
        debug_style.padding = (2.0, 6.0); // minimal horizontal, some vertical padding
        debug_style.spacing = crate::renderer::ui::buttons::ButtonSpacing::Wrap;
        // Measure the text width for three lines
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

        // Add buttons to manager
        button_manager.add_button(resume_button);
        button_manager.add_button(settings_button);
        button_manager.add_button(test_mode_button);
        button_manager.add_button(restart_button);
        button_manager.add_button(quit_menu_button);
        button_manager.add_button(debug_button);

        // Update button positions to ensure text is properly centered
        button_manager.update_button_positions();
    }

    pub fn show(&mut self, is_test_mode: bool) {
        self.visible = true;
        self.last_action = PauseMenuAction::None;

        // Show all buttons
        for button in self.button_manager.buttons.values_mut() {
            button.set_visible(true);
        }
        // Ensure button text is made visible and styled immediately
        self.button_manager.update_button_states();
        // Update the test mode button text
        self.update_test_mode_button_text(is_test_mode);
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.last_action = PauseMenuAction::None;

        // Hide all buttons
        for button in self.button_manager.buttons.values_mut() {
            button.set_visible(false);
        }
    }

    pub fn toggle(&mut self, is_test_mode: bool) {
        if self.visible {
            self.hide();
        } else {
            self.show(is_test_mode);
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn handle_input(&mut self, event: &WindowEvent) {
        if !self.visible {
            return;
        }

        self.button_manager.handle_input(event);

        // Check for button clicks
        if self.button_manager.is_button_clicked("resume") {
            self.last_action = PauseMenuAction::Resume;
        }

        if self.button_manager.is_button_clicked("settings") {
            self.last_action = PauseMenuAction::Settings;
        }

        if self.button_manager.is_button_clicked("restart") {
            self.last_action = PauseMenuAction::Restart;
        }

        if self.button_manager.is_button_clicked("toggle_test_mode") {
            self.last_action = PauseMenuAction::ToggleTestMode;
        }

        if self.button_manager.is_button_clicked("quit_menu") {
            self.last_action = PauseMenuAction::QuitToMenu;
        }
        if self.button_manager.is_button_clicked("debug") {
            self.show_debug_panel = !self.show_debug_panel;
        }
    }

    pub fn get_last_action(&mut self) -> PauseMenuAction {
        let action = self.last_action.clone();
        self.last_action = PauseMenuAction::None;
        action
    }

    pub fn resize(&mut self, queue: &Queue, resolution: Resolution) {
        self.button_manager.resize(queue, resolution);
        // Update window_size for correct centering
        self.button_manager.window_size = winit::dpi::PhysicalSize {
            width: resolution.width,
            height: resolution.height,
        };
        // Recreate buttons with new positions if menu is visible
        self.recreate_buttons_for_new_size();
    }

    fn recreate_buttons_for_new_size(&mut self) {
        let window_size = self.button_manager.window_size;
        // Make button width proportional to window width, clamped to a reasonable range
        let button_width = (window_size.width as f32 * 0.38).clamp(180.0, 480.0); // 38% of width, min 180, max 480
        let button_height = (window_size.height as f32 * 0.09).clamp(32.0, 120.0); // 9% of height, min 32, max 120
        let button_spacing = (window_size.height as f32 * 0.015).clamp(2.0, 16.0); // 1.5% of height, min 2, max 16
        let total_height = button_height * 5.0 + button_spacing * 4.0; // 5 buttons now
        let center_x = window_size.width as f32 / 2.0;
        let start_y = (window_size.height as f32 - total_height) / 2.0;
        let text_style = Self::scaled_text_style(window_size.height as f32);
        let y =
            |i: usize| start_y + button_height / 2.0 + i as f32 * (button_height + button_spacing);

        // Update button positions and text style
        if let Some(resume_button) = self.button_manager.get_button_mut("resume") {
            resume_button.position.x = center_x;
            resume_button.position.y = y(0);
            resume_button.position.width = button_width;
            resume_button.position.height = button_height;
            resume_button.position.anchor = ButtonAnchor::Center;
            resume_button.style.text_style = text_style.clone();
        }

        if let Some(settings_button) = self.button_manager.get_button_mut("settings") {
            settings_button.text = "Restart Run".to_string();
            settings_button.style = create_goldenrod_button_style();
            settings_button.style.text_style = text_style.clone();
            settings_button.position.x = center_x;
            settings_button.position.y = y(1);
            settings_button.position.width = button_width;
            settings_button.position.height = button_height;
            settings_button.position.anchor = ButtonAnchor::Center;
        }

        if let Some(test_mode_button) = self.button_manager.get_button_mut("toggle_test_mode") {
            test_mode_button.text = "Toggle Test Mode".to_string();
            test_mode_button.style = create_goldenrod_button_style();
            test_mode_button.style.text_style = text_style.clone();
            test_mode_button.position.x = center_x;
            test_mode_button.position.y = y(2);
            test_mode_button.position.width = button_width;
            test_mode_button.position.height = button_height;
            test_mode_button.position.anchor = ButtonAnchor::Center;
        }

        if let Some(restart_button) = self.button_manager.get_button_mut("restart") {
            restart_button.text = "Quit to Lobby".to_string();
            restart_button.style = create_lobby_button_style();
            restart_button.style.text_style = text_style.clone();
            restart_button.position.x = center_x;
            restart_button.position.y = y(3);
            restart_button.position.width = button_width;
            restart_button.position.height = button_height;
            restart_button.position.anchor = ButtonAnchor::Center;
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
                debug_button.style.spacing = crate::renderer::ui::buttons::ButtonSpacing::Wrap;
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

        // Update text positions
        self.button_manager.update_button_positions();
    }

    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        surface_config: &SurfaceConfiguration,
    ) -> Result<(), glyphon::PrepareError> {
        self.button_manager.prepare(device, queue, surface_config)
    }

    pub fn render(
        &mut self,
        device: &Device,
        render_pass: &mut RenderPass,
    ) -> Result<(), glyphon::RenderError> {
        self.button_manager.render(device, render_pass)
    }

    pub fn is_debug_panel_visible(&self) -> bool {
        self.show_debug_panel
    }

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
