pub mod keys;
pub mod player;

use self::player::Player;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct GameState {
    pub player: Player,
    pub last_frame_time: Instant,
    pub delta_time: f32,
    pub frame_count: u32,
    pub current_fps: u32,
    pub last_fps_time: Instant,
    pub title_screen: bool,
    pub maze_path: Option<PathBuf>,
    pub capture_mouse: bool,
}

impl GameState {
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
        }
    }
}
