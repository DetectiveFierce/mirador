//! Enemy AI system for the Mirador game.
//!
//! This module provides a sophisticated enemy AI system with pathfinding, level-based scaling,
//! and strategic placement. The enemy AI adapts its behavior based on the current game level,
//! becoming more aggressive and intelligent as the player progresses.
//!
//! # Key Features
//!
//! - **Level-based scaling**: Enemy speed, aggression, and intelligence scale with game level
//! - **Advanced pathfinding**: Uses rotation-based pathfinding with collision detection
//! - **Strategic placement**: Enemies are placed intelligently relative to player and exit
//! - **Stuck detection**: AI can detect when stuck and attempt escape maneuvers
//! - **Pursuit behavior**: Enemies become more aggressive when player is within detection range
//!
//! # Usage
//!
//! ```rust
//! use crate::game::enemy::{Enemy, place_enemy_standard};
//!
//! // Create an enemy at a specific position
//! let mut enemy = Enemy::new([100.0, 30.0, 100.0], 150.0);
//!
//! // Update enemy AI each frame
//! enemy.update(player_position, delta_time, current_level, collision_checker);
//!
//! // Place enemy strategically
//! let enemy = place_enemy_standard(exit_pos, player_pos, level, collision_checker);
//! ```

use crate::math::vec::Vec3;
use std::f32::consts::PI;

/// Represents an enemy entity in the game with AI-driven behavior.
///
/// The enemy uses a pathfinding system to navigate toward the player while avoiding
/// obstacles. Its behavior scales with the game level, becoming more aggressive
/// and intelligent as the player progresses.
///
/// # Behavior Scaling
///
/// - **Speed**: Increases by 20% per level, capped at 500% of base speed
/// - **Path radius**: Decreases with level (enemy gets closer before acting)
/// - **Arrival threshold**: Decreases with level (enemy is more persistent)
/// - **Rotation step**: Increases with level (enemy tries more directions faster)
/// - **Pursuit distance**: Increases with level (enemy detects player from farther away)
///
/// # Example
///
/// ```rust
/// let mut enemy = Enemy::new([100.0, 30.0, 100.0], 150.0);
///
/// // Update enemy behavior each frame
/// enemy.update(
///     player_position,
///     delta_time,
///     current_level,
///     |start, end| collision_system.intersects(start, end)
/// );
/// ```
#[derive(Debug, Clone)]
pub struct Enemy {
    /// The visual size of the enemy sprite in pixels
    pub size: f32,
    /// The pathfinding system that controls enemy movement
    pub pathfinder: EnemyPathfinder,
    /// Base movement speed in units per second (before level scaling)
    pub base_speed: f32,
    /// Current movement speed after level-based scaling
    pub current_speed: f32,
}

impl Enemy {
    /// Creates a new enemy at the specified position with the given path radius.
    ///
    /// # Arguments
    ///
    /// * `position` - The initial 3D position of the enemy `[x, y, z]`
    /// * `path_radius` - The radius within which the enemy will actively pathfind toward the player
    ///
    /// # Returns
    ///
    /// A new `Enemy` instance with default settings.
    ///
    /// # Example
    ///
    /// ```rust
    /// let enemy = Enemy::new([100.0, 30.0, 100.0], 150.0);
    /// ```
    pub fn new(position: [f32; 3], path_radius: f32) -> Self {
        Self {
            size: 100.0, // Default sprite size
            pathfinder: EnemyPathfinder::new(position, path_radius),
            base_speed: 150.0, // Slightly reduced base speed for better scaling
            current_speed: 150.0,
        }
    }

