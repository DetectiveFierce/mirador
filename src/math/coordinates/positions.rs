//! Special positions and cell finding utilities.
//!
//! This module provides functions to find and work with special positions
//! in the maze, such as the entrance, exit, and cardinal directions.

use crate::game::maze::generator::Cell;

/// Enum representing cardinal directions in the maze
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// North direction (0°)
    North,
    /// East direction (90°)
    East,
    /// South direction (180°)
    South,
    /// West direction (270°)
    West,
}

/// Gets the bottom-left cell of the maze.
///
/// # Arguments
/// * `maze_dimensions` - The dimensions of the maze (width, height) in cells
///
/// # Returns
/// The cell representing the bottom-left corner
pub fn get_bottom_left_cell(maze_dimensions: (usize, usize)) -> Cell {
    let (_, maze_height) = maze_dimensions;
    Cell::new(maze_height - 1, 0)
}

/// Gets the top-left cell of the maze.
///
/// # Arguments
/// * `maze_dimensions` - The dimensions of the maze (width, height) in cells
///
/// # Returns
/// The cell representing the top-left corner
pub fn get_top_left_cell(_maze_dimensions: (usize, usize)) -> Cell {
    Cell::new(0, 0)
}

/// Gets the top-right cell of the maze.
///
/// # Arguments
/// * `maze_dimensions` - The dimensions of the maze (width, height) in cells
///
/// # Returns
/// The cell representing the top-right corner
pub fn get_top_right_cell(maze_dimensions: (usize, usize)) -> Cell {
    let (maze_width, _) = maze_dimensions;
    Cell::new(0, maze_width - 1)
}

/// Gets the bottom-right cell of the maze.
///
/// # Arguments
/// * `maze_dimensions` - The dimensions of the maze (width, height) in cells
///
/// # Returns
/// The cell representing the bottom-right corner
pub fn get_bottom_right_cell(maze_dimensions: (usize, usize)) -> Cell {
    let (maze_width, maze_height) = maze_dimensions;
    Cell::new(maze_height - 1, maze_width - 1)
}

/// Gets the cell adjacent to the given cell in the specified direction.
///
/// # Arguments
/// * `cell` - The current cell
/// * `direction` - The direction to move
/// * `maze_dimensions` - The dimensions of the maze (width, height) in cells
///
/// # Returns
/// Option containing the adjacent cell, or None if it would be outside the maze
pub fn get_adjacent_cell(
    cell: &Cell,
    direction: Direction,
    maze_dimensions: (usize, usize),
) -> Option<Cell> {
    let (maze_width, maze_height) = maze_dimensions;
    let Cell { row, col } = *cell;

    match direction {
        Direction::North if row > 0 => Some(Cell::new(row - 1, col)),
        Direction::South if row < maze_height - 1 => Some(Cell::new(row + 1, col)),
        Direction::East if col < maze_width - 1 => Some(Cell::new(row, col + 1)),
        Direction::West if col > 0 => Some(Cell::new(row, col - 1)),
        _ => None, // Outside the maze bounds
    }
}

/// Converts a yaw angle (in degrees) to a cardinal direction.
///
/// # Arguments
/// * `yaw` - The yaw angle in degrees
///
/// # Returns
/// The closest cardinal direction
pub fn yaw_to_direction(yaw: f32) -> Direction {
    // Normalize angle to 0-360
    let normalized_yaw = ((yaw % 360.0) + 360.0) % 360.0;

    // Convert to cardinal direction (North = 0°, East = 90°, etc.)
    match normalized_yaw as u32 {
        315..=359 | 0..=45 => Direction::North,
        46..=135 => Direction::East,
        136..=225 => Direction::South,
        226..=314 => Direction::West,
        _ => Direction::North, // Should never happen due to normalization
    }
}

/// Gets the relative direction from one cell to another.
///
/// # Arguments
/// * `from` - The starting cell
/// * `to` - The destination cell
///
/// # Returns
/// The cardinal direction from the starting cell to the destination
pub fn get_direction_between_cells(from: &Cell, to: &Cell) -> Option<Direction> {
    let row_diff = to.row as isize - from.row as isize;
    let col_diff = to.col as isize - from.col as isize;

    // Determine primary direction based on which difference is larger
    if row_diff.abs() > col_diff.abs() {
        if row_diff < 0 {
            Some(Direction::North)
        } else {
            Some(Direction::South)
        }
    } else if col_diff.abs() > row_diff.abs() {
        if col_diff > 0 {
            Some(Direction::East)
        } else {
            Some(Direction::West)
        }
    } else {
        // Cells are diagonal or the same - no clear direction
        None
    }
}

/// Translates a direction into a yaw angle.
///
/// # Arguments
/// * `direction` - The cardinal direction
///
/// # Returns
/// The corresponding yaw angle in degrees
pub fn direction_to_yaw(direction: Direction) -> f32 {
    match direction {
        Direction::North => 0.0,
        Direction::East => 90.0,
        Direction::South => 180.0,
        Direction::West => 270.0,
    }
}
