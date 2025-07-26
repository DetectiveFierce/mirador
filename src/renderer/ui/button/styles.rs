//! # Button Styles Module
//!
//! This module provides predefined button styles for a user interface application.
//! It implements a cohesive design system with semantic color variants and consistent
//! styling parameters across different button types.
//!
//! ## Design Philosophy
//!
//! The button styles follow modern design principles:
//! - **Semantic Colors**: Each button type (primary, warning, danger) uses colors
//!   that communicate their intended purpose to users
//! - **Accessibility**: High contrast ratios between text and background colors
//! - **Consistency**: Uniform padding, border radius, and typography across all styles
//! - **Interactive States**: Distinct visual feedback for hover, pressed, and disabled states
//! - **DPI Awareness**: Automatic scaling based on display density
//!
//! ## Color Palette
//!
//! The module uses a professional slate-based color scheme with semantic variants:
//! - **Primary (Green)**: For primary actions and positive confirmations
//! - **Warning (Orange)**: For actions that require caution
//! - **Danger (Red)**: For destructive or irreversible actions
//!
//! ## Usage Example
//!
//! ```rust
//! use crate::button_styles::{create_primary_button_style, create_warning_button_style};
//!
//! let primary_style = create_primary_button_style();
//! let warning_style = create_warning_button_style();
//! ```

use crate::renderer::text::TextStyle;
use crate::renderer::ui::button::utils::dpi_scale;
use crate::renderer::ui::button::{ButtonSpacing, ButtonStyle, TextAlign};
use glyphon::{Color, Style, Weight};

// Professional color palette based on modern design systems
// Using a cohesive slate-based color scheme with semantic variants

/// Creates a primary button style with a mint green color scheme.
///
/// Primary buttons are used for the main action on a screen or form.
/// They should be used sparingly - typically only one primary button per view.
///
/// ## Visual Characteristics
/// - **Base Color**: Dark mint green (`rgb(30, 110, 30)`)
/// - **Hover State**: Darker mint green for visual feedback
/// - **Pressed State**: Darkest mint green to indicate active press
/// - **Disabled State**: Muted light green to indicate unavailability
/// - **Typography**: Medium weight, white text for high contrast
///
/// ## Design Rationale
/// The green color palette communicates positive action and success.
/// The slightly desaturated tones provide a professional appearance while
/// maintaining accessibility standards.
///
/// ## Returns
/// A `ButtonStyle` struct configured for primary actions with DPI-aware scaling.
///
/// ## Example
/// ```rust
/// let submit_button_style = create_primary_button_style();
/// ```
pub fn create_primary_button_style() -> ButtonStyle {
    let scale = dpi_scale(1080.0); // Assuming a default window height for default values
    ButtonStyle {
        background_color: Color::rgb(30, 110, 30), // Slightly less saturated, dark mint green
        hover_color: Color::rgb(25, 85, 25),       // Even darker, maintaining hue
        pressed_color: Color::rgb(20, 65, 20),     // Darkest mint for pressed state
        disabled_color: Color::rgb(110, 140, 110), // Muted, lighter mint for disabled state
        border_color: Color::rgb(25, 85, 25),      // Matches hover color
        border_width: 1.0,
        corner_radius: 8.0,
        padding: (16.0, 10.0),
        text_style: TextStyle {
            font_family: "HankenGrotesk".to_string(),
            font_size: 18.0 * scale,
            line_height: 20.0 * scale,
            color: Color::rgb(255, 255, 255), // white
            weight: Weight::MEDIUM,
            style: Style::Normal,
        },
        text_align: TextAlign::Center,
        spacing: ButtonSpacing::Hbar(0.3),
    }
}

