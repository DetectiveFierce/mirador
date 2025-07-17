use crate::math::vec::Vec3;
use std::f32::consts::PI;

// 1. Add Enemy struct to your game state module
#[derive(Debug, Clone)]
pub struct Enemy {
    pub size: f32,
    pub pathfinder: EnemyPathfinder,
    pub base_speed: f32,
    pub current_speed: f32,
}

impl Enemy {
    pub fn new(position: [f32; 3], path_radius: f32) -> Self {
        Self {
            size: 100.0, // Default sprite size
            pathfinder: EnemyPathfinder::new(position, path_radius),
            base_speed: 150.0, // Slightly reduced base speed for better scaling
            current_speed: 150.0,
        }
    }

    /// Updates enemy with level-based aggression scaling
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

    /// Scales enemy aggression based on level
    fn scale_aggression_by_level(&mut self, level: u32) {
        let level_f = level as f32;

        // Speed scaling: increases by 20% per level, capped at 500% of base speed
        let speed_multiplier = (1.0 + (level_f * 0.2)).min(5.0);
        self.current_speed = self.base_speed * speed_multiplier;

        // Update pathfinder aggression parameters
        self.pathfinder.update_aggression_for_level(level);
    }

    /// Get current aggression level for debugging
    pub fn get_aggression_stats(&self) -> (f32, f32, f32, f32) {
        (
            self.current_speed,
            self.pathfinder.path_radius,
            self.pathfinder.arrival_threshold,
            self.pathfinder.rotation_step,
        )
    }
}

#[derive(Debug, Clone)]
pub struct EnemyPathfinder {
    pub position: [f32; 3],
    pub current_target: Option<[f32; 3]>,
    pub path_radius: f32,
    pub base_path_radius: f32,
    pub rotation_step: f32,
    pub base_rotation_step: f32,
    pub arrival_threshold: f32,
    pub base_arrival_threshold: f32,
    pub stuck_counter: i32,
    pub last_position: [f32; 3],
    pub reached_player: bool,
    pub locked: bool,
    pub aggression_level: u32,
    pub pursuit_distance: f32, // Distance at which enemy starts pursuing more aggressively
    pub base_pursuit_distance: f32,
}

impl EnemyPathfinder {
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

    /// Updates aggression parameters based on level
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

    /// Main pathfinding update function with level awareness
    pub fn update<F>(
        &mut self,
        player_position: [f32; 3],
        level: u32,
        line_intersects_geometry: F,
    ) -> Option<[f32; 3]>
    where
        F: Fn([f32; 3], [f32; 3]) -> bool,
    {
        // Update stuck detection with level-based tolerance
        self.update_stuck_detection(level);

        // Check if we need to calculate a new target
        if self.needs_new_target(level) {
            self.calculate_new_target(player_position, level, line_intersects_geometry);
        }

        self.current_target
    }

    /// Updates stuck detection with level-based sensitivity
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

    /// Determines if the enemy needs a new target with level-based urgency
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

    /// Calculates a new target point with level-based aggression
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

    /// Enhanced path safety checking with level-based precision
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

/// Places an enemy strategically with level-based positioning
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

/// Convenience function for level-aware standard enemy placement
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
