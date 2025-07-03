use crate::game::GameUIManager;
use egui_wgpu::wgpu::{self, Device, Queue, RenderPass, SurfaceConfiguration};
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, Style,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer as GlyphonTextRenderer, Viewport,
    Weight,
};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use winit::window::Window;

#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub font_family: String,
    pub font_size: f32,
    pub line_height: f32,
    pub color: Color,
    pub weight: Weight,
    pub style: Style,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: "DejaVu Sans".to_string(), // Fallback to a common system font
            font_size: 16.0,
            line_height: 20.0,
            color: Color::rgb(255, 255, 255),
            weight: Weight::NORMAL,
            style: Style::Normal,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextPosition {
    pub x: f32,
    pub y: f32,
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,
}

impl Default for TextPosition {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            max_width: None,
            max_height: None,
        }
    }
}

#[derive(Debug)]
pub struct TextBuffer {
    pub buffer: Buffer,
    pub style: TextStyle,
    pub position: TextPosition,
    pub scale: f32,
    pub visible: bool,
    pub text_content: String, // Store text content for style updates
}

pub struct TextRenderer {
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub viewport: Viewport,
    pub atlas: TextAtlas,
    pub glyph_renderer: GlyphonTextRenderer,
    pub text_buffers: HashMap<String, TextBuffer>,
    pub window_scale_factor: f32,
    pub window_size: winit::dpi::PhysicalSize<u32>,
    pub loaded_fonts: Vec<String>,
}