    /// Updates the enemy's behavior and position based on the current game state.
    ///
    /// This method handles level-based scaling, pathfinding updates, and movement.
    /// The enemy will attempt to move toward the player while avoiding obstacles.
    ///
    /// # Arguments
    ///
    /// * `player_position` - Current 3D position of the player `[x, y, z]`
    /// * `delta_time` - Time elapsed since last frame in seconds
    /// * `level` - Current game level (affects enemy aggression)
    /// * `line_intersects_geometry` - Function to check if a line intersects with game geometry
    ///
    /// # Behavior
    ///
    /// - Scales enemy aggression based on level
    /// - Updates pathfinding to find optimal path to player
    /// - Moves enemy toward current target while respecting speed limits
    /// - Handles collision detection and avoidance
    ///
    /// # Example
    ///
    /// ```rust
    /// enemy.update(
    ///     [50.0, 30.0, 50.0],  // player position
    ///     0.016,               // delta time (60 FPS)
    ///     3,                   // level 3
    ///     |start, end| collision_system.line_intersects_wall(start, end)
    /// );
    /// ```
    pub fn update<F>(
        &mut self,
        player_position: [f32; 3],
        delta_time: f32,
        level: u32,
        line_intersects_geometry: F,
    ) where
        F: Fn([f32; 3], [f32; 3]) -> bool,
    {
        // Prevent movement if locked
        if self.pathfinder.locked {
            return;
        }
        // Scale aggression based on level
        self.scale_aggression_by_level(level);

        // Update pathfinding with level-aware parameters
        if let Some(_target) =
            self.pathfinder
                .update(player_position, level, line_intersects_geometry)
        {
            // Move towards the target
            if let Some(direction) = self.pathfinder.get_movement_direction() {
                let direction_vec = Vec3(direction);
                if let Some(target) = self.pathfinder.current_target {
                    let position_vec = Vec3(self.pathfinder.position);
                    let target_vec = Vec3(target);
                    let distance_to_target = position_vec.distance_to(&target_vec);
                    let max_step = self.current_speed * delta_time;
                    let step = max_step.min(distance_to_target);
                    let movement = direction_vec * step;
                    let new_position = position_vec + movement;
                    self.pathfinder.set_position(*new_position.as_array());
                }
            }
        }
    }

    /// Scales enemy aggression parameters based on the current game level.
    ///
    /// This method adjusts various enemy attributes to make them more challenging
    /// as the player progresses through levels.
    ///
    /// # Scaling Factors
    ///
    /// - **Speed**: Increases by 20% per level, capped at 500% of base speed
    /// - **Path radius**: Decreases by 15% per level, minimum 30% of original
    /// - **Arrival threshold**: Decreases by 10% per level, minimum 20% of original
    /// - **Rotation step**: Increases by 25% per level, maximum 300% of original
    /// - **Pursuit distance**: Increases by 30% per level, maximum 400% of original
    ///
    /// # Arguments
    ///
    /// * `level` - Current game level (1-based)
    ///
    /// # Example
    ///
    /// ```rust
    /// enemy.scale_aggression_by_level(5); // Scale for level 5
    /// ```
    fn scale_aggression_by_level(&mut self, level: u32) {
        let level_f = level as f32;

        // Speed scaling: increases by 20% per level, capped at 500% of base speed
        let speed_multiplier = (1.0 + (level_f * 0.2)).min(5.0);
        self.current_speed = self.base_speed * speed_multiplier;

        // Update pathfinder aggression parameters
        self.pathfinder.update_aggression_for_level(level);
    }

    /// Returns current aggression statistics for debugging purposes.
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// - `f32`: Current movement speed
    /// - `f32`: Current path radius
    /// - `f32`: Current arrival threshold
    /// - `f32`: Current rotation step
    ///
    /// # Example
    ///
    /// ```rust
    /// let (speed, radius, threshold, rotation) = enemy.get_aggression_stats();
    /// println!("Enemy speed: {}, radius: {}, threshold: {}, rotation: {}",
    ///          speed, radius, threshold, rotation);
    /// ```
    pub fn get_aggression_stats(&self) -> (f32, f32, f32, f32) {
        (
            self.current_speed,
            self.pathfinder.path_radius,
            self.pathfinder.arrival_threshold,
            self.pathfinder.rotation_step,
        )
    }
}

