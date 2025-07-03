//! Maze generation using Kruskal's algorithm with Union-Find data structure.
//!
//! This module provides functionality to generate random mazes using Kruskal's algorithm,
//! visualize the generation process, and save the resulting mazes to files.
//!
//! # Examples
//!
//! ```rust
//! use maze_generator::{MazeGenerator, Cell};
//!
//! // Create a 10x10 maze generator
//! let (mut generator, maze) = MazeGenerator::new(10, 10);
//!
//! // Generate the maze step by step
//! while !generator.is_complete() {
//!     generator.step();
//! }
//!
//! // Save the maze to a file
//! maze.lock().unwrap().save_to_file().expect("Failed to save maze");
//! ```
use chrono::Local;
use rand::prelude::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::hash_map::Entry;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Represents a cell in the maze grid
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell {
    /// Row index of the cell
    pub row: usize,
    /// Column index of the cell
    pub col: usize,
}

impl Cell {
    /// Creates a new Cell with the given coordinates
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

/// Represents an edge between two cells in the maze
#[derive(Debug, Clone, Copy)]
pub struct Edge {
    /// First cell connected by the edge
    pub cell1: Cell,
    /// Second cell connected by the edge
    pub cell2: Cell,
}

impl Edge {
    /// Creates a new Edge between two cells
    pub fn new(cell1: Cell, cell2: Cell) -> Self {
        Self { cell1, cell2 }
    }
}

/// Union-Find data structure for Kruskal's algorithm
pub struct UnionFind {
    parent: HashMap<Cell, Cell>,
    rank: HashMap<Cell, usize>,
}

impl Default for UnionFind {
    fn default() -> Self {
        Self::new()
    }
}

impl UnionFind {
    /// Creates a new UnionFind structure
    pub fn new() -> Self {
        Self {
            parent: HashMap::new(),
            rank: HashMap::new(),
        }
    }

    /// Adds a new cell to the UnionFind structure
    pub fn make_set(&mut self, cell: Cell) {
        if let Entry::Vacant(entry) = self.parent.entry(cell) {
            entry.insert(cell);
            self.rank.insert(cell, 0);
        }
    }

    /// Finds the root of the set containing the given cell
    pub fn find(&mut self, cell: Cell) -> Cell {
        if self.parent[&cell] != cell {
            let root = self.find(self.parent[&cell]);
            self.parent.insert(cell, root);
        }
        self.parent[&cell]
    }

    /// Unions two sets containing cell1 and cell2
    /// Returns true if the sets were merged, false if they were already in the same set
    pub fn union(&mut self, cell1: Cell, cell2: Cell) -> bool {
        let root1 = self.find(cell1);
        let root2 = self.find(cell2);

        if root1 == root2 {
            return false;
        }

        let rank1 = self.rank[&root1];
        let rank2 = self.rank[&root2];

        if rank1 < rank2 {
            self.parent.insert(root1, root2);
        } else if rank1 > rank2 {
            self.parent.insert(root2, root1);
        } else {
            self.parent.insert(root2, root1);
            self.rank.insert(root1, rank1 + 1);
        }

        true
    }
}

/// Represents a maze with walls and passages
#[derive(Clone)]
pub struct Maze {
    /// Width of the maze in cells
    pub width: usize,
    /// Height of the maze in cells
    pub height: usize,
    /// 2D vector representing walls (true) and passages (false)
    pub walls: Vec<Vec<bool>>,
    /// Total number of edges in the maze
    pub total_edges: usize,
    /// Number of edges processed during generation
    pub processed_edges: usize,
    /// Exit cell of the maze (if set)
    pub exit_cell: Option<Cell>,
}

impl Maze {
    /// Creates a new maze with all walls present
    pub fn new(width: usize, height: usize) -> Self {
        let walls = vec![vec![true; width * 2 + 1]; height * 2 + 1];
        Self {
            width,
            height,
            walls,
            total_edges: 0,
            processed_edges: 0,
            exit_cell: None,
        }
    }

    /// Sets a random cell as the exit
    pub fn set_random_exit(&mut self) {
        let mut rng = thread_rng();
        let row = rng.gen_range(0..self.height);
        let col = rng.gen_range(0..self.width);
        self.exit_cell = Some(Cell::new(row, col));
    }

