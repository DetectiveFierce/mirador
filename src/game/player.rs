//! Player state and movement logic.
//!
//! This module defines the [`Player`] struct, which tracks the player's position, orientation,
//! and movement parameters, and provides methods for movement and view matrix calculation.
//!
//! # Overview
//!
//! The player system handles:
//! - **Position and Orientation**: 3D world position, pitch/yaw angles for camera control
//! - **Movement**: Forward/backward/left/right movement with speed-based physics
//! - **Stamina System**: Sprint mechanics with depletion and regeneration
//! - **Maze Integration**: Cell-based positioning and spawn logic
//! - **View Matrix**: Camera transformation calculations for rendering
//!
//! # Coordinate System
//!
//! The player uses a right-handed coordinate system:
//! - X-axis: Left/Right movement
//! - Y-axis: Up/Down movement (height)
//! - Z-axis: Forward/Backward movement
//!
//! Angles are measured in degrees:
//! - **Pitch**: Up/down look angle (-89° to +89°)
//! - **Yaw**: Left/right look angle (0° to 360°)
//!
//! # Usage Example
//!
//! ```rust
//! use crate::game::player::Player;
//!
//! let mut player = Player::new();
//!
//! // Handle mouse movement
//! player.mouse_movement(10.0, 5.0);
//!
//! // Move player forward
//! player.move_forward(0.016); // 60 FPS delta time
//!
//! // Update stamina
//! player.update_stamina(true, true, 0.016);
//!
//! // Get view matrix for rendering
//! let view_matrix = player.get_view_matrix();
//! ```

use crate::game::maze::generator::Cell;
use crate::math::coordinates::{self, constants::PLAYER_HEIGHT};
use crate::math::mat::Mat4;

/// Represents the player character's state in the world.
///
/// The `Player` struct encapsulates all player-related state including position,
/// orientation, movement parameters, and stamina system. It provides methods for
/// movement, camera control, and integration with the maze system.
///
/// # Fields
///
/// ## Position and Orientation
/// - `position`: 3D world coordinates `[x, y, z]` where y represents height
/// - `pitch`: Vertical look angle in degrees (-89° to +89°)
/// - `yaw`: Horizontal look angle in degrees (0° to 360°)
/// - `fov`: Field of view in degrees for perspective projection
///
/// ## Movement Parameters
/// - `base_speed`: Base movement speed in units per second
/// - `speed`: Current movement speed (can be modified by upgrades/effects)
/// - `mouse_sensitivity`: Multiplier for mouse movement sensitivity
///
/// ## Maze Integration
/// - `current_cell`: The maze cell the player is currently occupying
///
/// ## Stamina System
/// - `stamina`: Current stamina value (0.0 to max_stamina)
/// - `max_stamina`: Maximum stamina capacity
/// - `stamina_regen_cooldown`: Seconds to wait before stamina regeneration starts
/// - `stamina_regen_rate`: Stamina points regenerated per second
/// - `last_sprint_time`: Time accumulator for regeneration cooldown
///
/// # Examples
///
/// ```rust
/// use crate::game::player::Player;
///
/// let player = Player::new();
/// assert_eq!(player.position[1], crate::math::coordinates::constants::PLAYER_HEIGHT);
/// assert_eq!(player.stamina, 1.0);
/// assert_eq!(player.max_stamina, 2.0);
/// ```
#[derive(Debug, Default, Clone)]
pub struct Player {
    /// Player's world position in 3D space `[x, y, z]`.
    ///
    /// - `x`: Left/right position
    /// - `y`: Height/vertical position (typically at `PLAYER_HEIGHT`)
    /// - `z`: Forward/backward position
    pub position: [f32; 3],

    /// Pitch angle in degrees for vertical camera control.
    ///
    /// Controls looking up/down:
    /// - Positive values: Looking up
    /// - Negative values: Looking down
    /// - Clamped between -89° and +89° to prevent camera flipping
    pub pitch: f32,

