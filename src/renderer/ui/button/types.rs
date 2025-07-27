//! # Button Configuration Module
//!
//! This module provides comprehensive configuration types and utilities for UI button components.
//! It includes styling, positioning, and state management for customizable button interfaces.
//!
//! ## Features
//!
//! - **Flexible Styling**: Configurable colors, borders, padding, and typography
//! - **Smart Positioning**: Anchor-based positioning with automatic coordinate calculation
//! - **Responsive Layout**: Multiple spacing modes including wrap-to-content and proportional sizing
//! - **State Management**: Support for normal, hover, pressed, and disabled button states
//! - **DPI Scaling**: Automatic scaling for high-DPI displays
//!
//! ## Usage Example
//!
//! ```rust
//! use crate::button_config::*;
//! use glyphon::Color;
//!
//! // Create a custom button style
//! let style = ButtonStyle {
//!     background_color: Color::rgb(59, 130, 246), // blue-500
//!     hover_color: Color::rgb(37, 99, 235),       // blue-600
//!     spacing: ButtonSpacing::Hbar(0.4),
//!     ..Default::default()
//! };
//!
//! // Position the button
//! let position = ButtonPosition::new(100.0, 50.0, 120.0, 40.0)
//!     .with_anchor(ButtonAnchor::Center);
//! ```

use crate::renderer::text::TextStyle;
use glyphon::{Color, Style, Weight};

/// Text alignment options for button content.
///
/// Determines how text is horizontally aligned within the button's bounds.
/// The default alignment is `Center` for optimal visual balance.
///
/// # Examples
///
/// ```rust
/// let left_aligned = TextAlign::Left;   // Text aligned to left edge
/// let right_aligned = TextAlign::Right; // Text aligned to right edge  
/// let centered = TextAlign::Center;     // Text centered (default)
/// ```
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TextAlign {
    /// Align text to the left edge of the button
    Left,
    /// Align text to the right edge of the button
    Right,
    /// Center text horizontally within the button (default)
    #[default]
    Center,
}

/// Spacing and sizing behavior for buttons.
///
/// Controls how buttons are sized and spaced within their container.
/// Each variant provides different layout strategies for various UI needs.
///
/// # Variants
///
/// - `Wrap`: Button size wraps tightly around text content
/// - `Hbar(f32)`: Button width as proportion of container (0.0-1.0)
/// - `Tall(f32)`: Button fills container height with specified margin
///
/// # Examples
///
/// ```rust
/// let compact = ButtonSpacing::Wrap;        // Minimal size, wraps content
/// let half_width = ButtonSpacing::Hbar(0.5); // 50% of container width
/// let tall = ButtonSpacing::Tall(10.0);     // Full height with 10px margin
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum ButtonSpacing {
    /// Square button that wraps tightly around text content.
    /// Best for compact layouts and icon buttons.
    Wrap,

    /// Horizontal bar button with proportional width.
    ///
    /// The `f32` value represents the proportion of the container width (0.0-1.0).
    /// Values outside this range may cause layout issues.
    ///
    /// # Parameters
    /// - `f32`: Width proportion (0.0 = 0% width, 1.0 = 100% width)
    Hbar(f32),

    /// Tall button that fills the container height with specified margin.
    ///
    /// Useful for sidebar buttons or navigation elements that need
    /// to span the full height of their container.
    ///
    /// # Parameters  
    /// - `f32`: Margin in pixels from container edges
    Tall(f32),
}

/// Comprehensive styling configuration for button appearance.
///
/// Defines all visual aspects of a button including colors for different states,
/// border properties, text styling, and layout behavior. The default implementation
/// provides a modern, accessible design with slate colors and proper contrast ratios.
///
/// # Color Scheme
///
/// The default color scheme uses Tailwind CSS slate colors:
/// - Normal: slate-700 background with slate-50 text
/// - Hover: slate-600 background  
/// - Pressed: slate-800 background
/// - Disabled: slate-400 background
///
/// # DPI Scaling
///
/// Text sizes are automatically scaled based on display DPI using the
/// `dpi_scale` utility function with a base resolution of 1080p.
///
/// # Examples
///
/// ```rust
/// // Use default styling
/// let default_style = ButtonStyle::default();
///
/// // Create custom styling
/// let custom_style = ButtonStyle {
///     background_color: Color::rgb(239, 68, 68), // red-500
///     hover_color: Color::rgb(220, 38, 38),      // red-600
///     corner_radius: 12.0,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ButtonStyle {
    /// Background color in normal state
    pub background_color: Color,

    /// Background color when mouse is hovering over button
    pub hover_color: Color,

    /// Background color when button is being pressed/clicked
    pub pressed_color: Color,

    /// Background color when button is disabled/inactive
    pub disabled_color: Color,

    /// Color of the button border
    pub border_color: Color,

    /// Width of the button border in pixels
    pub border_width: f32,

    /// Radius of rounded corners in pixels (0.0 = square corners)
    pub corner_radius: f32,

    /// Internal padding as (horizontal, vertical) in pixels
    /// Controls spacing between button edge and content
    pub padding: (f32, f32),

    /// Text styling configuration including font, size, and color
    pub text_style: TextStyle,

    /// Horizontal alignment of text within the button
    pub text_align: TextAlign,

    /// Button sizing and spacing behavior
    pub spacing: ButtonSpacing,
}

