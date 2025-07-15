use crate::renderer::rectangle::{Rectangle, RectangleRenderer};
use crate::renderer::text::{TextPosition, TextRenderer, TextStyle};
use egui_wgpu::wgpu::{self, Device, Queue, RenderPass, SurfaceConfiguration};
use glyphon::{Color, Style, Weight};
use std::collections::HashMap;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::window::Window;

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
                font_size: 18.0,
                line_height: 20.0,
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
        // let (window_width, window_height) = (window_size.width as f32, window_size.height as f32);

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

#[derive(Debug)]
pub struct Button {
    pub id: String,
    pub text: String,
    pub style: ButtonStyle,
    pub position: ButtonPosition,
    pub enabled: bool,
    pub visible: bool,
    pub state: ButtonState,
    pub text_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ButtonState {
    Normal,
    Hover,
    Pressed,
    Disabled,
}

impl Button {
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
        }
    }

    pub fn with_style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    pub fn with_position(mut self, position: ButtonPosition) -> Self {
        self.position = position;
        self
    }

    pub fn with_text_align(mut self, text_align: TextAlign) -> Self {
        self.style.text_align = text_align;
        self
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

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

// Color manipulation helpers for glyphon::Color
trait ColorExt {
    fn darken(&self, factor: f32) -> Self;
    fn brighten(&self, factor: f32) -> Self;
    fn saturate(&self, factor: f32) -> Self;
}

impl ColorExt for Color {
    fn darken(&self, factor: f32) -> Self {
        let factor = factor.clamp(0.0, 1.0);
        Color::rgba(
            (self.r() as f32 * (1.0 - factor)) as u8,
            (self.g() as f32 * (1.0 - factor)) as u8,
            (self.b() as f32 * (1.0 - factor)) as u8,
            self.a(),
        )
    }
    fn brighten(&self, factor: f32) -> Self {
        let factor = factor.clamp(0.0, 1.0);
        Color::rgba(
            (self.r() as f32 + (255.0 - self.r() as f32) * factor) as u8,
            (self.g() as f32 + (255.0 - self.g() as f32) * factor) as u8,
            (self.b() as f32 + (255.0 - self.b() as f32) * factor) as u8,
            self.a(),
        )
    }
    fn saturate(&self, factor: f32) -> Self {
        // Convert RGB to HSL, increase saturation, then convert back
        let r = self.r() as f32 / 255.0;
        let g = self.g() as f32 / 255.0;
        let b = self.b() as f32 / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;
        let d = max - min;
        let mut s = if d == 0.0 {
            0.0
        } else {
            d / (1.0 - (2.0 * l - 1.0).abs())
        };
        s = (s + factor).min(1.0);
        // Recompute RGB from HSL (approximate, since hue is not changed)
        // We'll just scale the color channels away from the gray axis
        let gray = l;
        let scale = if s == 0.0 { 0.0 } else { s };
        let new_r = gray + (r - gray) * (1.0 + scale);
        let new_g = gray + (g - gray) * (1.0 + scale);
        let new_b = gray + (b - gray) * (1.0 + scale);
        Color::rgba(
            (new_r.clamp(0.0, 1.0) * 255.0) as u8,
            (new_g.clamp(0.0, 1.0) * 255.0) as u8,
            (new_b.clamp(0.0, 1.0) * 255.0) as u8,
            self.a(),
        )
    }
}

pub struct ButtonManager {
    pub buttons: HashMap<String, Button>,
    pub button_order: Vec<String>, // Track the order buttons were added
    pub text_renderer: TextRenderer,
    pub rectangle_renderer: RectangleRenderer,
    pub window_size: PhysicalSize<u32>,
    pub mouse_position: (f32, f32),
    pub mouse_pressed: bool,
    pub just_clicked: Option<String>,
}

impl ButtonManager {
    pub fn new(
        device: &Device,
        queue: &Queue,
        surface_format: wgpu::TextureFormat,
        window: &Window,
    ) -> Self {
        let text_renderer = TextRenderer::new(device, queue, surface_format, window);
        let rectangle_renderer = RectangleRenderer::new(device, surface_format);
        let window_size = window.inner_size();

        Self {
            buttons: HashMap::new(),
            button_order: Vec::new(), // Initialize the order tracking
            text_renderer,
            rectangle_renderer,
            window_size,
            mouse_position: (0.0, 0.0),
            mouse_pressed: false,
            just_clicked: None,
        }
    }

    pub fn add_button(&mut self, button: Button) {
        let text_id = button.text_id.clone();
        let text = button.text.clone();
        let style = button.style.clone();
        let button_id = button.id.clone();

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
                color: Color::rgba(0, 0, 0, 0), // Start fully transparent
                ..style.text_style.clone()
            }),
            Some(text_position),
        );

        // Track button order
        if !self.button_order.contains(&button_id) {
            self.button_order.push(button_id.clone());
        }

        self.buttons
            .insert(button_with_size.id.clone(), button_with_size);
    }

    pub fn get_button_mut(&mut self, id: &str) -> Option<&mut Button> {
        self.buttons.get_mut(id)
    }

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

    pub fn handle_input(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.mouse_pressed = true;
                self.update_button_states();
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                // Check for button clicks when mouse is released
                for button in self.buttons.values() {
                    if button.visible && button.enabled && button.state == ButtonState::Pressed {
                        // Button was clicked
                        self.just_clicked = Some(button.id.clone());
                        break;
                    }
                }

                self.mouse_pressed = false;
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

    pub fn update_button_states(&mut self) {
        for button in self.buttons.values_mut() {
            if !button.visible || !button.enabled {
                button.state = ButtonState::Disabled;
                // Hide text if not visible
                let _ = self.text_renderer.update_style(
                    &button.text_id,
                    TextStyle {
                        color: Color::rgba(0, 0, 0, 0),
                        ..button.style.text_style.clone()
                    },
                );
                continue;
            }

            let is_hovered = button.contains_point(self.mouse_position.0, self.mouse_position.1);

            // Update button state based on mouse interaction
            if self.mouse_pressed && is_hovered {
                button.state = ButtonState::Pressed;
            } else if is_hovered {
                button.state = ButtonState::Hover;
            } else {
                button.state = ButtonState::Normal;
            }

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

            // Only update style if color or weight changed
            let mut new_style = button.style.text_style.clone();
            new_style.color = text_color;
            new_style.weight = text_weight;
            // Make text visible now that color is correct
            let _ = self
                .text_renderer
                .update_style(&button.text_id, new_style.clone());

            // Recalculate text width and update position for correct centering and wrapping
            let (actual_x, actual_y) = button.position.calculate_actual_position();
            let horizontal_padding = button.style.padding.0;
            let vertical_padding = button.style.padding.1;
            let max_text_width = button.position.width - 2.0 * horizontal_padding;
            let (_min_x, _wrap_width, wrap_height) =
                self.text_renderer.measure_text(&button.text, &new_style);
            button.position.height = wrap_height + 2.0 * vertical_padding;
            let text_x = match button.style.text_align {
                TextAlign::Left => actual_x + horizontal_padding,
                TextAlign::Right => {
                    actual_x + button.position.width - horizontal_padding - _wrap_width
                }
                TextAlign::Center => actual_x + (button.position.width - _wrap_width) / 2.0,
            };
            let text_y = actual_y + vertical_padding;
            let text_position = TextPosition {
                x: text_x,
                y: text_y,
                max_width: Some(max_text_width),
                max_height: Some(wrap_height),
            };
            if let Err(e) = self
                .text_renderer
                .update_position(&button.text_id, text_position)
            {
                println!("Failed to update button position: {}", e);
            }
        }
    }

    pub fn update_button_positions(&mut self) {
        for button in self.buttons.values_mut() {
            let (actual_x, actual_y) = button.position.calculate_actual_position();
            let horizontal_padding = button.style.padding.0;
            let max_text_width = button.position.width - 2.0 * horizontal_padding;
            let (_min_x, wrap_width, wrap_height) = self
                .text_renderer
                .measure_text(&button.text, &button.style.text_style);
            // Do NOT overwrite button.position.height here!
            // Instead, center the text vertically within the button rectangle
            let text_x = match button.style.text_align {
                TextAlign::Left => actual_x + horizontal_padding,
                TextAlign::Right => {
                    actual_x + button.position.width - horizontal_padding - wrap_width
                }
                TextAlign::Center => actual_x + (button.position.width - wrap_width) / 2.0,
            };
            // Center text vertically in the button
            let text_y = actual_y + (button.position.height - wrap_height) / 2.0;
            let text_position = TextPosition {
                x: text_x,
                y: text_y,
                max_width: Some(max_text_width),
                max_height: Some(wrap_height),
            };
            if let Err(e) = self
                .text_renderer
                .update_position(&button.text_id, text_position)
            {
                println!("Failed to update button position: {}", e);
            }
        }
    }

    pub fn resize(&mut self, queue: &Queue, resolution: glyphon::Resolution) {
        self.text_renderer.resize(queue, resolution);
        self.rectangle_renderer
            .resize(resolution.width as f32, resolution.height as f32);
    }

    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        surface_config: &SurfaceConfiguration,
    ) -> Result<(), glyphon::PrepareError> {
        self.text_renderer.prepare(device, queue, surface_config)
    }

    pub fn render(
        &mut self,
        device: &Device,
        render_pass: &mut RenderPass,
    ) -> Result<(), glyphon::RenderError> {
        // Clear previous rectangles
        self.rectangle_renderer.clear_rectangles();

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

                    let rectangle = Rectangle::new(
                        actual_x,
                        actual_y,
                        button.position.width,
                        button.position.height,
                        color_array,
                    )
                    .with_corner_radius(button.style.corner_radius);

                    self.rectangle_renderer.add_rectangle(rectangle);
                }
            }
        }

        // Render the rectangles first (backgrounds)
        self.rectangle_renderer.render(device, render_pass);

        // Then render the text on top
        self.text_renderer.render(render_pass)
    }
}

