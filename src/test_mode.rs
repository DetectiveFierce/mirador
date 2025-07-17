//! Test mode functionality for Mirador.
//!
//! This module provides a simplified test environment when TEST_MODE is enabled.
//! It creates a small maze with only perimeter walls, places the exit in the center,
//! locks the enemy in a fixed position, and freezes the timer at 420.00 seconds.

use crate::game::GameState;
use crate::game::TimerConfig;
use crate::game::enemy::Enemy;
use crate::game::maze::generator::Cell;
use crate::renderer::primitives::Vertex;
use crate::renderer::wgpu_lib::WgpuRenderer;
use glyphon::Color;
use std::time::Duration;
use wgpu;
use wgpu::util::DeviceExt;

/// Test maze dimensions (smaller than normal maze)
const TEST_MAZE_WIDTH: usize = 6;
const TEST_MAZE_HEIGHT: usize = 6;

/// Creates a test maze grid with only perimeter walls
pub fn create_test_maze_grid() -> Vec<Vec<bool>> {
    let mut maze_grid = vec![vec![false; TEST_MAZE_WIDTH * 2 + 1]; TEST_MAZE_HEIGHT * 2 + 1];

    // Add perimeter walls
    let width = maze_grid[0].len();
    let height = maze_grid.len();

    // Top and bottom walls
    for col in 0..width {
        maze_grid[0][col] = true; // Top wall
        maze_grid[height - 1][col] = true; // Bottom wall
    }

    // Left and right walls
    for row in 0..height {
        maze_grid[row][0] = true; // Left wall
        maze_grid[row][width - 1] = true; // Right wall
    }

    // Debug: Print the maze grid
    println!("Test maze grid ({}x{}):", width, height);
    for (i, row) in maze_grid.iter().enumerate() {
        let row_str: String = row
            .iter()
            .map(|&cell| if cell { '#' } else { ' ' })
            .collect();
        println!("Row {:2}: {}", i, row_str);
    }

    maze_grid
}

/// Gets the center cell of the test maze for the exit
pub fn get_test_exit_cell() -> Cell {
    // For even-sized mazes, pick the cell closest to the world center
    // (for 6x6, this is (2,2) or (3,3); we'll use (2,2) for logic)
    Cell::new((TEST_MAZE_HEIGHT - 1) / 2, (TEST_MAZE_WIDTH - 1) / 2)
}

/// Returns the world coordinates of the center of the room for exit patch rendering
pub fn get_world_center_for_exit_patch(maze_grid: &[Vec<bool>]) -> (f32, f32) {
    let maze_width = maze_grid[0].len();
    let maze_height = maze_grid.len();
    let max_dimension = maze_width.max(maze_height) as f32;
    let floor_size = crate::math::coordinates::constants::get_floor_size(true); // Test mode floor size
    let cell_size = floor_size / max_dimension;
    let origin_x = -(maze_width as f32 * cell_size) / 2.0;
    let origin_z = -(maze_height as f32 * cell_size) / 2.0;
    let center_x = origin_x + (maze_width as f32) * cell_size / 2.0;
    let center_z = origin_z + (maze_height as f32) * cell_size / 2.0;
    (center_x, center_z)
}

/// Converts maze cell to wall grid coordinates for exit placement
pub fn maze_cell_to_wall_grid(cell: &Cell) -> (usize, usize) {
    // Convert maze cell coordinates to wall grid coordinates
    // Wall grid has walls at even indices, cells at odd indices
    let wall_row = cell.row * 2 + 1;
    let wall_col = cell.col * 2 + 1;
    (wall_row, wall_col)
}

/// Places the exit marker in the maze grid
pub fn place_exit_in_maze_grid(maze_grid: &mut Vec<Vec<bool>>, exit_cell: &Cell) {
    let (wall_row, wall_col) = maze_cell_to_wall_grid(exit_cell);

    // Ensure we're within bounds
    if wall_row < maze_grid.len() && wall_col < maze_grid[0].len() {
        // Mark the exit cell (we'll use a special value that parse_maze_file recognizes)
        // Since parse_maze_file looks for '*' character, we need to handle this differently
        // For now, we'll just mark it as a non-wall (false) and handle exit placement separately
        maze_grid[wall_row][wall_col] = false;
    }
}

