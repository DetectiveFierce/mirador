//! # Text Rendering System
//!
//! A comprehensive text rendering system built on top of the Glyphon library for WGPU-based applications.
//! This module provides a high-level interface for rendering text with custom fonts, styles, and positioning
//! in games or graphical applications.
//!
//! ## Features
//!
//! - Custom font loading and management
//! - Dynamic text styling (font size, color, weight, style)
//! - Flexible positioning with constraints
//! - Text measurement and layout calculation
//! - Game-specific UI elements (game over screens, score displays)
//! - DPI-aware scaling for different screen sizes
//! - Multiple text buffer management with unique IDs
//!
//! ## Usage
//!
//! ```rust
//! // Create a text renderer
//! let mut text_renderer = TextRenderer::new(&device, &queue, surface_format, &window);
//!
//! // Create a text buffer
//! text_renderer.create_text_buffer(
//!     "my_text",
//!     "Hello, World!",
//!     Some(TextStyle::default()),
//!     Some(TextPosition::default())
//! );
//!
//! // Render in your main loop
//! text_renderer.prepare(&device, &queue, &surface_config)?;
//! text_renderer.render(&mut render_pass)?;
//! ```

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

/// Defines the visual styling properties for text rendering.
///
/// This struct encapsulates all the visual aspects of text including font family,
/// size, color, and typographic properties like weight and style.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    /// The font family name (e.g., "Arial", "HankenGrotesk")
    pub font_family: String,
    /// Font size in pixels
    pub font_size: f32,
    /// Line height (spacing between lines) in pixels
    pub line_height: f32,
    /// Text color in RGB format
    pub color: Color,
    /// Font weight (normal, bold, etc.)
    pub weight: Weight,
    /// Font style (normal, italic, etc.)
    pub style: Style,
}

impl Default for TextStyle {
    /// Creates a default text style with sensible defaults.
    ///
    /// Returns a white, 16px "DejaVu Sans" font with normal weight and style.
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

/// Defines the positioning and size constraints for text rendering.
///
/// This struct controls where text appears on screen and how much space it can occupy.
/// Max width and height are optional - if not specified, the text will use available space.
#[derive(Debug, Clone)]
pub struct TextPosition {
    /// X coordinate (left edge) in pixels from screen origin
    pub x: f32,
    /// Y coordinate (top edge) in pixels from screen origin  
    pub y: f32,
    /// Maximum width constraint in pixels (None = no constraint)
    pub max_width: Option<f32>,
    /// Maximum height constraint in pixels (None = no constraint)
    pub max_height: Option<f32>,
}

impl Default for TextPosition {
    /// Creates a default position at the origin (0,0) with no size constraints.
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            max_width: None,
            max_height: None,
        }
    }
}

/// A text buffer that holds rendered text with its associated styling and positioning.
///
/// This struct represents a single piece of text that can be rendered to the screen.
/// Each buffer maintains its own content, style, position, and visibility state.
#[derive(Debug)]
pub struct TextBuffer {
    /// The underlying Glyphon buffer containing shaped text
    pub buffer: Buffer,
    /// Visual styling properties for this text
    pub style: TextStyle,
    /// Position and size constraints
    pub position: TextPosition,
    /// Scaling factor for the text (1.0 = normal size)
    pub scale: f32,
    /// Whether this text buffer should be rendered
    pub visible: bool,
    /// The original text content (stored for re-styling)
    pub text_content: String,
}

/// The main text rendering system that manages fonts, text buffers, and rendering.
///
/// This struct provides a high-level interface for text rendering in WGPU applications.
/// It handles font loading, text shaping, layout, and rendering through the Glyphon library.
/// Multiple text buffers can be managed simultaneously using unique string identifiers.
pub struct TextRenderer {
    /// Font system for loading and managing fonts
    pub font_system: FontSystem,
    /// Cache for glyph rasterization
    pub swash_cache: SwashCache,
    /// Viewport for coordinate system management
    pub viewport: Viewport,
    /// Texture atlas for storing glyph textures
    pub atlas: TextAtlas,
    /// The underlying Glyphon renderer
    pub glyph_renderer: GlyphonTextRenderer,
    /// Collection of all text buffers indexed by unique IDs
    pub text_buffers: HashMap<String, TextBuffer>,
    /// Current window size for layout calculations
    pub window_size: winit::dpi::PhysicalSize<u32>,
    /// List of successfully loaded custom font names
    pub loaded_fonts: Vec<String>,
}