// Professional color palette based on modern design systems
// Using a cohesive slate-based color scheme with semantic variants

pub fn create_primary_button_style() -> ButtonStyle {
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
            font_size: 18.0,
            line_height: 20.0,
            color: Color::rgb(255, 255, 255), // white
            weight: Weight::MEDIUM,
            style: Style::Normal,
        },
        text_align: TextAlign::Center,
        spacing: ButtonSpacing::Hbar(0.3),
    }
}

pub fn create_warning_button_style() -> ButtonStyle {
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
            font_size: 18.0,
            line_height: 20.0,
            color: Color::rgb(255, 255, 255), // white
            weight: Weight::MEDIUM,
            style: Style::Normal,
        },
        text_align: TextAlign::Center,
        spacing: ButtonSpacing::Hbar(0.3),
    }
}

pub fn create_danger_button_style() -> ButtonStyle {
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
            font_size: 18.0,
            line_height: 20.0,
            color: Color::rgb(255, 255, 255), // white
            weight: Weight::MEDIUM,
            style: Style::Normal,
        },
        text_align: TextAlign::Center,
        spacing: ButtonSpacing::Hbar(0.3),
    }
}

pub fn create_secondary_button_style() -> ButtonStyle {
    ButtonStyle {
        background_color: Color::rgb(248, 250, 252), // slate-50
        hover_color: Color::rgb(241, 245, 249),      // slate-100
        pressed_color: Color::rgb(226, 232, 240),    // slate-200
        disabled_color: Color::rgb(203, 213, 225),   // slate-300
        border_color: Color::rgb(203, 213, 225),     // slate-300
        border_width: 1.0,
        corner_radius: 8.0,
        padding: (16.0, 10.0),
        text_style: TextStyle {
            font_family: "HankenGrotesk".to_string(),
            font_size: 18.0,
            line_height: 20.0,
            color: Color::rgb(30, 41, 59), // slate-800
            weight: Weight::MEDIUM,
            style: Style::Normal,
        },
        text_align: TextAlign::Center,
        spacing: ButtonSpacing::Hbar(0.3),
    }
}