/// Advanced pathfinding system for enemy movement and navigation.
///
/// The `EnemyPathfinder` handles all aspects of enemy movement including:
/// - Target calculation and pathfinding
/// - Collision detection and avoidance
/// - Stuck detection and recovery
/// - Level-based behavior scaling
/// - Pursuit behavior when player is detected
///
/// # Pathfinding Strategy
///
/// The pathfinder uses a rotation-based approach where it:
/// 1. Calculates an ideal path toward the player
/// 2. If blocked, rotates around the ideal direction to find alternative paths
/// 3. Uses level-based parameters to adjust aggressiveness
/// 4. Implements stuck detection to prevent infinite loops
///
/// # Example
///
/// ```rust
/// let mut pathfinder = EnemyPathfinder::new([100.0, 30.0, 100.0], 150.0);
///
/// // Update pathfinding
/// if let Some(target) = pathfinder.update(player_pos, level, collision_checker) {
///     // Enemy has a valid target to move toward
/// }
/// ```
#[derive(Debug, Clone)]
pub struct EnemyPathfinder {
    /// Current 3D position of the enemy `[x, y, z]`
    pub position: [f32; 3],
    /// Current target position the enemy is moving toward
    pub current_target: Option<[f32; 3]>,
    /// Current path radius (scales with level)
    pub path_radius: f32,
    /// Base path radius (before level scaling)
    pub base_path_radius: f32,
    /// Current rotation step for pathfinding (scales with level)
    pub rotation_step: f32,
    /// Base rotation step (before level scaling)
    pub base_rotation_step: f32,
    /// Distance threshold for considering target reached (scales with level)
    pub arrival_threshold: f32,
    /// Base arrival threshold (before level scaling)
    pub base_arrival_threshold: f32,
    /// Counter for stuck detection
    pub stuck_counter: i32,
    /// Previous position for stuck detection
    pub last_position: [f32; 3],
    /// Whether the enemy has reached the player
    pub reached_player: bool,
    /// Whether the enemy is locked (cannot move)
    pub locked: bool,
    /// Current aggression level (matches game level)
    pub aggression_level: u32,
    /// Distance at which enemy starts pursuing more aggressively (scales with level)
    pub pursuit_distance: f32,
    /// Base pursuit distance (before level scaling)
    pub base_pursuit_distance: f32,
}

impl EnemyPathfinder {
    /// Creates a new pathfinder at the specified position with the given path radius.
    ///
    /// # Arguments
    ///
    /// * `position` - Initial 3D position `[x, y, z]`
    /// * `path_radius` - Base path radius for pathfinding calculations
    ///
    /// # Returns
    ///
    /// A new `EnemyPathfinder` instance with default settings.
    ///
    /// # Example
    ///
    /// ```rust
    /// let pathfinder = EnemyPathfinder::new([100.0, 30.0, 100.0], 150.0);
    /// ```
    pub fn new(position: [f32; 3], path_radius: f32) -> Self {
        Self {
            position,
            current_target: None,
            path_radius,
            base_path_radius: path_radius,
            rotation_step: PI / 8.0, // 11.25 degrees per step
            base_rotation_step: PI / 8.0,
            arrival_threshold: 0.65,
            base_arrival_threshold: 0.65,
            stuck_counter: 0,
            last_position: position,
            reached_player: false,
            locked: true,
            aggression_level: 1,
            pursuit_distance: path_radius * 2.0,
            base_pursuit_distance: path_radius * 2.0,
        }
    }

    /// Updates aggression parameters based on the current game level.
    ///
    /// This method scales various pathfinding parameters to make the enemy
    /// more challenging and intelligent at higher levels.
    ///
    /// # Scaling Details
    ///
    /// - **Path radius**: Decreases by 15% per level (enemy gets closer before acting)
    /// - **Arrival threshold**: Decreases by 10% per level (enemy is more persistent)
    /// - **Rotation step**: Increases by 25% per level (enemy tries more directions faster)
    /// - **Pursuit distance**: Increases by 30% per level (enemy detects player from farther away)
    ///
    /// # Arguments
    ///
    /// * `level` - Current game level (1-based)
    ///
    /// # Example
    ///
    /// ```rust
    /// pathfinder.update_aggression_for_level(3); // Scale for level 3
    /// ```
    pub fn update_aggression_for_level(&mut self, level: u32) {
        self.aggression_level = level;
        let level_f = level as f32;

        // Path radius decreases with level (enemy gets closer before acting)
        // Reduces by 15% per level, minimum 30% of original
        let radius_multiplier = (1.0 - (level_f * 0.15)).max(0.3);
        self.path_radius = self.base_path_radius * radius_multiplier;

        // Arrival threshold decreases (enemy is more persistent)
        // Reduces by 10% per level, minimum 20% of original
        let threshold_multiplier = (1.0 - (level_f * 0.1)).max(0.2);
        self.arrival_threshold = self.base_arrival_threshold * threshold_multiplier;

        // Rotation step increases (enemy tries more directions faster)
        // Increases by 25% per level, maximum 300% of original
        let rotation_multiplier = (1.0 + (level_f * 0.25)).min(3.0);
        self.rotation_step = self.base_rotation_step * rotation_multiplier;

        // Pursuit distance increases (enemy detects player from farther away)
        // Increases by 30% per level, maximum 400% of original
        let pursuit_multiplier = (1.0 + (level_f * 0.3)).min(4.0);
        self.pursuit_distance = self.base_pursuit_distance * pursuit_multiplier;
    }