impl TextRenderer {
    /// Creates a new text renderer instance.
    ///
    /// Initializes all the necessary Glyphon components and attempts to load
    /// a custom HankenGrotesk font. If the font loading fails, it falls back
    /// to system fonts gracefully.
    ///
    /// # Arguments
    ///
    /// * `device` - WGPU device for GPU operations
    /// * `queue` - WGPU command queue
    /// * `surface_format` - The texture format of the render surface
    /// * `window` - Window reference for getting dimensions
    ///
    /// # Returns
    ///
    /// A new `TextRenderer` instance ready for use.
    pub fn new(
        device: &Device,
        queue: &Queue,
        surface_format: wgpu::TextureFormat,
        window: &Window,
    ) -> Self {
        use crate::benchmarks::{BenchmarkConfig, Profiler};

        // Initialize profiler for TextRenderer initialization benchmarking
        let mut init_profiler = Profiler::new(BenchmarkConfig {
            enabled: true,
            print_results: false, // Respect user's console output preference
            write_to_file: false,
            min_duration_threshold: std::time::Duration::from_micros(1),
            max_samples: 1000,
        });

        // Benchmark font system initialization
        init_profiler.start_section("font_system_initialization");
        let font_system = FontSystem::new();
        init_profiler.end_section("font_system_initialization");

        // Benchmark swash cache creation
        init_profiler.start_section("swash_cache_creation");
        let swash_cache = SwashCache::new();
        init_profiler.end_section("swash_cache_creation");

        // Benchmark cache creation
        init_profiler.start_section("cache_creation");
        let cache = Cache::new(device);
        init_profiler.end_section("cache_creation");

        // Benchmark viewport creation
        init_profiler.start_section("viewport_creation");
        let viewport = Viewport::new(device, &cache);
        init_profiler.end_section("viewport_creation");

        // Benchmark text atlas creation
        init_profiler.start_section("text_atlas_creation");
        let mut atlas = TextAtlas::new(device, queue, &cache, surface_format);
        init_profiler.end_section("text_atlas_creation");

        // Benchmark glyph renderer creation
        init_profiler.start_section("glyph_renderer_creation");
        let glyph_renderer =
            GlyphonTextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);
        init_profiler.end_section("glyph_renderer_creation");

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

        // Benchmark custom font loading
        init_profiler.start_section("custom_font_loading");
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
        init_profiler.end_section("custom_font_loading");

