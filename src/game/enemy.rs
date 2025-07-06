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
            speed: 100.0,
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
}

impl EnemyPathfinder {
    pub fn new(position: [f32; 3], path_radius: f32) -> Self {
        Self {
            position,
            current_target: None,
            path_radius,
            rotation_step: PI / 16.0, // 11.25 degrees per step
            arrival_threshold: 0.5,
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
        // Check if we need to calculate a new target
        if self.needs_new_target() {
            self.calculate_new_target(player_position, line_intersects_geometry);
        }

        self.current_target
    }

    /// Determines if the enemy needs a new target
    fn needs_new_target(&self) -> bool {
        match self.current_target {
            None => true,
            Some(target) => {
                // Check if we've reached the current target
                let enemy_vec = Vec3(self.position);
                let target_vec = Vec3(target);
                enemy_vec.distance_to(&target_vec) <= self.arrival_threshold
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

        if enemy_2d.distance_to(&player_2d) < self.path_radius {
            self.current_target = Some(self.position);
            return;
        }

        let ideal_target_2d = enemy_2d + direction_to_player * self.path_radius;
        let ideal_target = Vec3::from_2d(ideal_target_2d, self.position[1]);

        if !line_intersects_geometry(self.position, *ideal_target.as_array()) {
            self.current_target = Some(*ideal_target.as_array());
            return;
        }

        // Rotation bias: rotate slightly more per step
        let rotation_bias = 1.25; // 25% more than base
        let adjusted_step = self.rotation_step * rotation_bias;
        let max_rotations = (2.0 * PI / adjusted_step) as i32;

        let mut current_direction = direction_to_player;

        for _ in 0..max_rotations {
            current_direction = current_direction.rotate(adjusted_step);
            let test_target_2d = enemy_2d + current_direction * self.path_radius;
            let test_target = Vec3::from_2d(test_target_2d, self.position[1]);

            if !line_intersects_geometry(self.position, *test_target.as_array()) {
                self.current_target = Some(*test_target.as_array());
                return;
            }
        }

        self.current_target = Some(self.position);
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