    /// Main pathfinding update function with level awareness.
    ///
    /// This method handles the complete pathfinding cycle including:
    /// - Stuck detection and recovery
    /// - Target calculation and validation
    /// - Level-based behavior adjustments
    ///
    /// # Arguments
    ///
    /// * `player_position` - Current 3D position of the player `[x, y, z]`
    /// * `level` - Current game level for behavior scaling
    /// * `line_intersects_geometry` - Function to check for collision with geometry
    ///
    /// # Returns
    ///
    /// `Some(target_position)` if a valid target was found, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// if let Some(target) = pathfinder.update(player_pos, level, collision_checker) {
    ///     // Enemy has a valid target to move toward
    ///     println!("Moving toward: {:?}", target);
    /// }
    /// ```
    pub fn update<F>(
        &mut self,
        player_position: [f32; 3],
        level: u32,
        line_intersects_geometry: F,
    ) -> Option<[f32; 3]>
    where
        F: Fn([f32; 3], [f32; 3]) -> bool,
    {
        // Use benchmark macro for pathfinding timing
        crate::benchmark!("enemy_pathfinding_update", {
            // Update stuck detection with level-based tolerance
            self.update_stuck_detection(level);

            // Check if we need to calculate a new target
            if self.needs_new_target(level) {
                self.calculate_new_target(player_position, level, line_intersects_geometry);
            }

            self.current_target
        })
    }

    /// Updates stuck detection with level-based sensitivity.
    ///
    /// This method tracks enemy movement and detects when the enemy is stuck.
    /// Higher levels have lower tolerance for being stuck and reset faster.
    ///
    /// # Arguments
    ///
    /// * `level` - Current game level for tolerance scaling
    ///
    /// # Behavior
    ///
    /// - Tracks distance moved since last update
    /// - Higher levels have lower stuck thresholds
    /// - Resets stuck counter faster at higher levels
    /// - Updates last position for next frame
    ///
    /// # Example
    ///
    /// ```rust
    /// pathfinder.update_stuck_detection(5); // Update for level 5
    /// ```
    fn update_stuck_detection(&mut self, level: u32) {
        let current_vec = Vec3(self.position);
        let last_vec = Vec3(self.last_position);
        let distance_moved = current_vec.distance_to(&last_vec);

        // Higher levels have lower tolerance for being stuck
        let stuck_threshold = (5.0 / (1.0 + level as f32 * 0.2)).max(1.0);

        if distance_moved < stuck_threshold {
            self.stuck_counter += 1;
        } else {
            self.stuck_counter = 0;
        }

        // Reset stuck counter faster at higher levels
        let max_stuck_time = (120.0 / (1.0 + level as f32 * 0.1)) as i32;
        if self.stuck_counter > max_stuck_time {
            self.stuck_counter = 0;
        }

        self.last_position = self.position;
    }

    /// Determines if the enemy needs a new target with level-based urgency.
    ///
    /// This method checks various conditions to determine if the enemy should
    /// recalculate its path. Higher levels recalculate targets more frequently.
    ///
    /// # Arguments
    ///
    /// * `level` - Current game level for urgency scaling
    ///
    /// # Returns
    ///
    /// `true` if a new target should be calculated, `false` otherwise.
    ///
    /// # Conditions
    ///
    /// - No current target exists
    /// - Distance to target is within arrival threshold
    /// - Stuck counter exceeds level-based threshold
    ///
    /// # Example
    ///
    /// ```rust
    /// if pathfinder.needs_new_target(3) {
    ///     // Calculate new target for level 3
    /// }
    /// ```
    fn needs_new_target(&self, level: u32) -> bool {
        match self.current_target {
            None => true,
            Some(target) => {
                let enemy_vec = Vec3(self.position);
                let target_vec = Vec3(target);
                let distance = enemy_vec.distance_to(&target_vec);

                // Higher levels recalculate targets more frequently
                let stuck_threshold = (60.0 / (1.0 + level as f32 * 0.2)) as i32;

                distance <= self.arrival_threshold || self.stuck_counter > stuck_threshold
            }
        }
    }