    /// Yaw angle in degrees for horizontal camera control.
    ///
    /// Controls looking left/right:
    /// - 0°: Looking north
    /// - 90°: Looking east
    /// - 180°: Looking south
    /// - 270°: Looking west
    /// - Wraps around 360°
    pub yaw: f32,

    /// Field of view in degrees for perspective projection.
    ///
    /// Controls how wide the camera view is:
    /// - Higher values: Wider, more fisheye effect
    /// - Lower values: Narrower, more zoomed in
    /// - Typical range: 60° to 120°
    pub fov: f32,

    /// Base movement speed in units per second.
    ///
    /// This is the default movement speed before any modifications
    /// from upgrades, sprinting, or other effects.
    pub base_speed: f32,

    /// Current movement speed in units per second.
    ///
    /// This is the actual speed used for movement calculations.
    /// Can be modified by sprinting, upgrades, or other game effects.
    pub speed: f32,

    /// Mouse sensitivity multiplier for camera control.
    ///
    /// Higher values make mouse movement more responsive.
    /// Lower values make mouse movement more precise.
    pub mouse_sensitivity: f32,

    /// The maze cell the player is currently occupying.
    ///
    /// Used for collision detection, game logic, and maze navigation.
    pub current_cell: Cell,

    /// Current stamina value (0.0 to max_stamina).
    ///
    /// Stamina is consumed when sprinting and regenerates over time.
    /// When stamina reaches 0.0, sprinting is disabled.
    pub stamina: f32,

    /// Maximum stamina capacity.
    ///
    /// The upper limit for the stamina value. Stamina regeneration
    /// stops when this value is reached.
    pub max_stamina: f32,

    /// Seconds to wait before stamina regeneration starts.
    ///
    /// After sprinting stops, the game waits this many seconds
    /// before beginning stamina regeneration.
    pub stamina_regen_cooldown: f32,

    /// Stamina points regenerated per second.
    ///
    /// The rate at which stamina recovers when regeneration is active.
    pub stamina_regen_rate: f32,

    /// Time accumulator for stamina regeneration cooldown.
    ///
    /// Tracks how much time has passed since the last sprint.
    /// When this exceeds `stamina_regen_cooldown`, regeneration begins.
    pub last_sprint_time: f32,
}

impl Player {
    /// Creates a new [`Player`] with default starting position and parameters.
    ///
    /// # Returns
    ///
    /// A new `Player` instance with the following defaults:
    /// - Position: `[0.0, PLAYER_HEIGHT, 0.0]` (center of world at player height)
    /// - Pitch: `3.0°` (slightly looking down)
    /// - Yaw: `316.0°` (facing northwest)
    /// - FOV: `100.0°` (wide field of view)
    /// - Base Speed: `120.0` units/second
    /// - Mouse Sensitivity: `1.0` (normal sensitivity)
    /// - Stamina: `1.0` / `2.0` max (50% full)
    /// - Stamina Regen: `1.5` points/second after `0.7` second cooldown
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::game::player::Player;
    ///
    /// let player = Player::new();
    /// assert_eq!(player.position[1], crate::math::coordinates::constants::PLAYER_HEIGHT);
    /// assert_eq!(player.stamina, 1.0);
    /// assert_eq!(player.max_stamina, 2.0);
    /// ```
    pub fn new() -> Self {
        Self {
            position: [0.0, PLAYER_HEIGHT, 0.0], // Will be set correctly when spawning
            pitch: 3.0,
            yaw: 316.0,
            fov: 100.0,
            base_speed: 120.0,
            speed: 120.0,
            mouse_sensitivity: 1.0,
            current_cell: Cell::default(),
            stamina: 1.0,
            max_stamina: 2.0,
            stamina_regen_cooldown: 0.7,
            stamina_regen_rate: 1.5,
            last_sprint_time: 0.0,
        }
    }

