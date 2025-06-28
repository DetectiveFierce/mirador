//! Maze Generation, Encoding, and Animation module.
//!
//! This module provides maze generation, parsing, and title screen logic.
//! It includes utilities for reading maze files into a 2D wall representation.

pub mod generator;
pub mod title_screen;

use self::generator::Cell;
use std::fs::File;
use std::io::{BufRead, BufReader};
/// Parses a maze file into a 2D vector of wall booleans.
///
/// Each line of the file is read as a row of the maze. Each character is mapped as follows:
/// - `#` becomes `true` (wall)
/// - Any other character becomes `false` (open space)
///
/// # Arguments
/// * `path` - Path to the maze file to parse.
///
/// # Returns
/// A 2D vector (`Vec<Vec<bool>>`) where `true` indicates a wall and `false` an open cell.
///
/// # Panics
/// - If the file cannot be opened.
/// - If any line cannot be read.
///
/// # Example
/// ```text
/// # # #
/// #   #
/// # # #
/// ```
/// becomes:
/// ```ignore
/// vec![
///     vec![true, false, true, false, true],
///     vec![true, false, false, false, true],
///     vec![true, false, true, false, true],
/// ]
/// ```
pub fn parse_maze_file(path: &str) -> (Vec<Vec<bool>>, Option<Cell>) {
    let file = File::open(path).expect("Failed to open maze file");
    let reader = BufReader::new(file);

    let mut maze_grid = Vec::new();
    let mut exit_cell = None;

    for (row_idx, line) in reader.lines().enumerate() {
        let line = line.expect("Failed to read line");
        let mut row = Vec::new();

        for (col_idx, c) in line.chars().enumerate() {
            let is_wall = c == '#';
            row.push(is_wall);

            // Check for exit marker
            if c == '*' {
                // Convert wall grid coordinates back to maze cell coordinates
                if row_idx % 2 == 1 && col_idx % 2 == 1 {
                    let maze_row = row_idx / 2;
                    let maze_col = col_idx / 2;
                    exit_cell = Some(Cell::new(maze_row, maze_col));
                }
            }
        }
        maze_grid.push(row);
    }

    (maze_grid, exit_cell)
}