/// Creates a locked enemy positioned further from the exit
pub fn create_test_enemy(exit_patch_position: [f32; 3]) -> Enemy {
    // Move the enemy 2 cells away in both X and Z directions
    let floor_size = crate::math::coordinates::constants::get_floor_size(true); // Test mode floor size
    let patch_size = floor_size / 13.0; // 13 is the wall grid size for 6x6 maze
    let enemy_position = [
        exit_patch_position[0] - 4.0 * patch_size, // 2 cells to the left
        30.0,                                      // Same height
        exit_patch_position[2] - 4.0 * patch_size, // 2 cells back
    ];

    let mut enemy = Enemy::new(enemy_position, 150.0);
    // Lock the enemy in place
    enemy.pathfinder.locked = true;

    enemy
}

/// Creates a test timer configuration with frozen time
pub fn create_test_timer_config() -> TimerConfig {
    TimerConfig {
        duration: Duration::from_secs_f32(420.0), // 420.00 seconds
        warning_threshold: Duration::from_secs_f32(420.0), // Never warn
        critical_threshold: Duration::from_secs_f32(420.0), // Never critical
        normal_color: Color::rgb(100, 255, 100),
        warning_color: Color::rgb(255, 255, 100),
        critical_color: Color::rgb(255, 100, 100),
    }
}

/// Sets up the test environment
pub fn setup_test_environment(game_state: &mut GameState, wgpu_renderer: &mut WgpuRenderer) {
    // Create test maze grid
    let mut maze_grid = create_test_maze_grid();
    let exit_cell = get_test_exit_cell();

    // Place exit in the grid
    place_exit_in_maze_grid(&mut maze_grid, &exit_cell);

    // Generate geometry from the test maze
    // Use the world center for the exit patch
    let exit_patch_position = get_world_center_for_exit_patch(&maze_grid);
    let (mut floor_vertices, _) = Vertex::create_floor_vertices(&maze_grid, None, true); // Test mode floor size
    // Add a green exit patch at the world center
    floor_vertices.extend(Vertex::create_exit_patch_at_world_position(
        exit_patch_position,
        true, // Test mode
    ));

    // Set exit position in renderer (as tuple)
    wgpu_renderer.game_renderer.exit_position = Some(exit_patch_position);

    // Add wall vertices (test mode always uses perimeter walls)
    floor_vertices.append(&mut Vertex::create_wall_vertices(&maze_grid, true));

    // Create vertex buffer
    wgpu_renderer.game_renderer.vertex_buffer =
        wgpu_renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Test Maze Vertex Buffer"),
                contents: bytemuck::cast_slice(&floor_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

    // Update vertex count so the renderer knows how many vertices to draw
    wgpu_renderer.game_renderer.vertex_count = floor_vertices.len() as u32;

    println!(
        "Debug: Created {} vertices for test maze",
        floor_vertices.len()
    );
    println!(
        "Debug: Maze grid dimensions: {}x{}",
        maze_grid.len(),
        maze_grid[0].len()
    );
    println!(
        "Debug: Wall count: {}",
        maze_grid
            .iter()
            .flatten()
            .filter(|&&is_wall| is_wall)
            .count()
    );

    // Set exit cell in game state
    game_state.exit_cell = Some(exit_cell);

    // Create and place locked enemy (convert tuple to array for enemy position)
    let enemy_position = [exit_patch_position.0, 30.0, exit_patch_position.1];
    game_state.enemy = create_test_enemy(enemy_position);

    // Build collision system from test maze (test mode always uses perimeter walls)
    game_state
        .collision_system
        .build_from_maze(&maze_grid, true);

    // Spawn player at the entrance (bottom-left corner)
    game_state.player.spawn_at_maze_entrance(&maze_grid, true); // Test mode

    // Set up frozen timer
    let timer_config = create_test_timer_config();
    game_state.start_game_timer(Some(timer_config));

    // Immediately pause the timer to freeze it at 420.00
    game_state.game_ui.pause_timer();

    println!("Test mode initialized:");
    println!("  - Maze size: {}x{}", TEST_MAZE_WIDTH, TEST_MAZE_HEIGHT);
    println!("  - Exit at: {:?}", exit_cell);
    println!("  - Timer frozen at 420.00 seconds");
    println!(
        "  - Enemy locked at position: {:?}",
        game_state.enemy.pathfinder.position
    );
}