    /// Computes the view matrix for the player's current position and orientation.
    ///
    /// The view matrix transforms world coordinates into camera/view space.
    /// This matrix is used by the rendering system to position the camera
    /// in the 3D world.
    ///
    /// # Returns
    ///
    /// A 4x4 transformation matrix that combines:
    /// 1. Translation to move the world relative to the camera position
    /// 2. Rotation to orient the camera based on pitch and yaw angles
    ///
    /// # Algorithm
    ///
    /// 1. Creates rotation matrices for pitch (X-axis) and yaw (Y-axis)
    /// 2. Combines rotations: applies yaw first, then pitch
    /// 3. Creates translation matrix (negative position to move world opposite to camera)
    /// 4. Multiplies translation by rotation: `translation * rotation`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::game::player::Player;
    /// use crate::math::mat::Mat4;
    ///
    /// let player = Player::new();
    /// let view_matrix = player.get_view_matrix();
    ///
    /// // The view matrix can be used in shaders for camera transformation
    /// // view_matrix is a 4x4 matrix that transforms world coordinates to view space
    /// ```
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

    /// Computes the combined view-projection matrix for rendering.
    ///
    /// This method combines the view matrix (camera transformation) with
    /// the projection matrix (perspective transformation) into a single
    /// matrix for efficient GPU usage.
    ///
    /// # Arguments
    ///
    /// * `aspect_ratio` - Width divided by height of the viewport
    /// * `near` - Distance to the near clipping plane
    /// * `far` - Distance to the far clipping plane
    ///
    /// # Returns
    ///
    /// A 4x4 transformation matrix that transforms world coordinates
    /// directly to clip space: `projection * view`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::game::player::Player;
    ///
    /// let player = Player::new();
    /// let view_proj_matrix = player.get_view_proj_matrix(16.0/9.0, 0.1, 1000.0);
    ///
    /// // This matrix can be passed directly to shaders for efficient rendering
    /// ```
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
    /// This method handles mouse input to control the camera orientation.
    /// The mouse movement is converted to angular changes in pitch and yaw.
    ///
    /// # Arguments
    ///
    /// * `delta_x` - Mouse movement in the X direction (positive = right, negative = left)
    /// * `delta_y` - Mouse movement in the Y direction (positive = down, negative = up)
    ///
    /// # Behavior
    ///
    /// - **Yaw**: Decreases with positive delta_x (mouse right = look left)
    /// - **Pitch**: Decreases with positive delta_y (mouse down = look down)
    /// - **Pitch Clamping**: Automatically clamped to [-89°, +89°] to prevent camera flipping
    /// - **Sensitivity**: Movement is scaled by `mouse_sensitivity` multiplier
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::game::player::Player;
    ///
    /// let mut player = Player::new();
    /// let initial_yaw = player.yaw;
    /// let initial_pitch = player.pitch;
    ///
    /// // Move mouse right (positive delta_x)
    /// player.mouse_movement(10.0, 0.0);
    /// assert!(player.yaw < initial_yaw); // Looking more left
    ///
    /// // Move mouse down (positive delta_y)
    /// player.mouse_movement(0.0, 5.0);
    /// assert!(player.pitch < initial_pitch); // Looking more down
    /// ```
    pub fn mouse_movement(&mut self, delta_x: f64, delta_y: f64) {
        self.yaw -= delta_x as f32 * self.mouse_sensitivity;
        self.pitch -= delta_y as f32 * self.mouse_sensitivity;

        // Clamp pitch to prevent flipping
        self.pitch = self.pitch.clamp(-89.0, 89.0);
    }

    /// Moves the player forward based on current yaw and speed.
    ///
    /// Moves the player in the direction they are currently facing.
    /// The movement is based on the player's yaw angle and current speed.
    ///
    /// # Arguments
    ///
    /// * `delta_time` - Time elapsed since last frame in seconds
    ///
    /// # Movement Calculation
    ///
    /// The forward direction is calculated as:
    /// - `forward_x = sin(yaw_radians)`
    /// - `forward_z = cos(yaw_radians)`
    ///
    /// Position is updated as:
    /// - `position.x -= forward_x * speed * delta_time`
    /// - `position.z -= forward_z * speed * delta_time`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::game::player::Player;
    ///
    /// let mut player = Player::new();
    /// let initial_pos = player.position;
    ///
    /// // Move forward for 1 second at 120 units/second
    /// player.move_forward(1.0);
    ///
    /// // Position should have changed based on initial yaw
    /// assert_ne!(player.position, initial_pos);
    /// ```
    pub fn move_forward(&mut self, delta_time: f32) {
        let forward_x = self.yaw.to_radians().sin();
        let forward_z = self.yaw.to_radians().cos();
        self.position[0] -= forward_x * self.speed * delta_time;
        self.position[2] -= forward_z * self.speed * delta_time;
    }

