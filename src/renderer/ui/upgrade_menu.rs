//! Upgrade Menu System
//!
//! This module provides a comprehensive upgrade menu interface for the game,
//! allowing players to select from randomly presented upgrades between levels.
//! The menu displays three upgrade options in a visually appealing layout with
//! buttons, icons, and tooltips.

use crate::game::upgrades::{AvailableUpgrade, Upgrade, UpgradeManager};
use crate::renderer::ui::button::{
    Button, ButtonAnchor, ButtonManager, ButtonPosition, TextAlign, create_primary_button_style,
};
use glyphon::{Color, Resolution};
use wgpu::{self, Device, Queue, RenderPass, SurfaceConfiguration};
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::window::Window;

/// Represents the possible actions that can be taken in the upgrade menu.
///
/// This enum is used to track which upgrade slot was selected by the player,
/// allowing the game loop to respond appropriately to user choices.
#[derive(Debug, Clone, PartialEq)]
pub enum UpgradeMenuAction {
    /// Player selected the first upgrade option
    SelectUpgrade1,
    /// Player selected the second upgrade option
    SelectUpgrade2,
    /// Player selected the third upgrade option
    SelectUpgrade3,
    /// No action was taken or action was reset
    None,
}

/// The main upgrade menu system that handles display and interaction logic.
///
/// This struct manages the entire upgrade selection process, including:
/// - Rendering the upgrade menu UI with three selectable options
/// - Managing button interactions and visual feedback
/// - Applying selected upgrades to the game state
/// - Handling menu visibility and layout changes
///
/// # Layout
/// The menu displays as a centered modal with three vertical upgrade slots,
/// each showing the upgrade name, icon, level information, and description tooltip.
pub struct UpgradeMenu {
    /// Manages all UI buttons within the upgrade menu
    pub button_manager: ButtonManager,
    /// Handles upgrade selection, application, and persistence
    pub upgrade_manager: UpgradeManager,
    /// The three currently available upgrade options presented to the player
    pub current_upgrades: Vec<Upgrade>,
    /// Whether the upgrade menu is currently visible and active
    pub visible: bool,
    /// The last action performed by the player (which upgrade was selected)
    pub last_action: UpgradeMenuAction,
    /// Prevents content from being reinitialized after first setup
    ///
    /// This flag ensures that upgrade text, icons, and tooltips remain stable
    /// once displayed, preventing flickering or content changes during interaction.
    pub content_initialized: bool,
}

impl UpgradeMenu {
    /// Creates a new upgrade menu instance with the specified rendering context.
    ///
    /// # Arguments
    /// * `device` - WGPU device for GPU operations
    /// * `queue` - WGPU command queue for rendering operations
    /// * `surface_format` - The texture format of the rendering surface
    /// * `window` - Window reference for layout calculations
    ///
    /// # Returns
    /// A new `UpgradeMenu` instance with initialized button layout but hidden by default.
    ///
    /// # Example
    /// ```rust
    /// let upgrade_menu = UpgradeMenu::new(&device, &queue, surface_format, &window);
    /// ```
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

    /// Creates a scaled text style based on the window height.
    ///
    /// This ensures consistent text sizing across different screen resolutions
    /// by scaling relative to a reference height of 1080p.
    ///
    /// # Arguments
    /// * `window_height` - The current window height in pixels
    ///
    /// # Returns
    /// A `TextStyle` with appropriately scaled font size and line height
    fn scaled_text_style(window_height: f32) -> crate::renderer::text::TextStyle {
        // Virtual DPI scaling based on reference height
        let reference_height = 1080.0;
        let scale = (window_height / reference_height).clamp(0.7, 2.0);
        let font_size = (32.0 * scale).clamp(16.0, 48.0); // 32px at 1080p, min 16, max 48
        let line_height = (48.0 * scale).clamp(24.0, 72.0); // 48px at 1080p, min 24, max 72

        crate::renderer::text::TextStyle {
            font_family: "Hanken Grotesk".to_string(),
            font_size,
            line_height,
            color: Color::rgb(50, 50, 50), // Dark text for contrast
            weight: glyphon::Weight::MEDIUM,
            style: glyphon::Style::Normal,
        }
    }