impl Default for ButtonStyle {
    /// Creates a default button style with modern, accessible design.
    ///
    /// Provides a professional appearance using slate colors with proper
    /// contrast ratios for accessibility. Text sizes are DPI-scaled for
    /// consistent appearance across different display densities.
    ///
    /// # Default Values
    ///
    /// - **Colors**: Slate color scheme (dark gray tones)
    /// - **Border**: 1px solid border with 8px corner radius
    /// - **Padding**: 16px horizontal, 8px vertical
    /// - **Font**: Hanken Grotesk Medium, DPI-scaled 18px
    /// - **Layout**: Center-aligned text, 30% container width
    ///
    /// # Returns
    ///
    /// A `ButtonStyle` instance with sensible defaults for most use cases.
    fn default() -> Self {
        // Calculate DPI scaling based on 1080p baseline
        let scale = crate::renderer::ui::button::utils::dpi_scale(1080.0);

        Self {
            // Slate color scheme for modern appearance
            background_color: Color::rgb(55, 65, 81), // slate-700
            hover_color: Color::rgb(71, 85, 105),     // slate-600
            pressed_color: Color::rgb(30, 41, 59),    // slate-800
            disabled_color: Color::rgb(148, 163, 184), // slate-400
            border_color: Color::rgb(71, 85, 105),    // slate-600

            // Border and corner styling
            border_width: 1.0,
            corner_radius: 8.0,

            // Comfortable padding for text
            padding: (16.0, 8.0),

            // DPI-scaled text configuration
            text_style: TextStyle {
                font_family: "Hanken Grotesk".to_string(),
                font_size: 18.0 * scale,
                line_height: 20.0 * scale,
                color: Color::rgb(248, 250, 252), // slate-50 for contrast
                weight: Weight::MEDIUM,
                style: Style::Normal,
            },

            // Standard layout configuration
            text_align: TextAlign::Center,
            spacing: ButtonSpacing::Hbar(0.3), // 30% container width
        }
    }
}

/// Position and dimensions of a button within its container.
///
/// Defines the button's location, size, and positioning anchor point.
/// The anchor determines how the x,y coordinates are interpreted relative
/// to the button's bounds, enabling flexible positioning strategies.
///
/// # Coordinate System
///
/// - **Origin**: Top-left corner of the container (0,0)
/// - **X-axis**: Increases rightward
/// - **Y-axis**: Increases downward
/// - **Units**: All measurements in pixels
///
/// # Anchoring
///
/// The anchor point affects how coordinates are interpreted:
/// - `TopLeft`: (x,y) represents the top-left corner of the button
/// - `Center`: (x,y) represents the center point of the button
///
/// # Examples
///
/// ```rust
/// // Position button at top-left corner
/// let top_left = ButtonPosition::new(10.0, 10.0, 100.0, 30.0);
///
/// // Position button centered at coordinates  
/// let centered = ButtonPosition::new(200.0, 150.0, 100.0, 30.0)
///     .with_anchor(ButtonAnchor::Center);
///
/// // Calculate actual rendering position
/// let (render_x, render_y) = centered.calculate_actual_position();
/// ```
#[derive(Debug, Clone)]
pub struct ButtonPosition {
    /// X coordinate in pixels (interpretation depends on anchor)
    pub x: f32,

    /// Y coordinate in pixels (interpretation depends on anchor)
    pub y: f32,

    /// Button width in pixels
    pub width: f32,

    /// Button height in pixels  
    pub height: f32,

    /// Anchor point that determines coordinate interpretation
    pub anchor: ButtonAnchor,
}

/// Anchor points for button positioning.
///
/// Determines how the button's (x,y) coordinates are interpreted
/// relative to the button's rectangular bounds. This enables
/// flexible positioning without manual offset calculations.
///
/// # Variants
///
/// - `TopLeft`: Coordinates specify the top-left corner
/// - `Center`: Coordinates specify the center point (default)
///
/// # Visual Reference
///
/// ```text
/// TopLeft Anchor:        Center Anchor:
/// (x,y)●─────────        ┌─────────┐
///      │ Button  │        │  Button │
///      │         │        │    ●(x,y)
///      └─────────┘        └─────────┘
/// ```
#[derive(Debug, Clone, Default)]
pub enum ButtonAnchor {
    /// Position relative to top-left corner
    TopLeft,

