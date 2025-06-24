use crate::math::mat::Mat4;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct GameState {
    pub player: Player,
    pub last_frame_time: Instant,
    pub delta_time: f32,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            player: Player::new(),
            last_frame_time: Instant::now(),
            delta_time: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Player {
    pub position: [f32; 3], // World position (x, y, z)
    pub pitch: f32,         // Rotation around X axis (up/down look)
    pub yaw: f32,           // Rotation around Y axis (left/right look)
    pub fov: f32,
    pub speed: f32,
    pub mouse_sensitivity: f32,
}

impl Player {
    pub fn new() -> Self {
        Self {
            position: [0.0, 50.0, 100.0], // Start above and behind the floor
            pitch: 0.0,
            yaw: 0.0,
            fov: 100.0,
            speed: 30.0,
            mouse_sensitivity: 1.0,
        }
    }

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

    // Handle mouse movement for looking around
    pub fn handle_mouse_movement(&mut self, delta_x: f64, delta_y: f64) {
        self.yaw -= delta_x as f32 * self.mouse_sensitivity;
        self.pitch -= delta_y as f32 * self.mouse_sensitivity;

        // Clamp pitch to prevent flipping
        self.pitch = self.pitch.clamp(-89.0, 89.0);
    }

    pub fn move_forward(&mut self, delta_time: f32) {
        let forward_x = self.yaw.to_radians().sin();
        let forward_z = self.yaw.to_radians().cos();
        self.position[0] -= forward_x * self.speed * delta_time;
        self.position[2] -= forward_z * self.speed * delta_time;
    }

    pub fn move_backward(&mut self, delta_time: f32) {
        let forward_x = self.yaw.to_radians().sin();
        let forward_z = self.yaw.to_radians().cos();
        self.position[0] += forward_x * self.speed * delta_time;
        self.position[2] += forward_z * self.speed * delta_time;
    }

    pub fn move_left(&mut self, delta_time: f32) {
        let right_x = self.yaw.to_radians().cos();
        let right_z = self.yaw.to_radians().sin();
        self.position[0] -= right_x * self.speed * delta_time;
        self.position[2] += right_z * self.speed * delta_time;
    }

    pub fn move_right(&mut self, delta_time: f32) {
        let right_x = self.yaw.to_radians().cos();
        let right_z = self.yaw.to_radians().sin();
        self.position[0] += right_x * self.speed * delta_time;
        self.position[2] -= right_z * self.speed * delta_time;
    }
}