    /// Calculates a new target point with level-based aggression.
    ///
    /// This method implements the core pathfinding algorithm using a rotation-based
    /// approach. It tries to find the optimal path toward the player while avoiding
    /// obstacles and adapting behavior based on the current level.
    ///
    /// # Arguments
    ///
    /// * `player_position` - Current 3D position of the player `[x, y, z]`
    /// * `level` - Current game level for behavior scaling
    /// * `line_intersects_geometry` - Function to check for collision with geometry
    ///
    /// # Algorithm
    ///
    /// 1. **Direct pursuit**: If player is within close pursuit radius, move directly toward player
    /// 2. **Ideal path**: Try to move toward ideal target point
    /// 3. **Rotation search**: If blocked, rotate around ideal direction to find alternative paths
    /// 4. **Escape behavior**: If stuck, try escape maneuvers with increased radius
    ///
    /// # Level Scaling
    ///
    /// - Higher levels have tighter pursuit radius
    /// - More aggressive rotation patterns at higher levels
    /// - Enhanced escape behavior with more directions
    /// - Varied radius calculations based on stuck counter
    ///
    /// # Example
    ///
    /// ```rust
    /// pathfinder.calculate_new_target(
    ///     [50.0, 30.0, 50.0],  // player position
    ///     3,                   // level 3
    ///     |start, end| collision_system.intersects(start, end)
    /// );
    /// ```
    fn calculate_new_target<F>(
        &mut self,
        player_position: [f32; 3],
        level: u32,
        line_intersects_geometry: F,
    ) where
        F: Fn([f32; 3], [f32; 3]) -> bool,
    {
        let enemy_vec = Vec3(self.position);
        let player_vec = Vec3(player_position);
        let enemy_2d = enemy_vec.to_2d();
        let player_2d = player_vec.to_2d();

        let direction_to_player = (player_2d - enemy_2d).normalize();
        let distance_to_player = enemy_2d.distance_to(&player_2d);

        if self.locked {
            self.current_target = Some(self.position);
            return;
        }

        // Check if player is within pursuit distance (scales with level)
        if distance_to_player < self.pursuit_distance {
            // Higher levels are more aggressive in close pursuit
            let close_pursuit_radius = self.path_radius * (1.0 - (level as f32 * 0.1).min(0.5));

            if distance_to_player < close_pursuit_radius {
                self.current_target = Some(player_position);

                // Tighter capture radius at higher levels
                let capture_distance = (15.0 / (1.0 + level as f32 * 0.2)).max(10.0);
                if distance_to_player < capture_distance {
                    self.reached_player = true;
                    self.position = [0.0, 30.0, 0.0];
                    self.locked = true;
                }
                return;
            }
        }

        // Calculate ideal target with level-based aggressiveness
        let ideal_target_2d = enemy_2d + direction_to_player * self.path_radius;
        let ideal_target = Vec3::from_2d(ideal_target_2d, self.position[1]);

        if self.is_safe_path(
            self.position,
            *ideal_target.as_array(),
            &line_intersects_geometry,
        ) {
            self.current_target = Some(*ideal_target.as_array());
            return;
        }

        // Enhanced rotation strategy with level scaling
        let base_bias = if self.stuck_counter > 30 { 1.8 } else { 1.25 };
        let level_bias = 1.0 + (level as f32 * 0.15); // Higher levels try more rotations
        let rotation_bias = base_bias * level_bias;
        let adjusted_step = self.rotation_step * rotation_bias;
        let max_rotations = ((PI / adjusted_step) as i32).min(16); // Cap to prevent infinite loops

        // More aggressive movement patterns at higher levels
        let base_direction = if self.stuck_counter > (30 / level.max(1)) as i32 {
            (enemy_2d - player_2d).normalize() // Move away from player
        } else {
            direction_to_player // Move toward player
        };

        // Try both directions with level-based persistence
        for direction_multiplier in [1.0, -1.0] {
            let mut current_direction = base_direction;

            for i in 1..=max_rotations {
                current_direction = current_direction.rotate(adjusted_step * direction_multiplier);

                // More varied radius at higher levels
                let radius_variation = if self.stuck_counter > (30 / level.max(1)) as i32 {
                    let variation_factor =
                        1.0 + (0.5 + level as f32 * 0.1) * (i as f32 / max_rotations as f32);
                    self.path_radius * variation_factor
                } else {
                    self.path_radius
                };

                let test_target_2d = enemy_2d + current_direction * radius_variation;
                let test_target = Vec3::from_2d(test_target_2d, self.position[1]);

                if self.is_safe_path(
                    self.position,
                    *test_target.as_array(),
                    &line_intersects_geometry,
                ) {
                    self.current_target = Some(*test_target.as_array());
                    return;
                }
            }
        }

        // Enhanced escape behavior for higher levels
        if self.stuck_counter > (30 / level.max(1)) as i32 {
            let escape_radius = self.path_radius * (2.0 + level as f32 * 0.3);
            let num_test_directions = 6 + (level * 2) as usize; // More directions at higher levels

            for i in 0..num_test_directions {
                let angle = (i as f32 * 2.0 * PI) / num_test_directions as f32;
                let escape_direction = base_direction.rotate(angle);
                let test_target_2d = enemy_2d + escape_direction * escape_radius;
                let test_target = Vec3::from_2d(test_target_2d, self.position[1]);

                if self.is_safe_path(
                    self.position,
                    *test_target.as_array(),
                    &line_intersects_geometry,
                ) {
                    self.current_target = Some(*test_target.as_array());
                    return;
                }
            }
        }
    }

