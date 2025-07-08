use crate::math::vec::Vec3;
use std::f32::consts::PI;

// 1. Add Enemy struct to your game state module
#[derive(Debug, Clone)]
pub struct Enemy {
    pub size: f32,
    pub pathfinder: EnemyPathfinder,
    pub speed: f32,
}

impl Enemy {
    pub fn new(position: [f32; 3], path_radius: f32) -> Self {
        Self {
            size: 100.0, // Default sprite size
            pathfinder: EnemyPathfinder::new(position, path_radius),
            speed: 200.0,
        }
    }

    pub fn update<F>(
        &mut self,
        player_position: [f32; 3],
        delta_time: f32,
        line_intersects_geometry: F,
    ) where
        F: Fn([f32; 3], [f32; 3]) -> bool,
    {
        // Update pathfinding
        if let Some(_target) = self
            .pathfinder
            .update(player_position, line_intersects_geometry)
        {
            // Move towards the target
            if let Some(direction) = self.pathfinder.get_movement_direction() {
                let direction_vec = Vec3(direction);
                let movement = direction_vec * self.speed * delta_time;

                let position_vec = Vec3(self.pathfinder.position);
                let new_position = position_vec + movement;

                self.pathfinder.set_position(*new_position.as_array());
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnemyPathfinder {
    pub position: [f32; 3],
    pub current_target: Option<[f32; 3]>,
    pub path_radius: f32,
    pub rotation_step: f32, // Radians per step when rotating around obstacles
    pub arrival_threshold: f32, // Distance threshold to consider target reached
    pub stuck_counter: i32, // Counter for detecting stuck state
    pub last_position: [f32; 3], // Track previous position
    pub reached_player: bool,
    pub locked: bool, // Whether the enemy is locked in place
}

impl EnemyPathfinder {
    pub fn new(position: [f32; 3], path_radius: f32) -> Self {
        Self {
            position,
            current_target: None,
            path_radius,
            rotation_step: PI / 8.0, // 11.25 degrees per step
            arrival_threshold: 0.65,
            stuck_counter: 0,
            last_position: position,
            reached_player: false,
            locked: true,
        }
    }

    /// Main pathfinding update function
    /// Should be called every frame or when the enemy needs to recalculate its path
    pub fn update<F>(
        &mut self,
        player_position: [f32; 3],
        line_intersects_geometry: F,
    ) -> Option<[f32; 3]>
    where
        F: Fn([f32; 3], [f32; 3]) -> bool,
    {
        // Update stuck detection
        self.update_stuck_detection();

        // Check if we need to calculate a new target
        if self.needs_new_target() {
            self.calculate_new_target(player_position, line_intersects_geometry);
        }

        self.current_target
    }

    /// Updates stuck detection logic
    fn update_stuck_detection(&mut self) {
        let current_vec = Vec3(self.position);
        let last_vec = Vec3(self.last_position);
        let distance_moved = current_vec.distance_to(&last_vec);

        if distance_moved < 5.0 {
            self.stuck_counter += 1;
        } else {
            self.stuck_counter = 0;
        }

        // Reset stuck counter after a while to prevent permanent stuck state
        if self.stuck_counter > 120 {
            // 2 seconds at 60 FPS
            self.stuck_counter = 0;
        }

        self.last_position = self.position;
    }

    /// Determines if the enemy needs a new target
    fn needs_new_target(&self) -> bool {
        match self.current_target {
            None => true,
            Some(target) => {
                // Check if we've reached the current target
                let enemy_vec = Vec3(self.position);
                let target_vec = Vec3(target);
                let distance = enemy_vec.distance_to(&target_vec);

                // If stuck, consider target reached to force recalculation
                distance <= self.arrival_threshold || self.stuck_counter > 60
            }
        }
    }

    /// Calculates a new target point for the enemy
    fn calculate_new_target<F>(&mut self, player_position: [f32; 3], line_intersects_geometry: F)
    where
        F: Fn([f32; 3], [f32; 3]) -> bool,
    {
        let enemy_vec = Vec3(self.position);
        let player_vec = Vec3(player_position);
        let enemy_2d = enemy_vec.to_2d();
        let player_2d = player_vec.to_2d();

        let direction_to_player = (player_2d - enemy_2d).normalize();

        if self.locked {
            // If locked, stay in place
            self.current_target = Some(self.position);
            return;
        }

        if enemy_2d.distance_to(&player_2d) < self.path_radius {
            self.current_target = Some(player_position);
            if enemy_2d.distance_to(&player_2d) < 10.0 {
                self.reached_player = true;
                self.position = [0.0, 30.0, 0.0];
                self.locked = true; // Lock enemy when close to player
            }
            return;
        }

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

        // Improved rotation strategy - try both directions and vary the approach
        let rotation_bias = if self.stuck_counter > 30 { 1.8 } else { 1.25 };
        let adjusted_step = self.rotation_step * rotation_bias;
        let max_rotations = (PI / adjusted_step) as i32;

        // If stuck, try moving away from player first
        let base_direction = if self.stuck_counter > 30 {
            (enemy_2d - player_2d).normalize() // Move away from player
        } else {
            direction_to_player // Move toward player
        };

        // Try both clockwise and counter-clockwise
        for direction_multiplier in [1.0, -1.0] {
            let mut current_direction = base_direction;

            for i in 1..=max_rotations {
                current_direction = current_direction.rotate(adjusted_step * direction_multiplier);

                // Vary the radius slightly to avoid getting stuck at same distance
                let radius_variation = if self.stuck_counter > 30 {
                    self.path_radius * (1.0 + 0.5 * (i as f32 / max_rotations as f32))
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

        // If still no valid target, try a few random directions with larger radius
        if self.stuck_counter > 30 {
            let escape_radius = self.path_radius * 2.0;
            let test_directions = [
                PI / 4.0,
                -PI / 4.0,
                3.0 * PI / 4.0,
                -3.0 * PI / 4.0,
                PI / 2.0,
                -PI / 2.0,
            ];

            for &angle in &test_directions {
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

    /// Checks if a path is safe by testing multiple points along the route
    /// This helps avoid getting stuck between wall collision planes
    fn is_safe_path<F>(&self, start: [f32; 3], end: [f32; 3], line_intersects_geometry: F) -> bool
    where
        F: Fn([f32; 3], [f32; 3]) -> bool,
    {
        let start_vec = Vec3(start);
        let end_vec = Vec3(end);
        let direction = (end_vec - start_vec).normalize();
        let distance = start_vec.distance_to(&end_vec);

        // Test multiple points along the path
        let num_checks = 5;
        let step_size = distance / num_checks as f32;
        let collision_buffer = 25.0; // Buffer distance from walls

        for i in 0..=num_checks {
            let t = i as f32 * step_size;
            let test_point = start_vec + direction * t;

            // Check direct collision
            if line_intersects_geometry(start, *test_point.as_array()) {
                return false;
            }

            // Check collision buffer around the test point to avoid narrow gaps
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

    /// Updates the enemy's position (call this when the enemy moves)
    pub fn set_position(&mut self, new_position: [f32; 3]) {
        self.position = new_position;
    }

    /// Gets the current movement direction
    pub fn get_movement_direction(&self) -> Option<[f32; 3]> {
        self.current_target.map(|target| {
            let position_vec = Vec3(self.position);
            let target_vec = Vec3(target);
            let direction = (target_vec - position_vec).normalize();
            *direction.as_array()
        })
    }

    /// Gets the distance to the current target
    pub fn distance_to_target(&self) -> Option<f32> {
        self.current_target.map(|target| {
            let position_vec = Vec3(self.position);
            let target_vec = Vec3(target);
            position_vec.distance_to(&target_vec)
        })
    }

    /// Check if the enemy has reached its current target
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

/// Places an enemy strategically between the player and exit position
///
/// # Arguments
/// * `exit_position` - The position of the exit [x, y, z]
/// * `player_position` - The current position of the player [x, y, z]
/// * `placement_factor` - How close to the exit to place the enemy (0.0 = at player, 1.0 = at exit)
/// * `offset_distance` - Optional distance to offset the enemy from the direct line (for more interesting placement)
/// * `line_intersects_geometry` - Function to check if a line intersects with level geometry
///
/// # Returns
/// * `Enemy` - A new enemy instance positioned between player and exit
pub fn place_enemy<F>(
    exit_position: [f32; 3],
    player_position: [f32; 3],
    placement_factor: f32,
    offset_distance: Option<f32>,
    line_intersects_geometry: F,
) -> Enemy
where
    F: Fn([f32; 3], [f32; 3]) -> bool,
{
    let player_vec = Vec3(player_position);
    let exit_vec = Vec3(exit_position);

    // Calculate the direction from player to exit
    let direction_to_exit = (exit_vec - player_vec).normalize();
    let distance_to_exit = player_vec.distance_to(&exit_vec);

    // Clamp placement factor to reasonable range
    let clamped_factor = placement_factor.clamp(0.1, 0.9);

    // Calculate base position along the line from player to exit
    let base_position = player_vec + direction_to_exit * (distance_to_exit * clamped_factor);

    // Apply offset if specified (perpendicular to the player-exit line)
    let final_position = if let Some(offset) = offset_distance {
        // Create a perpendicular vector for offsetting
        let perpendicular = Vec3([
            -direction_to_exit.as_array()[1], // Rotate 90 degrees in 2D
            direction_to_exit.as_array()[0],
            0.0,
        ])
        .normalize();

        // Try both sides of the line to find a valid position
        let offset_positions = [
            base_position + perpendicular * offset,
            base_position - perpendicular * offset,
        ];

        // Check which offset position is valid (doesn't intersect geometry)
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

    // Ensure the enemy position doesn't intersect with geometry
    let validated_position = validate_enemy_position(
        *final_position.as_array(),
        player_position,
        exit_position,
        &line_intersects_geometry,
    );

    // Calculate appropriate path radius based on distance to exit
    let path_radius = (distance_to_exit * 0.3).clamp(50.0, 200.0);

    Enemy::new(validated_position, path_radius)
}

/// Validates and adjusts enemy position to ensure it doesn't intersect with geometry
fn validate_enemy_position<F>(
    proposed_position: [f32; 3],
    player_position: [f32; 3],
    exit_position: [f32; 3],
    line_intersects_geometry: F,
) -> [f32; 3]
where
    F: Fn([f32; 3], [f32; 3]) -> bool,
{
    // Check if the proposed position creates line-of-sight issues
    if line_intersects_geometry(player_position, proposed_position)
        || line_intersects_geometry(proposed_position, exit_position)
    {
        // Try to find a nearby valid position
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

            // Check if this position works
            if !line_intersects_geometry(player_position, test_pos_array)
                && !line_intersects_geometry(test_pos_array, exit_position)
            {
                return test_pos_array;
            }
        }

        // If no valid position found, try moving closer to the center between player and exit
        let player_vec = Vec3(player_position);
        let exit_vec = Vec3(exit_position);
        let center_position = (player_vec + exit_vec) * 0.5;

        *center_position.as_array()
    } else {
        proposed_position
    }
}

/// Convenience function for standard enemy placement
/// Places enemy at 60% of the distance from player to exit
pub fn place_enemy_standard<F>(
    exit_position: [f32; 3],
    player_position: [f32; 3],
    line_intersects_geometry: F,
) -> Enemy
where
    F: Fn([f32; 3], [f32; 3]) -> bool,
{
    place_enemy(
        exit_position,
        player_position,
        0.6,
        None,
        line_intersects_geometry,
    )
}
