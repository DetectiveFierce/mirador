//! Maze Generation, Encoding, and Animation module.
//!
//! This module provides maze generation, parsing, and title screen logic.
//! It includes utilities for reading maze files into a 2D wall representation.

pub mod generator;
pub mod maze_animation;

use self::generator::Cell;
use std::fs::File;
use std::io::{BufRead, BufReader};
/// Parses a maze file into a 2D vector of wall booleans and detects the exit cell if marked.
///
/// Each line of the file is read as a row of the maze. Each character is mapped as follows:
/// - `#` becomes `true` (wall)
/// - Any other character becomes `false` (open space)
///
/// The function also detects an optional exit cell, marked with `*`.
/// The coordinates of the `*` character are interpreted in the wall grid, which includes both walls
/// and spaces. To convert this to the maze cell coordinates (excluding walls), the function checks:
/// - The row and column indices of `*` must both be **odd** (i.e., part of a valid cell, not a wall)
/// - The maze cell coordinate is then computed by halving both indices:
///   `maze_row = (maze_height - 1) - (row_idx / 2)`, `maze_col = col_idx / 2`
///
/// # Arguments
/// * `path` - Path to the maze file to parse.
///
/// # Returns
/// A tuple:
/// - A 2D vector (`Vec<Vec<bool>>`) where `true` indicates a wall and `false` an open cell
/// - An optional `Cell` representing the location of the exit cell, if found (with bottom-left origin)
///
/// # Panics
/// - If the file cannot be opened.
/// - If any line cannot be read.
///
/// # Example
/// ```text
/// # # #
/// # * #
/// # # #
/// ```
/// becomes:
/// ```ignore
/// vec![
///     vec![true, false, true],
///     vec![true, false, true],
///     vec![true, false, true],
/// ]
/// exit_cell: Some(Cell { row: 0, col: 0 }) // bottom-left origin
/// ```
pub fn parse_maze_file(path: &str) -> (Vec<Vec<bool>>, Option<Cell>) {
    let file = File::open(path).expect("Failed to open maze file");
    let reader = BufReader::new(file);

    let lines: Vec<String> = reader
        .lines()
        .map(|line| line.expect("Failed to read line"))
        .collect();

    let mut maze_grid = Vec::with_capacity(lines.len());
    let mut exit_cell = None;

    for (wall_row_idx, line) in lines.into_iter().enumerate() {
        let mut row = Vec::new();
        for (wall_col_idx, c) in line.chars().enumerate() {
            let is_wall = c == '#';
            row.push(is_wall);

            // Check for exit marker
            if c == '*' && wall_row_idx % 2 == 1 && wall_col_idx % 2 == 1 {
                let maze_row = wall_row_idx;
                let maze_col = wall_col_idx;
                exit_cell = Some(Cell::new(maze_row, maze_col));
            }
        }
        maze_grid.push(row);
    }

    (maze_grid, exit_cell)
}
