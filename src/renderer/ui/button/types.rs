use crate::renderer::text::TextStyle;
use glyphon::{Color, Style, Weight};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TextAlign {
    Left,
    Right,
    #[default]
    Center,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ButtonSpacing {
    Wrap,      // Square, fits text
    Hbar(f32), // Proportional width (0.0-1.0)
    Tall(f32), // Tall buttons that fill container height with margin
}

#[derive(Debug, Clone, PartialEq)]
pub struct ButtonStyle {
    pub background_color: Color,
    pub hover_color: Color,
    pub pressed_color: Color,
    pub disabled_color: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub corner_radius: f32,
    pub padding: (f32, f32), // (horizontal, vertical)
    pub text_style: TextStyle,
    pub text_align: TextAlign,
    pub spacing: ButtonSpacing,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        let scale = crate::renderer::ui::button::utils::dpi_scale(1080.0); // Assuming a default window height for default values
        Self {
            background_color: Color::rgb(55, 65, 81),  // slate-700
            hover_color: Color::rgb(71, 85, 105),      // slate-600
            pressed_color: Color::rgb(30, 41, 59),     // slate-800
            disabled_color: Color::rgb(148, 163, 184), // slate-400
            border_color: Color::rgb(71, 85, 105),     // slate-600
            border_width: 1.0,
            corner_radius: 8.0,
            padding: (16.0, 8.0),
            text_style: TextStyle {
                font_family: "HankenGrotesk".to_string(),
                font_size: 18.0 * scale,
                line_height: 20.0 * scale,
                color: Color::rgb(248, 250, 252), // slate-50
                weight: Weight::MEDIUM,
                style: Style::Normal,
            },
            text_align: TextAlign::Center,
            spacing: ButtonSpacing::Hbar(0.3),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ButtonPosition {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub anchor: ButtonAnchor,
}

#[derive(Debug, Clone, Default)]
pub enum ButtonAnchor {
    TopLeft,
    #[default]
    Center,
}

impl ButtonPosition {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            anchor: ButtonAnchor::TopLeft,
        }
    }

    pub fn with_anchor(mut self, anchor: ButtonAnchor) -> Self {
        self.anchor = anchor;
        self
    }

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

#[derive(Debug, Clone, PartialEq)]
pub enum ButtonState {
    Normal,
    Hover,
    Pressed,
    Disabled,
}
