use crate::renderer::text::TextStyle;
use crate::renderer::ui::button::utils::dpi_scale;
use crate::renderer::ui::button::{ButtonSpacing, ButtonStyle, TextAlign};
use glyphon::{Color, Style, Weight};

// Professional color palette based on modern design systems
// Using a cohesive slate-based color scheme with semantic variants

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

// Legacy function names for backward compatibility
pub fn create_goldenrod_button_style() -> ButtonStyle {
    create_warning_button_style()
}

pub fn create_lobby_button_style() -> ButtonStyle {
    create_danger_button_style()
}
