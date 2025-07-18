use crate::game::upgrades::{AvailableUpgrade, Upgrade, UpgradeManager};
use crate::renderer::ui::button::{
    Button, ButtonAnchor, ButtonManager, ButtonPosition, TextAlign, create_primary_button_style,
};
use glyphon::{Color, Resolution};
use wgpu::{self, Device, Queue, RenderPass, SurfaceConfiguration};
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::window::Window;

#[derive(Debug, Clone, PartialEq)]
pub enum UpgradeMenuAction {
    SelectUpgrade1,
    SelectUpgrade2,
    SelectUpgrade3,
    None,
}

pub struct UpgradeMenu {
    pub button_manager: ButtonManager,
    pub upgrade_manager: UpgradeManager,
    pub current_upgrades: Vec<Upgrade>,
    pub visible: bool,
    pub last_action: UpgradeMenuAction,
    pub content_initialized: bool, // Flag to prevent content from changing after initialization
}

impl UpgradeMenu {
    pub fn new(
        device: &Device,
        queue: &Queue,
        surface_format: wgpu::TextureFormat,
        window: &Window,
    ) -> Self {
        let mut button_manager = ButtonManager::new(device, queue, surface_format, window);

        // Create upgrade menu layout
        Self::create_upgrade_layout(&mut button_manager, window.inner_size());

        Self {
            button_manager,
            upgrade_manager: UpgradeManager::new(),
            current_upgrades: Vec::new(),
            visible: false,
            last_action: UpgradeMenuAction::None,
            content_initialized: false,
        }
    }

    fn create_upgrade_layout(button_manager: &mut ButtonManager, window_size: PhysicalSize<u32>) {
        let window_width = window_size.width as f32;
        let window_height = window_size.height as f32;

        // Main container dimensions (large rounded rectangle)
        let container_width = window_width * 0.8;
        let container_height = window_height * 0.7;
        let container_x = (window_width - container_width) / 2.0;
        let container_y = (window_height - container_height) / 2.0;

        // Store container dimensions for rendering
        button_manager.container_rect = Some(
            crate::renderer::rectangle::Rectangle::new(
                container_x,
                container_y,
                container_width,
                container_height,
                [0.4, 0.4, 0.4, 1.0], // Medium grey
            )
            .with_corner_radius(20.0),
        );

        // Three upgrade slots (tall rounded rectangles)
        let slot_width = container_width * 0.25; // 25% of container width
        let slot_spacing = container_width * 0.05; // 5% spacing between slots
        let total_slots_width = slot_width * 3.0 + slot_spacing * 2.0;
        let slots_start_x = container_x + (container_width - total_slots_width) / 2.0;

        // Create three upgrade slot buttons
        for i in 0..3 {
            let slot_x = slots_start_x + i as f32 * (slot_width + slot_spacing);

            // Create a custom style for the upgrade slots (lighter grey)
            let mut slot_style = create_primary_button_style();
            slot_style.background_color = Color::rgb(200, 200, 200); // Light grey
            slot_style.hover_color = Color::rgb(180, 180, 180); // Slightly darker on hover
            slot_style.pressed_color = Color::rgb(160, 160, 160); // Even darker when pressed
            slot_style.corner_radius = 12.0; // Rounded corners
            slot_style.padding = (8.0, 8.0); // Minimal padding
            slot_style.text_style.font_size = 32.0; // Doubled from 16.0
            slot_style.text_style.line_height = 48.0; // Doubled from 18.0 (approximate)
            slot_style.text_style.color = Color::rgb(50, 50, 50); // Dark text for contrast

            let upgrade_text = match i {
                0 => "Upgrade 1",
                1 => "Upgrade 2",
                2 => "Upgrade 3",
                _ => "Unknown",
            };

            // Calculate height proportion for tall buttons
            let margin = 0.1; // 10% margin
            let height_proportion = (container_height * (1.0 - 2.0 * margin)) / window_height;
            slot_style.spacing =
                crate::renderer::ui::button::ButtonSpacing::Tall(height_proportion);

            let button = Button::new(&format!("upgrade_{}", i + 1), upgrade_text)
                .with_style(slot_style)
                .with_text_align(TextAlign::Center)
                .with_level_text()
                .with_tooltip_text()
                .with_position(
                    ButtonPosition::new(
                        slot_x,
                        container_y + container_height * 0.1,
                        slot_width,
                        0.0,
                    ) // Width set, height will be calculated by ButtonManager
                    .with_anchor(ButtonAnchor::TopLeft),
                );

            button_manager.add_button(button);
        }

        // Update button positions to ensure proper layout
        button_manager.update_button_positions();
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.last_action = UpgradeMenuAction::None;

        // Get 3 random upgrades
        self.current_upgrades = self.upgrade_manager.select_random_upgrades(3);

        // Show all buttons first
        for button in self.button_manager.buttons.values_mut() {
            button.set_visible(true);
        }

        // Update upgrade buttons separately to avoid borrow conflicts
        self.update_upgrade_buttons();

        // Don't call update_button_states here - it will be called by the main loop
        // and we don't want to interfere with the stable text content
    }