    /// Position relative to center point (default)
    #[default]
    Center,
}

impl ButtonPosition {
    /// Creates a new button position with top-left anchoring.
    ///
    /// This is the most common constructor for button positioning,
    /// using the intuitive top-left coordinate system.
    ///
    /// # Parameters
    ///
    /// - `x`: Horizontal position in pixels
    /// - `y`: Vertical position in pixels  
    /// - `width`: Button width in pixels
    /// - `height`: Button height in pixels
    ///
    /// # Returns
    ///
    /// A `ButtonPosition` instance with `ButtonAnchor::TopLeft`
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Create a 100x30 button at position (50, 20)
    /// let position = ButtonPosition::new(50.0, 20.0, 100.0, 30.0);
    /// assert_eq!(position.anchor, ButtonAnchor::TopLeft);
    /// ```
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            anchor: ButtonAnchor::TopLeft,
        }
    }

    /// Sets the anchor point for position interpretation.
    ///
    /// This builder method allows changing the anchor point after
    /// construction, enabling method chaining for concise setup.
    ///
    /// # Parameters
    ///
    /// - `anchor`: The new anchor point to use
    ///
    /// # Returns
    ///
    /// Self with the updated anchor (enables method chaining)
    ///
    /// # Examples
    ///
    /// ```rust
    /// let centered = ButtonPosition::new(200.0, 100.0, 80.0, 25.0)
    ///     .with_anchor(ButtonAnchor::Center);
    /// ```
    pub fn with_anchor(mut self, anchor: ButtonAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// Calculates the actual top-left rendering position.
    ///
    /// Converts the anchored position to absolute top-left coordinates
    /// that can be used directly for rendering. This handles the math
    /// for different anchor points automatically.
    ///
    /// # Algorithm
    ///
    /// - **TopLeft anchor**: Returns coordinates unchanged
    /// - **Center anchor**: Subtracts half width/height to find top-left
    ///
    /// # Returns
    ///
    /// A tuple `(actual_x, actual_y)` representing the top-left corner
    /// in absolute coordinates for rendering purposes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Top-left anchored button
    /// let tl_pos = ButtonPosition::new(10.0, 20.0, 100.0, 30.0);
    /// let (x, y) = tl_pos.calculate_actual_position();
    /// assert_eq!((x, y), (10.0, 20.0)); // Unchanged
    ///
    /// // Center anchored button  
    /// let center_pos = ButtonPosition::new(100.0, 50.0, 80.0, 20.0)
    ///     .with_anchor(ButtonAnchor::Center);
    /// let (x, y) = center_pos.calculate_actual_position();
    /// assert_eq!((x, y), (60.0, 40.0)); // Offset by half dimensions
    /// ```
    pub fn calculate_actual_position(&self) -> (f32, f32) {
        let actual_x = match self.anchor {
            ButtonAnchor::TopLeft => self.x,
            ButtonAnchor::Center => self.x - self.width / 2.0,
        };

        let actual_y = match self.anchor {
            ButtonAnchor::TopLeft => self.y,
            ButtonAnchor::Center => self.y - self.height / 2.0,
        };

        (actual_x, actual_y)
    }
}

/// Current interaction state of a button.
///
/// Represents the button's state in response to user interaction,
/// which determines the visual appearance and behavior. States
/// typically correspond to different colors defined in `ButtonStyle`.
///
/// # State Transitions
///
/// Normal button interaction follows this state flow:
/// ```text
/// Normal ←→ Hover ←→ Pressed
///   ↓         ↓        ↓
/// Disabled  Disabled Disabled
/// ```
///
/// # Usage in Rendering
///
/// The renderer uses this state to select appropriate colors:
/// - `Normal`: Uses `ButtonStyle::background_color`
/// - `Hover`: Uses `ButtonStyle::hover_color`  
/// - `Pressed`: Uses `ButtonStyle::pressed_color`
/// - `Disabled`: Uses `ButtonStyle::disabled_color`
///
/// # Examples
///
/// ```rust
/// let mut state = ButtonState::Normal;
///
/// // Handle mouse hover
/// if mouse_over_button {
///     state = ButtonState::Hover;
/// }
///
/// // Handle mouse click
/// if mouse_pressed {
///     state = ButtonState::Pressed;
/// }
///
/// // Handle disabled condition
/// if !button_enabled {
///     state = ButtonState::Disabled;
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum ButtonState {
    /// Default state when button is not being interacted with
    Normal,

    /// State when mouse cursor is hovering over the button
    Hover,

    /// State when button is being actively clicked/pressed
    Pressed,

    /// State when button is disabled and cannot be activated
    Disabled,
}