    /// Enhanced path safety checking with level-based precision.
    ///
    /// This method validates that a path from start to end is safe by checking
    /// for collisions at multiple points along the path. Higher levels use
    /// more thorough checking with smaller collision buffers.
    ///
    /// # Arguments
    ///
    /// * `start` - Starting position `[x, y, z]`
    /// * `end` - Ending position `[x, y, z]`
    /// * `line_intersects_geometry` - Function to check for collision with geometry
    ///
    /// # Returns
    ///
    /// `true` if the path is safe, `false` if it intersects with geometry.
    ///
    /// # Algorithm
    ///
    /// 1. Divides path into multiple check points based on level
    /// 2. Checks each point for collision with geometry
    /// 3. Performs buffer checking perpendicular to path direction
    /// 4. Higher levels use more check points and smaller buffers
    ///
    /// # Example
    ///
    /// ```rust
    /// let is_safe = pathfinder.is_safe_path(
    ///     [100.0, 30.0, 100.0],  // start
    ///     [150.0, 30.0, 150.0],  // end
    ///     |start, end| collision_system.intersects(start, end)
    /// );
    ///
    /// if is_safe {
    ///     // Path is clear, enemy can move
    /// }
    /// ```
    fn is_safe_path<F>(&self, start: [f32; 3], end: [f32; 3], line_intersects_geometry: F) -> bool
    where
        F: Fn([f32; 3], [f32; 3]) -> bool,
    {
        let start_vec = Vec3(start);
        let end_vec = Vec3(end);
        let direction = (end_vec - start_vec).normalize();
        let distance = start_vec.distance_to(&end_vec);

        // More thorough checking at higher levels
        let num_checks = 5 + (self.aggression_level * 2) as usize;
        let step_size = distance / num_checks as f32;

        // Smaller collision buffer at higher levels (more risk-taking)
        let collision_buffer = (25.0 / (1.0 + self.aggression_level as f32 * 0.1)).max(10.0);

        for i in 0..=num_checks {
            let t = i as f32 * step_size;
            let test_point = start_vec + direction * t;

            if line_intersects_geometry(start, *test_point.as_array()) {
                return false;
            }

            // Buffer checking with level-based precision
            let perpendicular = Vec3([direction.as_array()[1], -direction.as_array()[0], 0.0]);

            for &side in &[-1.0, 1.0] {
                let buffer_point = test_point + perpendicular * collision_buffer * side;
                if line_intersects_geometry(*test_point.as_array(), *buffer_point.as_array()) {
                    return false;
                }
            }
        }

        true
    }

    /// Updates the enemy's position (call this when the enemy moves).
    ///
    /// # Arguments
    ///
    /// * `new_position` - New 3D position `[x, y, z]`
    ///
    /// # Example
    ///
    /// ```rust
    /// pathfinder.set_position([120.0, 30.0, 120.0]);
    /// ```
    pub fn set_position(&mut self, new_position: [f32; 3]) {
        self.position = new_position;
    }

    /// Gets the current movement direction toward the target.
    ///
    /// # Returns
    ///
    /// `Some(direction)` if there's a valid target, `None` otherwise.
    /// The direction is a normalized 3D vector `[x, y, z]`.
    ///
    /// # Example
    ///
    /// ```rust
    /// if let Some(direction) = pathfinder.get_movement_direction() {
    ///     println!("Moving in direction: {:?}", direction);
    /// }
    /// ```
    pub fn get_movement_direction(&self) -> Option<[f32; 3]> {
        self.current_target.map(|target| {
            let position_vec = Vec3(self.position);
            let target_vec = Vec3(target);
            let direction = (target_vec - position_vec).normalize();
            *direction.as_array()
        })
    }

