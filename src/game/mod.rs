//! Game state management module.
//!
//! This module defines the [`GameState`] struct, which tracks all mutable state for the game loop,
//! including the player, timing, UI state, and maze path.

pub mod collision;
pub mod keys;
pub mod player;

use self::collision::CollisionSystem;
use self::player::Player;
use std::path::PathBuf;
use std::time::Instant;
/// Represents the entire mutable state of the game.
///
/// This struct is updated every frame and contains:
/// - The player and their position/orientation.
/// - Timing information for frame updates and FPS calculation.
/// - UI state (title screen, mouse capture).
/// - The currently loaded maze path, if any.
#[derive(Debug, Clone)]
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
    /// Whether the title screen is currently shown.
    pub title_screen: bool,
    /// Path to the currently loaded maze, if any.
    pub maze_path: Option<PathBuf>,
    /// Whether the mouse is captured for camera movement.
    pub capture_mouse: bool,
    /// Handles collisions between game entities.
    pub collision_system: CollisionSystem,
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
            title_screen: true,
            maze_path: None,
            capture_mouse: true,
            collision_system: CollisionSystem::new(
                10.0,  // player_radius (adjust based on your player size)
                100.0, // player_height (adjust based on your player size)),
            ),
        }
    }
}