    /// Creates the visual layout for the upgrade menu with three selectable upgrade slots.
    ///
    /// This method sets up:
    /// - A centered modal container (80% of window width, 70% of window height)
    /// - Three evenly spaced upgrade slot buttons within the container
    /// - Proper styling, spacing, and positioning for all UI elements
    ///
    /// # Arguments
    /// * `button_manager` - Mutable reference to the button manager for adding UI elements
    /// * `window_size` - Current window dimensions for layout calculations
    ///
    /// # Layout Details
    /// - Container: Rounded rectangle with medium grey background
    /// - Slots: 25% container width each, with 5% spacing between them
    /// - Buttons: Tall aspect ratio with scaled text and rounded corners
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

        // Get scaled text style for consistent sizing across resolutions
        let text_style = Self::scaled_text_style(window_height);

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
            slot_style.text_style = text_style.clone(); // Use scaled text style

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

    /// Makes the upgrade menu visible and initializes it with three random upgrade options.
    ///
    /// This method:
    /// 1. Sets the menu to visible state
    /// 2. Resets the last action to None
    /// 3. Selects 3 random upgrades from the available pool
    /// 4. Makes all UI buttons visible
    /// 5. Updates button content with upgrade information
    ///
    /// # Side Effects
    /// - Modifies `self.visible`, `self.last_action`, and `self.current_upgrades`
    /// - Updates button text, icons, and tooltips through the button manager
    /// - Triggers content initialization if not already done
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

    /// Updates the content of upgrade buttons with current upgrade information.
    ///
    /// This method performs a three-pass update to avoid borrow conflicts:
    /// 1. **First pass**: Updates button text, icons, and prepares text content
    /// 2. **Second pass**: Applies main text updates to the text renderer
    /// 3. **Third pass**: Updates level text and recalculates positions
    ///
    /// The method only runs once per menu display (controlled by `content_initialized`)
    /// to prevent content flickering and ensure stable UI presentation.
    ///
    /// # Content Updates
    /// - Button text: Set to upgrade name (e.g., "Speed Up", "Dash")
    /// - Icons: Matched to upgrade type using `get_icon_id_for_upgrade_name`
    /// - Level text: Shows current upgrade level (e.g., "Level 2")
    /// - Tooltips: Displays upgrade description and effects
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

    /// Maps upgrade names to their corresponding icon identifiers.
    ///
    /// This function provides a centralized mapping between upgrade names
    /// and their visual icon representations in the UI.
    ///
    /// # Arguments
    /// * `upgrade_name` - The name of the upgrade (e.g., "Speed Up", "Dash")
    ///
    /// # Returns
    /// A `String` containing the icon identifier for the upgrade.
    /// Returns "blank_icon" for unknown upgrade names.
    ///
    /// # Supported Upgrades
    /// - "Speed Up" → "speed_up_icon"
    /// - "Slow Time" → "slow_down_icon"
    /// - "Silent Step" → "silent_step_icon"
    /// - "Tall Boots" → "tall_boots_icon"
    /// - "Head Start" → "head_start_icon"
    /// - "Dash" → "dash_icon"
    /// - "Unknown" → "unknown_icon"
    /// - Others → "blank_icon"
    fn get_icon_id_for_upgrade_name(upgrade_name: &str) -> String {
        match upgrade_name {
            "Speed Up" => "speed_up_icon".to_string(),
            "Slow Time" => "slower_seconds_icon".to_string(),
            "Silent Step" => "silent_step_icon".to_string(),
            "Tall Boots" => "tall_boots_icon".to_string(),
            "Head Start" => "head_start_icon".to_string(),
            "Dash" => "dash_icon".to_string(),
            "Unknown" => "unknown_icon".to_string(),
            _ => "blank_icon".to_string(),
        }
    }

    /// Hides the upgrade menu and resets its state for the next use.
    ///
    /// This method:
    /// - Sets visibility to false
    /// - Resets the last action to None
    /// - Clears the content initialization flag for next display
    /// - Hides all UI buttons
    ///
    /// After calling this method, the menu can be shown again with new upgrade options.
    pub fn hide(&mut self) {
        self.visible = false;
        self.last_action = UpgradeMenuAction::None;
        self.content_initialized = false; // Reset flag so content can be reinitialized

        // Hide all buttons
        for button in self.button_manager.buttons.values_mut() {
            button.set_visible(false);
        }
    }