    /// Gets the distance to the current target.
    ///
    /// # Returns
    ///
    /// `Some(distance)` if there's a valid target, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// if let Some(distance) = pathfinder.distance_to_target() {
    ///     println!("Distance to target: {:.2}", distance);
    /// }
    /// ```
    pub fn distance_to_target(&self) -> Option<f32> {
        self.current_target.map(|target| {
            let position_vec = Vec3(self.position);
            let target_vec = Vec3(target);
            position_vec.distance_to(&target_vec)
        })
    }

    /// Check if the enemy has reached its current target.
    ///
    /// # Returns
    ///
    /// `true` if the enemy is within the arrival threshold of its target,
    /// `false` otherwise. Returns `true` if there's no current target.
    ///
    /// # Example
    ///
    /// ```rust
    /// if pathfinder.has_reached_target() {
    ///     println!("Enemy has reached its target");
    ///     // Calculate new target
    /// }
    /// ```
    pub fn has_reached_target(&self) -> bool {
        match self.current_target {
            Some(target) => {
                let position_vec = Vec3(self.position);
                let target_vec = Vec3(target);
                position_vec.distance_to(&target_vec) <= self.arrival_threshold
            }
            None => true,
        }
    }
}

/// Places an enemy strategically with level-based positioning.
///
/// This function calculates an optimal position for an enemy based on the
/// player's position, exit position, and current game level. Higher levels
/// place enemies closer to the exit for increased challenge.
///
/// # Arguments
///
/// * `exit_position` - 3D position of the level exit `[x, y, z]`
/// * `player_position` - 3D position of the player `[x, y, z]`
/// * `level` - Current game level for positioning scaling
/// * `placement_factor` - Factor between 0.0 and 1.0 for placement along player-exit line
/// * `offset_distance` - Optional perpendicular offset from the main path
/// * `line_intersects_geometry` - Function to check for collision with geometry
///
/// # Returns
///
/// A new `Enemy` instance positioned strategically relative to the player and exit.
///
/// # Placement Strategy
///
/// 1. **Base position**: Calculated along the line from player to exit
/// 2. **Level scaling**: Higher levels place enemies closer to exit
/// 3. **Offset application**: Applies perpendicular offset if specified
/// 4. **Validation**: Ensures position doesn't intersect with geometry
/// 5. **Path radius**: Scaled based on level and distance to exit
///
/// # Example
///
/// ```rust
/// let enemy = place_enemy(
///     [200.0, 30.0, 200.0],  // exit position
///     [50.0, 30.0, 50.0],    // player position
///     3,                     // level 3
///     0.6,                   // 60% along player-exit line
///     Some(50.0),            // 50 unit offset
///     |start, end| collision_system.intersects(start, end)
/// );
/// ```
pub fn place_enemy<F>(
    exit_position: [f32; 3],
    player_position: [f32; 3],
    level: u32,
    placement_factor: f32,
    offset_distance: Option<f32>,
    line_intersects_geometry: F,
) -> Enemy
where
    F: Fn([f32; 3], [f32; 3]) -> bool,
{
    let player_vec = Vec3(player_position);
    let exit_vec = Vec3(exit_position);

    let direction_to_exit = (exit_vec - player_vec).normalize();
    let distance_to_exit = player_vec.distance_to(&exit_vec);

    // Higher levels place enemies closer to the exit (more challenging)
    let level_factor = (level as f32 * 0.05).min(0.3);
    let adjusted_factor = (placement_factor + level_factor).clamp(0.1, 0.95);

    let base_position = player_vec + direction_to_exit * (distance_to_exit * adjusted_factor);

    // Apply offset with level-based variation
    let final_position = if let Some(offset) = offset_distance {
        let level_offset_multiplier = 1.0 + (level as f32 * 0.1);
        let adjusted_offset = offset * level_offset_multiplier;

        let perpendicular = Vec3([
            -direction_to_exit.as_array()[1],
            direction_to_exit.as_array()[0],
            0.0,
        ])
        .normalize();

        let offset_positions = [
            base_position + perpendicular * adjusted_offset,
            base_position - perpendicular * adjusted_offset,
        ];

        let mut valid_position = base_position;
        for &test_position in &offset_positions {
            if !line_intersects_geometry(player_position, *test_position.as_array())
                && !line_intersects_geometry(*test_position.as_array(), exit_position)
            {
                valid_position = test_position;
                break;
            }
        }

        valid_position
    } else {
        base_position
    };

    let validated_position = validate_enemy_position(
        *final_position.as_array(),
        player_position,
        exit_position,
        &line_intersects_geometry,
    );

    // Smaller path radius at higher levels (more aggressive)
    let base_radius = (distance_to_exit * 0.3).clamp(50.0, 200.0);
    let level_radius_reduction = (level as f32 * 0.05).min(0.4);
    let path_radius = base_radius * (1.0 - level_radius_reduction);

    let mut enemy = Enemy::new(validated_position, path_radius);
    enemy.scale_aggression_by_level(level);
    enemy
}

