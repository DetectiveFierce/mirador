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

    /// Size of the world floor
    pub const FLOOR_SIZE: f32 = 3000.0;
}
