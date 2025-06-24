use std::ops::{Add, Mul, Sub};

/*
Requirements for Memory Compatibility with WGPU:
   1. Standard layout (like C structs).
   2. Alignment that matches WGSL expectations.
   3. Sized correctly for GPU buffers.
   4. Can be safely cast to [f32; N] or bytes.
*/

#[repr(transparent)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vec3([f32; 3]);

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3([x, y, z])
    }

    pub fn dot(&self, other: &Self) -> f32 {
        self.x() * other.x() + self.y() * other.y() + self.z() * other.z()
    }

    pub fn cross(&self, other: &Self) -> Self {
        Vec3([
            self.y() * other.z() - self.z() * other.y(),
            self.z() * other.x() - self.x() * other.z(),
            self.x() * other.y() - self.y() * other.x(),
        ])
    }

    pub fn length(&self) -> f32 {
        (self.x().powi(2) + self.y().powi(2) + self.z().powi(2)).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let length = self.length();
        if length.eq(&0.0) {
            Self([0.0, 0.0, 0.0]);
        }

        Self([self.x() / length, self.y() / length, self.z() / length])
    }

    pub fn subtract(&self, other: &Self) -> Self {
        Vec3([
            self.x() - other.x(),
            self.y() - other.y(),
            self.z() - other.z(),
        ])
    }

    pub fn as_array(&self) -> &[f32; 3] {
        &self.0
    }
    pub fn x(&self) -> f32 {
        self.0[0]
    }
    pub fn y(&self) -> f32 {
        self.0[1]
    }
    pub fn z(&self) -> f32 {
        self.0[2]
    }
}

// Implement From<[f32; 3]> for Vec3
impl From<[f32; 3]> for Vec3 {
    fn from(values: [f32; 3]) -> Self {
        Vec3(values)
    }
}

// Also implement the reverse conversion (optional but often useful)
impl From<Vec3> for [f32; 3] {
    fn from(vec: Vec3) -> Self {
        vec.0
    }
}
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

// Implement vector subtraction for Vertex
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

impl Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, scalar: f32) -> Self {
        Self([self.x() * scalar, self.y() * scalar, self.z() * scalar])
    }
}
