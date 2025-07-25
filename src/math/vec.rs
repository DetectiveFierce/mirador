//! 3D vector type and operations for graphics and math.
//!
//! This module defines a memory-compatible 3D vector (`Vec3`) with common
//! vector operations, designed for use with WGPU and WGSL shaders.
//!
//! # Implementation Notes
//!
//! - Vectors are stored as `[f32; 3]` with no padding (12 bytes total)
//! - All operations maintain WGPU memory compatibility
//! - Uses right-handed coordinate system by default
//! - Normalization handles zero vectors gracefully
//! - No SIMD optimizations in current implementation
//!
//! # Performance Considerations
//!
//! - Basic operations are implemented naively - consider SIMD for performance-critical code
//! - Normalization includes a branch for zero vectors
//! - No explicit alignment directives (relies on `f32`'s natural alignment)
//!
//! # Coordinate System Conventions
//!
//! - X-axis: Right
//! - Y-axis: Up
//! - Z-axis: Forward (right-handed system)
//! - Cross products follow right-hand rule

use std::ops::{Add, Mul, Sub};

/*
Requirements for Memory Compatibility with WGPU:
   1. Standard layout (like C structs)
   2. Alignment that matches WGSL expectations (4-byte aligned for f32)
   3. Sized correctly for GPU buffers (exactly 12 bytes)
   4. Can be safely cast to [f32; 3] or bytes
   5. No padding between elements
*/

/// A 3D vector with memory layout compatible for GPU buffers.
///
/// Provides basic vector math operations and conversions. The memory layout
/// is exactly 3 contiguous `f32` values with no padding, matching WGSL's `vec3<f32>`.
///
/// # Memory Layout
///
/// ```text
/// [x: f32, y: f32, z: f32] // 12 bytes total, no padding
/// ```
///
/// # Examples
///
/// Basic usage:
/// ```
/// # use your_crate::math::Vec3;
/// let v = Vec3::new(1.0, 2.0, 3.0);
/// let w = Vec3::new(4.0, 5.0, 6.0);
/// let dot = v.dot(&w);
/// ```
#[repr(transparent)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vec3(pub [f32; 3]);

#[allow(dead_code)]
impl Vec3 {
    /// Creates a new `Vec3` from components.
    ///
    /// # Example
    /// ```
    /// # use your_crate::math::Vec3;
    /// let v = Vec3::new(1.0, 2.0, 3.0);
    /// ```
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3([x, y, z])
    }

    /// Computes the dot product of two vectors.
    ///
    /// This is equivalent to:
    /// `self.x() * other.x() + self.y() * other.y() + self.z() * other.z()`
    ///
    /// # Example
    /// ```
    /// # use your_crate::math::Vec3;
    /// let v = Vec3::new(1.0, 2.0, 3.0);
    /// let w = Vec3::new(4.0, 5.0, 6.0);
    /// assert_eq!(v.dot(&w), 32.0);
    /// ```
    pub fn dot(&self, other: &Self) -> f32 {
        self.x() * other.x() + self.y() * other.y() + self.z() * other.z()
    }

    /// Computes the cross product of two vectors.
    ///
    /// Follows the right-hand rule. For vectors A and B, the cross product A×B:
    /// - Is perpendicular to both A and B
    /// - Length equals area of parallelogram formed by A and B
    /// - Direction follows right-hand rule
    ///
    /// # Example
    /// ```
    /// # use your_crate::math::Vec3;
    /// let x = Vec3::new(1.0, 0.0, 0.0);
    /// let y = Vec3::new(0.0, 1.0, 0.0);
    /// let z = x.cross(&y);  // Should be (0, 0, 1)
    /// ```
    pub fn cross(&self, other: &Self) -> Self {
        Vec3([
            self.y() * other.z() - self.z() * other.y(),
            self.z() * other.x() - self.x() * other.z(),
            self.x() * other.y() - self.y() * other.x(),
        ])
    }

    /// Computes the Euclidean length (magnitude) of the vector.
    ///
    /// This is `sqrt(x² + y² + z²)`.
    ///
    /// # Performance Note
    /// Contains a square root operation - consider `length_squared()` for comparisons.
    pub fn length(&self) -> f32 {
        (self.x().powi(2) + self.y().powi(2) + self.z().powi(2)).sqrt()
    }
    /// Calculates the Euclidean distance between this vector and another.
    ///
    /// # Arguments
    /// * `other` - The other vector to calculate distance to
    ///
    /// # Returns
    /// The distance between the two vectors
    ///
    /// # Example
    /// ```
    /// # use your_crate::math::Vec3;
    /// let v1 = Vec3::new(0.0, 0.0, 0.0);
    /// let v2 = Vec3::new(3.0, 4.0, 0.0);
    /// assert_eq!(v1.distance_to(&v2), 5.0);
    /// ```
    pub fn distance_to(&self, other: &Self) -> f32 {
        (*self - *other).length()
    }

    /// Convert to 2D vector (ignoring Y component)
    pub fn to_2d(&self) -> Vec2 {
        Vec2([self.x(), self.z()])
    }

    /// Create Vec3 from 2D vector with specified Y component
    pub fn from_2d(vec2: Vec2, y: f32) -> Self {
        Vec3([vec2.x(), y, vec2.z()])
    }

    /// Normalizes the vector to unit length.
    ///
    /// # Behavior
    /// - Returns a zero vector if length is zero
    /// - Otherwise returns vector with same direction but length 1
    ///
    /// # Example
    /// ```
    /// # use your_crate::math::Vec3;
    /// let v = Vec3::new(1.0, 2.0, 3.0).normalize();
    /// assert!((v.length() - 1.0).abs() < 1e-6);
    /// ```
    pub fn normalize(&self) -> Self {
        let length = self.length();
        if length <= f32::EPSILON {
            return Self([0.0, 0.0, 0.0]);
        }
        Self([self.x() / length, self.y() / length, self.z() / length])
    }

    /// Returns a reference to the vector's components as an array.
    ///
    /// Useful for passing to GPU buffers or other FFI contexts.
    pub fn as_array(&self) -> &[f32; 3] {
        &self.0
    }

    /// Returns the x component of the vector.
    pub fn x(&self) -> f32 {
        self.0[0]
    }

    /// Returns the y component of the vector.
    pub fn y(&self) -> f32 {
        self.0[1]
    }

    /// Returns the z component of the vector.
    pub fn z(&self) -> f32 {
        self.0[2]
    }
}