/// Validates and adjusts enemy position to ensure it doesn't intersect with geometry.
///
/// This function checks if the proposed enemy position creates valid paths to both
/// the player and exit. If not, it searches for alternative positions in a circular
/// pattern around the proposed position.
///
/// # Arguments
///
/// * `proposed_position` - The initial proposed position `[x, y, z]`
/// * `player_position` - Current player position `[x, y, z]`
/// * `exit_position` - Level exit position `[x, y, z]`
/// * `line_intersects_geometry` - Function to check for collision with geometry
///
/// # Returns
///
/// A validated position that doesn't intersect with geometry, or a fallback position.
///
/// # Algorithm
///
/// 1. Check if proposed position creates valid paths to player and exit
/// 2. If invalid, search in 8 directions around the proposed position
/// 3. If still invalid, return midpoint between player and exit
///
/// # Example
///
/// ```rust
/// let valid_position = validate_enemy_position(
///     [100.0, 30.0, 100.0],  // proposed position
///     [50.0, 30.0, 50.0],    // player position
///     [200.0, 30.0, 200.0],  // exit position
///     |start, end| collision_system.intersects(start, end)
/// );
/// ```
fn validate_enemy_position<F>(
    proposed_position: [f32; 3],
    player_position: [f32; 3],
    exit_position: [f32; 3],
    line_intersects_geometry: F,
) -> [f32; 3]
where
    F: Fn([f32; 3], [f32; 3]) -> bool,
{
    if line_intersects_geometry(player_position, proposed_position)
        || line_intersects_geometry(proposed_position, exit_position)
    {
        let base_vec = Vec3(proposed_position);
        let search_radius = 50.0;
        let search_angles = [0.0, 45.0, 90.0, 135.0, 180.0, 225.0, 270.0, 315.0];

        for &angle in &search_angles {
            let angle_rad = (angle as f32).to_radians();
            let offset = Vec3([
                search_radius * angle_rad.cos(),
                search_radius * angle_rad.sin(),
                0.0,
            ]);

            let test_position = base_vec + offset;
            let test_pos_array = *test_position.as_array();

            if !line_intersects_geometry(player_position, test_pos_array)
                && !line_intersects_geometry(test_pos_array, exit_position)
            {
                return test_pos_array;
            }
        }

        let player_vec = Vec3(player_position);
        let exit_vec = Vec3(exit_position);
        let center_position = (player_vec + exit_vec) * 0.5;

        *center_position.as_array()
    } else {
        proposed_position
    }
}

/// Convenience function for level-aware standard enemy placement.
///
/// This function provides a simplified interface for placing enemies with
/// standard parameters. It uses a 60% placement factor and no offset.
///
/// # Arguments
///
/// * `exit_position` - 3D position of the level exit `[x, y, z]`
/// * `player_position` - 3D position of the player `[x, y, z]`
/// * `level` - Current game level for positioning and behavior scaling
/// * `line_intersects_geometry` - Function to check for collision with geometry
///
/// # Returns
///
/// A new `Enemy` instance with standard placement parameters.
///
/// # Example
///
/// ```rust
/// let enemy = place_enemy_standard(
///     [200.0, 30.0, 200.0],  // exit position
///     [50.0, 30.0, 50.0],    // player position
///     3,                     // level 3
///     |start, end| collision_system.intersects(start, end)
/// );
/// ```
pub fn place_enemy_standard<F>(
    exit_position: [f32; 3],
    player_position: [f32; 3],
    level: i32,
    line_intersects_geometry: F,
) -> Enemy
where
    F: Fn([f32; 3], [f32; 3]) -> bool,
{
    place_enemy(
        exit_position,
        player_position,
        level as u32,
        0.6,
        None,
        line_intersects_geometry,
    )
}
