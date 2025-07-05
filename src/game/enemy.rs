// 1. Add Enemy struct to your game state module
#[derive(Debug, Clone)]
pub struct Enemy {
    pub position: [f32; 3],
    pub size: f32,
    // Add other enemy properties as needed
}

impl Enemy {
    pub fn new(position: [f32; 3]) -> Self {
        Self {
            position,
            size: 100.0, // Default sprite size
        }
    }
}

#[derive(Debug)]
pub struct EnemyPathfinder {
    pub position: [f32; 3],
    pub current_target: Option<[f32; 3]>,
    pub path_radius: f32,
    pub rotation_step: f32, // Radians per step when rotating around obstacles
    pub arrival_threshold: f32, // Distance threshold to consider target reached
}
