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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell {
    pub row: usize,
    pub col: usize,
}

impl Cell {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Edge {
    pub cell1: Cell,
    pub cell2: Cell,
}

impl Edge {
    pub fn new(cell1: Cell, cell2: Cell) -> Self {
        Self { cell1, cell2 }
    }
}

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
    pub fn new() -> Self {
        Self {
            parent: HashMap::new(),
            rank: HashMap::new(),
        }
    }

    pub fn make_set(&mut self, cell: Cell) {
        if let Entry::Vacant(entry) = self.parent.entry(cell) {
            entry.insert(cell);
            self.rank.insert(cell, 0);
        }
    }

    pub fn find(&mut self, cell: Cell) -> Cell {
        if self.parent[&cell] != cell {
            let root = self.find(self.parent[&cell]);
            self.parent.insert(cell, root);
        }
        self.parent[&cell]
    }

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

#[derive(Clone)]
pub struct Maze {
    pub width: usize,
    pub height: usize,
    pub walls: Vec<Vec<bool>>,
    pub total_edges: usize,
    pub processed_edges: usize,
}

impl Maze {
    pub fn new(width: usize, height: usize) -> Self {
        let walls = vec![vec![true; width * 2 + 1]; height * 2 + 1];
        Self {
            width,
            height,
            walls,
            total_edges: 0,
            processed_edges: 0,
        }
    }

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
                        if connected.contains(&cell) {
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

    pub fn get_dimensions(&self) -> (usize, usize) {
        let cell_px = 4;
        let wall_px = 1;

        let width = self.width * cell_px + (self.width + 1) * wall_px;
        let height = self.height * cell_px + (self.height + 1) * wall_px;

        (width, height)
    }

    pub fn save_to_file(&self) -> Result<PathBuf, std::io::Error> {
        let timestamp = Local::now().format("Maze_%m-%d-%y_%I-%M%p.mz").to_string();
        let output_path = Path::new("src/maze/saved-mazes/generated").join(timestamp);

        if let Err(e) = fs::create_dir_all("output") {
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

        for row in &self.walls {
            for &cell in row {
                let symbol = if cell { b'#' } else { b' ' };
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
        Ok(output_path)
    }
}

pub struct MazeGenerator {
    pub maze: Arc<Mutex<Maze>>,
    pub union_find: UnionFind,
    pub edges: Vec<Edge>,
    pub current_edge: usize,
    pub generation_complete: bool,
    pub connected_cells: HashSet<Cell>,
    pub fast_threshold: usize, // Number of edges remaining when we switch to fast mode
    pub fast_mode: bool,       // Whether we're in fast mode
}

impl MazeGenerator {
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
            fast_threshold: 600, // Switch to fast mode when 200 edges remain
            fast_mode: false,
        };

        (generator, maze_clone)
    }

    pub fn step(&mut self) -> bool {
        if self.generation_complete || self.current_edge >= self.edges.len() {
            self.generation_complete = true;
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

    pub fn is_complete(&self) -> bool {
        self.generation_complete
    }

    pub fn get_progress(&self) -> (usize, usize) {
        (self.current_edge, self.edges.len())
    }

    pub fn get_progress_ratio(&self) -> f32 {
        if self.edges.is_empty() {
            1.0
        } else {
            self.current_edge as f32 / self.edges.len() as f32
        }
    }
}
