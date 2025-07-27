//! Button module - contains all button-related functionality for the UI system
//!
//! This module provides a complete button system for the Mirador game UI, including:
//! - Button creation and management
//! - Interactive states (normal, hover, pressed, disabled)
//! - Text rendering with multiple text elements (main text, level text, tooltips)
//! - Icon support for upgrade buttons
//! - Responsive positioning and scaling
//! - Mouse input handling
//!
//! The button system supports various button types:
//! - Standard buttons with text
//! - Upgrade menu buttons with icons, level text, and tooltips
//! - Responsive buttons that adapt to window size
//! - Buttons with different spacing strategies (wrap, horizontal bar, tall)
//!
//! # Examples
//!
//! ```rust
//! // Create a basic button
//! let button = Button::new("start_game", "Start Game")
//!     .with_style(ButtonStyle::default())
//!     .with_position(ButtonPosition::new(100.0, 100.0, 200.0, 50.0));
//!
//! // Create an upgrade button with level text and tooltip
//! let upgrade_button = Button::new("speed_upgrade", "Speed Boost")
//!     .with_style(upgrade_style)
//!     .with_level_text()
//!     .with_tooltip_text();
//! ```

// Button module - contains all button-related functionality
/// Button styling and theme definitions.
pub mod styles;
/// Button type definitions and enums.
pub mod types;
/// Button utility functions and extensions.
pub mod utils;

// Re-export types for convenience
pub use styles::*;
pub use types::{ButtonAnchor, ButtonPosition, ButtonSpacing, ButtonState, ButtonStyle, TextAlign};
pub use utils::ColorExt;

use crate::assets;
use crate::renderer::icon::{Icon, IconRenderer};
use crate::renderer::rectangle::{Rectangle, RectangleRenderer};
use crate::renderer::text::{TextPosition, TextRenderer, TextStyle};
use glyphon::{Color, Style, Weight};
use std::collections::{HashMap, HashSet};
use wgpu::{self, Device, Queue, RenderPass, SurfaceConfiguration};
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::window::Window;

/// Represents a UI button with text, styling, and interactive behavior
///
/// A Button can contain multiple text elements:
/// - Main text (required): The primary button label
/// - Level text (optional): Smaller text showing level information (e.g., "Level 1")
/// - Tooltip text (optional): Descriptive text explaining the button's function
/// - Icon (optional): Visual representation for upgrade buttons
///
/// Buttons support various interactive states and can be positioned using different
/// anchor points and spacing strategies.
#[derive(Debug)]
pub struct Button {
    /// Unique identifier for the button
    pub id: String,
    /// The main text displayed on the button
    pub text: String,
    /// Visual styling configuration (colors, padding, corner radius, etc.)
    pub style: ButtonStyle,
    /// Position and size of the button
    pub position: ButtonPosition,
    /// Whether the button can be interacted with
    pub enabled: bool,
    /// Whether the button is visible
    pub visible: bool,
    /// Current interactive state (normal, hover, pressed, disabled)
    pub state: ButtonState,
    /// Internal ID for the main text buffer
    pub text_id: String,
    /// Internal ID for the level text buffer (if level text is enabled)
    pub level_text_id: Option<String>,
    /// Internal ID for the tooltip text buffer (if tooltip is enabled)
    pub tooltip_text_id: Option<String>,
    /// ID of the icon to display (for upgrade buttons)
    pub icon_id: Option<String>,
}

impl Button {
    /// Creates a new button with the given ID and text
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the button
    /// * `text` - The text to display on the button
    ///
    /// # Returns
    /// A new Button instance with default styling and positioning
    pub fn new(id: &str, text: &str) -> Self {
        let text_id = format!("button_{}", id);
        Self {
            id: id.to_string(),
            text: text.to_string(),
            style: ButtonStyle::default(),
            position: ButtonPosition::new(0.0, 0.0, 200.0, 50.0),
            enabled: true,
            visible: true,
            state: ButtonState::Normal,
            text_id,
            level_text_id: None,
            tooltip_text_id: None,
            icon_id: None,
        }
    }

    /// Sets the button's visual style
    ///
    /// # Arguments
    /// * `style` - The ButtonStyle to apply
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets the button's position and size
    ///
    /// # Arguments
    /// * `position` - The ButtonPosition defining location and dimensions
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_position(mut self, position: ButtonPosition) -> Self {
        self.position = position;
        self
    }

    /// Sets the text alignment within the button
    ///
    /// # Arguments
    /// * `text_align` - The TextAlign value (Left, Center, Right)
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_text_align(mut self, text_align: TextAlign) -> Self {
        self.style.text_align = text_align;
        self
    }

    /// Enables level text display for this button
    ///
    /// Level text is typically used for upgrade buttons to show the current level
    /// (e.g., "Level 1", "Level 2"). The text will be displayed in a smaller,
    /// italic font below the main text.
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_level_text(mut self) -> Self {
        self.level_text_id = Some(format!("level_{}", self.id));
        self
    }

    /// Enables tooltip text display for this button
    ///
    /// Tooltip text provides additional information about the button's function.
    /// It's displayed in a smaller font below the level text (if present) or
    /// below the main text.
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_tooltip_text(mut self) -> Self {
        self.tooltip_text_id = Some(format!("tooltip_{}", self.id));
        self
    }

    /// Sets the button's visibility
    ///
    /// # Arguments
    /// * `visible` - Whether the button should be visible
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Checks if the given point (x, y) is within the button's bounds
    ///
    /// This method is used for hit detection during mouse interactions.
    /// Only visible and enabled buttons can be hit.
    ///
    /// # Arguments
    /// * `x` - X coordinate of the point to test
    /// * `y` - Y coordinate of the point to test
    ///
    /// # Returns
    /// `true` if the point is within the button's bounds, `false` otherwise
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        if !self.visible || !self.enabled {
            return false;
        }