    /// Returns whether the upgrade menu is currently visible.
    ///
    /// # Returns
    /// `true` if the menu is visible and active, `false` otherwise.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Handles user input events for the upgrade menu.
    ///
    /// This method processes window events when the menu is visible, including:
    /// - Mouse clicks on upgrade buttons
    /// - Keyboard input for upgrade selection
    /// - Playing appropriate sound effects
    /// - Applying selected upgrades to the game state
    /// - Automatically hiding the menu after selection
    ///
    /// # Arguments
    /// * `event` - The window event to process
    /// * `game_state` - Mutable reference to the game state for applying upgrades
    ///
    /// # Side Effects
    /// - Updates `self.last_action` based on user interaction
    /// - Applies upgrade effects to the player and game state
    /// - Plays upgrade selection sound effects
    /// - Hides the menu after successful upgrade selection
    /// - Prints confirmation message to console
    pub fn handle_input(&mut self, event: &WindowEvent, game_state: &mut crate::game::GameState) {
        if !self.visible {
            return;
        }

        self.button_manager.handle_input(event);

        // Check for button clicks and apply upgrades
        let mut upgrade_selected = false;
        let mut selected_upgrade_name = String::new();

        if self.button_manager.is_button_clicked("upgrade_1") {
            // Play upgrade sound
            let _ = game_state.audio_manager.play_upgrade();

            if let Some(upgrade) = self.current_upgrades.get(0) {
                selected_upgrade_name = upgrade.name.clone();
                self.apply_upgrade_by_name(&selected_upgrade_name, game_state);
                upgrade_selected = true;
            }
            self.last_action = UpgradeMenuAction::SelectUpgrade1;
        }

        if self.button_manager.is_button_clicked("upgrade_2") {
            // Play upgrade sound
            let _ = game_state.audio_manager.play_upgrade();

            if let Some(upgrade) = self.current_upgrades.get(1) {
                selected_upgrade_name = upgrade.name.clone();
                self.apply_upgrade_by_name(&selected_upgrade_name, game_state);
                upgrade_selected = true;
            }
            self.last_action = UpgradeMenuAction::SelectUpgrade2;
        }

        if self.button_manager.is_button_clicked("upgrade_3") {
            // Play upgrade sound
            let _ = game_state.audio_manager.play_upgrade();

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
            
            // Force a redraw to ensure the menu disappears immediately
            // This helps prevent freezing on Windows
            println!("[DEBUG] Upgrade selected, requesting redraw");
        }
    }

    /// Updates the upgrade menu's internal state.
    ///
    /// This method should be called every frame when the menu is visible.
    /// It updates button states, handles hover effects, and maintains
    /// proper UI responsiveness.
    ///
    /// Does nothing if the menu is not visible.
    pub fn update(&mut self) {
        if !self.visible {
            return;
        }

        self.button_manager.update_button_states();
    }

    /// Applies all owned upgrades to the player and game state, with proper stacking effects.
    ///
    /// This method handles the complete upgrade application process:
    /// 1. **Reset Phase**: Resets affected player fields to their base values
    /// 2. **Application Phase**: Applies all owned upgrades with proper stacking
    /// 3. **Finalization Phase**: Updates derived values (like current speed)
    ///
    /// # Arguments
    /// * `game_state` - Mutable reference to the game state to modify
    ///
    /// # Upgrade Effects
    /// - **Speed Up**: +10% movement and sprint speed per level (multiplicative)
    /// - **Dash**: +10% max stamina per level (multiplicative)
    /// - **Tall Boots**: +3 height units per level (additive)
    /// - **Slow Time**: +5 seconds to level timer per level (additive)
    /// - **Silent Step**: 5% worse enemy pathfinding per level
    /// - **Head Start**: +3 seconds enemy lock delay per level
    ///
    /// # Implementation Notes
    /// - Multiplicative effects use `powi()` for proper stacking
    /// - Additive effects use simple multiplication
    /// - Some effects are applied at level start (timer, enemy delays)
    /// - Player speed is synchronized with base speed after application
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

    /// Applies a specific upgrade by name to the player's upgrade collection.
    ///
    /// This is a convenience method that:
    /// 1. Converts the upgrade name string to an `AvailableUpgrade` enum
    /// 2. Adds the upgrade to the player's owned upgrades
    /// 3. Immediately applies all upgrade effects to the game state
    ///
    /// # Arguments
    /// * `upgrade_name` - The name of the upgrade to apply (e.g., "Speed Up")
    /// * `game_state` - Mutable reference to the game state to modify
    ///
    /// # Fallback Behavior
    /// If an unknown upgrade name is provided, defaults to "Speed Up".
    ///
    /// # Example
    /// ```rust
    /// upgrade_menu.apply_upgrade_by_name("Dash", &mut game_state);
    /// ```
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