        renderer
    }

    /// Loads a custom font from a file and registers it with the font system.
    ///
    /// This method reads font data from the specified file path and adds it to
    /// the font database. The font can then be referenced by the provided name
    /// in text styles.
    ///
    /// # Arguments
    ///
    /// * `font_path` - Path to the font file (typically .ttf or .otf)
    /// * `font_name` - Name to register the font under for later reference
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the font was loaded successfully
    /// * `Err(std::io::Error)` if the file could not be read
    ///
    /// # Example
    ///
    /// ```rust
    /// renderer.load_font("fonts/MyFont.ttf", "MyCustomFont")?;
    /// ```
    pub fn load_font(&mut self, font_path: &str, font_name: &str) -> Result<(), std::io::Error> {
        let font_data = fs::read(Path::new(font_path))?;
        self.font_system.db_mut().load_font_data(font_data);
        self.loaded_fonts.push(font_name.to_string());
        println!("Loaded font: {} from {}", font_name, font_path);
        Ok(())
    }

    /// Creates a new text buffer with the specified content, style, and position.
    ///
    /// This method creates a new text buffer that can be rendered to the screen.
    /// Each buffer is identified by a unique string ID that can be used to update
    /// or reference the buffer later.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this text buffer
    /// * `text` - The text content to display
    /// * `style` - Optional text styling (uses default if None)
    /// * `position` - Optional positioning (uses default if None)
    ///
    /// # Behavior
    ///
    /// - If the requested font family isn't loaded, falls back to "DejaVu Sans"
    /// - Automatically shapes the text for proper rendering
    /// - Sets buffer size based on position constraints or window size
    ///
    /// # Example
    ///
    /// ```rust
    /// let style = TextStyle {
    ///     font_size: 24.0,
    ///     color: Color::rgb(255, 0, 0),
    ///     ..Default::default()
    /// };
    /// let position = TextPosition {
    ///     x: 100.0,
    ///     y: 50.0,
    ///     max_width: Some(300.0),
    ///     ..Default::default()
    /// };
    /// renderer.create_text_buffer("title", "Hello World", Some(style), Some(position));
    /// ```
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

    /// Updates the visual style of an existing text buffer.
    ///
    /// This method allows you to change the appearance of existing text without
    /// recreating the entire buffer. The text content remains the same but is
    /// re-shaped with the new styling attributes.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the text buffer to update
    /// * `style` - The new text style to apply
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the style was updated successfully
    /// * `Err(String)` if the text buffer with the given ID was not found
    ///
    /// # Behavior
    ///
    /// - Falls back to system fonts if the requested font isn't loaded
    /// - Updates metrics if font size or line height changed
    /// - Re-shapes the text with new attributes
    ///
    /// # Example
    ///
    /// ```rust
    /// let new_style = TextStyle {
    ///     font_size: 32.0,
    ///     color: Color::rgb(0, 255, 0),
    ///     weight: Weight::BOLD,
    ///     ..Default::default()
    /// };
    /// renderer.update_style("title", new_style)?;
    /// ```
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

    /// Updates the position and size constraints of an existing text buffer.
    ///
    /// This method allows you to move text around the screen or change its
    /// maximum dimensions without affecting the content or styling.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the text buffer to update
    /// * `position` - The new position and size constraints
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the position was updated successfully  
    /// * `Err(String)` if the text buffer with the given ID was not found
    ///
    /// # Behavior
    ///
    /// - Updates buffer size if max dimensions changed
    /// - Uses window size as fallback for unconstrained dimensions
    ///
    /// # Example
    ///
    /// ```rust
    /// let new_position = TextPosition {
    ///     x: 200.0,
    ///     y: 100.0,
    ///     max_width: Some(400.0),
    ///     max_height: Some(200.0),
    /// };
    /// renderer.update_position("title", new_position)?;
    /// ```
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

    /// Updates the viewport when the window is resized.
    ///
    /// This method should be called whenever the window size changes to ensure
    /// that text rendering coordinates remain correct and the viewport is properly
    /// synchronized with the new window dimensions.
    ///
    /// # Arguments
    ///
    /// * `queue` - WGPU command queue for GPU operations
    /// * `resolution` - New resolution/size information from the window resize event
    ///
    /// # Behavior
    ///
    /// - Updates the internal viewport with new resolution information
    /// - Ensures text rendering coordinates remain accurate after window resize
    /// - Maintains proper scaling and positioning of all text elements
    ///
    /// # Example
    ///
    /// ```rust
    /// // In your window resize event handler
    /// renderer.resize(&queue, Resolution::new(new_width, new_height));
    /// ```
    pub fn resize(&mut self, queue: &Queue, resolution: Resolution) {
        self.viewport.update(queue, resolution);
    }

    /// Prepares all visible text buffers for rendering.
    ///
    /// This method must be called before rendering to update the texture atlas
    /// with any changes to text content, styling, or positioning. It processes
    /// all visible text buffers and prepares them for GPU rendering.
    ///
    /// # Arguments
    ///
    /// * `device` - WGPU device for GPU operations
    /// * `queue` - WGPU command queue
    /// * `_surface_config` - Surface configuration (currently unused)
    ///
    /// # Returns
    ///
    /// * `Ok(())` if preparation was successful
    /// * `Err(glyphon::PrepareError)` if preparation failed
    ///
    /// # Behavior
    ///
    /// - Only processes visible text buffers
    /// - Calculates text bounds based on position and constraints
    /// - Updates the glyph texture atlas as needed
    ///
    /// # Example
    ///
    /// ```rust
    /// // In your render loop
    /// renderer.prepare(&device, &queue, &surface_config)?;
    /// renderer.render(&mut render_pass)?;
    /// ```
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

    /// Renders all prepared text to the current render pass.
    ///
    /// This method should be called during your render loop after calling `prepare()`.
    /// It renders all visible text buffers that were prepared in the previous step.
    ///
    /// # Arguments
    ///
    /// * `render_pass` - The active WGPU render pass to draw into
    ///
    /// # Returns
    ///
    /// * `Ok(())` if rendering was successful
    /// * `Err(glyphon::RenderError)` if rendering failed
    ///
    /// # Example
    ///
    /// ```rust
    /// // In your render loop
    /// let mut render_pass = encoder.begin_render_pass(&render_pass_descriptor);
    /// renderer.render(&mut render_pass)?;
    /// ```
    pub fn render(&mut self, render_pass: &mut RenderPass) -> Result<(), glyphon::RenderError> {
        self.glyph_renderer
            .render(&self.atlas, &self.viewport, render_pass)
    }

    /// Measures the dimensions of text without creating a buffer.
    ///
    /// This utility method calculates how much space text will occupy when
    /// rendered with the given style. Useful for layout calculations and
    /// positioning decisions.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content to measure
    /// * `style` - The text style to use for measurement
    ///
    /// # Returns
    ///
    /// A tuple containing `(min_x, width, height)`:
    /// - `min_x`: The leftmost x coordinate of the text
    /// - `width`: The total width of the text
    /// - `height`: The total height of the text
    ///
    /// # Behavior
    ///
    /// - Creates a temporary buffer for measurement
    /// - Calculates dimensions from layout runs
    /// - Provides fallback estimates for empty text
    ///
    /// # Example
    ///
    /// ```rust
    /// let style = TextStyle::default();
    /// let (min_x, width, height) = renderer.measure_text("Hello World", &style);
    /// println!("Text dimensions: {}x{} at x={}", width, height, min_x);
    /// ```
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

    /// Creates a game over display with title and restart instruction.
    ///
    /// This convenience method creates two text buffers for a typical game over screen:
    /// - A large "Game Over!" title
    /// - A smaller instruction to restart the game
    ///
    /// The display uses DPI-aware scaling and is initially hidden.
    ///
    /// # Arguments
    ///
    /// * `width` - Screen width in pixels for positioning calculations
    /// * `height` - Screen height in pixels for positioning calculations
    ///
    /// # Behavior
    ///
    /// - Creates buffers with IDs "game_over_title" and "game_over_restart"
    /// - Applies DPI scaling based on a 1080p reference resolution
    /// - Centers text horizontally and vertically
    /// - Initially hides both text buffers
    ///
    /// # Example
    ///
    /// ```rust
    /// // Create the game over display
    /// renderer.create_game_over_display(1920, 1080);
    ///
    /// // Show it when the game ends
    /// renderer.show_game_over_display();
    /// ```
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

    /// Makes the game over display visible.
    ///
    /// Shows both the game over title and restart instruction text that were
    /// created by `create_game_over_display()`. This method should be called
    /// when the game ends to display the game over screen to the player.
    ///
    /// # Behavior
    ///
    /// - Sets the visibility of "game_over_title" buffer to true
    /// - Sets the visibility of "game_over_restart" buffer to true
    /// - Both text elements will be rendered in the next render cycle
    ///
    /// # Prerequisites
    ///
    /// Requires that `create_game_over_display()` has been called previously
    /// to create the necessary text buffers.
    ///
    /// # Example
    ///
    /// ```rust
    /// // When the game ends
    /// renderer.show_game_over_display();
    /// ```
    pub fn show_game_over_display(&mut self) {
        if let Some(title_buffer) = self.text_buffers.get_mut("game_over_title") {
            title_buffer.visible = true;
        }
        if let Some(restart_buffer) = self.text_buffers.get_mut("game_over_restart") {
            restart_buffer.visible = true;
        }
    }

    /// Hides the game over display.
    ///
    /// Hides both the game over title and restart instruction text, making
    /// them invisible during rendering. This method should be called when
    /// starting a new game or transitioning away from the game over state.
    ///
    /// # Behavior
    ///
    /// - Sets the visibility of "game_over_title" buffer to false
    /// - Sets the visibility of "game_over_restart" buffer to false
    /// - Both text elements will not be rendered in the next render cycle
    ///
    /// # Prerequisites
    ///
    /// Requires that `create_game_over_display()` has been called previously
    /// to create the necessary text buffers.
    ///
    /// # Example
    ///
    /// ```rust
    /// // When starting a new game
    /// renderer.hide_game_over_display();
    /// ```
    pub fn hide_game_over_display(&mut self) {
        if let Some(title_buffer) = self.text_buffers.get_mut("game_over_title") {
            title_buffer.visible = false;
        }
        if let Some(restart_buffer) = self.text_buffers.get_mut("game_over_restart") {
            restart_buffer.visible = false;
        }
    }

    /// Checks if the game over display is currently visible.
    ///
    /// This method can be used to determine the current state of the game over
    /// display, which is useful for game state management and UI logic.
    ///
    /// # Returns
    ///
    /// `true` if the game over title is visible, `false` otherwise
    ///
    /// # Behavior
    ///
    /// - Checks the visibility state of the "game_over_title" buffer
    /// - Returns false if the buffer doesn't exist
    /// - Assumes that both title and restart text have the same visibility state
    ///
    /// # Example
    ///
    /// ```rust
    /// // Check if game over screen is currently shown
    /// if renderer.is_game_over_visible() {
    ///     // Handle game over state logic
    /// }
    /// ```
    pub fn is_game_over_visible(&self) -> bool {
        self.text_buffers
            .get("game_over_title")
            .map(|buffer| buffer.visible)
            .unwrap_or(false)
    }

    /// Updates game over display positioning for different screen sizes.
    ///
    /// This method should be called when the window is resized to ensure
    /// the game over display remains properly centered and scaled.
    ///
    /// # Arguments
    ///
    /// * `width` - New screen width in pixels
    /// * `height` - New screen height in pixels
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the update was successful
    /// * `Err(String)` if the game over buffers don't exist
    ///
    /// # Behavior
    ///
    /// - Applies DPI scaling based on screen height
    /// - Measures actual text dimensions for precise centering
    /// - Updates both title and restart text positions
    /// - Adds padding to prevent text clipping

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

    /// Dynamically adjusts game over text sizing and positioning based on window dimensions.
    ///
    /// This method provides responsive text scaling for the game over display, similar to the title screen.
    /// It automatically adjusts font sizes, line heights, and positions to maintain readability across
    /// different screen sizes and resolutions.
    ///
    /// # Arguments
    ///
    /// * `width` - Current window width in pixels
    /// * `height` - Current window height in pixels
    ///
    /// # Behavior
    ///
    /// - Applies DPI scaling based on a 1080p reference resolution
    /// - Scales title font size to 12% of window width (clamped between 48-240px)
    /// - Scales subtitle font size to 2.5% of window width (clamped between 16-120px)
    /// - Centers text horizontally and positions vertically with appropriate spacing
    /// - Adds padding to prevent text clipping at edges
    ///
    /// # Example
    ///
    /// ```rust
    /// // Call when window is resized or game over display is shown
    /// renderer.handle_game_over_text(1920, 1080);
    /// ```
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

    /// Dynamically adjusts score and level text sizing and positioning for responsive UI.
    ///
    /// This method provides responsive text scaling for score and level displays, making them
    /// smaller than subtitle text but still legible across different screen sizes and resolutions.
    /// It automatically adjusts font sizes, line heights, and positions to maintain consistent
    /// visual hierarchy in the game interface.
    ///
    /// # Arguments
    ///
    /// * `width` - Current window width in pixels
    /// * `height` - Current window height in pixels
    ///
    /// # Behavior
    ///
    /// - Applies DPI scaling based on a 1080p reference resolution
    /// - Scales font size to 2.2% of window width (clamped between 16-48px)
    /// - Positions score text in top-left corner with consistent padding
    /// - Positions level text below score text with appropriate spacing
    /// - Adds padding to prevent text clipping
    /// - Uses consistent spacing and alignment for UI consistency
    ///
    /// # Prerequisites
    ///
    /// Requires text buffers with IDs "score" and "level" to exist.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Call when window is resized or score/level changes
    /// renderer.handle_score_and_level_text(1920, 1080);
    /// ```
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