#[allow(dead_code)]
pub fn create_outline_button_style() -> ButtonStyle {
    ButtonStyle {
        background_color: Color::rgba(0, 0, 0, 0), // transparent
        hover_color: Color::rgb(248, 250, 252),    // slate-50
        pressed_color: Color::rgb(241, 245, 249),  // slate-100
        disabled_color: Color::rgba(0, 0, 0, 0),   // transparent
        border_color: Color::rgb(203, 213, 225),   // slate-300
        border_width: 1.0,
        corner_radius: 8.0,
        padding: (16.0, 10.0),
        text_style: TextStyle {
            font_family: "HankenGrotesk".to_string(),
            font_size: 18.0,
            line_height: 20.0,
            color: Color::rgb(55, 65, 81), // slate-700
            weight: Weight::MEDIUM,
            style: Style::Normal,
        },
        text_align: TextAlign::Center,
        spacing: ButtonSpacing::Hbar(0.3),
    }
}

#[allow(dead_code)]
pub fn create_ghost_button_style() -> ButtonStyle {
    ButtonStyle {
        background_color: Color::rgba(0, 0, 0, 0), // transparent
        hover_color: Color::rgb(248, 250, 252),    // slate-50
        pressed_color: Color::rgb(241, 245, 249),  // slate-100
        disabled_color: Color::rgba(0, 0, 0, 0),   // transparent
        border_color: Color::rgba(0, 0, 0, 0),     // transparent
        border_width: 0.0,
        corner_radius: 8.0,
        padding: (16.0, 10.0),
        text_style: TextStyle {
            font_family: "HankenGrotesk".to_string(),
            font_size: 18.0,
            line_height: 20.0,
            color: Color::rgb(55, 65, 81), // slate-700
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

#[allow(dead_code)]
pub fn create_left_aligned_button_style() -> ButtonStyle {
    let mut style = create_secondary_button_style();
    style.text_align = TextAlign::Left;
    style
}

#[allow(dead_code)]
pub fn create_right_aligned_button_style() -> ButtonStyle {
    let mut style = create_secondary_button_style();
    style.text_align = TextAlign::Right;
    style
}