    fn update_upgrade_buttons(&mut self) {
        // Only update if content hasn't been initialized yet
        if self.content_initialized {
            return;
        }

        // First pass: update button text and collect text updates
        let mut text_updates = Vec::new();
        let mut level_text_updates = Vec::new();

        for (i, upgrade) in self.current_upgrades.iter().enumerate() {
            let button_id = format!("upgrade_{}", i + 1);
            if let Some(button) = self.button_manager.get_button_mut(&button_id) {
                // Update button text to the upgrade name
                button.text = upgrade.name.clone();

                // Set the correct icon for this upgrade
                button.icon_id = Some(Self::get_icon_id_for_upgrade_name(&upgrade.name));

                // Get display info (level text and tooltip)
                let (level_text, tooltip_text) =
                    self.upgrade_manager.get_upgrade_display_info(upgrade);

                // Store text updates for later application
                if let Some(level_id) = &button.level_text_id {
                    level_text_updates.push((level_id.clone(), level_text));
                }

                if let Some(tooltip_id) = &button.tooltip_text_id {
                    text_updates.push((tooltip_id.clone(), tooltip_text));
                }

                // Also update the main text buffer for the button name
                text_updates.push((button.text_id.clone(), button.text.clone()));
            }
        }

        // Second pass: apply text updates to text renderer
        for (text_id, new_text) in text_updates {
            if let Some(buffer) = self
                .button_manager
                .text_renderer
                .text_buffers
                .get_mut(&text_id)
            {
                buffer.text_content = new_text;
                // Re-apply style to update the buffer
                let style = buffer.style.clone();
                let _ = self
                    .button_manager
                    .text_renderer
                    .update_style(&text_id, style);
            }
        }

        // Third pass: update level text buffers and recalculate their positions
        for (level_id, new_text) in level_text_updates {
            if let Some(buffer) = self
                .button_manager
                .text_renderer
                .text_buffers
                .get_mut(&level_id)
            {
                buffer.text_content = new_text;
                // Re-apply style to update the buffer
                let style = buffer.style.clone();
                let _ = self
                    .button_manager
                    .text_renderer
                    .update_style(&level_id, style);
            }
        }

        // After all text updates, recalculate button positions (including text centering)
        self.button_manager.update_button_positions();

        // Update icon positions to reflect the new upgrade icons
        self.button_manager.update_icon_positions();

        // Mark content as initialized to prevent further updates
        self.content_initialized = true;
    }

