pub mod mat;
pub mod vec;
pub fn deg_to_rad(degrees: f32) -> f32 {
    (degrees % 360.0) * (std::f32::consts::PI / 180.0)
}