impl TextRenderer {
    pub fn new(
        device: &Device,
        queue: &Queue,
        surface_format: wgpu::TextureFormat,
        window: &Window,
    ) -> Self {
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, surface_format);
        let glyph_renderer =
            GlyphonTextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);

        let scale_factor = window.scale_factor() as f32;
        let size = window.inner_size();

        let mut renderer = Self {
            font_system,
            swash_cache,
            viewport,
            atlas,
            glyph_renderer,
            text_buffers: HashMap::new(),
            window_scale_factor: scale_factor,
            window_size: size,
            loaded_fonts: Vec::new(),
        };

        // Try to load the custom font, but don't fail if it doesn't exist
        match renderer.load_font(
            "fonts/HankenGrotesk/HankenGrotesk-Medium.ttf",
            "HankenGrotesk",
        ) {
            Ok(_) => println!("Successfully loaded HankenGrotesk font\n"),
            Err(e) => {
                println!(
                    "Failed to load HankenGrotesk font: {}. Using system fonts.",
                    e
                );
                // The font system will fall back to system fonts automatically
            }
        }

        renderer
    }

    /// Initialize all game UI elements
    pub fn initialize_game_ui(&mut self, game_ui: &GameUIManager, width: u32, height: u32) {
        self.create_timer_display(width, height);
        self.create_level_display(game_ui);
        self.create_score_display(game_ui);
        self.create_game_over_display(width, height);
    }

    /// Create the main timer display
    fn create_timer_display(&mut self, width: u32, _height: u32) {
        let timer_style = TextStyle {
            font_family: "HankenGrotesk".to_string(),
            font_size: 48.0,
            line_height: 60.0,
            color: Color::rgb(100, 255, 100),
            weight: Weight::BOLD,
            style: Style::Normal,
        };
        let timer_position = TextPosition {
            x: (width as f32 / 2.0) - 50.0,
            y: 0.0,
            max_width: Some(300.0),
            max_height: Some(100.0),
        };

        self.create_text_buffer(
            "main_timer",
            "60.00",
            Some(timer_style),
            Some(timer_position),
        );
    }

    /// Create the level display
    fn create_level_display(&mut self, game_ui: &GameUIManager) {
        let level_style = TextStyle {
            font_family: "HankenGrotesk".to_string(),
            font_size: 16.0,
            line_height: 20.0,
            color: Color::rgb(255, 255, 150),
            weight: Weight::NORMAL,
            style: Style::Normal,
        };
        let level_position = TextPosition {
            x: 20.0,
            y: 20.0,
            max_width: Some(100.0),
            max_height: Some(25.0),
        };

        self.create_text_buffer(
            "level",
            &game_ui.get_level_text(),
            Some(level_style),
            Some(level_position),
        );
    }

    /// Create the score display
    fn create_score_display(&mut self, game_ui: &GameUIManager) {
        let score_style = TextStyle {
            font_family: "HankenGrotesk".to_string(),
            font_size: 16.0,
            line_height: 20.0,
            color: Color::rgb(150, 255, 255),
            weight: Weight::NORMAL,
            style: Style::Normal,
        };
        let score_position = TextPosition {
            x: 20.0,
            y: 50.0,
            max_width: Some(150.0),
            max_height: Some(25.0),
        };

        self.create_text_buffer(
            "score",
            &game_ui.get_score_text(),
            Some(score_style),
            Some(score_position),
        );
    }

    /// Update all game UI elements - call this every frame
    pub fn update_game_ui(&mut self, game_ui: &mut GameUIManager) -> bool {
        let timer_expired = game_ui.update_timer();

        // Update timer display
        let timer_text = game_ui.get_timer_text();
        if let Err(e) = self.update_text("main_timer", &timer_text) {
            println!("Failed to update main_timer text: {}", e);
        }

        // Update timer color based on remaining time
        let timer_color = game_ui.get_timer_color();
        if let Some(text_buffer) = self.text_buffers.get("main_timer") {
            if text_buffer.style.color != timer_color {
                let mut new_style = text_buffer.style.clone();
                new_style.color = timer_color;
                if let Err(e) = self.update_style("main_timer", new_style) {
                    println!("Failed to update main_timer style: {}", e);
                }
            }
        }

        timer_expired
    }

    /// Load a font from a file path and register it with a name
    pub fn load_font(&mut self, font_path: &str, font_name: &str) -> Result<(), std::io::Error> {
        let font_data = fs::read(Path::new(font_path))?;
        self.font_system.db_mut().load_font_data(font_data);
        self.loaded_fonts.push(font_name.to_string());
        println!("Loaded font: {} from {}\n", font_name, font_path);
        Ok(())
    }

    /// Create a new text buffer with the given ID, text, style, and position
    pub fn create_text_buffer(
        &mut self,
        id: &str,
        text: &str,
        style: Option<TextStyle>,
        position: Option<TextPosition>,
    ) {
        let mut style = style.unwrap_or_default();
        let position = position.unwrap_or_default();

        // If the requested font isn't loaded, fall back to a system font
        if !self.loaded_fonts.contains(&style.font_family) && style.font_family == "HankenGrotesk" {
            println!(
                "Font '{}' not found, falling back to system font\n",
                style.font_family
            );
            style.font_family = "DejaVu Sans".to_string();
        }

        let metrics = Metrics::new(style.font_size, style.line_height);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        // Set buffer size based on position constraints or window size
        let width = position.max_width.unwrap_or(self.window_size.width as f32);
        let height = position
            .max_height
            .unwrap_or(self.window_size.height as f32);

        println!(
            "Creating text buffer '{}' with size: {}x{}, text: '{}'\n",
            id, width, height, text
        );
        buffer.set_size(&mut self.font_system, Some(width), Some(height));

        let attrs = Attrs::new()
            .family(Family::Name(&style.font_family))
            .weight(style.weight)
            .style(style.style);

        buffer.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        // Debug: Check if buffer has any content
        if buffer.lines.is_empty() {
            println!("WARNING: Buffer '{}' has no lines after text setting", id);
        } else {
            println!(
                "Buffer '{}' created with {} lines\n",
                id,
                buffer.lines.len()
            );
        }

        let text_buffer = TextBuffer {
            buffer,
            style,
            position,
            scale: 1.0,
            visible: true,
            text_content: text.to_string(),
        };

        self.text_buffers.insert(id.to_string(), text_buffer);
        println!("Text buffer '{}' added to collection\n", id);
    }

    /// Update the text content of an existing buffer
    pub fn update_text(&mut self, id: &str, text: &str) -> Result<(), String> {
        let text_buffer = self
            .text_buffers
            .get_mut(id)
            .ok_or_else(|| format!("Text buffer '{}' not found", id))?;

        let attrs = Attrs::new()
            .family(Family::Name(&text_buffer.style.font_family))
            .weight(text_buffer.style.weight)
            .style(text_buffer.style.style);

        text_buffer
            .buffer
            .set_text(&mut self.font_system, text, attrs, Shaping::Advanced);
        text_buffer
            .buffer
            .shape_until_scroll(&mut self.font_system, false);

        text_buffer.text_content = text.to_string();
        Ok(())
    }

    /// Update only the text color of an existing buffer
    pub fn update_text_color(&mut self, id: &str, color: Color) -> Result<(), String> {
        let text_buffer = self
            .text_buffers
            .get_mut(id)
            .ok_or_else(|| format!("Text buffer '{}' not found", id))?;

        // Update just the color in the existing style
        let mut style = text_buffer.style.clone();
        style.color = color;

        // Use the existing update_style method to apply the change
        self.update_style(id, style)
    }

    /// Update the style of an existing buffer
    pub fn update_style(&mut self, id: &str, mut style: TextStyle) -> Result<(), String> {
        let text_buffer = self
            .text_buffers
            .get_mut(id)
            .ok_or_else(|| format!("Text buffer '{}' not found", id))?;

        // If the requested font isn't loaded, fall back to a system font
        if !self.loaded_fonts.contains(&style.font_family) && style.font_family == "HankenGrotesk" {
            style.font_family = "DejaVu Sans".to_string();
        }

        // Update metrics if font size or line height changed
        if text_buffer.style.font_size != style.font_size
            || text_buffer.style.line_height != style.line_height
        {
            let metrics = Metrics::new(style.font_size, style.line_height);
            text_buffer
                .buffer
                .set_metrics(&mut self.font_system, metrics);
        }

        text_buffer.style = style;

        // Re-apply text with new attributes using stored content
        let attrs = Attrs::new()
            .family(Family::Name(&text_buffer.style.font_family))
            .weight(text_buffer.style.weight)
            .style(text_buffer.style.style);

        text_buffer.buffer.set_text(
            &mut self.font_system,
            &text_buffer.text_content,
            attrs,
            Shaping::Advanced,
        );
        text_buffer
            .buffer
            .shape_until_scroll(&mut self.font_system, false);
        Ok(())
    }

    /// Update the position of an existing buffer
    pub fn update_position(&mut self, id: &str, position: TextPosition) -> Result<(), String> {
        let text_buffer = self
            .text_buffers
            .get_mut(id)
            .ok_or_else(|| format!("Text buffer '{}' not found", id))?;

        // Update buffer size if max dimensions changed
        if text_buffer.position.max_width != position.max_width
            || text_buffer.position.max_height != position.max_height
        {
            let width = position.max_width.unwrap_or(self.window_size.width as f32);
            let height = position
                .max_height
                .unwrap_or(self.window_size.height as f32);
            text_buffer
                .buffer
                .set_size(&mut self.font_system, Some(width), Some(height));
        }

        text_buffer.position = position;
        Ok(())
    }

    /// Resize the viewport and atlas
    pub fn resize(&mut self, queue: &Queue, resolution: Resolution) {
        self.viewport.update(queue, resolution);
    }

    /// Prepare text rendering for the current frame
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        surface_config: &SurfaceConfiguration,
    ) -> Result<(), glyphon::PrepareError> {
        // Collect all visible text areas
        let text_areas: Vec<TextArea> = self
            .text_buffers
            .iter()
            .filter(|(_, buffer)| buffer.visible)
            .map(|(_, buffer)| TextArea {
                buffer: &buffer.buffer,
                left: buffer.position.x,
                top: buffer.position.y,
                scale: buffer.scale * self.window_scale_factor,
                bounds: TextBounds {
                    left: buffer.position.x as i32,
                    top: buffer.position.y as i32,
                    right: (buffer.position.x
                        + buffer
                            .position
                            .max_width
                            .unwrap_or(surface_config.width as f32))
                        as i32,
                    bottom: (buffer.position.y
                        + buffer
                            .position
                            .max_height
                            .unwrap_or(surface_config.height as f32))
                        as i32,
                },
                default_color: buffer.style.color,
                custom_glyphs: &[],
            })
            .collect();

        // Prepare the text renderer
        self.glyph_renderer.prepare(
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            text_areas,
            &mut self.swash_cache,
        )?;

        Ok(())
    }

    /// Render all visible text buffers
    pub fn render(&mut self, render_pass: &mut RenderPass) -> Result<(), glyphon::RenderError> {
        // Render the text
        self.glyph_renderer
            .render(&self.atlas, &self.viewport, render_pass)?;
        Ok(())
    }

    /// Trim the atlas to free up unused space
    pub fn trim(&mut self) {
        self.atlas.trim();
    }

    pub fn create_game_over_display(&mut self, width: u32, height: u32) {
        // Main "Game Over!" text - large and centered
        let game_over_style = TextStyle {
            font_family: "HankenGrotesk".to_string(),
            font_size: 72.0,
            line_height: 90.0,
            color: Color::rgb(255, 255, 255), // White color
            weight: Weight::BOLD,
            style: Style::Normal,
        };

        // Calculate center position for "Game Over!" text
        // Approximate text width for centering (adjust as needed)
        let text_width = 450.0; // Approximate width for "Game Over!" at 72px
        let text_height = 90.0;

        let game_over_position = TextPosition {
            x: (width as f32 / 2.0) - (text_width),
            y: (height as f32 / 2.0) - (text_height / 2.0) - 50.0, // Offset up a bit
            max_width: Some(text_width),
            max_height: Some(text_height),
        };

        self.create_text_buffer(
            "game_over_title",
            "Game Over!",
            Some(game_over_style),
            Some(game_over_position),
        );

        // Restart instruction text - smaller and below the main text
        let restart_style = TextStyle {
            font_family: "HankenGrotesk".to_string(),
            font_size: 24.0,
            line_height: 30.0,
            color: Color::rgb(255, 255, 255), // White color
            weight: Weight::NORMAL,
            style: Style::Normal,
        };

        // Calculate center position for restart text
        let restart_text_width = 350.0; // Approximate width for restart message
        let restart_text_height = 30.0;

        let restart_position = TextPosition {
            x: (width as f32 / 2.0) - (restart_text_width),
            y: (height as f32 / 2.0) + 40.0, // Below the main text
            max_width: Some(restart_text_width),
            max_height: Some(restart_text_height),
        };

        self.create_text_buffer(
            "game_over_restart",
            "Click anywhere to play again.",
            Some(restart_style),
            Some(restart_position),
        );

        // Initially hide the game over display
        self.hide_game_over_display();
    }

    /// Show the game over display
    pub fn show_game_over_display(&mut self) {
        if let Some(title_buffer) = self.text_buffers.get_mut("game_over_title") {
            title_buffer.visible = true;
        }
        if let Some(restart_buffer) = self.text_buffers.get_mut("game_over_restart") {
            restart_buffer.visible = true;
        }
    }

    /// Hide the game over display
    pub fn hide_game_over_display(&mut self) {
        if let Some(title_buffer) = self.text_buffers.get_mut("game_over_title") {
            title_buffer.visible = false;
        }
        if let Some(restart_buffer) = self.text_buffers.get_mut("game_over_restart") {
            restart_buffer.visible = false;
        }
    }

    /// Check if game over display is currently visible
    pub fn is_game_over_visible(&self) -> bool {
        self.text_buffers
            .get("game_over_title")
            .map(|buffer| buffer.visible)
            .unwrap_or(false)
    }

    /// Update game over display for different screen sizes (call on window resize)
    pub fn update_game_over_position(&mut self, width: u32, height: u32) -> Result<(), String> {
        // Update main title position
        let text_width = 450.0;
        let text_height = 90.0;
        let game_over_position = TextPosition {
            x: (width as f32 / 2.0) - (text_width / 2.0),
            y: (height as f32 / 2.0) - (text_height / 2.0) - 50.0,
            max_width: Some(text_width),
            max_height: Some(text_height),
        };
        self.update_position("game_over_title", game_over_position)?;

        // Update restart text position
        let restart_text_width = 350.0;
        let restart_text_height = 30.0;
        let restart_position = TextPosition {
            x: (width as f32 / 2.0) - (restart_text_width / 2.0),
            y: (height as f32 / 2.0) + 40.0,
            max_width: Some(restart_text_width),
            max_height: Some(restart_text_height),
        };
        self.update_position("game_over_restart", restart_position)?;

        Ok(())
    }
}