        let (actual_x, actual_y) = self.position.calculate_actual_position();

        x >= actual_x
            && x <= actual_x + self.position.width
            && y >= actual_y
            && y <= actual_y + self.position.height
    }
}

/// Manages a collection of buttons and handles their rendering and interaction
///
/// ButtonManager is responsible for:
/// - Storing and organizing buttons
/// - Handling mouse input and button state updates
/// - Rendering buttons with their associated text and icons
/// - Managing text buffers for button text elements
/// - Providing click detection and event handling
///
/// The manager maintains button order for consistent rendering and supports
/// various button types including upgrade menu buttons with icons and tooltips.
pub struct ButtonManager {
    /// Map of button ID to Button instance
    pub buttons: HashMap<String, Button>,
    /// Ordered list of button IDs to maintain rendering order
    pub button_order: Vec<String>,
    /// Text renderer for button text elements
    pub text_renderer: TextRenderer,
    /// Rectangle renderer for button backgrounds
    pub rectangle_renderer: RectangleRenderer,
    /// Icon renderer for upgrade button icons
    pub icon_renderer: IconRenderer,
    /// Current window dimensions for responsive positioning
    pub window_size: PhysicalSize<u32>,
    /// Current mouse cursor position
    pub mouse_position: (f32, f32),
    /// Whether the left mouse button is currently pressed
    pub mouse_pressed: bool,
    /// ID of the button that was just clicked (if any)
    pub just_clicked: Option<String>,
    /// Optional container rectangle for upgrade menu background
    pub container_rect: Option<Rectangle>,
    /// Previous mouse position for change detection optimization
    pub last_mouse_position: (f32, f32),
    /// Previous mouse press state for change detection optimization
    pub last_mouse_pressed: bool,
    /// Set of buttons that were pressed during the current mouse press cycle
    /// This helps handle platform-specific timing differences in mouse event processing
    pub pressed_buttons: std::collections::HashSet<String>,
}

impl ButtonManager {
    /// Creates a new ButtonManager with initialized renderers and loaded icons
    ///
    /// This constructor:
    /// - Initializes text, rectangle, and icon renderers
    /// - Loads all upgrade icons from the assets directory
    /// - Sets up the window size for responsive positioning
    /// - Prepares the manager for button management and rendering
    ///
    /// # Arguments
    /// * `device` - WGPU device for creating render resources
    /// * `queue` - WGPU queue for uploading resources
    /// * `surface_format` - Texture format for the render surface
    /// * `window` - Window reference for size information
    ///
    /// # Returns
    /// A new ButtonManager instance ready for use
    pub fn new(
        device: &Device,
        queue: &Queue,
        surface_format: wgpu::TextureFormat,
        window: &Window,
    ) -> Self {
        let text_renderer = TextRenderer::new(device, queue, surface_format, window);
        let rectangle_renderer = RectangleRenderer::new(device, surface_format);
        let mut icon_renderer = IconRenderer::new(device, surface_format);
        let window_size = window.inner_size();

        // Load all upgrade icons from embedded assets
        for (id, texture_data) in assets::icon_textures() {
            if let Err(e) = icon_renderer.load_texture_from_data(device, queue, texture_data, id) {
                println!("Failed to load icon texture {}: {}", id, e);
            }
        }

        Self {
            buttons: HashMap::new(),
            button_order: Vec::new(), // Initialize the order tracking
            text_renderer,
            rectangle_renderer,
            icon_renderer,
            window_size,
            mouse_position: (0.0, 0.0),
            mouse_pressed: false,
            just_clicked: None,
            container_rect: None,
            last_mouse_position: (0.0, 0.0),
            last_mouse_pressed: false,
            pressed_buttons: HashSet::new(),
        }
    }