    /// Moves the player backward based on current yaw and speed.
    ///
    /// Moves the player in the opposite direction they are currently facing.
    /// This is the inverse of `move_forward()`.
    ///
    /// # Arguments
    ///
    /// * `delta_time` - Time elapsed since last frame in seconds
    ///
    /// # Movement Calculation
    ///
    /// Uses the same forward direction calculation as `move_forward()`,
    /// but adds to position instead of subtracting:
    /// - `position.x += forward_x * speed * delta_time`
    /// - `position.z += forward_z * speed * delta_time`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::game::player::Player;
    ///
    /// let mut player = Player::new();
    /// let initial_pos = player.position;
    ///
    /// // Move backward for 1 second at 120 units/second
    /// player.move_backward(1.0);
    ///
    /// // Position should have changed in opposite direction from forward
    /// assert_ne!(player.position, initial_pos);
    /// ```
    pub fn move_backward(&mut self, delta_time: f32) {
        let forward_x = self.yaw.to_radians().sin();
        let forward_z = self.yaw.to_radians().cos();
        self.position[0] += forward_x * self.speed * delta_time;
        self.position[2] += forward_z * self.speed * delta_time;
    }

    /// Moves the player left (strafe) based on current yaw and speed.
    ///
    /// Moves the player perpendicular to their facing direction, to the left.
    /// This is calculated using the right vector rotated 90° counterclockwise.
    ///
    /// # Arguments
    ///
    /// * `delta_time` - Time elapsed since last frame in seconds
    ///
    /// # Movement Calculation
    ///
    /// The right direction is calculated as:
    /// - `right_x = cos(yaw_radians)`
    /// - `right_z = sin(yaw_radians)`
    ///
    /// Left movement is the negative of right:
    /// - `position.x -= right_x * speed * delta_time`
    /// - `position.z += right_z * speed * delta_time`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::game::player::Player;
    ///
    /// let mut player = Player::new();
    /// let initial_pos = player.position;
    ///
    /// // Strafe left for 1 second at 120 units/second
    /// player.move_left(1.0);
    ///
    /// // Position should have changed perpendicular to facing direction
    /// assert_ne!(player.position, initial_pos);
    /// ```
    pub fn move_left(&mut self, delta_time: f32) {
        let right_x = self.yaw.to_radians().cos();
        let right_z = self.yaw.to_radians().sin();
        self.position[0] -= right_x * self.speed * delta_time;
        self.position[2] += right_z * self.speed * delta_time;
    }

    /// Moves the player right (strafe) based on current yaw and speed.
    ///
    /// Moves the player perpendicular to their facing direction, to the right.
    /// This is the inverse of `move_left()`.
    ///
    /// # Arguments
    ///
    /// * `delta_time` - Time elapsed since last frame in seconds
    ///
    /// # Movement Calculation
    ///
    /// Uses the same right direction calculation as `move_left()`,
    /// but with opposite signs:
    /// - `position.x += right_x * speed * delta_time`
    /// - `position.z -= right_z * speed * delta_time`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::game::player::Player;
    ///
    /// let mut player = Player::new();
    /// let initial_pos = player.position;
    ///
    /// // Strafe right for 1 second at 120 units/second
    /// player.move_right(1.0);
    ///
    /// // Position should have changed perpendicular to facing direction
    /// assert_ne!(player.position, initial_pos);
    /// ```
    pub fn move_right(&mut self, delta_time: f32) {
        let right_x = self.yaw.to_radians().cos();
        let right_z = self.yaw.to_radians().sin();
        self.position[0] += right_x * self.speed * delta_time;
        self.position[2] -= right_z * self.speed * delta_time;
    }

