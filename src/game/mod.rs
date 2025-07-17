//! Game state management module.
//!
//! This module defines the [`GameState`] struct, which tracks all mutable state for the game loop,
//! including the player, timing, UI state, and maze path.

// Timer decimal alignment: The timer's decimal point is always aligned with the vertical center of the screen by measuring the width of the timer string up to and including the decimal and offsetting the x position accordingly. See initialize_game_ui and update_game_ui for details.
pub mod audio;
pub mod collision;
pub mod enemy;
pub mod keys;
pub mod maze;
pub mod player;
use self::audio::GameAudioManager;
use self::collision::CollisionSystem;
use self::player::Player;
use crate::game::enemy::Enemy;

use crate::game::maze::generator::Cell;
use crate::renderer::text::TextPosition;
use crate::renderer::text::TextRenderer;
use crate::renderer::text::TextStyle;
use glyphon::Color;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use winit::window::Window;
/// Represents the entire mutable state of the game.
///
/// This struct is updated every frame and contains:
/// - The player and their position/orientation.
/// - Timing information for frame updates and FPS calculation.
/// - UI state (title screen, mouse capture).
/// - The currently loaded maze path, if any.
pub struct GameState {
    /// The player character.
    pub player: Player,
    /// Time of the last frame.
    pub last_frame_time: Instant,
    /// Time elapsed since the last frame (seconds).
    pub delta_time: f32,
    /// Number of frames rendered since start.
    pub frame_count: u32,
    /// Current frames per second.
    pub current_fps: u32,
    /// Time of the last FPS update.
    pub last_fps_time: Instant,
    /// Path to the currently loaded maze, if any.
    pub maze_path: Option<PathBuf>,
    /// Whether the mouse is captured for camera movement.
    pub capture_mouse: bool,
    /// Handles collisions between game entities.
    pub collision_system: CollisionSystem,
    /// Whether the exit has been reached.
    pub exit_reached: bool,
    pub exit_cell: Option<Cell>,
    pub game_ui: GameUIManager,
    pub current_screen: CurrentScreen,
    /// The screen that was active before entering pause menu
    pub previous_screen: Option<CurrentScreen>,
    pub enemy: Enemy,
    pub audio_manager: GameAudioManager,
    /// Whether the player is currently in test mode
    pub is_test_mode: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseMenuState {
    Open,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentScreen {
    Title,
    Loading,
    Game,
    Pause,
    GameOver,
    NewGame,
}

impl Default for GameState {
    /// Returns a new [`GameState`] with default values.
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    /// Creates a new [`GameState`] with default player, timing, and UI state.
    pub fn new() -> Self {
        let mut audio_manager =
            GameAudioManager::new().expect("Failed to initialize audio manager");
        audio_manager
            .spawn_enemy("enemy".to_string(), [-0.5, 30.0, 0.0])
            .expect("Failed to spawn enemy");
        Self {
            player: Player::new(),
            last_frame_time: Instant::now(),
            delta_time: 0.0,

            frame_count: 0,
            current_fps: 0,
            last_fps_time: Instant::now(),
            maze_path: None,
            capture_mouse: true,
            collision_system: CollisionSystem::new(
                5.0,   // player_radius (adjust based on your player size)
                100.0, // player_height (adjust based on your player size)),
            ),
            exit_reached: false,
            exit_cell: None,
            game_ui: GameUIManager::new(),
            current_screen: CurrentScreen::Title,
            previous_screen: None,
            enemy: Enemy::new([-0.5, 30.0, 0.0], 150.0),
            audio_manager,
            is_test_mode: false,
        }
    }

    /// Start the game timer
    pub fn start_game_timer(&mut self, config: Option<TimerConfig>) {
        self.game_ui.start_timer(config);
    }

    /// Stop the game timer
    pub fn stop_game_timer(&mut self) {
        self.game_ui.stop_timer();
    }

    /// Reset the game timer
    pub fn reset_game_timer(&mut self) {
        self.game_ui.reset_timer();
    }

    /// Check if the game timer is expired
    pub fn is_game_timer_expired(&self) -> bool {
        self.game_ui.is_timer_expired()
    }

    /// Update game level
    pub fn set_level(&mut self, level: i32) {
        self.game_ui.set_level(level);
    }

    /// Update game score
    pub fn set_score(&mut self, score: u32) {
        self.game_ui.set_score(score);
    }
}

#[derive(Debug, Clone)]
pub struct TimerConfig {
    pub duration: Duration,
    pub warning_threshold: Duration,
    pub critical_threshold: Duration,
    pub normal_color: Color,
    pub warning_color: Color,
    pub critical_color: Color,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(30),
            warning_threshold: Duration::from_secs(20),
            critical_threshold: Duration::from_secs(10),
            normal_color: Color::rgb(100, 255, 100),
            warning_color: Color::rgb(255, 255, 100),
            critical_color: Color::rgb(255, 100, 100),
        }
    }
}

