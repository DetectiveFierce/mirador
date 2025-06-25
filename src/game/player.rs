//! Player state and movement logic.
//!
//! This module defines the [`Player`] struct, which tracks the player's position, orientation,
//! and movement parameters, and provides methods for movement and view matrix calculation.

use crate::math::mat::Mat4;

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
    /// Movement speed (units per second).
    pub speed: f32,
    /// Mouse sensitivity multiplier.
    pub mouse_sensitivity: f32,
}

impl Player {
    /// Creates a new [`Player`] with default starting position and parameters.
    pub fn new() -> Self {
        Self {
            position: [-1475.0, 50.0, 1475.0], // Start above and behind the floor
            pitch: 3.0,
            yaw: 316.0,
            fov: 100.0,
            speed: 60.0,
            mouse_sensitivity: 1.0,
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
        let rotation_matrix = pitch_matrix.multiply(&yaw_matrix);

        // Create translation matrix (negative because we move the world opposite to camera)
        let translation_matrix =
            Mat4::translation(-self.position[0], -self.position[1], -self.position[2]);

        // View matrix = rotation * translation
        rotation_matrix.multiply(&translation_matrix)
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
}