    /// Moves the player upward in the Y direction.
    ///
    /// Increases the player's height (Y coordinate) at half the normal movement speed.
    /// This is typically used for flying or vertical movement in test modes.
    ///
    /// # Arguments
    ///
    /// * `delta_time` - Time elapsed since last frame in seconds
    ///
    /// # Movement Calculation
    ///
    /// - `position.y += (speed / 2.0) * delta_time`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::game::player::Player;
    ///
    /// let mut player = Player::new();
    /// let initial_height = player.position[1];
    ///
    /// // Move up for 1 second at 60 units/second (half speed)
    /// player.move_up(1.0);
    ///
    /// assert!(player.position[1] > initial_height);
    /// ```
    pub fn move_up(&mut self, delta_time: f32) {
        self.position[1] += (self.speed / 2.0) * delta_time;
    }

    /// Moves the player downward in the Y direction.
    ///
    /// Decreases the player's height (Y coordinate) at half the normal movement speed.
    /// This is typically used for flying or vertical movement in test modes.
    ///
    /// # Arguments
    ///
    /// * `delta_time` - Time elapsed since last frame in seconds
    ///
    /// # Movement Calculation
    ///
    /// - `position.y -= (speed / 2.0) * delta_time`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::game::player::Player;
    ///
    /// let mut player = Player::new();
    /// let initial_height = player.position[1];
    ///
    /// // Move down for 1 second at 60 units/second (half speed)
    /// player.move_down(1.0);
    ///
    /// assert!(player.position[1] < initial_height);
    /// ```
    pub fn move_down(&mut self, delta_time: f32) {
        self.position[1] -= (self.speed / 2.0) * delta_time;
    }

    /// Updates the player's current maze cell based on their world position.
    ///
    /// Converts the player's world coordinates to maze cell coordinates
    /// and updates the `current_cell` field. This is used for collision
    /// detection, game logic, and maze navigation.
    ///
    /// # Arguments
    ///
    /// * `maze_grid` - 2D grid representing the maze layout (true = wall, false = passage)
    /// * `is_test_mode` - Whether test mode is enabled (affects floor size calculations)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::game::player::Player;
    ///
    /// let mut player = Player::new();
    /// let maze_grid = vec![vec![false, true], vec![false, false]]; // 2x2 maze
    ///
    /// // Update cell based on current position
    /// player.update_cell(&maze_grid, false);
    ///
    /// // current_cell should now reflect the player's position in maze coordinates
    /// ```
    pub fn update_cell(&mut self, maze_grid: &[Vec<bool>], is_test_mode: bool) {
        let maze_width = maze_grid[0].len();
        let maze_height = maze_grid.len();
        let maze_dimensions = (maze_width, maze_height);

        self.current_cell =
            coordinates::world_to_maze(self.position, maze_dimensions, is_test_mode);
    }

    /// Spawns the player at the bottom-left cell of the maze.
    ///
    /// Sets the player's position to the maze entrance (bottom-left cell)
    /// and orients them to face north (into the maze). This is typically
    /// called when starting a new game or respawning.
    ///
    /// # Arguments
    ///
    /// * `maze_grid` - 2D grid representing the maze layout (true = wall, false = passage)
    /// * `is_test_mode` - Whether test mode is enabled (affects floor size calculations)
    ///
    /// # Behavior
    ///
    /// 1. Calculates maze dimensions from the grid
    /// 2. Gets the bottom-left cell coordinates
    /// 3. Converts cell coordinates to world coordinates
    /// 4. Sets player position to the entrance
    /// 5. Sets yaw to face north (0°)
    /// 6. Updates current_cell to match the entrance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::game::player::Player;
    ///
    /// let mut player = Player::new();
    /// let maze_grid = vec![vec![false, true], vec![false, false]]; // 2x2 maze
    ///
    /// // Spawn at maze entrance
    /// player.spawn_at_maze_entrance(&maze_grid, false);
    ///
    /// // Player should now be at the bottom-left cell facing north
    /// assert_eq!(player.yaw, 0.0); // North
    /// ```
    pub fn spawn_at_maze_entrance(&mut self, maze_grid: &[Vec<bool>], is_test_mode: bool) {
        let maze_width = maze_grid[0].len();
        let maze_height = maze_grid.len();
        let maze_dimensions = (maze_width, maze_height);

        // Set the player at the bottom-left cell of the maze
        let entrance_cell = coordinates::get_bottom_left_cell(maze_dimensions);
        self.position = coordinates::maze_to_world(
            &entrance_cell,
            maze_dimensions,
            self.position[1],
            is_test_mode,
        );
        self.current_cell = entrance_cell;

        // Set the initial orientation to face north (into the maze)
        self.yaw = coordinates::direction_to_yaw(coordinates::Direction::North);
    }

