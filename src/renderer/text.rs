use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, Style,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer as GlyphonTextRenderer, Viewport,
    Weight,
};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use wgpu::{self, Device, Queue, RenderPass, SurfaceConfiguration};
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
            font_family: "DejaVu Sans".to_string(),
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
    pub text_content: String,
}

pub struct TextRenderer {
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub viewport: Viewport,
    pub atlas: TextAtlas,
    pub glyph_renderer: GlyphonTextRenderer,
    pub text_buffers: HashMap<String, TextBuffer>,
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

        let size = window.inner_size();

        let mut renderer = Self {
            font_system,
            swash_cache,
            viewport,
            atlas,
            glyph_renderer,
            text_buffers: HashMap::new(),
            window_size: size,
            loaded_fonts: Vec::new(),
        };

        // Try to load the custom font, but don't fail if it doesn't exist
        match renderer.load_font(
            "fonts/HankenGrotesk/HankenGrotesk-Medium.ttf",
            "HankenGrotesk",
        ) {
            Ok(_) => println!("Successfully loaded HankenGrotesk font"),
            Err(e) => {
                println!(
                    "Failed to load HankenGrotesk font: {}. Using system fonts.",
                    e
                );
            }
        }

        renderer
    }

    /// Load a font from a file path and register it with a name
    pub fn load_font(&mut self, font_path: &str, font_name: &str) -> Result<(), std::io::Error> {
        let font_data = fs::read(Path::new(font_path))?;
        self.font_system.db_mut().load_font_data(font_data);
        self.loaded_fonts.push(font_name.to_string());
        println!("Loaded font: {} from {}", font_name, font_path);
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
            style.font_family = "DejaVu Sans".to_string();
        }

        let metrics = Metrics::new(style.font_size, style.line_height);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        // Set buffer size based on position constraints or window size
        let width = position.max_width.unwrap_or(self.window_size.width as f32);
        let height = position
            .max_height
            .unwrap_or(self.window_size.height as f32);

        buffer.set_size(&mut self.font_system, Some(width), Some(height));

        let attrs = Attrs::new()
            .family(Family::Name(&style.font_family))
            .weight(style.weight)
            .style(style.style);

        buffer.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let text_buffer = TextBuffer {
            buffer,
            style,
            position,
            scale: 1.0,
            visible: true,
            text_content: text.to_string(),
        };

        self.text_buffers.insert(id.to_string(), text_buffer);
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

    pub fn resize(&mut self, queue: &Queue, resolution: Resolution) {
        self.viewport.update(queue, resolution);
    }

    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        _surface_config: &SurfaceConfiguration,
    ) -> Result<(), glyphon::PrepareError> {
        let mut text_areas = Vec::new();

        for text_buffer in self.text_buffers.values() {
            if !text_buffer.visible {
                continue;
            }

            let bounds = TextBounds {
                left: text_buffer.position.x as i32,
                top: text_buffer.position.y as i32,
                right: (text_buffer.position.x
                    + text_buffer
                        .position
                        .max_width
                        .unwrap_or(self.window_size.width as f32)) as i32,
                bottom: (text_buffer.position.y
                    + text_buffer
                        .position
                        .max_height
                        .unwrap_or(self.window_size.height as f32)) as i32,
            };

            let text_area = TextArea {
                buffer: &text_buffer.buffer,
                left: text_buffer.position.x,
                top: text_buffer.position.y,
                scale: text_buffer.scale,
                bounds,
                default_color: text_buffer.style.color,
                custom_glyphs: &[],
            };

            text_areas.push(text_area);
        }

        self.glyph_renderer.prepare(
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            text_areas,
            &mut self.swash_cache,
        )
    }

    pub fn render(&mut self, render_pass: &mut RenderPass) -> Result<(), glyphon::RenderError> {
        self.glyph_renderer
            .render(&self.atlas, &self.viewport, render_pass)
    }

    pub fn measure_text(&mut self, text: &str, style: &TextStyle) -> (f32, f32, f32) {
        let metrics = Metrics::new(style.font_size, style.line_height);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        let attrs = Attrs::new()
            .family(Family::Name(&style.font_family))
            .weight(style.weight)
            .style(style.style);

        buffer.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        // Calculate text dimensions from layout runs
        let mut min_x = f32::MAX;
        let mut max_x: f32 = 0.0;
        let mut height: f32 = 0.0;

        for run in buffer.layout_runs() {
            if let Some(first_glyph) = run.glyphs.first() {
                min_x = min_x.min(first_glyph.x);
            }
            if let Some(last_glyph) = run.glyphs.last() {
                max_x = max_x.max(last_glyph.x + last_glyph.w);
            }
            height += run.line_height;
        }

        // If no runs, estimate based on text length and font size
        if min_x == f32::MAX && !text.is_empty() {
            min_x = 0.0;
            max_x = text.len() as f32 * style.font_size * 0.6;
            height = style.line_height;
        }

        let width = max_x - min_x;
        (min_x, width, height)
    }

    pub fn create_game_over_display(&mut self, width: u32, height: u32) {
        // Virtual DPI scaling based on reference height
        let reference_height = 1080.0;
        let scale = (height as f32 / reference_height).clamp(0.7, 2.0);
        // Main "Game Over!" text - large and centered
        let game_over_style = TextStyle {
            font_family: "HankenGrotesk".to_string(),
            font_size: (72.0 * scale).clamp(32.0, 180.0),
            line_height: (90.0 * scale).clamp(36.0, 220.0),
            color: Color::rgb(255, 255, 255), // White color
            weight: Weight::BOLD,
            style: Style::Normal,
        };
        // Calculate center position for "Game Over!" text
        let text_width = 450.0 * scale; // Approximate width for "Game Over!" at scaled size
        let text_height = 90.0 * scale;
        let game_over_position = TextPosition {
            x: (width as f32 / 2.0) - (text_width),
            y: (height as f32 / 2.0) - (text_height / 2.0) - 50.0 * scale, // Offset up a bit
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
            font_size: (24.0 * scale).clamp(12.0, 60.0),
            line_height: (30.0 * scale).clamp(16.0, 80.0),
            color: Color::rgb(255, 255, 255), // White color
            weight: Weight::NORMAL,
            style: Style::Normal,
        };
        let restart_text_width = 350.0 * scale; // Approximate width for restart message
        let restart_text_height = 30.0 * scale;
        let restart_position = TextPosition {
            x: (width as f32 / 2.0) - (restart_text_width),
            y: (height as f32 / 2.0) + 40.0 * scale, // Below the main text
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
    /// Update game over display for different screen sizes (call on window resize)
    pub fn update_game_over_position(&mut self, width: u32, height: u32) -> Result<(), String> {
        let reference_height = 1080.0;
        let scale = (height as f32 / reference_height).clamp(0.7, 2.0);
        // Get the styles from existing buffers to measure text
        let game_over_style = self
            .text_buffers
            .get("game_over_title")
            .map(|buffer| buffer.style.clone())
            .unwrap_or_else(|| TextStyle {
                font_family: "HankenGrotesk".to_string(),
                font_size: (72.0 * scale).clamp(32.0, 180.0),
                line_height: (90.0 * scale).clamp(36.0, 220.0),
                color: Color::rgb(255, 255, 255),
                weight: Weight::BOLD,
                style: Style::Normal,
            });
        let restart_style = self
            .text_buffers
            .get("game_over_restart")
            .map(|buffer| buffer.style.clone())
            .unwrap_or_else(|| TextStyle {
                font_family: "HankenGrotesk".to_string(),
                font_size: (24.0 * scale).clamp(12.0, 60.0),
                line_height: (30.0 * scale).clamp(16.0, 80.0),
                color: Color::rgb(255, 255, 255),
                weight: Weight::NORMAL,
                style: Style::Normal,
            });
        // Measure the actual text dimensions
        let (_, text_width, text_height) = self.measure_text("Game Over!", &game_over_style);
        let (_, restart_text_width, restart_text_height) =
            self.measure_text("Click anywhere to play again.", &restart_style);
        // Update main title position
        let game_over_position = TextPosition {
            x: (width as f32 / 2.0) - (text_width / 2.0),
            y: (height as f32 / 2.0) - (text_height / 2.0) - 50.0 * scale,
            max_width: Some(text_width + 20.0 * scale), // Add some padding
            max_height: Some(text_height + 10.0 * scale), // Add some padding
        };
        self.update_position("game_over_title", game_over_position)?;
        // Update restart text position
        let restart_position = TextPosition {
            x: (width as f32 / 2.0) - (restart_text_width / 2.0),
            y: (height as f32 / 2.0) + 40.0 * scale,
            max_width: Some(restart_text_width + 20.0 * scale), // Add some padding
            max_height: Some(restart_text_height + 10.0 * scale), // Add some padding
        };
        self.update_position("game_over_restart", restart_position)?;
        Ok(())
    }

    /// Handle game over text auto-sizing and positioning (similar to title screen)
    /// This function dynamically updates font sizes, line heights, and positions based on window dimensions
    pub fn handle_game_over_text(&mut self, width: u32, height: u32) {
        let width = width as f32;
        let height = height as f32;
        
        // Apply DPI scaling based on height (consistent with other UI elements)
        let reference_height = 1080.0;
        let scale = (height / reference_height).clamp(0.7, 2.0);
        
        // Dynamically scale font sizes with DPI scaling
        let title_font_size = (width * 0.12 * scale).clamp(48.0, 240.0); // 12% of width, min 48, max 240
        let title_line_height = (title_font_size * 1.25).clamp(60.0, 300.0);
        let subtitle_font_size = (width * 0.025 * scale).clamp(16.0, 120.0); // 2.5% of width, min 16, max 120
        let subtitle_line_height = (subtitle_font_size * 1.3).clamp(20.0, 156.0);
        
        // Update game over title
        if let Some(title_buffer) = self.text_buffers.get_mut("game_over_title") {
            let mut style = title_buffer.style.clone();
            style.font_size = title_font_size;
            style.line_height = title_line_height;
            let text = title_buffer.text_content.clone();
            
            let _ = self.update_style("game_over_title", style.clone());
            let (_min_x, text_width, text_height) = self.measure_text(&text, &style);
            
            let pos = TextPosition {
                x: (width / 2.0) - (text_width / 2.0),
                y: (height / 2.0) - (text_height / 2.0) - 60.0 * scale,
                max_width: Some(text_width + 40.0 * scale), // Add padding to prevent clipping
                max_height: Some(text_height + 20.0 * scale),
            };
            let _ = self.update_position("game_over_title", pos);
        }
        
        // Update restart text
        if let Some(restart_buffer) = self.text_buffers.get_mut("game_over_restart") {
            let mut style = restart_buffer.style.clone();
            style.font_size = subtitle_font_size;
            style.line_height = subtitle_line_height;
            let text = restart_buffer.text_content.clone();
            
            let _ = self.update_style("game_over_restart", style.clone());
            let (_min_x, text_width, text_height) = self.measure_text(&text, &style);
            
            let pos = TextPosition {
                x: (width / 2.0) - (text_width / 2.0),
                y: (height / 2.0) + 60.0 * scale,
                max_width: Some(text_width + 60.0 * scale), // Add more padding for subtitle to prevent clipping
                max_height: Some(text_height + 30.0 * scale),
            };
            let _ = self.update_position("game_over_restart", pos);
        }
    }

    /// Handle score and level text auto-sizing and positioning (smaller than subtitles)
    /// This function dynamically updates font sizes, line heights, and positions based on window dimensions
    pub fn handle_score_and_level_text(&mut self, width: u32, height: u32) {
        let width = width as f32;
        let height = height as f32;
        let reference_height = 1080.0;
        let scale = (height / reference_height).clamp(0.7, 2.0);
        // Make this text smaller than subtitles, but more legible on high-DPI
        let font_size = (width * 0.022 * scale).clamp(16.0, 48.0); // 2.2% of width, min 16, max 48
        let line_height = (font_size * 1.25).clamp(20.0, 60.0);
        let padding_x = 32.0 * scale;
        let padding_y = 24.0 * scale;
        // Score text
        if let Some(score_buffer) = self.text_buffers.get_mut("score") {
            let mut style = score_buffer.style.clone();
            style.font_size = font_size;
            style.line_height = line_height;
            let text = score_buffer.text_content.clone();
            let _ = self.update_style("score", style.clone());
            let (_min_x, text_width, text_height) = self.measure_text(&text, &style);
            let pos = TextPosition {
                x: padding_x,
                y: padding_y,
                max_width: Some(text_width + 20.0 * scale),
                max_height: Some(text_height + 10.0 * scale),
            };
            let _ = self.update_position("score", pos);
        }
        // Level text (place below score)
        if let Some(level_buffer) = self.text_buffers.get_mut("level") {
            let mut style = level_buffer.style.clone();
            style.font_size = font_size;
            style.line_height = line_height;
            let text = level_buffer.text_content.clone();
            let _ = self.update_style("level", style.clone());
            let (_min_x, text_width, text_height) = self.measure_text(&text, &style);
            let pos = TextPosition {
                x: padding_x,
                y: padding_y + line_height + 8.0 * scale, // 8px vertical gap
                max_width: Some(text_width + 20.0 * scale),
                max_height: Some(text_height + 10.0 * scale),
            };
            let _ = self.update_position("level", pos);
        }
    }
}