    /// Retrieves and resets the last action performed in the upgrade menu.
    ///
    /// This method implements a "consume-on-read" pattern, returning the
    /// last action and immediately resetting it to `None`. This prevents
    /// the same action from being processed multiple times.
    ///
    /// # Returns
    /// The last `UpgradeMenuAction` that was performed, or `None` if no
    /// action was taken or the action was already consumed.
    ///
    /// # Usage Pattern
    /// ```rust
    /// match upgrade_menu.get_last_action() {
    ///     UpgradeMenuAction::SelectUpgrade1 => { /* handle upgrade 1 */ },
    ///     UpgradeMenuAction::SelectUpgrade2 => { /* handle upgrade 2 */ },
    ///     UpgradeMenuAction::SelectUpgrade3 => { /* handle upgrade 3 */ },
    ///     UpgradeMenuAction::None => { /* no action */ },
    /// }
    /// ```
    pub fn get_last_action(&mut self) -> UpgradeMenuAction {
        let action = self.last_action.clone();
        self.last_action = UpgradeMenuAction::None;
        action
    }

    /// Handles window resize events by updating the button manager and recreating the layout.
    ///
    /// This method ensures the upgrade menu remains properly sized and positioned
    /// when the game window is resized. It updates both the rendering resolution
    /// and the UI layout to match the new window dimensions with proper text scaling.
    ///
    /// # Arguments
    /// * `queue` - WGPU command queue for rendering operations
    /// * `resolution` - New window resolution for text rendering
    ///
    /// # Side Effects
    /// - Updates button manager's internal resolution and window size
    /// - Recreates the entire UI layout with new dimensions and scaled text
    /// - Resets content initialization if menu is currently visible
    pub fn resize(&mut self, queue: &Queue, resolution: Resolution) {
        self.button_manager.resize(queue, resolution);

        // Update window_size for correct scaling calculations
        self.button_manager.window_size = winit::dpi::PhysicalSize {
            width: resolution.width,
            height: resolution.height,
        };

        self.recreate_layout_for_new_size();
    }

    /// Recreates the upgrade menu layout for the current window size.
    ///
    /// This method is called after window resize events to ensure the
    /// UI layout matches the new window dimensions. It:
    /// 1. Clears all existing buttons and layout data
    /// 2. Recreates the layout using current window size with proper text scaling
    /// 3. Resets content initialization flag
    /// 4. Re-initializes content if menu is currently visible
    ///
    /// # Layout Preservation
    /// The method maintains the same visual proportions and styling
    /// while adapting to the new window size and ensuring proper text scaling.
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

    /// Prepares the upgrade menu for rendering by updating text buffers and GPU resources.
    ///
    /// This method must be called before `render()` each frame when the menu is visible.
    /// It handles text layout, buffer updates, and other preparation tasks required
    /// for proper rendering.
    ///
    /// # Arguments
    /// * `device` - WGPU device for GPU operations
    /// * `queue` - WGPU command queue for buffer updates
    /// * `surface_config` - Current surface configuration for rendering context
    ///
    /// # Returns
    /// * `Ok(())` - Preparation completed successfully
    /// * `Err(PrepareError)` - Text preparation failed (e.g., layout issues, GPU errors)
    ///
    /// # Usage
    /// ```rust
    /// upgrade_menu.prepare(&device, &queue, &surface_config)?;
    /// upgrade_menu.render(&device, &mut render_pass)?;
    /// ```
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        surface_config: &SurfaceConfiguration,
    ) -> Result<(), glyphon::PrepareError> {
        self.button_manager.prepare(device, queue, surface_config)
    }

    /// Renders the upgrade menu to the current render pass.
    ///
    /// This method draws all visible upgrade menu elements, including:
    /// - Background container with rounded corners
    /// - Three upgrade slot buttons with styling
    /// - Button text, icons, and level indicators
    /// - Tooltip text (if hovering over buttons)
    ///
    /// # Arguments
    /// * `device` - WGPU device for GPU operations
    /// * `render_pass` - Current render pass to draw into
    ///
    /// # Returns
    /// * `Ok(())` - Rendering completed successfully
    /// * `Err(RenderError)` - Rendering failed (e.g., GPU errors, resource issues)
    ///
    /// # Behavior
    /// - Does nothing and returns `Ok(())` if the menu is not visible
    /// - All rendering is handled by the internal `ButtonManager`
    ///
    /// # Prerequisites
    /// Must call `prepare()` before calling this method each frame.
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