/// Creates a warning button style with an orange color scheme.
///
/// Warning buttons are used for actions that require user attention or caution.
/// They indicate that the action might have significant consequences or requires
/// careful consideration before proceeding.
///
/// ## Visual Characteristics
/// - **Base Color**: Dark orange (`rgb(170, 100, 10)`)
/// - **Hover State**: Deeper orange with slight intensity increase
/// - **Pressed State**: Darkest, richest orange for pressed feedback
/// - **Disabled State**: Muted warm yellow-gray for disabled state
/// - **Typography**: Medium weight, white text for readability
///
/// ## Design Rationale
/// Orange is universally recognized as a cautionary color, making it ideal
/// for warning actions. The warm tones maintain approachability while
/// clearly communicating the need for attention.
///
/// ## Use Cases
/// - Form validation warnings
/// - Actions with potential data loss
/// - Confirmation dialogs for significant changes
/// - Temporary or reversible destructive actions
///
/// ## Returns
/// A `ButtonStyle` struct configured for warning actions with DPI-aware scaling.
///
/// ## Example
/// ```rust
/// let reset_form_button_style = create_warning_button_style();
/// ```
pub fn create_warning_button_style() -> ButtonStyle {
    let scale = dpi_scale(1080.0); // Assuming a default window height for default values
    ButtonStyle {
        background_color: Color::rgb(170, 100, 10), // Slightly less saturated, dark orange
        hover_color: Color::rgb(140, 80, 5),        // Deeper, slightly more intense
        pressed_color: Color::rgb(110, 60, 0),      // Darkest, richest for pressed
        disabled_color: Color::rgb(160, 140, 115), // Muted, desaturated warm yellow-gray for disabled
        border_color: Color::rgb(140, 80, 5),      // Matches hover color
        border_width: 1.0,
        corner_radius: 8.0,
        padding: (16.0, 10.0),
        text_style: TextStyle {
            font_family: "HankenGrotesk".to_string(),
            font_size: 18.0 * scale,
            line_height: 20.0 * scale,
            color: Color::rgb(255, 255, 255), // white
            weight: Weight::MEDIUM,
            style: Style::Normal,
        },
        text_align: TextAlign::Center,
        spacing: ButtonSpacing::Hbar(0.3),
    }
}

/// Creates a danger button style with a red color scheme.
///
/// Danger buttons are reserved for destructive or irreversible actions.
/// They should be used only when the action will result in data loss,
/// account deletion, or other permanent changes that cannot be undone.
///
/// ## Visual Characteristics
/// - **Base Color**: Dark red (`rgb(110, 20, 10)`)
/// - **Hover State**: Even darker, more intense red for hover feedback
/// - **Pressed State**: Darkest, most saturated red for pressed state
/// - **Disabled State**: Muted slate color to indicate unavailability
/// - **Typography**: Medium weight, white text for maximum contrast
///
/// ## Design Rationale
/// Red is the universal color for danger and stop actions. The dark,
/// saturated tones convey seriousness while maintaining professional
/// appearance. The high contrast ensures accessibility compliance.
///
/// ## Use Cases
/// - Delete account buttons
/// - Permanent data removal
/// - System shutdown/reset actions
/// - Irreversible configuration changes
///
/// ## UX Considerations
/// Danger buttons should typically be:
/// - Accompanied by confirmation dialogs
/// - Placed away from primary actions
/// - Used sparingly to maintain their impact
/// - Clearly labeled with specific action names
///
/// ## Returns
/// A `ButtonStyle` struct configured for dangerous actions with DPI-aware scaling.
///
/// ## Example
/// ```rust
/// let delete_account_button_style = create_danger_button_style();
/// ```
pub fn create_danger_button_style() -> ButtonStyle {
    let scale = dpi_scale(1080.0); // Assuming a default window height for default values
    ButtonStyle {
        background_color: Color::rgb(110, 20, 10), // Slightly less saturated, dark red
        hover_color: Color::rgb(90, 15, 5),        // Even darker, more intense red
        pressed_color: Color::rgb(70, 10, 0),      // Darkest, most saturated red
        disabled_color: Color::rgb(80, 96, 119),   // Slightly darker slate-500, muted
        border_color: Color::rgb(90, 15, 5),       // Match hover color
        border_width: 1.0,
        corner_radius: 8.0,
        padding: (16.0, 10.0),
        text_style: TextStyle {
            font_family: "HankenGrotesk".to_string(),
            font_size: 18.0 * scale,
            line_height: 20.0 * scale,
            color: Color::rgb(255, 255, 255), // white
            weight: Weight::MEDIUM,
            style: Style::Normal,
        },
        text_align: TextAlign::Center,
        spacing: ButtonSpacing::Hbar(0.3),
    }
}