    /// Generates pixel data for rendering the maze
    pub fn get_render_data(&self, connected: &HashSet<Cell>) -> Vec<u8> {
        let cell_px = 4;
        let wall_px = 1;
        let render_width = self.width * cell_px + (self.width + 1) * wall_px;
        let render_height = self.height * cell_px + (self.height + 1) * wall_px;
        let mut data = vec![0u8; render_width * render_height * 4];

        for row in 0..self.walls.len() {
            for col in 0..self.walls[0].len() {
                let is_wall = self.walls[row][col];
                let px_row = row / 2;
                let px_col = col / 2;
                let x = px_col * (cell_px + wall_px) + if col % 2 == 0 { 0 } else { wall_px };
                let y = px_row * (cell_px + wall_px) + if row % 2 == 0 { 0 } else { wall_px };
                let w = if col % 2 == 0 { wall_px } else { cell_px };
                let h = if row % 2 == 0 { wall_px } else { cell_px };

                let color = if is_wall {
                    [0, 0, 0, 255] // wall
                } else {
                    // Identify if this is a cell (not wall row/col)
                    if row % 2 == 1 && col % 2 == 1 {
                        let cell = Cell::new(row / 2, col / 2);
                        if Some(cell) == self.exit_cell {
                            [255, 0, 0, 255] // exit cell = red
                        } else if connected.contains(&cell) {
                            [255, 255, 255, 255] // connected cell = white
                        } else {
                            [0, 0, 0, 255] // unconnected cell = black
                        }
                    } else {
                        [255, 255, 255, 255] // passage or non-cell = white
                    }
                };

                for dy in 0..h {
                    for dx in 0..w {
                        let xi = x + dx;
                        let yi = y + dy;
                        if xi < render_width && yi < render_height {
                            let idx = (yi * render_width + xi) * 4;
                            data[idx..idx + 4].copy_from_slice(&color);
                        }
                    }
                }
            }
        }

        data
    }

    /// Returns the dimensions of the rendered maze in pixels
    pub fn get_dimensions(&self) -> (usize, usize) {
        let cell_px = 4;
        let wall_px = 1;
        let width = self.width * cell_px + (self.width + 1) * wall_px;
        let height = self.height * cell_px + (self.height + 1) * wall_px;
        (width, height)
    }

    /// Check if a position is walkable (not a wall)
    /// This method converts grid coordinates to the maze's internal wall representation
    pub fn is_walkable(&self, x: usize, y: usize) -> bool {
        // Check bounds first
        if x >= self.width || y >= self.height {
            return false;
        }

        // Convert grid coordinates to wall array coordinates
        // In your maze representation, cells are at odd coordinates (row*2+1, col*2+1)
        let wall_row = y * 2 + 1;
        let wall_col = x * 2 + 1;

        // Check if the wall array indices are valid
        if wall_row >= self.walls.len() || wall_col >= self.walls[0].len() {
            return false;
        }

        // A position is walkable if it's not a wall (false in the walls array)
        !self.walls[wall_row][wall_col]
    }

    /// Alternative method: check if a position is a wall
    pub fn is_wall(&self, x: usize, y: usize) -> bool {
        !self.is_walkable(x, y)
    }

    /// Get the actual maze dimensions (number of cells, not wall array size)
    pub fn get_maze_dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// Saves the current maze to a timestamped file in the `src/maze/saved-mazes/generated` directory.
    ///
    /// # File Naming
    /// The output file is named using the current local time in the format:
    /// `Maze_MM-DD-YY_HH-MMPM.mz` (e.g., `Maze_06-24-25_11-24PM.mz`).
    ///
    /// # File Format
    /// The maze is saved as a plain text file, where each cell is represented by either:
    /// - `#` for a wall cell (`true` in `self.walls`)
    /// - ` `(space) for an open cell (`false` in `self.walls`)
    /// - `*` for the exit cell (if one is set)
    ///
    /// Each row of the maze is written on a new line, preserving the maze's 2D structure.
    ///
    /// # Side Effects
    /// - Ensures the output directory exists (creates it if necessary).
    /// - Prints the output file path to stdout upon success.
    /// - Prints error messages to stderr if directory creation, file creation, or writing fails.
    ///
    /// # Returns
    /// - `Ok(PathBuf)` with the path to the saved file on success.
    /// - `Err(std::io::Error)` if any I/O operation fails.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The output directory cannot be created.
    /// - The file cannot be created.
    /// - Any write operation fails.
    ///
    /// # Example Output
    /// ```text
    /// ###################################################
    /// #     #   # #   # # #   # # #     # #     # #   # #
    /// ### ### # # # ### # # ### # ##### # # ### # ### # #
    /// #   #   # #     #     #   #   #     # #     #     #
    /// # ##### # # ####### # ### ### # ####### ##### # # #
    /// #  *  #   # #   #   #       #   # # # # # #   # # #
    /// ##### # # # # ####### ##### ### # # # # # # # #####
    /// # #     # #         # #       # # #     #   #     #
    /// ###################################################
    /// ```
    ///
    /// # See Also
    /// - The generated file can be found in `src/maze/saved-mazes/generated/`.
    pub fn save_to_file(&self) -> Result<PathBuf, std::io::Error> {
        let timestamp = Local::now().format("Maze_%m-%d-%y_%I-%M%p.mz").to_string();
        let output_path = Path::new("src/maze/saved-mazes/generated").join(timestamp);

        if let Err(e) = fs::create_dir_all("src/maze/saved-mazes/generated") {
            eprintln!("Failed to create output directory: {}", e);
            return Err(e);
        }

        let mut file = match fs::File::create(&output_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Failed to create maze file: {}", e);
                return Err(e);
            }
        };