    /// Updates the player's stamina based on sprinting state and time.
    ///
    /// This method should be called every frame to manage the stamina system.
    /// It handles stamina depletion during sprinting and regeneration during rest.
    ///
    /// # Arguments
    ///
    /// * `is_sprinting` - Whether the player is currently sprinting
    /// * `is_moving` - Whether the player is currently moving (required for sprinting)
    /// * `delta_time` - Time elapsed since last frame in seconds
    ///
    /// # Stamina System Behavior
    ///
    /// ## Stamina Depletion
    /// - Occurs when `is_sprinting && is_moving && stamina > 0.0`
    /// - Depletes at rate of `0.7` points per second
    /// - Clamps to minimum of `0.0`
    /// - Resets regeneration cooldown timer
    ///
    /// ## Stamina Regeneration
    /// - Starts after `stamina_regen_cooldown` seconds of not sprinting
    /// - Regenerates at rate of `stamina_regen_rate` points per second
    /// - Clamps to maximum of `max_stamina`
    /// - Cooldown timer accumulates when not sprinting
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::game::player::Player;
    ///
    /// let mut player = Player::new();
    /// let initial_stamina = player.stamina;
    ///
    /// // Sprint for 1 second
    /// player.update_stamina(true, true, 1.0);
    /// assert!(player.stamina < initial_stamina);
    ///
    /// // Rest for 1 second (should start regenerating after 0.7s cooldown)
    /// player.update_stamina(false, false, 1.0);
    /// assert!(player.stamina > initial_stamina - 0.7);
    /// ```
    pub fn update_stamina(&mut self, is_sprinting: bool, is_moving: bool, delta_time: f32) {
        if is_sprinting && is_moving && self.stamina > 0.0 {
            self.stamina -= 0.7 * delta_time; // Deplete stamina
            if self.stamina < 0.0 {
                self.stamina = 0.0;
            }
            self.last_sprint_time = 0.0;
        } else {
            self.last_sprint_time += delta_time;
            if self.last_sprint_time > self.stamina_regen_cooldown {
                self.stamina += self.stamina_regen_rate * delta_time;
                if self.stamina > self.max_stamina {
                    self.stamina = self.max_stamina;
                }
            }
        }
    }

    /// Returns the current stamina as a ratio from 0.0 to 1.0.
    ///
    /// This is useful for UI elements like stamina bars that need
    /// to display stamina as a percentage or normalized value.
    ///
    /// # Returns
    ///
    /// A value between 0.0 and 1.0 representing the stamina ratio:
    /// - `0.0`: No stamina (empty)
    /// - `1.0`: Full stamina
    /// - Values in between: Partial stamina
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::game::player::Player;
    ///
    /// let mut player = Player::new();
    ///
    /// // Full stamina
    /// assert_eq!(player.stamina_ratio(), 0.5); // 1.0 / 2.0 = 0.5
    ///
    /// // Deplete stamina
    /// player.stamina = 0.0;
    /// assert_eq!(player.stamina_ratio(), 0.0);
    ///
    /// // Full stamina
    /// player.stamina = player.max_stamina;
    /// assert_eq!(player.stamina_ratio(), 1.0);
    /// ```
    pub fn stamina_ratio(&self) -> f32 {
        (self.stamina / self.max_stamina).clamp(0.0, 1.0)
    }
}