    fn get_icon_id_for_upgrade_name(upgrade_name: &str) -> String {
        match upgrade_name {
            "Speed Up" => "speed_up_icon".to_string(),
            "Slow Time" => "slow_down_icon".to_string(),
            "Silent Step" => "silent_step_icon".to_string(),
            "Tall Boots" => "tall_boots_icon".to_string(),
            "Head Start" => "head_start_icon".to_string(),
            "Dash" => "dash_icon".to_string(),
            "Unknown" => "unknown_icon".to_string(),
            _ => "blank_icon".to_string(),
        }
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.last_action = UpgradeMenuAction::None;
        self.content_initialized = false; // Reset flag so content can be reinitialized

        // Hide all buttons
        for button in self.button_manager.buttons.values_mut() {
            button.set_visible(false);
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn handle_input(&mut self, event: &WindowEvent, game_state: &mut crate::game::GameState) {
        if !self.visible {
            return;
        }

        self.button_manager.handle_input(event);

        // Check for button clicks and apply upgrades
        let mut upgrade_selected = false;
        let mut selected_upgrade_name = String::new();

        if self.button_manager.is_button_clicked("upgrade_1") {
            if let Some(upgrade) = self.current_upgrades.get(0) {
                selected_upgrade_name = upgrade.name.clone();
                self.apply_upgrade_by_name(&selected_upgrade_name, game_state);
                upgrade_selected = true;
            }
            self.last_action = UpgradeMenuAction::SelectUpgrade1;
        }

        if self.button_manager.is_button_clicked("upgrade_2") {
            if let Some(upgrade) = self.current_upgrades.get(1) {
                selected_upgrade_name = upgrade.name.clone();
                self.apply_upgrade_by_name(&selected_upgrade_name, game_state);
                upgrade_selected = true;
            }
            self.last_action = UpgradeMenuAction::SelectUpgrade2;
        }

        if self.button_manager.is_button_clicked("upgrade_3") {
            if let Some(upgrade) = self.current_upgrades.get(2) {
                selected_upgrade_name = upgrade.name.clone();
                self.apply_upgrade_by_name(&selected_upgrade_name, game_state);
                upgrade_selected = true;
            }
            self.last_action = UpgradeMenuAction::SelectUpgrade3;
        }

        // If an upgrade was selected, hide the menu
        if upgrade_selected {
            println!(
                "Upgrade '{}' applied! Menu will close.",
                selected_upgrade_name
            );
            self.hide();
        }
    }

    pub fn update(&mut self) {
        if !self.visible {
            return;
        }

        self.button_manager.update_button_states();
    }

    /// Applies all owned upgrades to the player and game state, stacking effects as appropriate.
    pub fn apply_upgrade_effects(&self, game_state: &mut crate::game::GameState) {
        // Reset affected player fields to base values
        game_state.player.base_speed = 100.0;
        game_state.player.max_stamina = 2.0;
        game_state.player.position[1] = crate::math::coordinates::constants::PLAYER_HEIGHT;
        // TODO: Reset any other affected fields as needed

        // Apply stacking upgrades
        for (upgrade, count) in self.upgrade_manager.player_upgrades.iter() {
            match upgrade {
                AvailableUpgrade::SpeedUp => {
                    // +10% movement and sprint speed per instance
                    game_state.player.base_speed *= 1.1_f32.powi(*count as i32);
                }
                AvailableUpgrade::Dash => {
                    // +10% max stamina per instance
                    game_state.player.max_stamina *= 1.1_f32.powi(*count as i32);
                }
                AvailableUpgrade::TallBoots => {
                    // +3 height per instance
                    game_state.player.position[1] =
                        crate::math::coordinates::constants::PLAYER_HEIGHT + 3.0 * (*count as f32);
                }
                AvailableUpgrade::SlowTime => {
                    // +5 seconds per instance to timer (handled at level start)
                    if let Some(timer) = &mut game_state.game_ui.timer {
                        let extra = 5 * *count as u64;
                        timer.config.duration += std::time::Duration::from_secs(extra);
                    }
                }
                AvailableUpgrade::SilentStep => {
                    // 5% worse enemy pathfinding per instance (handled elsewhere)
                    // Could set a field in game_state or player for enemy logic to read
                }
                AvailableUpgrade::HeadStart => {
                    // +3 seconds enemy lock per instance (handled at level start)
                    // Could set a field in game_state for enemy logic to read
                }
                _ => {}
            }
        }
        // After applying, update current speed to base
        game_state.player.speed = game_state.player.base_speed;
    }

    fn apply_upgrade_by_name(
        &mut self,
        upgrade_name: &str,
        game_state: &mut crate::game::GameState,
    ) {
        let available_upgrade = match upgrade_name {
            "Speed Up" => AvailableUpgrade::SpeedUp,
            "Slow Time" => AvailableUpgrade::SlowTime,
            "Silent Step" => AvailableUpgrade::SilentStep,
            "Tall Boots" => AvailableUpgrade::TallBoots,
            "Head Start" => AvailableUpgrade::HeadStart,
            "Dash" => AvailableUpgrade::Dash,
            "Unknown" => AvailableUpgrade::Unknown,
            _ => AvailableUpgrade::SpeedUp, // Fallback
        };
        self.upgrade_manager.apply_upgrade(&available_upgrade);
        self.apply_upgrade_effects(game_state);
    }

    pub fn get_last_action(&mut self) -> UpgradeMenuAction {
        let action = self.last_action.clone();
        self.last_action = UpgradeMenuAction::None;
        action
    }

    pub fn resize(&mut self, queue: &Queue, resolution: Resolution) {
        self.button_manager.resize(queue, resolution);
        self.recreate_layout_for_new_size();
    }

    fn recreate_layout_for_new_size(&mut self) {
        // Clear existing buttons
        self.button_manager.buttons.clear();
        self.button_manager.button_order.clear();

        // Recreate layout with new window size
        let window_size = PhysicalSize::new(
            self.button_manager.window_size.width,
            self.button_manager.window_size.height,
        );
        Self::create_upgrade_layout(&mut self.button_manager, window_size);

        // Reset content initialization flag
        self.content_initialized = false;

        // If menu is visible, reinitialize content
        if self.visible {
            self.update_upgrade_buttons();
        }
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
        if !self.visible {
            return Ok(());
        }

        self.button_manager.render(device, render_pass)
    }
}