// Conversion implementations...

/// Adds two vectors component-wise.
///
/// # Example
/// ```
/// # use your_crate::math::Vec3;
/// let v = Vec3::new(1.0, 2.0, 3.0);
/// let w = Vec3::new(4.0, 5.0, 6.0);
/// let sum = v + w;  // Vec3::new(5.0, 7.0, 9.0)
/// ```
impl Add for Vec3 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self([
            self.x() + other.x(),
            self.y() + other.y(),
            self.z() + other.z(),
        ])
    }
}

/// Subtracts two vectors component-wise.
impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self([
            self.x() - other.x(),
            self.y() - other.y(),
            self.z() - other.z(),
        ])
    }
}

/// Multiplies vector by scalar (component-wise).
///
/// # Example
/// ```
/// # use your_crate::math::Vec3;
/// let v = Vec3::new(1.0, 2.0, 3.0);
/// let scaled = v * 2.0;  // Vec3::new(2.0, 4.0, 6.0)
/// ```
impl Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, scalar: f32) -> Self {
        Self([self.x() * scalar, self.y() * scalar, self.z() * scalar])
    }
}

/// A 2D vector for 2D mathematical operations.
///
/// This struct represents a 2D vector with x and z components, commonly used
/// for 2D positioning and calculations in the game.
#[derive(Copy, Clone, Debug)]
pub struct Vec2(pub [f32; 2]);

impl Vec2 {
    /// Creates a new 2D vector with the given x and z components.
    ///
    /// # Arguments
    /// * `x` - The x component
    /// * `z` - The z component
    ///
    /// # Returns
    /// A new Vec2 instance
    pub fn new(x: f32, z: f32) -> Self {
        Vec2([x, z])
    }

    /// Returns the x component of the vector.
    pub fn x(&self) -> f32 {
        self.0[0]
    }

    /// Returns the z component of the vector.
    pub fn z(&self) -> f32 {
        self.0[1]
    }

    /// Calculates the length (magnitude) of the vector.
    ///
    /// # Returns
    /// The Euclidean length of the vector
    pub fn length(&self) -> f32 {
        (self.x().powi(2) + self.z().powi(2)).sqrt()
    }

    /// Normalizes the vector to unit length.
    ///
    /// # Returns
    /// A normalized vector with length 1, or a zero vector if the original length is zero
    pub fn normalize(&self) -> Self {
        let length = self.length();
        if length <= f32::EPSILON {
            return Self([0.0, 0.0]);
        }
        Self([self.x() / length, self.z() / length])
    }

    /// Calculates the Euclidean distance between this vector and another.
    ///
    /// # Arguments
    /// * `other` - The other vector to calculate distance to
    ///
    /// # Returns
    /// The distance between the two vectors
    pub fn distance_to(&self, other: &Self) -> f32 {
        (*self - *other).length()
    }

    /// Rotates the vector by the specified angle in radians.
    ///
    /// This method applies a 2D rotation transformation to the vector.
    /// The rotation is counterclockwise around the origin.
    ///
    /// # Arguments
    /// * `angle` - The rotation angle in radians
    ///
    /// # Returns
    /// A new vector rotated by the specified angle
    ///
    /// # Example
    /// ```
    /// # use your_crate::math::Vec2;
    /// let v = Vec2::new(1.0, 0.0);
    /// let rotated = v.rotate(std::f32::consts::PI / 2.0); // Rotate 90 degrees
    /// ```
    pub fn rotate(&self, angle: f32) -> Self {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        Self([
            self.x() * cos_a - self.z() * sin_a,
            self.x() * sin_a + self.z() * cos_a,
        ])
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self([self.x() + other.x(), self.z() + other.z()])
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self([self.x() - other.x(), self.z() - other.z()])
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, scalar: f32) -> Self {
        Self([self.x() * scalar, self.z() * scalar])
    }
}
