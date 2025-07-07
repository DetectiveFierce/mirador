//! Game state management module.
//!
//! This module defines the [`GameState`] struct, which tracks all mutable state for the game loop,
//! including the player, timing, UI state, and maze path.

pub mod collision;
pub mod enemy;
pub mod keys;
pub mod player;
use self::collision::CollisionSystem;
use self::player::Player;
use crate::game::enemy::Enemy;
use crate::maze::generator::Cell;
use crate::renderer::text::TextRenderer;
use glyphon::Color;
use std::path::PathBuf;
use std::time::{Duration, Instant};
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
    pub exit_cell: Cell,
    pub game_ui: GameUIManager,
    pub current_screen: CurrentScreen,
    pub enemy: Enemy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentScreen {
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
            exit_cell: Cell::default(),
            game_ui: GameUIManager::new(),
            current_screen: CurrentScreen::Loading,
            enemy: Enemy::new([-0.5, 30.0, 0.0], 100.0),
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
    pub fn set_level(&mut self, text_renderer: &mut TextRenderer, level: i32) {
        self.game_ui.set_level(level);
        if let Err(e) = text_renderer.update_text("level", &self.game_ui.get_level_text()) {
            println!("Failed to update level text: {}", e);
        }
    }

    /// Update game score
    pub fn set_score(&mut self, text_renderer: &mut TextRenderer, score: u32) {
        self.game_ui.set_score(score);
        if let Err(e) = text_renderer.update_text("score", &self.game_ui.get_score_text()) {
            println!("Failed to update score text: {}", e);
        }
    }
}

/// Timer configuration for game elements
#[derive(Debug, Clone)]
pub struct TimerConfig {
    pub duration: Duration,
    pub warning_threshold: Duration,  // When to turn yellow
    pub critical_threshold: Duration, // When to turn red
    pub normal_color: Color,
    pub warning_color: Color,
    pub critical_color: Color,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(60),
            warning_threshold: Duration::from_secs(30),
            critical_threshold: Duration::from_secs(15),
            normal_color: Color::rgb(100, 255, 100),
            warning_color: Color::rgb(255, 255, 100),
            critical_color: Color::rgb(255, 100, 100),
        }
    }
}

/// Game timer state
#[derive(Debug)]
pub struct GameTimer {
    pub start_time: Instant,
    pub config: TimerConfig,
    pub is_running: bool,
    pub is_expired: bool,
    pub prev_time: Duration,
}

impl GameTimer {
    pub fn new(config: TimerConfig) -> Self {
        Self {
            start_time: Instant::now(),
            config,
            is_running: false,
            is_expired: false,
            prev_time: Duration::from_secs(60),
        }
    }

    pub fn start(&mut self) {
        self.start_time = Instant::now();
        self.is_running = true;
        self.is_expired = false;
    }

    pub fn stop(&mut self) {
        self.is_running = false;
    }

    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.is_expired = false;
    }

    pub fn get_remaining_time(&self) -> Duration {
        if !self.is_running || self.is_expired {
            return Duration::ZERO;
        }

        let elapsed = self.start_time.elapsed();
        self.config
            .duration
            .checked_sub(elapsed)
            .unwrap_or(Duration::ZERO)
    }

    pub fn is_expired(&self) -> bool {
        self.is_expired || (!self.is_running && self.get_remaining_time().is_zero())
    }

    pub fn update(&mut self) -> bool {
        if !self.is_running {
            return false;
        }

        let remaining = self.get_remaining_time();
        let was_expired = self.is_expired;
        self.is_expired = remaining.is_zero();

        // Return true if timer just expired (transition from running to expired)
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

    pub fn add_time(&mut self, duration: Duration) {
        self.config.duration += duration;
    }

    pub fn get_total_time(&self) -> Duration {
        self.config.duration
    }
}

/// Manages game-specific UI elements like timers, scores, levels, etc.
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
}
