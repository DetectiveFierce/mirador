//! Vertex definitions and geometry generation for maze and floor rendering.
//!
//! This module provides the [`Vertex`] struct, which describes the layout of vertex data for the renderer,
//! and utility functions for generating floor and wall geometry from maze data.

use egui_wgpu::wgpu;

/// Vertex data for rendering maze and floor geometry.
///
/// Each vertex contains:
/// - `position`: 3D position in world space.
/// - `color`: RGBA color (as 4 normalized u8 values).
/// - `material`: Material type (0 = floor, 1 = wall).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    /// 3D position in world space.
    pub position: [f32; 3],
    /// RGBA color (normalized 0-255).
    pub color: [u8; 4],
    /// Material type (0 = floor, 1 = wall).
    pub material: u32, // 0 = floor, 1 = wall
}

impl Vertex {
    /// Returns the vertex buffer layout for use in a wgpu pipeline.
    ///
    /// This describes the memory layout of [`Vertex`] for the GPU.
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress, // Correct overall stride
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position (3 floats)
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3, // <--- CHANGE THIS to Float32x3
                },
                // Color (4 u8 bytes, interpreted as normalized floats in shader)
                wgpu::VertexAttribute {
                    // Offset: size of 3 floats = 3 * 4 = 12 bytes
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress, // <--- CHANGE THIS to size_of::<[f32; 3]>()
                    shader_location: 1,
                    // Format: 4 unsigned 8-bit integers, normalized to floats (0.0 to 1.0)
                    format: wgpu::VertexFormat::Unorm8x4, // <--- CHANGE THIS to Unorm8x4
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress
                        + std::mem::size_of::<[u8; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }

    /// Generates vertices for a large square floor centered at the origin.
    ///
    /// # Returns
    /// A tuple containing a vector of [`Vertex`] and the number of vertices.
    pub fn create_floor_vertices() -> (Vec<Vertex>, usize) {
        let floor_size = 3000.0; // Size of the square floor
        let half_size = floor_size / 2.0;

        // Define the four corners of the square floor centered at origin
        let positions: Vec<f32> = vec![
            // Bottom face (y = 0, looking down from above)
            -half_size, 0.0, -half_size, // Bottom-left
            half_size, 0.0, -half_size, // Bottom-right
            half_size, 0.0, half_size, // Top-right
            -half_size, 0.0, half_size, // Top-left
        ];

        // Two triangles to form the square floor
        // Triangle 1: vertices 0, 1, 2
        // Triangle 2: vertices 0, 2, 3
        let indices: Vec<usize> = vec![
            0, 1, 2, // First triangle
            0, 2, 3, // Second triangle
        ];

        // Colors for each triangle (can be the same or different)
        let triangle_colors: Vec<[u8; 3]> = vec![
            [120, 80, 160],  // Purple-ish for first triangle
            [100, 120, 180], // Blue-ish for second triangle
        ];

        let num_vertices = indices.len();
        let vertex_data: Vec<Vertex> = (0..num_vertices)
            .map(|i| {
                let position_idx = indices[i] * 3;
                let position = [
                    positions[position_idx],
                    positions[position_idx + 1],
                    positions[position_idx + 2],
                ];

                let triangle_idx = i / 3; // Which triangle (0 or 1)
                let color = [
                    triangle_colors[triangle_idx][0],
                    triangle_colors[triangle_idx][1],
                    triangle_colors[triangle_idx][2],
                    255, // Alpha
                ];

                Vertex {
                    position,
                    color,
                    material: 0,
                }
            })
            .collect();

        (vertex_data, num_vertices)
    }

    /// Generates wall geometry for a maze grid.
    ///
    /// For each wall cell (`true`), creates the necessary wall faces (as quads) to form the maze.
    ///
    /// # Arguments
    /// * `maze_grid` - 2D grid of booleans, where `true` indicates a wall.
    ///
    /// # Returns
    /// A vector of [`Vertex`] representing all wall faces.
    pub fn create_wall_vertices(maze_grid: &[Vec<bool>]) -> Vec<Vertex> {
        let mut vertices = Vec::new();

        let floor_size = 3000.0; // Match the floor size from create_floor_vertices
        let maze_width = maze_grid[0].len();
        let maze_height = maze_grid.len();

        // Calculate cell size to scale the maze to fit the floor
        let max_dimension = maze_width.max(maze_height) as f32;
        let cell_size = floor_size / max_dimension;
        let wall_height = cell_size;

        // Calculate origin to center the maze
        let origin_x = -(maze_width as f32 * cell_size) / 2.0;
        let origin_z = -(maze_height as f32 * cell_size) / 2.0;

        for (z, row) in maze_grid.iter().enumerate() {
            for (x, &is_wall) in row.iter().enumerate() {
                if is_wall {
                    let wx = origin_x + x as f32 * cell_size;
                    let wz = origin_z + z as f32 * cell_size;

                    // Create both X-facing and Z-facing walls for each wall cell

                    // Check if we need an X-facing wall (along Z axis)
                    if z == 0 || !maze_grid[z - 1][x] {
                        vertices.extend(create_z_facing_wall(wx, 0.0, wz, cell_size, wall_height));
                    }

                    // Check if we need a Z-facing wall (along X axis)
                    if x == 0 || !maze_grid[z][x - 1] {
                        vertices.extend(create_x_facing_wall(wx, 0.0, wz, cell_size, wall_height));
                    }

                    // Always create the right and bottom walls if we're at the edge
                    if z == maze_height - 1 {
                        vertices.extend(create_z_facing_wall(
                            wx,
                            0.0,
                            wz + cell_size,
                            cell_size,
                            wall_height,
                        ));
                    }
                    if x == maze_width - 1 {
                        vertices.extend(create_x_facing_wall(
                            wx + cell_size,
                            0.0,
                            wz,
                            cell_size,
                            wall_height,
                        ));
                    }
                }
            }
        }

        vertices
    }
}

