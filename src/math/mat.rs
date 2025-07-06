//! 4x4 matrix type and operations for 3D graphics transformations.
//!
//! This module provides a memory-compatible 4x4 matrix (`Mat4`) with
//! common transformation constructors and operations, suitable for use
//! with WGPU and WGSL shaders.
//!
//! # Implementation Notes
//!
//! - Matrices are stored in column-major order (compatible with WGSL/GPU)
//! - All transformations assume a right-handed coordinate system by default
//! - Perspective matrices use OpenGL-style depth range (-1 to 1)
//! - Rotation angles are specified in degrees for convenience
//! - The `inverse()` method has a fallback to identity for singular matrices
//!
//! # Performance Considerations
//!
//! - Matrix multiplication is implemented naively - for production use,
//!   consider optimizing with SIMD or a dedicated math library
//! - The inverse calculation is optimized for affine transformations
//!
//! # Coordinate System Conventions
//!
//! - X-axis: Right
//! - Y-axis: Up
//! - Z-axis: Back (negative Z is forward)
//! - Rotation directions follow right-hand rule

use crate::math::deg_to_rad;

/// A 4x4 matrix with memory layout compatible for GPU buffers.
///
/// Provides constructors for identity, orthographic, perspective, translation,
/// scaling, and rotation matrices, as well as matrix multiplication and inversion.
///
/// # Memory Layout
///
/// The matrix is stored as `[[f32; 4]; 4]` with column-major ordering:
/// ```text
/// [
///     [m00, m10, m20, m30],  // First column
///     [m01, m11, m21, m31],  // Second column
///     [m02, m12, m22, m32],  // Third column
///     [m03, m13, m23, m33],  // Fourth column
/// ]
/// ```
///
/// This matches WGSL's `mat4x4<f32>` memory layout when transferred via buffers.
///
/// # Examples
///
/// Creating a translation matrix:
/// ```
/// # use your_crate::math::Mat4;
/// let translation = Mat4::translation(2.0, 3.0, 4.0);
/// ```
#[repr(transparent)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Mat4(pub [[f32; 4]; 4]);

impl Mat4 {
    /// Constructs an identity matrix.
    ///
    /// ```text
    /// 1 0 0 0
    /// 0 1 0 0
    /// 0 0 1 0
    /// 0 0 0 1
    /// ```
    pub fn identity() -> Mat4 {
        Mat4([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Constructs an orthographic projection matrix.
    ///
    /// # Parameters
    /// - `left`, `right`: X-axis clipping planes
    /// - `bottom`, `top`: Y-axis clipping planes
    /// - `near`, `far`: Z-axis clipping planes (positive values)
    ///
    /// # Note
    /// The `far` plane must be greater than the `near` plane.
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

    /// Constructs a perspective projection matrix.
    ///
    /// # Parameters
    /// - `field_of_view_y_in_radians`: Vertical field of view in radians
    /// - `aspect`: Aspect ratio (width / height)
    /// - `z_near`, `z_far`: Near and far clipping planes (positive values)
    ///
    /// # Note
    /// - Uses OpenGL-style depth range (-1 to 1)
    /// - `z_far` must be greater than `z_near`
    /// - `field_of_view_y_in_radians` should be in (0, π) range
    pub fn perspective(
        field_of_view_y_in_radians: f32,
        aspect: f32,
        z_near: f32,
        z_far: f32,
    ) -> Mat4 {
        let f = 1.0 / (field_of_view_y_in_radians * 0.5).tan();
        let range_reciprocal = 1.0 / (z_near - z_far);

        Mat4([
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, z_far * range_reciprocal, -1.0],
            [0.0, 0.0, z_far * z_near * range_reciprocal, 0.0],
        ])
    }

    /// Constructs a translation matrix.
    ///
    /// ```text
    /// 1 0 0 0
    /// 0 1 0 0
    /// 0 0 1 0
    /// x y z 1
    /// ```
    pub fn translation(tx: f32, ty: f32, tz: f32) -> Mat4 {
        Mat4([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [tx, ty, tz, 1.0],
        ])
    }

    /// Constructs a scaling matrix.
    ///
    /// ```text
    /// x 0 0 0
    /// 0 y 0 0
    /// 0 0 z 0
    /// 0 0 0 1
    /// ```
    pub fn scaling(sx: f32, sy: f32, sz: f32) -> Mat4 {
        Mat4([
            [sx, 0.0, 0.0, 0.0],
            [0.0, sy, 0.0, 0.0],
            [0.0, 0.0, sz, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Constructs a rotation matrix around the X axis.
    ///
    /// # Note
    /// - Angle is specified in degrees
    /// - Rotation follows right-hand rule (counter-clockwise when looking from +X)
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

    /// Constructs a rotation matrix around the Y axis.
    ///
    /// # Note
    /// - Angle is specified in degrees
    /// - Rotation follows right-hand rule (counter-clockwise when looking from +Y)
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

    /// Constructs a rotation matrix around the Z axis.
    ///
    /// # Note
    /// - Angle is specified in degrees
    /// - Rotation follows right-hand rule (counter-clockwise when looking from +Z)
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

    /// Computes the inverse of an affine transformation matrix.
    ///
    /// # Returns
    /// - Inverse matrix if invertible
    /// - Identity matrix as fallback for singular matrices
    ///
    /// # Note
    /// This implementation is optimized for affine transformations (where the
    /// last row is [0, 0, 0, 1]). For general 4x4 matrices, a different
    /// approach would be needed.
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

    /// Multiplies two matrices (self * b).
    ///
    /// # Note
    /// This implements standard matrix multiplication where:
    /// result[i][j] = sum over k (self[i][k] * b[k][j])
    ///
    pub fn multiply(&self, b: &Mat4) -> Mat4 {
        let mut result = [[0.0; 4]; 4];
        for (i, row) in result.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = self.0[i][0] * b.0[0][j]
                    + self.0[i][1] * b.0[1][j]
                    + self.0[i][2] * b.0[2][j]
                    + self.0[i][3] * b.0[3][j];
            }
        }
        Mat4(result)
    }
}

impl From<[[f32; 4]; 4]> for Mat4 {
    fn from(matrix: [[f32; 4]; 4]) -> Self {
        Mat4(matrix)
    }
}

impl From<Mat4> for [[f32; 4]; 4] {
    fn from(matrix: Mat4) -> Self {
        matrix.0
    }
}
