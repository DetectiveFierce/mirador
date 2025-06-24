use crate::math::deg_to_rad;

#[repr(transparent)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Mat4(pub [[f32; 4]; 4]);

impl Mat4 {
    pub fn identity() -> Mat4 {
        Mat4([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Mat4 {
        Mat4([
            [2.0 / (right - left), 0.0, 0.0, 0.0],
            [0.0, 2.0 / (top - bottom), 0.0, 0.0],
            [0.0, 0.0, 1.0 / (near - far), 0.0],
            [
                (right + left) / (left - right),
                (top + bottom) / (bottom - top),
                near / (near - far),
                1.0,
            ],
        ])
    }

    pub fn perspective(
        field_of_view_y_in_radians: f32,
        aspect: f32,
        z_near: f32,
        z_far: f32,
    ) -> Mat4 {
        let f = 1.0 / (field_of_view_y_in_radians * 0.5).tan(); // Use positive f for standard perspective
        let range_reciprocal = 1.0 / (z_near - z_far); // N-F for consistent sign

        Mat4([
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, z_far * range_reciprocal, -1.0], // OpenGL style Z: -1 to 1
            [0.0, 0.0, z_far * z_near * range_reciprocal, 0.0], // OpenGL style Z: -1 to 1
        ])
    }

    pub fn translation(tx: f32, ty: f32, tz: f32) -> Mat4 {
        Mat4([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [tx, ty, tz, 1.0],
        ])
    }

    pub fn scaling(sx: f32, sy: f32, sz: f32) -> Mat4 {
        Mat4([
            [sx, 0.0, 0.0, 0.0],
            [0.0, sy, 0.0, 0.0],
            [0.0, 0.0, sz, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn rotation_x(angle_in_radians: f32) -> Mat4 {
        let c = (deg_to_rad(angle_in_radians)).cos();
        let s = (deg_to_rad(angle_in_radians)).sin();
        Mat4([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, c, -s, 0.0],
            [0.0, s, c, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }
    pub fn rotation_y(angle_in_radians: f32) -> Mat4 {
        let c = (deg_to_rad(angle_in_radians)).cos();
        let s = (deg_to_rad(angle_in_radians)).sin();
        Mat4([
            [c, 0.0, s, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [-s, 0.0, c, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn rotation_z(angle_in_radians: f32) -> Mat4 {
        let c = (deg_to_rad(angle_in_radians)).cos();
        let s = (deg_to_rad(angle_in_radians)).sin();
        Mat4([
            [c, s, 0.0, 0.0],
            [-s, c, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn multiply(&self, b: &Mat4) -> Mat4 {
        let mut result = [[0.0; 4]; 4];
        for (i, row) in result.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = (0..4).map(|k| b.0[i][k] * self.0[k][j]).sum();
            }
        }
        Mat4(result)
    }

    pub fn inverse(&self) -> Mat4 {
        let m = self.0;

        // Extract the 3x3 linear part (A) and translation (t)
        let a = [
            [m[0][0], m[0][1], m[0][2]],
            [m[1][0], m[1][1], m[1][2]],
            [m[2][0], m[2][1], m[2][2]],
        ];
        let t = [m[0][3], m[1][3], m[2][3]];

        // Compute the inverse of the 3x3 part (A⁻¹)
        let det = a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
            - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
            + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]);

        if det.abs() < f32::EPSILON {
            return Mat4::identity(); // Fallback if singular
        }

        let inv_det = 1.0 / det;
        let mut a_inv = [[0.0; 3]; 3];

        // Compute adjugate of A and divide by determinant
        a_inv[0][0] = (a[1][1] * a[2][2] - a[1][2] * a[2][1]) * inv_det;
        a_inv[0][1] = -(a[0][1] * a[2][2] - a[0][2] * a[2][1]) * inv_det;
        a_inv[0][2] = (a[0][1] * a[1][2] - a[0][2] * a[1][1]) * inv_det;
        a_inv[1][0] = -(a[1][0] * a[2][2] - a[1][2] * a[2][0]) * inv_det;
        a_inv[1][1] = (a[0][0] * a[2][2] - a[0][2] * a[2][0]) * inv_det;
        a_inv[1][2] = -(a[0][0] * a[1][2] - a[0][2] * a[1][0]) * inv_det;
        a_inv[2][0] = (a[1][0] * a[2][1] - a[1][1] * a[2][0]) * inv_det;
        a_inv[2][1] = -(a[0][0] * a[2][1] - a[0][1] * a[2][0]) * inv_det;
        a_inv[2][2] = (a[0][0] * a[1][1] - a[0][1] * a[1][0]) * inv_det;

        // Compute -A⁻¹ * t for the new translation
        let new_t = [
            -(a_inv[0][0] * t[0] + a_inv[0][1] * t[1] + a_inv[0][2] * t[2]),
            -(a_inv[1][0] * t[0] + a_inv[1][1] * t[1] + a_inv[1][2] * t[2]),
            -(a_inv[2][0] * t[0] + a_inv[2][1] * t[1] + a_inv[2][2] * t[2]),
        ];

        // Build the inverse affine matrix
        Mat4([
            [a_inv[0][0], a_inv[0][1], a_inv[0][2], new_t[0]],
            [a_inv[1][0], a_inv[1][1], a_inv[1][2], new_t[1]],
            [a_inv[2][0], a_inv[2][1], a_inv[2][2], new_t[2]],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }
}

impl From<[[f32; 4]; 4]> for Mat4 {
    fn from(matrix: [[f32; 4]; 4]) -> Self {
        Mat4(matrix)
    }
}

// Convert from Mat4 to array
impl From<Mat4> for [[f32; 4]; 4] {
    fn from(matrix: Mat4) -> Self {
        matrix.0
    }
}