/// Creates a wall quad facing the Z direction (parallel to X axis).
///
/// # Arguments
/// * `x`, `y`, `z` - Starting position.
/// * `width` - Width of the wall.
/// * `height` - Height of the wall.
///
/// # Returns
/// An array of 6 [`Vertex`] forming two triangles (a quad).
pub fn create_z_facing_wall(x: f32, y: f32, z: f32, width: f32, height: f32) -> [Vertex; 6] {
    let color: [u8; 4] = [107, 55, 55, 255];
    [
        Vertex {
            position: [x, y, z],
            color,
            material: 1,
        },
        Vertex {
            position: [x + width, y, z],
            color,
            material: 1,
        },
        Vertex {
            position: [x + width, y + height, z],
            color,
            material: 1,
        },
        Vertex {
            position: [x, y, z],
            color,
            material: 1,
        },
        Vertex {
            position: [x + width, y + height, z],
            color,
            material: 1,
        },
        Vertex {
            position: [x, y + height, z],
            color,
            material: 1,
        },
    ]
}

/// Creates a wall quad facing the X direction (parallel to Z axis).
///
/// # Arguments
/// * `x`, `y`, `z` - Starting position.
/// * `depth` - Depth of the wall.
/// * `height` - Height of the wall.
///
/// # Returns
/// An array of 6 [`Vertex`] forming two triangles (a quad).
pub fn create_x_facing_wall(x: f32, y: f32, z: f32, depth: f32, height: f32) -> [Vertex; 6] {
    let color: [u8; 4] = [107, 55, 55, 255];
    [
        Vertex {
            position: [x, y, z],
            color,
            material: 1,
        },
        Vertex {
            position: [x, y, z + depth],
            color,
            material: 1,
        },
        Vertex {
            position: [x, y + height, z + depth],
            color,
            material: 1,
        },
        Vertex {
            position: [x, y, z],
            color,
            material: 1,
        },
        Vertex {
            position: [x, y + height, z + depth],
            color,
            material: 1,
        },
        Vertex {
            position: [x, y + height, z],
            color,
            material: 1,
        },
    ]
}
