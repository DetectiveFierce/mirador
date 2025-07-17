//! Coordinate system transformations between different representations.
//!
//! This module provides functions to convert between maze grid coordinates and
//! world coordinates, making it clear how to map between these systems.

use super::constants::get_floor_size;
use crate::maze::generator::Cell;

/// Converts a maze grid cell to world coordinates.
///
/// # Arguments
/// * `cell` - The maze cell in grid coordinates (row, col)
/// * `maze_dimensions` - The dimensions of the maze (width, height) in cells
/// * `y_position` - The desired y-coordinate (height) in the world
///
/// # Returns
/// The corresponding 3D world coordinates [x, y, z]
///
/// # Coordinate System
/// - Origin (0,0,0) is at the center of the world
/// - X increases to the right (east)
/// - Y increases upwards
/// - Z increases forward (north)
pub fn maze_to_world(
    cell: &Cell,
    maze_dimensions: (usize, usize),
    y_position: f32,
    is_test_mode: bool,
) -> [f32; 3] {
    let (maze_width, maze_height) = maze_dimensions;
    let max_dimension = maze_width.max(maze_height) as f32;
    let cell_size = get_floor_size(is_test_mode) / max_dimension;

    // Calculate the world origin offset (bottom-left corner of the maze)
    let origin_x = -(maze_width as f32 * cell_size) / 2.0;
    let origin_z = -(maze_height as f32 * cell_size) / 2.0;

    // Calculate the center of the cell
    let world_x = origin_x + (cell.col as f32 + 0.5) * cell_size;
    let world_z = origin_z + (cell.row as f32 + 0.5) * cell_size;

    [world_x, y_position, world_z]
}

/// Converts 3D world coordinates to a maze grid cell.
///
/// # Arguments
/// * `position` - The 3D world coordinates [x, y, z]
/// * `maze_dimensions` - The dimensions of the maze (width, height) in cells
///
/// # Returns
/// The corresponding maze grid cell
///
/// # Note
/// This function ignores the y-coordinate since the maze is 2D.
pub fn world_to_maze(
    position: [f32; 3],
    maze_dimensions: (usize, usize),
    is_test_mode: bool,
) -> Cell {
    let (maze_width, maze_height) = maze_dimensions;
    let max_dimension = maze_width.max(maze_height) as f32;
    let cell_size = get_floor_size(is_test_mode) / max_dimension;

    let origin_x = -(maze_width as f32 * cell_size) / 2.0;
    let origin_z = -(maze_height as f32 * cell_size) / 2.0;

    // Calculate cell coordinates
    let relative_x = position[0] - origin_x;
    let relative_z = position[2] - origin_z;

    let col = (relative_x / cell_size).floor() as usize;
    let row = (relative_z / cell_size).floor() as usize;

    // Clamp to valid maze bounds
    let col = col.min(maze_width - 1);
    let row = row.min(maze_height - 1);

    Cell::new(row, col)
}

/// Calculates the size of a single cell in world units.
///
/// # Arguments
/// * `maze_dimensions` - The dimensions of the maze (width, height) in cells
///
/// # Returns
/// The size of a cell in world units
pub fn calculate_cell_size(maze_dimensions: (usize, usize), is_test_mode: bool) -> f32 {
    let (maze_width, maze_height) = maze_dimensions;
    let max_dimension = maze_width.max(maze_height) as f32;
    get_floor_size(is_test_mode) / max_dimension
}

/// Calculates the world coordinates of the bottom-left cell in the maze.
///
/// # Arguments
/// * `maze_dimensions` - The dimensions of the maze (width, height) in cells
/// * `y_position` - The desired y-coordinate (height) in the world
///
/// # Returns
/// The world coordinates for the bottom-left cell
pub fn get_bottom_left_cell_position(
    maze_dimensions: (usize, usize),
    y_position: f32,
    is_test_mode: bool,
) -> [f32; 3] {
    // Bottom-left cell is at (height-1, 0) in our grid system
    let (_, maze_height) = maze_dimensions;
    let bottom_left = Cell::new(maze_height - 1, 0);
    maze_to_world(&bottom_left, maze_dimensions, y_position, is_test_mode)
}

/// Converts a position in the maze wall grid to a position in the maze cell grid.
///
/// # Arguments
/// * `wall_row` - Row in the wall grid
/// * `wall_col` - Column in the wall grid
///
/// # Returns
/// A tuple (is_cell, Cell) where is_cell indicates if this position represents a cell
/// (not a wall or passage), and Cell is the corresponding maze cell if applicable
pub fn wall_grid_to_maze_cell(wall_row: usize, wall_col: usize) -> (bool, Cell) {
    let is_cell = wall_row % 2 == 1 && wall_col % 2 == 1;
    let cell = if is_cell {
        Cell::new(wall_row / 2, wall_col / 2)
    } else {
        Cell::new(0, 0) // Default, not valid if is_cell is false
    };

    (is_cell, cell)
}