#[derive(Debug)]
pub struct GameTimer {
    pub start_time: Instant,
    pub config: TimerConfig,
    pub is_running: bool,
    pub is_expired: bool,
    pub paused_at: Option<Instant>,
    pub elapsed_paused: Duration,
}

impl GameTimer {
    pub fn new(config: TimerConfig) -> Self {
        Self {
            start_time: Instant::now(),
            config,
            is_running: false,
            is_expired: false,
            paused_at: None,
            elapsed_paused: Duration::ZERO,
        }
    }

    pub fn start(&mut self) {
        self.start_time = Instant::now();
        self.is_running = true;
        self.is_expired = false;
        self.paused_at = None;
        self.elapsed_paused = Duration::ZERO;
    }

    pub fn pause(&mut self) {
        if self.is_running && self.paused_at.is_none() {
            self.paused_at = Some(Instant::now());
        }
    }

    pub fn resume(&mut self) {
        if let Some(paused_at) = self.paused_at.take() {
            self.elapsed_paused += paused_at.elapsed();
        }
    }

    pub fn stop(&mut self) {
        self.is_running = false;
    }

    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.is_expired = false;
        self.paused_at = None;
        self.elapsed_paused = Duration::ZERO;
    }

    pub fn get_remaining_time(&self) -> Duration {
        if !self.is_running || self.is_expired {
            return Duration::ZERO;
        }
        let elapsed = if let Some(paused_at) = self.paused_at {
            paused_at.duration_since(self.start_time) - self.elapsed_paused
        } else {
            Instant::now().duration_since(self.start_time) - self.elapsed_paused
        };
        self.config
            .duration
            .checked_sub(elapsed)
            .unwrap_or(Duration::ZERO)
    }

    pub fn is_expired(&self) -> bool {
        self.is_expired || (!self.is_running && self.get_remaining_time().is_zero())
    }

    pub fn update(&mut self) -> bool {
        if !self.is_running || self.paused_at.is_some() {
            return false;
        }
        let remaining = self.get_remaining_time();
        let was_expired = self.is_expired;
        self.is_expired = remaining.is_zero();
        !was_expired && self.is_expired
    }

    pub fn get_current_color(&self) -> Color {
        let remaining = self.get_remaining_time();
        if remaining <= self.config.critical_threshold {
            self.config.critical_color
        } else if remaining <= self.config.warning_threshold {
            self.config.warning_color
        } else {
            self.config.normal_color
        }
    }

    pub fn format_time(&self) -> String {
        let remaining = self.get_remaining_time();
        let seconds = remaining.as_secs_f64();
        format!("{:05.2}", seconds)
    }
}

pub struct GameUIManager {
    pub timer: Option<GameTimer>,
    pub level: i32,
    pub score: u32,
}

impl Default for GameUIManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GameUIManager {
    pub fn new() -> Self {
        Self {
            timer: None,
            level: 1,
            score: 0,
        }
    }

