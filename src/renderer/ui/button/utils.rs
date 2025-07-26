use glyphon::Color;

/// Extension trait for color manipulation operations.
///
/// This trait provides methods for modifying colors by darkening, brightening,
/// and saturating them for UI button states and effects.
pub trait ColorExt {
    /// Darkens the color by the specified factor.
    ///
    /// # Arguments
    /// * `factor` - Darkening factor between 0.0 and 1.0
    ///
    /// # Returns
    /// A darker version of the color
    fn darken(&self, factor: f32) -> Self;
    /// Brightens the color by the specified factor.
    ///
    /// # Arguments
    /// * `factor` - Brightening factor between 0.0 and 1.0
    ///
    /// # Returns
    /// A brighter version of the color
    fn brighten(&self, factor: f32) -> Self;
    /// Increases the saturation of the color by the specified factor.
    ///
    /// # Arguments
    /// * `factor` - Saturation factor between 0.0 and 1.0
    ///
    /// # Returns
    /// A more saturated version of the color
    fn saturate(&self, factor: f32) -> Self;
}

impl ColorExt for Color {
    fn darken(&self, factor: f32) -> Self {
        let factor = factor.clamp(0.0, 1.0);
        Color::rgba(
            (self.r() as f32 * (1.0 - factor)) as u8,
            (self.g() as f32 * (1.0 - factor)) as u8,
            (self.b() as f32 * (1.0 - factor)) as u8,
            self.a(),
        )
    }
    fn brighten(&self, factor: f32) -> Self {
        let factor = factor.clamp(0.0, 1.0);
        Color::rgba(
            (self.r() as f32 + (255.0 - self.r() as f32) * factor) as u8,
            (self.g() as f32 + (255.0 - self.g() as f32) * factor) as u8,
            (self.b() as f32 + (255.0 - self.b() as f32) * factor) as u8,
            self.a(),
        )
    }
    fn saturate(&self, factor: f32) -> Self {
        // Convert RGB to HSL, increase saturation, then convert back
        let r = self.r() as f32 / 255.0;
        let g = self.g() as f32 / 255.0;
        let b = self.b() as f32 / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;
        let d = max - min;
        let mut s = if d == 0.0 {
            0.0
        } else {
            d / (1.0 - (2.0 * l - 1.0).abs())
        };
        s = (s + factor).min(1.0);
        // Recompute RGB from HSL (approximate, since hue is not changed)
        // We'll just scale the color channels away from the gray axis
        let gray = l;
        let scale = if s == 0.0 { 0.0 } else { s };
        let new_r = gray + (r - gray) * (1.0 + scale);
        let new_g = gray + (g - gray) * (1.0 + scale);
        let new_b = gray + (b - gray) * (1.0 + scale);
        Color::rgba(
            (new_r.clamp(0.0, 1.0) * 255.0) as u8,
            (new_g.clamp(0.0, 1.0) * 255.0) as u8,
            (new_b.clamp(0.0, 1.0) * 255.0) as u8,
            self.a(),
        )
    }
}

/// Calculates DPI scaling factor based on window height.
///
/// This function provides a simple DPI scaling calculation that scales UI elements
/// based on the window height, with reasonable bounds to prevent extreme scaling.
///
/// # Arguments
/// * `window_height` - The height of the window in pixels
///
/// # Returns
/// A scaling factor between 0.7 and 2.0
pub fn dpi_scale(window_height: f32) -> f32 {
    (window_height / 1080.0).clamp(0.7, 2.0)
}
