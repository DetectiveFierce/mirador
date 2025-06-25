//! Uniform buffer utilities for wgpu rendering.
//!
//! This module provides the [`Uniforms`] struct for storing and uploading uniform data
//! (such as transformation matrices) to the GPU, as well as helper methods for buffer and bind group creation.

use egui_wgpu::wgpu;
use egui_wgpu::wgpu::util::DeviceExt;

/// Uniforms for the main render pipeline.
///
/// This struct stores a 4x4 matrix (typically Model-View-Projection) to be sent to the GPU as a uniform buffer.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    /// The 4x4 transformation matrix (e.g., MVP matrix).
    pub matrix: [[f32; 4]; 4],
}

impl Default for Uniforms {
    /// Returns a new [`Uniforms`] with all elements set to zero.
    fn default() -> Self {
        Self::new()
    }
}

impl Uniforms {
    /// Creates a new [`Uniforms`] with all elements set to zero.
    pub fn new() -> Self {
        Self {
            matrix: [[0.0; 4]; 4],
        }
    }

    /// Returns the raw bytes of the uniform struct for uploading to the GPU.
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    /// Creates a GPU buffer containing the uniform data.
    ///
    /// # Arguments
    /// * `device` - The wgpu device to create the buffer with.
    ///
    /// # Returns
    /// A [`wgpu::Buffer`] with the uniform data, ready for use as a uniform buffer.
    pub fn create_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: self.as_bytes(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }

    /// Creates a bind group and layout for the uniform buffer.
    ///
    /// # Arguments
    /// * `buffer` - The uniform buffer to bind.
    /// * `device` - The wgpu device to create the bind group and layout.
    ///
    /// # Returns
    /// A tuple of (`wgpu::BindGroup`, `wgpu::BindGroupLayout`) for binding the uniform buffer in a pipeline.
    pub fn create_bind_group(
        &self,
        buffer: &wgpu::Buffer,
        device: &wgpu::Device,
    ) -> (wgpu::BindGroup, wgpu::BindGroupLayout) {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX, // Visible in vertex shader
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None, // Or Some(std::num::NonZeroU64::new(std::mem::size_of::<Uniforms>() as u64))
                },
                count: None,
            }],
            label: Some("uniform_bind_group_layout"),
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        });
        (bind_group, layout)
    }
}
