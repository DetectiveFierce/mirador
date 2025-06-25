//! Math utilities and types for 3D graphics and game logic.
//!
//! This module provides matrix and vector types and operations, as well as
//! helper functions for angle conversions. All types are designed to be
//! compatible with GPU memory layouts (e.g., for use with WGPU/WGSL).
//!
//! # Module Organization
//!
//! - [`vec`] module contains all vector operations (re-exported at root level)
//! - [`mat`] module contains all matrix operations (re-exported at root level)
//! - Utility functions like angle conversions are provided at root level

pub mod mat;
pub mod vec;

/// Converts degrees to radians.
///
/// This handles angle wrapping by first normalizing the input to the range [0, 360).
///
/// # Arguments
///
/// * `degrees` - The angle in degrees (can be any finite value)
///
/// # Returns
///
/// The angle in radians in range [0, 2π)
///
/// # Example
/// ```
/// use your_crate::math::deg_to_rad;
///
/// // Basic conversion
/// assert_eq!(deg_to_rad(180.0), std::f32::consts::PI);
///
/// // Handles angle wrapping
/// assert_eq!(deg_to_rad(540.0), std::f32::consts::PI);
/// ```
pub fn deg_to_rad(degrees: f32) -> f32 {
    (degrees % 360.0) * (std::f32::consts::PI / 180.0)
}

/// Converts radians to degrees.
///
/// This handles angle wrapping by first normalizing the input to the range [0, 2π).
///
/// # Arguments
///
/// * `radians` - The angle in radians (can be any finite value)
///
/// # Returns
///
/// The angle in degrees in range [0, 360)
///
/// # Example
/// ```
/// use your_crate::math::rad_to_deg;
///
/// // Basic conversion
/// assert_eq!(rad_to_deg(std::f32::consts::PI), 180.0);
///
/// // Handles angle wrapping
/// assert_eq!(rad_to_deg(3.0 * std::f32::consts::PI), 180.0);
/// ```
#[allow(dead_code)]
pub fn rad_to_deg(radians: f32) -> f32 {
    (radians % (2.0 * std::f32::consts::PI)) * (180.0 / std::f32::consts::PI)
}
