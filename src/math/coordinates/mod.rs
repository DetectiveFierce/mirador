//! Coordinate system transformations for the maze.
//!
//! This module provides utilities to convert between different coordinate systems used in the game:
//! - Maze Grid Coordinates: Used for maze generation (rows/columns)
//! - World Coordinates: 3D space where the player moves (x, y, z)
//!
//! It centralizes all coordinate transformations and provides utilities for finding
//! special cells like the entrance (bottom left) and exit.

mod positions;
mod transformations;

pub use positions::*;
pub use transformations::*;

/// Constants for special positions in the maze
pub mod constants {
    /// Standard height of the player in the world
    pub const PLAYER_HEIGHT: f32 = 50.0;

    /// Get the floor size based on test mode
    /// In test mode, the floor is 1/4 the size of normal mode
    pub fn get_floor_size(is_test_mode: bool) -> f32 {
        if is_test_mode {
            1500.0 // 1/4 of 3000.0 <-- meth-a-matics ??
        } else {
            3000.0
        }
    }
}