        for (row_idx, row) in self.walls.iter().enumerate() {
            for (col_idx, &cell) in row.iter().enumerate() {
                let symbol = if cell {
                    b'#' // Wall
                } else if row_idx % 2 == 1 && col_idx % 2 == 1 {
                    // This is a maze cell position
                    let maze_row = row_idx / 2;
                    let maze_col = col_idx / 2;
                    let current_cell = Cell::new(maze_row, maze_col);

                    if Some(current_cell) == self.exit_cell {
                        b'*' // Exit cell
                    } else {
                        b' ' // Regular open cell
                    }
                } else {
                    b' ' // Passage
                };

                if let Err(e) = file.write_all(&[symbol]) {
                    eprintln!("Failed to write to file: {}", e);
                    return Err(e);
                }
            }
            if let Err(e) = file.write_all(b"\n") {
                eprintln!("Failed to write newline: {}", e);
                return Err(e);
            }
        }

        println!("Maze saved to: {}", output_path.display());
        if let Some(exit) = self.exit_cell {
            println!("Exit marked at: row {}, col {}", exit.row, exit.col);
        }
        Ok(output_path)
    }
}

/// Maze generator using Kruskal's algorithm
pub struct MazeGenerator {
    /// The maze being generated (wrapped in Arc<Mutex> for thread safety)
    pub maze: Arc<Mutex<Maze>>,
    union_find: UnionFind,
    edges: Vec<Edge>,
    current_edge: usize,
    /// Indicates if generation is complete
    pub generation_complete: bool,
    /// Set of cells currently connected in the maze
    pub connected_cells: HashSet<Cell>,
    /// Number of edges remaining when we switch to fast mode
    pub fast_threshold: usize,
    /// Whether we're in fast mode
    pub fast_mode: bool,
}

impl MazeGenerator {
    /// Creates a new maze generator with the given dimensions
    /// Returns both the generator and a shared reference to the maze
    pub fn new(width: usize, height: usize) -> (Self, Arc<Mutex<Maze>>) {
        let maze = Arc::new(Mutex::new(Maze::new(width, height)));
        let maze_clone = Arc::clone(&maze);
        let mut rng = thread_rng();
        let mut union_find = UnionFind::new();
        let mut edges = Vec::new();

        // Initialize cells
        {
            let mut maze_lock = maze.lock().unwrap();
            for row in 0..height {
                for col in 0..width {
                    let cell = Cell::new(row, col);
                    union_find.make_set(cell);
                    maze_lock.walls[row * 2 + 1][col * 2 + 1] = false;
                }
            }
        }

        // Generate edges
        for row in 0..height {
            for col in 0..width {
                let current = Cell::new(row, col);
                if col + 1 < width {
                    let right = Cell::new(row, col + 1);
                    edges.push(Edge::new(current, right));
                }
                if row + 1 < height {
                    let bottom = Cell::new(row + 1, col);
                    edges.push(Edge::new(current, bottom));
                }
            }
        }

        edges.shuffle(&mut rng);

        {
            let mut maze_lock = maze.lock().unwrap();
            maze_lock.total_edges = edges.len();
            maze_lock.processed_edges = 0;
        }

        let generator = Self {
            maze: Arc::clone(&maze),
            union_find,
            edges,
            current_edge: 0,
            generation_complete: false,
            connected_cells: HashSet::new(),
            fast_threshold: 600, // Switch to fast mode when 600 edges remain
            fast_mode: false,
        };

        (generator, maze_clone)
    }

    /// Performs one step of maze generation
    /// Returns true if a wall was removed in this step
    pub fn step(&mut self) -> bool {
        if self.generation_complete || self.current_edge >= self.edges.len() {
            if !self.generation_complete {
                // Mark generation as complete and set random exit
                self.generation_complete = true;
                let mut maze = self.maze.lock().unwrap();
                maze.set_random_exit();
            }
            return false;
        }

        // Check if we should enter fast mode
        if !self.fast_mode && self.edges.len() - self.current_edge <= self.fast_threshold {
            self.fast_mode = true;
        }

        let edge = self.edges[self.current_edge];
        self.current_edge += 1;

        let mut maze = self.maze.lock().unwrap();
        maze.processed_edges += 1;

        if self.union_find.union(edge.cell1, edge.cell2) {
            let wall_row = edge.cell1.row + edge.cell2.row + 1;
            let wall_col = edge.cell1.col + edge.cell2.col + 1;
            maze.walls[wall_row][wall_col] = false;

            self.connected_cells.insert(edge.cell1);
            self.connected_cells.insert(edge.cell2);
            return true;
        }

        false
    }

    /// Checks if maze generation is complete
    pub fn is_complete(&self) -> bool {
        self.generation_complete
    }

    /// Returns the current progress of generation (processed edges, total edges)
    pub fn get_progress(&self) -> (usize, usize) {
        (self.current_edge, self.edges.len())
    }

    /// Returns the generation progress as a ratio (0.0 to 1.0)
    pub fn get_progress_ratio(&self) -> f32 {
        if self.edges.is_empty() {
            1.0
        } else {
            self.current_edge as f32 / self.edges.len() as f32
        }
    }
}