    pub fn start_timer(&mut self, config: Option<TimerConfig>) {
        let config = config.unwrap_or_default();
        let mut timer = GameTimer::new(config);
        timer.start();
        self.timer = Some(timer);
    }

    pub fn stop_timer(&mut self) {
        if let Some(timer) = &mut self.timer {
            timer.stop();
        }
    }

    pub fn reset_timer(&mut self) {
        if let Some(timer) = &mut self.timer {
            timer.reset();
            timer.start();
        }
    }

    pub fn update_timer(&mut self) -> bool {
        if let Some(timer) = &mut self.timer {
            timer.update()
        } else {
            false
        }
    }

    pub fn is_timer_expired(&self) -> bool {
        self.timer.as_ref().map(|t| t.is_expired()).unwrap_or(false)
    }

    pub fn get_timer_text(&self) -> String {
        self.timer
            .as_ref()
            .map_or("00.00".to_string(), |t| t.format_time())
    }

    pub fn get_timer_color(&self) -> Color {
        self.timer
            .as_ref()
            .map_or(Color::rgb(255, 255, 255), |t| t.get_current_color())
    }

    pub fn set_level(&mut self, level: i32) {
        self.level = level;
    }

    pub fn get_level(&self) -> i32 {
        self.level
    }

    pub fn get_level_text(&self) -> String {
        format!("Level: {}", self.level)
    }

    pub fn set_score(&mut self, score: u32) {
        self.score = score;
    }

    pub fn get_score(&self) -> u32 {
        self.score
    }

    pub fn get_score_text(&self) -> String {
        format!("Score: {}", self.score)
    }

    pub fn pause_timer(&mut self) {
        if let Some(timer) = &mut self.timer {
            timer.pause();
        }
    }

    pub fn resume_timer(&mut self) {
        if let Some(timer) = &mut self.timer {
            timer.resume();
        }
    }
}

/// Sets up the timer, score, and level display using the TextRenderer
pub fn initialize_game_ui(
    text_renderer: &mut TextRenderer,
    game_ui: &GameUIManager,
    window: &Window,
) {
    let size = window.inner_size();
    let width = size.width;
    let height = size.height;

    // --- Responsive scaling logic ---
    // If the window is large, scale up the text; otherwise, use default sizes
    let (timer_font_size, timer_line_height, timer_max_width, timer_max_height) = if width >= 1920 {
        (80.0, 100.0, 300.0, 120.0)
    } else if width >= 1600 || height >= 900 {
        (60.0, 76.0, 200.0, 80.0)
    } else {
        (48.0, 60.0, 150.0, 60.0)
    };
    let (label_font_size, label_line_height, label_max_width, label_max_height) =
        if width >= 1600 || height >= 900 {
            (24.0, 28.0, 160.0, 32.0)
        } else {
            (18.0, 22.0, 120.0, 25.0)
        };

    // Timer display (decimal-aligned at top)
    let timer_text = game_ui.get_timer_text();
    let timer_style = TextStyle {
        font_family: "HankenGrotesk".to_string(),
        font_size: timer_font_size,
        line_height: timer_line_height,
        color: Color::rgb(100, 255, 100),
        weight: glyphon::Weight::BOLD,
        style: glyphon::Style::Normal,
    };
    // Find decimal position in timer_text
    let decimal_index = timer_text.find('.').unwrap_or(timer_text.len() - 1) + 1;
    let decimal_substr = &timer_text[..decimal_index];
    let (_min_x, decimal_offset, _h) = text_renderer.measure_text(decimal_substr, &timer_style);
    let timer_position = TextPosition {
        x: (width as f32 / 2.0) - decimal_offset,
        y: 10.0,
        max_width: Some(timer_max_width),
        max_height: Some(timer_max_height),
    };
    text_renderer.create_text_buffer(
        "main_timer",
        &timer_text,
        Some(timer_style),
        Some(timer_position),
    );

    // Level display (top left, above score)
    let level_style = TextStyle {
        font_family: "HankenGrotesk".to_string(),
        font_size: label_font_size,
        line_height: label_line_height,
        color: Color::rgb(255, 255, 150),
        weight: glyphon::Weight::NORMAL,
        style: glyphon::Style::Normal,
    };
    let level_position = TextPosition {
        x: 20.0,
        y: 20.0,
        max_width: Some(label_max_width),
        max_height: Some(label_max_height),
    };
    text_renderer.create_text_buffer(
        "level",
        &game_ui.get_level_text(),
        Some(level_style),
        Some(level_position),
    );

    // Score display (top left, below level, left edge aligned)
    let score_style = TextStyle {
        font_family: "HankenGrotesk".to_string(),
        font_size: label_font_size,
        line_height: label_line_height,
        color: Color::rgb(150, 255, 255),
        weight: glyphon::Weight::NORMAL,
        style: glyphon::Style::Normal,
    };
    let score_position = TextPosition {
        x: 20.0,
        y: 50.0,
        max_width: Some(label_max_width),
        max_height: Some(label_max_height),
    };
    text_renderer.create_text_buffer(
        "score",
        &game_ui.get_score_text(),
        Some(score_style),
        Some(score_position),
    );
}

