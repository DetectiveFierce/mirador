//! Player state and movement logic.
//!
//! This module defines the [`Player`] struct, which tracks the player's position, orientation,
//! and movement parameters, and provides methods for movement and view matrix calculation.

use crate::math::coordinates::{self, constants::PLAYER_HEIGHT};
use crate::math::mat::Mat4;
use crate::maze::generator::Cell;

/// Represents the player character's state in the world.
///
/// Includes position, orientation (pitch/yaw), field of view, movement speed, and mouse sensitivity.
#[derive(Debug, Default, Clone)]
pub struct Player {
    /// Player's world position `[x, y, z]`.
    pub position: [f32; 3],
    /// Pitch (up/down look), in degrees.
    pub pitch: f32,
    /// Yaw (left/right look), in degrees.
    pub yaw: f32,
    /// Field of view (degrees).
    pub fov: f32,
    /// Base movement speed (units per second).
    pub base_speed: f32,
    /// Current movement speed (units per second).
    pub speed: f32,
    /// Mouse sensitivity multiplier.
    pub mouse_sensitivity: f32,
    /// Current Cell
    pub current_cell: Cell,
}

impl Player {
    /// Creates a new [`Player`] with default starting position and parameters.
    pub fn new() -> Self {
        Self {
            position: [0.0, PLAYER_HEIGHT, 0.0], // Will be set correctly when spawning
            pitch: 3.0,
            yaw: 316.0,
            fov: 100.0,
            base_speed: 100.0,
            speed: 100.0,
            mouse_sensitivity: 1.0,
            current_cell: Cell::default(),
        }
    }

    /// Computes the view matrix for the player's current position and orientation.
    ///
    /// Combines pitch and yaw rotations, then applies translation.
    pub fn get_view_matrix(&self) -> Mat4 {
        // Create rotation matrices for pitch and yaw
        let pitch_matrix = Mat4::rotation_x(self.pitch);
        let yaw_matrix = Mat4::rotation_y(self.yaw);

        // Combine rotations: apply yaw first, then pitch
        let rotation_matrix = yaw_matrix.multiply(&pitch_matrix);

        // Create translation matrix (negative because we move the world opposite to camera)
        let translation_matrix =
            Mat4::translation(-self.position[0], -self.position[1], -self.position[2]);

        // View matrix = rotation * translation
        translation_matrix.multiply(&rotation_matrix)
    }

    pub fn get_view_proj_matrix(&self, aspect_ratio: f32, near: f32, far: f32) -> Mat4 {
        let view_matrix = self.get_view_matrix();
        let projection_matrix = Mat4::perspective(
            self.fov, // Keep in degrees since your Mat4 likely expects degrees
            aspect_ratio,
            near,
            far,
        );

        // Projection * View (note the order)
        view_matrix.multiply(&projection_matrix)
    }

    /// Updates the player's orientation based on mouse movement.
    ///
    /// # Arguments
    /// * `delta_x` - Mouse movement in the X direction.
    /// * `delta_y` - Mouse movement in the Y direction.
    ///
    /// Clamps pitch to prevent flipping.
    pub fn mouse_movement(&mut self, delta_x: f64, delta_y: f64) {
        self.yaw -= delta_x as f32 * self.mouse_sensitivity;
        self.pitch -= delta_y as f32 * self.mouse_sensitivity;

        // Clamp pitch to prevent flipping
        self.pitch = self.pitch.clamp(-89.0, 89.0);
    }

    /// Moves the player forward based on current yaw and speed.
    pub fn move_forward(&mut self, delta_time: f32) {
        let forward_x = self.yaw.to_radians().sin();
        let forward_z = self.yaw.to_radians().cos();
        self.position[0] -= forward_x * self.speed * delta_time;
        self.position[2] -= forward_z * self.speed * delta_time;
    }

    /// Moves the player backward based on current yaw and speed.
    pub fn move_backward(&mut self, delta_time: f32) {
        let forward_x = self.yaw.to_radians().sin();
        let forward_z = self.yaw.to_radians().cos();
        self.position[0] += forward_x * self.speed * delta_time;
        self.position[2] += forward_z * self.speed * delta_time;
    }

    /// Moves the player left based on current yaw and speed.
    pub fn move_left(&mut self, delta_time: f32) {
        let right_x = self.yaw.to_radians().cos();
        let right_z = self.yaw.to_radians().sin();
        self.position[0] -= right_x * self.speed * delta_time;
        self.position[2] += right_z * self.speed * delta_time;
    }

    /// Moves the player right based on current yaw and speed.
    pub fn move_right(&mut self, delta_time: f32) {
        let right_x = self.yaw.to_radians().cos();
        let right_z = self.yaw.to_radians().sin();
        self.position[0] += right_x * self.speed * delta_time;
        self.position[2] -= right_z * self.speed * delta_time;
    }

    pub fn update_cell(&mut self, maze_grid: &[Vec<bool>]) {
        let maze_width = maze_grid[0].len();
        let maze_height = maze_grid.len();
        let maze_dimensions = (maze_width, maze_height);

        self.current_cell = coordinates::world_to_maze(self.position, maze_dimensions);
    }

    /// Spawns the player at the bottom-left cell of the maze.
    ///
    /// # Arguments
    /// * `maze_grid` - The maze grid representing walls and passages
    pub fn spawn_at_maze_entrance(&mut self, maze_grid: &[Vec<bool>]) {
        let maze_width = maze_grid[0].len();
        let maze_height = maze_grid.len();
        let maze_dimensions = (maze_width, maze_height);

        // Set the player at the bottom-left cell of the maze
        let entrance_cell = coordinates::get_bottom_left_cell(maze_dimensions);
        self.position = coordinates::maze_to_world(&entrance_cell, maze_dimensions, PLAYER_HEIGHT);
        self.current_cell = entrance_cell;

        // Set the initial orientation to face north (into the maze)
        self.yaw = coordinates::direction_to_yaw(coordinates::Direction::North);
    }
}