    /// Adds a button to the manager and sets up its text buffers
    ///
    /// This method:
    /// - Calculates button dimensions based on text content and spacing strategy
    /// - Creates text buffers for main text, level text, and tooltip text
    /// - Positions text elements within the button bounds
    /// - Tracks button order for consistent rendering
    /// - Handles different button spacing strategies (Wrap, Hbar, Tall)
    ///
    /// For upgrade buttons (ButtonSpacing::Tall), this method also:
    /// - Positions text at the top of the button
    /// - Sets up level text below the main text
    /// - Configures tooltip text at the bottom
    /// - Prepares icon positioning
    ///
    /// # Arguments
    /// * `button` - The Button instance to add
    pub fn add_button(&mut self, button: Button) {
        let text_id = button.text_id.clone();
        let text = button.text.clone();
        let style = button.style.clone();
        let button_id = button.id.clone();
        let level_text_id = button.level_text_id.clone();
        let tooltip_text_id = button.tooltip_text_id.clone();

        let horizontal_padding = style.padding.0;
        let vertical_padding = style.padding.1;
        let window_width = self.window_size.width as f32;

        // Measure the actual text size for positioning, allowing wrapping
        let (_min_x, text_width, text_height) = self.text_renderer.measure_text(
            &text,
            &TextStyle {
                ..style.text_style.clone()
            },
        );

        let (button_width, button_height) = match style.spacing {
            ButtonSpacing::Wrap => {
                let width = text_width + 2.0 * vertical_padding;
                let height = text_height + 2.0 * vertical_padding;
                (width, height)
            }
            ButtonSpacing::Hbar(prop) => {
                let width = window_width * prop;
                let height = text_height + 2.0 * vertical_padding;
                (width, height)
            }
            ButtonSpacing::Tall(height_proportion) => {
                // Tall buttons use a proportion of the window height
                // Use the position width if it's set, otherwise use text width
                let width = if button.position.width > 0.0 {
                    button.position.width
                } else {
                    text_width + 2.0 * vertical_padding
                };
                let height = self.window_size.height as f32 * height_proportion;
                (width, height)
            }
        };

        // Update the button's position with the calculated dimensions
        let mut button_with_size = button;
        button_with_size.position.width = button_width;
        button_with_size.position.height = button_height;

        // Calculate the actual position using the same transformation as hit detection
        let (actual_x, actual_y) = button_with_size.position.calculate_actual_position();

        // Calculate text position based on alignment using actual coordinates
        let text_x = match style.text_align {
            TextAlign::Left => actual_x + horizontal_padding,
            TextAlign::Right => actual_x + button_width - horizontal_padding - text_width,
            TextAlign::Center => actual_x + (button_width - text_width) / 2.0,
        };
        let text_y = actual_y + vertical_padding;

        let text_position = TextPosition {
            x: text_x,
            y: text_y,
            max_width: Some(button_width - 2.0 * horizontal_padding),
            max_height: Some(button_height - 2.0 * vertical_padding),
        };

        self.text_renderer.create_text_buffer(
            &text_id,
            &text,
            Some(TextStyle {
                color: style.background_color.darken(0.35), // Use proper color, not transparent
                ..style.text_style.clone()
            }),
            Some(text_position),
        );

        // Create level text if specified
        if let Some(level_id) = level_text_id {
            // Create a smaller, italic style for level text
            let mut level_style = style.text_style.clone();
            level_style.font_size = style.text_style.font_size * 0.7; // 70% of main text size
            level_style.line_height = style.text_style.line_height * 0.7;
            level_style.style = Style::Italic;
            level_style.color = style.background_color.darken(0.35); // Use same color as main text, not transparent

            // Use the actual initial text for the buffer ("Level 1" or "New Upgrade")
            let level_text = "Level 1";
            let (_min_x, level_text_width, level_text_height) =
                self.text_renderer.measure_text(level_text, &level_style);

            let level_text_x = match style.text_align {
                TextAlign::Left => actual_x + horizontal_padding,
                TextAlign::Right => actual_x + button_width - horizontal_padding - level_text_width,
                TextAlign::Center => actual_x + (button_width - level_text_width) / 2.0,
            };
            let level_text_y = actual_y + button_height * 0.55; // Slightly higher, still below the icon

            let level_text_position = TextPosition {
                x: level_text_x,
                y: level_text_y,
                max_width: Some(button_width - 2.0 * horizontal_padding), // Allow full width for centering
                max_height: Some(level_text_height),
            };

            self.text_renderer.create_text_buffer(
                &level_id,
                level_text,
                Some(level_style),
                Some(level_text_position),
            );
        }

        // Create tooltip text if specified
        if let Some(tooltip_id) = tooltip_text_id {
            // Create a larger style for tooltip text
            let mut tooltip_style = style.text_style.clone();
            tooltip_style.font_size = style.text_style.font_size * 0.7; // Increased from 0.55 to 0.7 (70% of main text size)
            tooltip_style.line_height = tooltip_style.font_size * 1.05;
            tooltip_style.style = Style::Normal;
            tooltip_style.color = style.background_color.darken(0.35); // Use same color as main text, not transparent

            // Position tooltip text below the level text
            let tooltip_text = "This is a place to describe an upgrade, and what effects it has on the game in a little more detail.";
            let extra_tooltip_padding = 20.0; // Increased from 10.0 to 20.0 for more margin
            let tooltip_horizontal_padding = horizontal_padding + extra_tooltip_padding;
            let tooltip_text_x = match style.text_align {
                TextAlign::Left => actual_x + tooltip_horizontal_padding,
                TextAlign::Right => actual_x + button_width - tooltip_horizontal_padding,
                TextAlign::Center => actual_x + tooltip_horizontal_padding, // Start from left padding, let text wrap
            };
            let tooltip_text_y = actual_y + button_height * 0.68; // Higher up than before

            let tooltip_text_position = TextPosition {
                x: tooltip_text_x,
                y: tooltip_text_y,
                max_width: Some(button_width - 2.0 * tooltip_horizontal_padding),
                max_height: Some(button_height * 0.28), // Allow for more lines
            };

            self.text_renderer.create_text_buffer(
                &tooltip_id,
                tooltip_text,
                Some(tooltip_style),
                Some(tooltip_text_position),
            );
        }

        // Track button order
        if !self.button_order.contains(&button_id) {
            self.button_order.push(button_id.clone());
        }

        self.buttons
            .insert(button_with_size.id.clone(), button_with_size);
    }

    /// Updates icon positions for all visible upgrade buttons
    ///
    /// This method:
    /// - Clears existing icons from the renderer
    /// - Only processes buttons with ButtonSpacing::Tall (upgrade buttons)
    /// - Calculates icon positions based on button scaling (hover effects)
    /// - Centers icons within the button bounds
    /// - Applies hover scaling to icons to match button scaling
    ///
    /// Icons are positioned at the center of the button with appropriate margins
    /// and scale with the button during hover/press states.
    pub fn update_icon_positions(&mut self) {
        // Clear existing icons
        self.icon_renderer.clear_icons();

        // Only add icons to buttons with ButtonSpacing::Tall (upgrade menu buttons)
        for button_id in &self.button_order {
            if let Some(button) = self.buttons.get(button_id) {
                if button.visible {
                    // Only add icons to Tall buttons (upgrade menu buttons)
                    if let ButtonSpacing::Tall(_) = button.style.spacing {
                        let (actual_x, actual_y) = button.position.calculate_actual_position();

                        // Calculate scale for hover effect on upgrade buttons
                        let scale = match button.state {
                            ButtonState::Hover => 1.1,    // 10% bigger on hover
                            ButtonState::Pressed => 1.05, // 5% bigger when pressed
                            _ => 1.0,                     // Normal size
                        };

                        // Calculate scaled button dimensions
                        let scaled_width = button.position.width * scale;
                        let scaled_height = button.position.height * scale;
                        let scaled_x = actual_x - (scaled_width - button.position.width) / 2.0;
                        let scaled_y = actual_y - (scaled_height - button.position.height) / 2.0;

                        // Calculate icon size and position with scaling
                        let margin = 16.0 * scale; // Scale margin too
                        let max_icon_width = scaled_width - 2.0 * margin;
                        let max_icon_height = scaled_height * 0.4;

                        // Calculate icon size (square, fit within constraints)
                        let icon_size = max_icon_width.min(max_icon_height);

                        // Position icon at center of scaled button
                        let icon_x = scaled_x + (scaled_width - icon_size) / 2.0;
                        let icon_y = scaled_y + scaled_height * 0.5;

                        let icon = Icon::new(
                            icon_x,
                            icon_y,
                            icon_size,
                            icon_size,
                            button
                                .icon_id
                                .clone()
                                .unwrap_or_else(|| "blank_icon".to_string()),
                        );
                        self.icon_renderer.add_icon(icon);
                    }
                }
            }
        }
    }