/// Helper to update the text content of a buffer and re-apply style
fn update_text_content(
    text_renderer: &mut TextRenderer,
    id: &str,
    new_text: &str,
) -> Result<(), String> {
    if let Some(buffer) = text_renderer.text_buffers.get_mut(id) {
        buffer.text_content = new_text.to_string();
        // Re-apply style to update the buffer
        let style = buffer.style.clone();
        text_renderer.update_style(id, style)
    } else {
        Err(format!("Text buffer '{}' not found", id))
    }
}

/// Call this every frame to update the timer, score, and level displays
pub fn update_game_ui(
    text_renderer: &mut TextRenderer,
    game_ui: &mut GameUIManager,
    current_screen: &CurrentScreen,
    window: &Window,
) -> bool {
    // Only update the timer if in Game
    let timer_expired = if let CurrentScreen::Game = current_screen {
        game_ui.update_timer()
    } else {
        false
    };

    // Update timer display
    let timer_text = game_ui.get_timer_text();
    let _ = update_text_content(text_renderer, "main_timer", &timer_text);
    // Update timer color by updating style
    if let Some(buffer) = text_renderer.text_buffers.get_mut("main_timer") {
        let mut style = buffer.style.clone();
        style.color = game_ui.get_timer_color();
        let _ = text_renderer.update_style("main_timer", style);
    }

    // Update level and score displays
    let _ = update_text_content(text_renderer, "level", &game_ui.get_level_text());
    let _ = update_text_content(text_renderer, "score", &game_ui.get_score_text());

    // Adjust timer position if window size changes
    let size = window.inner_size();
    let width = size.width;
    let height = size.height;
    let (timer_max_width, timer_max_height) = if width >= 1920 {
        (300.0, 120.0)
    } else if width >= 1600 || height >= 900 {
        (200.0, 80.0)
    } else {
        (150.0, 60.0)
    };
    // Align decimal point with center
    let timer_style = if let Some(buffer) = text_renderer.text_buffers.get("main_timer") {
        buffer.style.clone()
    } else {
        TextStyle::default()
    };
    let decimal_index = timer_text.find('.').unwrap_or(timer_text.len() - 1) + 1;
    let decimal_substr = &timer_text[..decimal_index];
    let (_min_x, decimal_offset, _h) = text_renderer.measure_text(decimal_substr, &timer_style);
    let timer_position = TextPosition {
        x: (width as f32 / 2.0) - decimal_offset,
        y: 10.0,
        max_width: Some(timer_max_width),
        max_height: Some(timer_max_height),
    };
    if let Some(buffer) = text_renderer.text_buffers.get_mut("main_timer") {
        buffer.position = timer_position;
    }

    timer_expired
}