    /// Gets a mutable reference to a button by ID
    ///
    /// # Arguments
    /// * `id` - The button ID to look up
    ///
    /// # Returns
    /// `Some(&mut Button)` if the button exists, `None` otherwise
    pub fn get_button_mut(&mut self, id: &str) -> Option<&mut Button> {
        self.buttons.get_mut(id)
    }

    /// Checks if a specific button was clicked in the last input cycle
    ///
    /// This method checks the `just_clicked` state and returns true if the
    /// specified button was clicked. It also resets the click state and
    /// prints a debug message with the button text.
    ///
    /// # Arguments
    /// * `id` - The button ID to check for clicks
    ///
    /// # Returns
    /// `true` if the button was clicked, `false` otherwise
    pub fn is_button_clicked(&mut self, id: &str) -> bool {
        if let Some(clicked_id) = &self.just_clicked {
            if clicked_id == id {
                self.just_clicked = None; // Reset after checking
                if let Some(button) = self.buttons.get(id) {
                    let clean_text = button
                        .text
                        .replace(|c: char| c.is_whitespace(), " ")
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    println!("Button '{}' was clicked!", clean_text.trim());
                } else {
                    let clean_id = id
                        .replace(|c: char| c.is_whitespace(), " ")
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    println!("Button '{}' was clicked!", clean_id.trim());
                }
                return true;
            }
        }
        false
    }

    /// Handles window events for button interaction
    ///
    /// This method processes mouse input events to:
    /// - Track mouse button press/release states
    /// - Update mouse cursor position
    /// - Detect button clicks when mouse is released over a pressed button
    /// - Handle window resize events
    /// - Trigger button state updates
    ///
    /// # Arguments
    /// * `event` - The window event to process
    pub fn handle_input(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.mouse_pressed = true;
                self.pressed_buttons.clear(); // Clear previous press cycle
                self.update_button_states();
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                // Check for button clicks using both current state and pressed_buttons set
                // This handles platform-specific timing differences in mouse event processing
                let mut clicked_button = None;

                // First check current button states
                for button in self.buttons.values() {
                    if button.visible && button.enabled && button.state == ButtonState::Pressed {
                        clicked_button = Some(button.id.clone());
                        break;
                    }
                }

                // If no button found in current state, check the pressed_buttons set
                // This handles cases where the mouse moved outside the button during press
                if clicked_button.is_none() {
                    for button_id in &self.pressed_buttons {
                        if let Some(button) = self.buttons.get(button_id) {
                            if button.visible && button.enabled {
                                // Check if mouse is still over the button or was over it during press
                                let is_hovered = button
                                    .contains_point(self.mouse_position.0, self.mouse_position.1);
                                if is_hovered {
                                    clicked_button = Some(button_id.clone());
                                    break;
                                }
                            }
                        }
                    }
                }

                if let Some(clicked_id) = clicked_button {
                    self.just_clicked = Some(clicked_id);
                }

                // Now update mouse state
                self.mouse_pressed = false;
                self.pressed_buttons.clear();
                self.update_button_states();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = (position.x as f32, position.y as f32);
                self.update_button_states();
            }
            WindowEvent::Resized(size) => {
                self.window_size = *size;
                self.update_button_positions();
            }
            _ => {}
        }
    }

    /// Updates button states based on mouse interaction and applies visual changes
    ///
    /// This method:
    /// - Optimizes performance by checking if mouse state has changed
    /// - Updates button states (Normal, Hover, Pressed, Disabled) based on mouse position
    /// - Applies visual changes including color, weight, and text size scaling
    /// - Handles text positioning for hover scaling effects
    /// - Updates level text and tooltip text styling and positioning
    /// - Manages visibility for disabled/invisible buttons
    /// - Triggers icon position updates for upgrade buttons
    ///
    /// For upgrade buttons (ButtonSpacing::Tall), this method also:
    /// - Scales text size on hover (20% larger) and press (10% larger)
    /// - Adjusts text positioning to account for button scaling
    /// - Updates level text and tooltip text with proper scaling
    pub fn update_button_states(&mut self) {
        // Early exit if mouse state hasn't changed
        if self.mouse_position == self.last_mouse_position
            && self.mouse_pressed == self.last_mouse_pressed
        {
            return;
        }

        // Update cached mouse state
        self.last_mouse_position = self.mouse_position;
        self.last_mouse_pressed = self.mouse_pressed;

        // To avoid borrow checker issues, first collect level text content for each button
        let mut level_texts: Vec<(String, String)> = Vec::new();
        for button in self.buttons.values() {
            if let Some(level_id) = &button.level_text_id {
                let text = if let Some(buffer) = self.text_renderer.text_buffers.get(level_id) {
                    buffer.text_content.clone()
                } else {
                    "Level 1".to_string()
                };
                level_texts.push((button.id.clone(), text));
            }
        }

        for button in self.buttons.values_mut() {
            if !button.visible || !button.enabled {
                if button.state != ButtonState::Disabled {
                    button.state = ButtonState::Disabled;
                    // Hide text if not visible
                    let _ = self.text_renderer.update_style(
                        &button.text_id,
                        TextStyle {
                            color: Color::rgba(0, 0, 0, 0),
                            ..button.style.text_style.clone()
                        },
                    );
                    // Hide level text if not visible
                    if let Some(level_id) = &button.level_text_id {
                        let _ = self.text_renderer.update_style(
                            level_id,
                            TextStyle {
                                color: Color::rgba(0, 0, 0, 0),
                                ..button.style.text_style.clone()
                            },
                        );
                    }
                    // Hide tooltip text if not visible
                    if let Some(tooltip_id) = &button.tooltip_text_id {
                        let _ = self.text_renderer.update_style(
                            tooltip_id,
                            TextStyle {
                                color: Color::rgba(0, 0, 0, 0),
                                ..button.style.text_style.clone()
                            },
                        );
                    }
                }
                continue;
            }

            let is_hovered = button.contains_point(self.mouse_position.0, self.mouse_position.1);

            // Determine new state
            let new_state = if self.mouse_pressed && is_hovered {
                ButtonState::Pressed
            } else if is_hovered {
                ButtonState::Hover
            } else {
                ButtonState::Normal
            };

            // Track pressed buttons for click detection
            if new_state == ButtonState::Pressed {
                self.pressed_buttons.insert(button.id.clone());
            }

            // Only update if state actually changed
            if button.state == new_state {
                continue;
            }

            button.state = new_state;

            // Calculate actual position and paddings at the start of the loop
            let (actual_x, actual_y) = button.position.calculate_actual_position();
            let horizontal_padding = button.style.padding.0;
            let vertical_padding = button.style.padding.1;

            // Update text color and weight based on button state
            let (text_color, text_weight) = match button.state {
                ButtonState::Normal => (
                    button.style.background_color.darken(0.35), // 35% darker than bg
                    button.style.text_style.weight,
                ),
                ButtonState::Hover => (
                    button.style.hover_color.saturate(0.90), // much brighter and more saturated
                    Weight::BOLD,
                ),
                ButtonState::Pressed => (
                    button.style.pressed_color.brighten(0.15).saturate(0.35), // brighter and more saturated
                    Weight::MEDIUM,
                ),
                ButtonState::Disabled => (
                    Color::rgb(100, 116, 139), // slate-500 - muted text
                    Weight::NORMAL,
                ),
            };

            // Update text size based on hover state for upgrade buttons
            let text_size_scale = if let ButtonSpacing::Tall(_) = button.style.spacing {
                match button.state {
                    ButtonState::Hover => 1.2,   // 20% bigger on hover
                    ButtonState::Pressed => 1.1, // 10% bigger when pressed
                    _ => 1.0,                    // Normal size
                }
            } else {
                1.0 // No scaling for non-tall buttons
            };

            // Only update style if color, weight, or size changed
            let mut new_style = button.style.text_style.clone();
            new_style.color = text_color;
            new_style.weight = text_weight;
            new_style.font_size = button.style.text_style.font_size * text_size_scale;
            new_style.line_height = button.style.text_style.line_height * text_size_scale;

            // Make text visible now that color is correct
            let _ = self
                .text_renderer
                .update_style(&button.text_id, new_style.clone());

            // --- Main text position update for Tall buttons (hover scaling) ---
            if let ButtonSpacing::Tall(_) = button.style.spacing {
                // Calculate text size for the new style
                let (_min_x, wrap_width, wrap_height) =
                    self.text_renderer.measure_text(&button.text, &new_style);

                // Use the button's scale for position transformation
                let button_scale = match button.state {
                    ButtonState::Hover => 1.1,
                    ButtonState::Pressed => 1.05,
                    _ => 1.0,
                };

                // Calculate scaled button dimensions and position
                let scaled_width = button.position.width * button_scale;
                let scaled_height = button.position.height * button_scale;
                let scaled_x = actual_x - (scaled_width - button.position.width) / 2.0;
                let scaled_y = actual_y - (scaled_height - button.position.height) / 2.0;

                // Position text relative to scaled button
                let base_text_x = match button.style.text_align {
                    TextAlign::Left => scaled_x + horizontal_padding * button_scale,
                    TextAlign::Right => {
                        scaled_x + scaled_width - horizontal_padding * button_scale - wrap_width
                    }
                    TextAlign::Center => scaled_x + (scaled_width - wrap_width) / 2.0,
                };
                let base_text_y = scaled_y + vertical_padding * button_scale;

                let scaled_max_text_width =
                    (button.position.width - 2.0 * horizontal_padding) * button_scale;
                let text_position = TextPosition {
                    x: base_text_x,
                    y: base_text_y,
                    max_width: Some(scaled_max_text_width),
                    max_height: Some(wrap_height * text_size_scale),
                };

                if let Err(e) = self
                    .text_renderer
                    .update_position(&button.text_id, text_position)
                {
                    println!("Failed to update main text position: {}", e);
                }
            }
            // --- End main text position update ---

            // --- Level text update logic (all variables in scope) ---
            if let Some(level_id) = &button.level_text_id {
                // Find the actual text for this button's level text
                let level_text = level_texts
                    .iter()
                    .find(|(id, _)| id == &button.id)
                    .map(|(_, text)| text.as_str())
                    .unwrap_or("Level 1");
                // Create level text style with smaller size and italic
                let mut level_style = button.style.text_style.clone();
                level_style.font_size = button.style.text_style.font_size * 0.7; // DO NOT scale by text_size_scale
                level_style.line_height = button.style.text_style.line_height * 0.7; // DO NOT scale by text_size_scale
                level_style.style = Style::Italic;
                level_style.color = text_color; // Use same color as main text
                level_style.weight = text_weight;

                let (_min_x, level_text_width, level_text_height) =
                    self.text_renderer.measure_text(level_text, &level_style);

                // Use the button's scale for position transformation
                let button_scale = match button.state {
                    ButtonState::Hover => 1.1,
                    ButtonState::Pressed => 1.05,
                    _ => 1.0,
                };

                // Calculate scaled button dimensions
                let scaled_width = button.position.width * button_scale;
                let scaled_height = button.position.height * button_scale;
                let scaled_x = actual_x - (scaled_width - button.position.width) / 2.0;
                let scaled_y = actual_y - (scaled_height - button.position.height) / 2.0;

                // Calculate base level text position relative to scaled button
                let base_level_x = match button.style.text_align {
                    TextAlign::Left => scaled_x + horizontal_padding * button_scale,
                    TextAlign::Right => {
                        scaled_x + scaled_width
                            - horizontal_padding * button_scale
                            - level_text_width
                    }
                    TextAlign::Center => scaled_x + (scaled_width - level_text_width) / 2.0,
                };
                let base_level_y = scaled_y + scaled_height * 0.55; // Just below the icon

                // Position level text directly (no need for offset calculation since we're using scaled coordinates)
                let scaled_level_x = base_level_x;
                let scaled_level_y = base_level_y;

                let level_text_position = TextPosition {
                    x: scaled_level_x,
                    y: scaled_level_y,
                    max_width: Some(level_text_width + 8.0), // Only as wide as the text, plus a small margin
                    max_height: Some(level_text_height),     // DO NOT scale by text_size_scale
                };

                if let Err(e) = self
                    .text_renderer
                    .update_position(level_id, level_text_position)
                {
                    println!("Failed to update level text position: {}", e);
                }
            }
            // --- End level text update logic ---

            // Update tooltip text if it exists
            if let Some(tooltip_id) = &button.tooltip_text_id {
                // Create tooltip text style with larger size
                let mut tooltip_style = button.style.text_style.clone();
                tooltip_style.font_size = button.style.text_style.font_size * 0.7 * text_size_scale; // Increased from 0.6 to 0.7
                tooltip_style.line_height = tooltip_style.font_size * 1.05;
                tooltip_style.style = Style::Normal;
                tooltip_style.color = text_color; // Use same color as main text
                tooltip_style.weight = text_weight;

                let _ = self.text_renderer.update_style(tooltip_id, tooltip_style);
            }
        }

        // Update icon positions to match button scaling
        self.update_icon_positions();
    }

    /// Updates button positions and text layout after window resize or other changes
    ///
    /// This method recalculates:
    /// - Button dimensions based on current window size
    /// - Text positioning within buttons
    /// - Level text and tooltip text positioning
    /// - Icon positions for upgrade buttons
    /// - Scaling effects for hover states
    ///
    /// The method handles different button spacing strategies:
    /// - Wrap: Button size matches text content
    /// - Hbar: Button width is proportional to window width
    /// - Tall: Button height is proportional to window height (for upgrade buttons)
    ///
    /// For upgrade buttons, this method also:
    /// - Positions main text at the top
    /// - Places level text below the main text
    /// - Positions tooltip text at the bottom
    /// - Applies hover scaling transformations
    pub fn update_button_positions(&mut self) {
        // To avoid borrow checker issues, first collect level text content for each button
        let mut level_texts: Vec<(String, String)> = Vec::new();
        for button in self.buttons.values() {
            if let Some(level_id) = &button.level_text_id {
                let text = if let Some(buffer) = self.text_renderer.text_buffers.get(level_id) {
                    buffer.text_content.clone()
                } else {
                    "Level 1".to_string()
                };
                level_texts.push((button.id.clone(), text));
            }
        }
        // Now update positions
        for button in self.buttons.values_mut() {
            let (actual_x, actual_y) = button.position.calculate_actual_position();
            let horizontal_padding = button.style.padding.0;
            let vertical_padding = button.style.padding.1;

            // Calculate scale for hover effect on upgrade buttons
            let scale = if let ButtonSpacing::Tall(_) = button.style.spacing {
                match button.state {
                    ButtonState::Hover => 1.1,    // 10% bigger on hover
                    ButtonState::Pressed => 1.05, // 5% bigger when pressed
                    _ => 1.0,                     // Normal size
                }
            } else {
                1.0 // No scaling for non-tall buttons
            };

            let scaled_max_text_width = (button.position.width - 2.0 * horizontal_padding) * scale;
            let (_min_x, wrap_width, wrap_height) = self
                .text_renderer
                .measure_text(&button.text, &button.style.text_style);

            // Position text - for Tall buttons, put text at the top
            let base_text_x = match button.style.text_align {
                TextAlign::Left => actual_x + horizontal_padding,
                TextAlign::Right => {
                    actual_x + button.position.width - horizontal_padding - wrap_width
                }
                TextAlign::Center => actual_x + (button.position.width - wrap_width) / 2.0,
            };

            let base_text_y = if let ButtonSpacing::Tall(_) = button.style.spacing {
                // For tall buttons, position text at the top with padding
                actual_y + vertical_padding
            } else {
                // For other buttons, center text vertically
                actual_y + (button.position.height - wrap_height) / 2.0
            };

            // Apply scaling transformation for Tall buttons
            let (text_x, text_y) = if let ButtonSpacing::Tall(_) = button.style.spacing {
                // Calculate scaled button dimensions and position
                let scaled_width = button.position.width * scale;
                let scaled_height = button.position.height * scale;
                let scaled_x = actual_x - (scaled_width - button.position.width) / 2.0;
                let scaled_y = actual_y - (scaled_height - button.position.height) / 2.0;

                // Position text relative to scaled button
                let text_x = match button.style.text_align {
                    TextAlign::Left => scaled_x + horizontal_padding * scale,
                    TextAlign::Right => {
                        scaled_x + scaled_width - horizontal_padding * scale - wrap_width
                    }
                    TextAlign::Center => scaled_x + (scaled_width - wrap_width) / 2.0,
                };
                let text_y = scaled_y + vertical_padding * scale;
                (text_x, text_y)
            } else {
                (base_text_x, base_text_y)
            };

            let text_position = TextPosition {
                x: text_x,
                y: text_y,
                max_width: Some(scaled_max_text_width),
                max_height: Some(wrap_height * scale), // Scale the max height too
            };

            if let Err(e) = self
                .text_renderer
                .update_position(&button.text_id, text_position)
            {
                println!("Failed to update button position: {}", e);
            }

            // Update level text position if it exists
            if let Some(level_id) = &button.level_text_id {
                // Find the actual text for this button's level text
                let level_text = level_texts
                    .iter()
                    .find(|(id, _)| id == &button.id)
                    .map(|(_, text)| text.as_str())
                    .unwrap_or("Level 1");
                // Create level text style for measurement
                let mut level_style = button.style.text_style.clone();
                level_style.font_size = button.style.text_style.font_size * 0.7;
                level_style.line_height = button.style.text_style.line_height * 0.7;
                level_style.style = Style::Italic;

                let (_min_x, level_text_width, level_text_height) =
                    self.text_renderer.measure_text(level_text, &level_style);

                // Apply scaling transformation for Tall buttons
                let (scaled_level_x, scaled_level_y) = if let ButtonSpacing::Tall(_) =
                    button.style.spacing
                {
                    // Calculate scaled button dimensions and position
                    let scaled_width = button.position.width * scale;
                    let scaled_height = button.position.height * scale;
                    let scaled_x = actual_x - (scaled_width - button.position.width) / 2.0;
                    let scaled_y = actual_y - (scaled_height - button.position.height) / 2.0;

                    // Position level text relative to scaled button
                    let level_text_x = match button.style.text_align {
                        TextAlign::Left => scaled_x + horizontal_padding * scale,
                        TextAlign::Right => {
                            scaled_x + scaled_width - horizontal_padding * scale - level_text_width
                        }
                        TextAlign::Center => scaled_x + (scaled_width - level_text_width) / 2.0,
                    };

                    let level_text_y = scaled_y + scaled_height * 0.55; // Just below the icon
                    (level_text_x, level_text_y)
                } else {
                    // For non-tall buttons, use original positioning
                    let level_text_x = match button.style.text_align {
                        TextAlign::Left => actual_x + horizontal_padding,
                        TextAlign::Right => {
                            actual_x + button.position.width - horizontal_padding - level_text_width
                        }
                        TextAlign::Center => {
                            actual_x + (button.position.width - level_text_width) / 2.0
                        }
                    };

                    let level_text_y =
                        actual_y + button.position.height - level_text_height - vertical_padding;
                    (level_text_x, level_text_y)
                };

                let level_text_position = TextPosition {
                    x: scaled_level_x,
                    y: scaled_level_y,
                    max_width: Some(level_text_width + 8.0), // Only as wide as the text, plus a small margin
                    max_height: Some(level_text_height * scale),
                };

                if let Err(e) = self
                    .text_renderer
                    .update_position(level_id, level_text_position)
                {
                    println!("Failed to update level text position: {}", e);
                }
            }

            // Update tooltip text position if it exists
            if let Some(tooltip_id) = &button.tooltip_text_id {
                // Get the existing tooltip text from the buffer for measurement
                let tooltip_text = if let Some(buffer) =
                    self.text_renderer.text_buffers.get(tooltip_id)
                {
                    buffer.text_content.clone()
                } else {
                    "This is a place to describe an upgrade, and what effects it has on the game in a little more detail.".to_string()
                };

                // Create tooltip text style for measurement - use the same style as in add_button
                let mut tooltip_style = button.style.text_style.clone();
                tooltip_style.font_size = button.style.text_style.font_size * 0.7; // Increased from 0.55 to 0.7 (70% of main text size)
                tooltip_style.line_height = tooltip_style.font_size * 1.05;
                tooltip_style.style = Style::Normal;

                let (_min_x, _tooltip_text_width, tooltip_text_height) = self
                    .text_renderer
                    .measure_text(&tooltip_text, &tooltip_style);

                // Position tooltip text below the level text
                let extra_tooltip_padding = 20.0; // Increased from 10.0 to 20.0 for more margin
                let tooltip_horizontal_padding = horizontal_padding + extra_tooltip_padding;
                let tooltip_text_x = match button.style.text_align {
                    TextAlign::Left => actual_x + tooltip_horizontal_padding,
                    TextAlign::Right => {
                        actual_x + button.position.width - tooltip_horizontal_padding
                    }
                    TextAlign::Center => actual_x + tooltip_horizontal_padding,
                };

                let tooltip_text_y = if let ButtonSpacing::Tall(_) = button.style.spacing {
                    // For tall buttons, position below the level text
                    actual_y + button.position.height * 0.68
                } else {
                    // For other buttons, position at the bottom
                    actual_y + button.position.height - tooltip_text_height - vertical_padding
                };

                // Apply scaling transformation for Tall buttons
                let (scaled_tooltip_x, scaled_tooltip_y) =
                    if let ButtonSpacing::Tall(_) = button.style.spacing {
                        // Calculate scaled button dimensions and position
                        let scaled_width = button.position.width * scale;
                        let scaled_height = button.position.height * scale;
                        let scaled_x = actual_x - (scaled_width - button.position.width) / 2.0;
                        let scaled_y = actual_y - (scaled_height - button.position.height) / 2.0;

                        // Position tooltip text relative to scaled button
                        let tooltip_text_x = match button.style.text_align {
                            TextAlign::Left => scaled_x + tooltip_horizontal_padding * scale,
                            TextAlign::Right => {
                                scaled_x + scaled_width - tooltip_horizontal_padding * scale
                            }
                            TextAlign::Center => scaled_x + tooltip_horizontal_padding * scale,
                        };

                        let tooltip_text_y = scaled_y + scaled_height * 0.68; // Below the level text
                        (tooltip_text_x, tooltip_text_y)
                    } else {
                        (tooltip_text_x, tooltip_text_y)
                    };

                let tooltip_text_position = TextPosition {
                    x: scaled_tooltip_x,
                    y: scaled_tooltip_y,
                    max_width: Some(
                        (button.position.width - 2.0 * tooltip_horizontal_padding) * scale,
                    ),
                    max_height: Some(button.position.height * 0.28 * scale), // Allow for more lines
                };

                if let Err(e) = self
                    .text_renderer
                    .update_position(tooltip_id, tooltip_text_position)
                {
                    println!("Failed to update tooltip text position: {}", e);
                }
            }

            // Only update height here, but respect Tall spacing
            if let ButtonSpacing::Tall(_) = button.style.spacing {
                // Don't override height for Tall buttons, it's already set correctly
            } else {
                button.position.height = wrap_height + 2.0 * button.style.padding.1;
            }
        }

        // Update icon positions to match button positions
        self.update_icon_positions();
    }

    /// Resizes the button manager and its renderers to match the new window resolution
    ///
    /// This method updates:
    /// - Window size for positioning calculations
    /// - Text renderer resolution
    /// - Rectangle renderer dimensions
    /// - Icon renderer dimensions
    ///
    /// # Arguments
    /// * `queue` - WGPU queue for uploading new resources
    /// * `resolution` - New window resolution
    pub fn resize(&mut self, queue: &Queue, resolution: glyphon::Resolution) {
        // Update window size for correct positioning calculations
        self.window_size = winit::dpi::PhysicalSize {
            width: resolution.width,
            height: resolution.height,
        };

        self.text_renderer.resize(queue, resolution);
        self.rectangle_renderer
            .resize(resolution.width as f32, resolution.height as f32);
        self.icon_renderer
            .resize(resolution.width as f32, resolution.height as f32);
    }

    /// Prepares the text renderer for rendering
    ///
    /// This method delegates to the text renderer's prepare method to set up
    /// text buffers and resources for the current frame.
    ///
    /// # Arguments
    /// * `device` - WGPU device for creating resources
    /// * `queue` - WGPU queue for uploading resources
    /// * `surface_config` - Surface configuration for rendering
    ///
    /// # Returns
    /// `Ok(())` on success, `Err(PrepareError)` on failure
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        surface_config: &SurfaceConfiguration,
    ) -> Result<(), glyphon::PrepareError> {
        self.text_renderer.prepare(device, queue, surface_config)
    }

    /// Renders all buttons and their associated elements
    ///
    /// This method renders in the following order:
    /// 1. Container rectangle (if present, for upgrade menu background)
    /// 2. Button background rectangles with proper colors and scaling
    /// 3. Button icons (for upgrade buttons)
    /// 4. Button text elements (main text, level text, tooltips)
    ///
    /// The rendering order ensures proper layering:
    /// - Backgrounds are rendered first
    /// - Icons are rendered on top of backgrounds
    /// - Text is rendered last for proper visibility
    ///
    /// For upgrade buttons, hover scaling is applied to both the background
    /// rectangle and the corner radius for smooth visual effects.
    ///
    /// # Arguments
    /// * `device` - WGPU device for rendering
    /// * `render_pass` - Render pass to record commands
    ///
    /// # Returns
    /// `Ok(())` on success, `Err(RenderError)` on failure
    pub fn render(
        &mut self,
        device: &Device,
        render_pass: &mut RenderPass,
    ) -> Result<(), glyphon::RenderError> {
        // Clear previous rectangles
        self.rectangle_renderer.clear_rectangles();

        // Render container rectangle first (if it exists)
        if let Some(container_rect) = &self.container_rect {
            self.rectangle_renderer
                .add_rectangle(container_rect.clone());
        }

        // Render buttons in the order they were added
        for button_id in &self.button_order {
            if let Some(button) = self.buttons.get(button_id) {
                if button.visible {
                    let (actual_x, actual_y) = button.position.calculate_actual_position();

                    // Use the button's style colors for each state
                    let color = if !button.enabled {
                        button.style.disabled_color
                    } else {
                        match button.state {
                            ButtonState::Normal => button.style.background_color,
                            ButtonState::Hover => button.style.hover_color,
                            ButtonState::Pressed => button.style.pressed_color,
                            ButtonState::Disabled => button.style.disabled_color,
                        }
                    };
                    let color_array = [
                        color.r() as f32 / 255.0,
                        color.g() as f32 / 255.0,
                        color.b() as f32 / 255.0,
                        color.a() as f32 / 255.0,
                    ];

                    // Calculate scale for hover effect on upgrade buttons
                    let scale = if let ButtonSpacing::Tall(_) = button.style.spacing {
                        match button.state {
                            ButtonState::Hover => 1.1,    // 10% bigger on hover
                            ButtonState::Pressed => 1.05, // 5% bigger when pressed
                            _ => 1.0,                     // Normal size
                        }
                    } else {
                        1.0 // No scaling for non-tall buttons
                    };

                    // Calculate scaled dimensions and position
                    let scaled_width = button.position.width * scale;
                    let scaled_height = button.position.height * scale;
                    let scaled_x = actual_x - (scaled_width - button.position.width) / 2.0; // Center the scaling
                    let scaled_y = actual_y - (scaled_height - button.position.height) / 2.0; // Center the scaling

                    let rectangle = Rectangle::new(
                        scaled_x,
                        scaled_y,
                        scaled_width,
                        scaled_height,
                        color_array,
                    )
                    .with_corner_radius(button.style.corner_radius * scale); // Scale corner radius too

                    self.rectangle_renderer.add_rectangle(rectangle);
                }
            }
        }

        // Render the rectangles first (backgrounds)
        self.rectangle_renderer.render(device, render_pass);

        // Then render the icons
        self.icon_renderer.render(device, render_pass);

        // Finally render the text on top
        self.text_renderer.render(render_pass)
    }
}
