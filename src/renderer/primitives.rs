//! Uniform buffer utilities for wgpu rendering.
//!
//! This module provides the [`Uniforms`] struct for storing and uploading uniform data
//! (such as transformation matrices) to the GPU, as well as helper methods for buffer and bind group creation.

use crate::game::maze::generator::Cell;
use crate::math::coordinates::constants::get_floor_size;
use bytemuck::{Pod, Zeroable};
use wgpu;
use wgpu::util::DeviceExt;
/// Uniforms for the main render pipeline.
///
/// This struct stores a 4x4 matrix (typically Model-View-Projection) to be sent to the GPU as a uniform buffer.
// Updated Uniforms structure to include time for animation
// Updated Uniforms structure to include time for animation
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Uniforms {
    pub matrix: [[f32; 4]; 4],
    pub time: f32,
    pub _padding: [f32; 7], // Padding for 16-byte alignment
}

impl Default for Uniforms {
    fn default() -> Self {
        Self::new()
    }
}

impl Uniforms {
    pub fn new() -> Self {
        Self {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            time: 0.0,
            _padding: [0.0; 7],
        }
    }

    pub fn create_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::bytes_of(self),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }

    pub fn create_bind_group(
        &self,
        buffer: &wgpu::Buffer,
        device: &wgpu::Device,
    ) -> (wgpu::BindGroup, wgpu::BindGroupLayout) {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        (bind_group, layout)
    }

    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

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
    pub fn create_floor_vertices(
        maze_grid: &[Vec<bool>],
        exit_cell: Option<Cell>,
        is_test_mode: bool,
    ) -> (Vec<Vertex>, (f32, f32)) {
        let floor_size = get_floor_size(is_test_mode); // Use test mode to determine floor size
        let half_size = floor_size / 2.0;

        // Create base floor vertices
        let mut vertices = Vec::new();

        // Define the four corners of the square floor centered at origin
        let positions: Vec<f32> = vec![
            // Bottom face (y = 0, looking down from above)
            -half_size, 0.0, -half_size, // Bottom-left
            half_size, 0.0, -half_size, // Bottom-right
            half_size, 0.0, half_size, // Top-right
            -half_size, 0.0, half_size, // Top-left
        ];

        // Two triangles to form the square floor
        let indices: Vec<usize> = vec![
            0, 1, 2, // First triangle
            0, 2, 3, // Second triangle
        ];

        // Colors for base floor triangles
        let base_triangle_colors: Vec<[u8; 3]> = vec![
            [120, 80, 160],  // Purple-ish for first triangle
            [100, 120, 180], // Blue-ish for second triangle
        ];

        // Add base floor vertices
        for (i, &index) in indices.iter().enumerate() {
            let position_idx = index * 3;
            let position = [
                positions[position_idx],
                positions[position_idx + 1],
                positions[position_idx + 2],
            ];
            let triangle_idx = i / 3;
            let color = [
                base_triangle_colors[triangle_idx][0],
                base_triangle_colors[triangle_idx][1],
                base_triangle_colors[triangle_idx][2],
                255,
            ];
            vertices.push(Vertex {
                position,
                color,
                material: 0,
            });
        }

        let mut exit_position = (0.0, 0.0);
        // Add green exit cell floor patch if exit exists
        if let Some(exit) = exit_cell {
            let exit_vertices = create_exit_cell_floor_patch(maze_grid, exit);
            vertices.extend(exit_vertices.0);
            exit_position = exit_vertices.1;
        }

        (vertices, exit_position)
    }
    /// Generates wall geometry for a maze grid.
    ///
    /// For each wall cell (`true`), creates the necessary wall faces (as quads) to form the maze.
    /// When test mode is enabled, only creates perimeter walls.
    ///
    /// # Arguments
    /// * `maze_grid` - 2D grid of booleans, where `true` indicates a wall.
    /// * `is_test_mode` - Whether test mode is enabled (affects wall generation)
    ///
    /// # Returns
    /// A vector of [`Vertex`] representing all wall faces.
    pub fn create_wall_vertices(maze_grid: &[Vec<bool>], is_test_mode: bool) -> Vec<Vertex> {
        let mut vertices = Vec::new();

        let floor_size = get_floor_size(is_test_mode); // Use test mode to determine floor size
        let maze_width = maze_grid[0].len();
        let maze_height = maze_grid.len();

        // Calculate cell size to scale the maze to fit the floor
        let max_dimension = maze_width.max(maze_height) as f32;
        let cell_size = floor_size / max_dimension;
        let wall_height = cell_size;

        // Calculate origin to center the maze
        let origin_x = -(maze_width as f32 * cell_size) / 2.0;
        let origin_z = -(maze_height as f32 * cell_size) / 2.0;

        if is_test_mode {
            // Test mode: only create perimeter walls
            // Top wall (row 0)
            for x in 0..maze_width {
                if maze_grid[0][x] {
                    let wx = origin_x + x as f32 * cell_size;
                    let wz = origin_z + 0.0 * cell_size;
                    vertices.extend(create_z_facing_wall(wx, 0.0, wz, cell_size, wall_height));
                }
            }

            // Bottom wall (row maze_height-1)
            for x in 0..maze_width {
                if maze_grid[maze_height - 1][x] {
                    let wx = origin_x + x as f32 * cell_size;
                    let wz = origin_z + (maze_height - 1) as f32 * cell_size;
                    vertices.extend(create_z_facing_wall(
                        wx,
                        0.0,
                        wz + cell_size,
                        cell_size,
                        wall_height,
                    ));
                }
            }

            // Left wall (column 0)
            for z in 0..maze_height {
                if maze_grid[z][0] {
                    let wx = origin_x + 0.0 * cell_size;
                    let wz = origin_z + z as f32 * cell_size;
                    vertices.extend(create_x_facing_wall(wx, 0.0, wz, cell_size, wall_height));
                }
            }

            // Right wall (column maze_width-1)
            for z in 0..maze_height {
                if maze_grid[z][maze_width - 1] {
                    let wx = origin_x + (maze_width - 1) as f32 * cell_size;
                    let wz = origin_z + z as f32 * cell_size;
                    vertices.extend(create_x_facing_wall(
                        wx + cell_size,
                        0.0,
                        wz,
                        cell_size,
                        wall_height,
                    ));
                }
            }
        } else {
            // Normal mode: create all walls from the maze grid
            for (z, row) in maze_grid.iter().enumerate() {
                for (x, &is_wall) in row.iter().enumerate() {
                    if is_wall {
                        let wx = origin_x + x as f32 * cell_size;
                        let wz = origin_z + z as f32 * cell_size;

                        // Create both X-facing and Z-facing walls for each wall cell

                        // Check if we need an X-facing wall (along Z axis)
                        if z == 0 || !maze_grid[z - 1][x] {
                            vertices.extend(create_z_facing_wall(
                                wx,
                                0.0,
                                wz,
                                cell_size,
                                wall_height,
                            ));
                        }

                        // Check if we need a Z-facing wall (along X axis)
                        if x == 0 || !maze_grid[z][x - 1] {
                            vertices.extend(create_x_facing_wall(
                                wx,
                                0.0,
                                wz,
                                cell_size,
                                wall_height,
                            ));
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
        }

        vertices
    }

    /// Creates a green exit patch at an arbitrary world position (centered at x, z)
    pub fn create_exit_patch_at_world_position(
        center: (f32, f32),
        is_test_mode: bool,
    ) -> Vec<Vertex> {
        let floor_size = get_floor_size(is_test_mode); // Use test mode to determine floor size
        // Use a patch size similar to a cell size
        let patch_size = floor_size / 13.0; // 13 is the wall grid size for 6x6 maze
        let (center_x, center_z) = center;
        let y = 1.0;
        let half = patch_size / 2.0;
        let green_color = [0, 255, 0, 255];
        let corners = [
            [center_x - half, y, center_z - half], // Bottom-left
            [center_x + half, y, center_z - half], // Bottom-right
            [center_x + half, y, center_z + half], // Top-right
            [center_x - half, y, center_z + half], // Top-left
        ];
        vec![
            // First triangle: 0, 1, 2
            Vertex {
                position: corners[0],
                color: green_color,
                material: 4,
            },
            Vertex {
                position: corners[1],
                color: green_color,
                material: 4,
            },
            Vertex {
                position: corners[2],
                color: green_color,
                material: 4,
            },
            // Second triangle: 0, 2, 3
            Vertex {
                position: corners[0],
                color: green_color,
                material: 4,
            },
            Vertex {
                position: corners[2],
                color: green_color,
                material: 4,
            },
            Vertex {
                position: corners[3],
                color: green_color,
                material: 4,
            },
        ]
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

fn create_exit_cell_floor_patch(
    maze_grid: &[Vec<bool>],
    exit_cell: Cell,
) -> (Vec<Vertex>, (f32, f32)) {
    let floor_size = get_floor_size(false); // Normal mode for exit cell patch
    let maze_width = maze_grid[0].len();
    let maze_height = maze_grid.len();

    let max_dimension = maze_width.max(maze_height) as f32;
    let cell_size = floor_size / max_dimension;

    let origin_x = -(maze_width as f32 * cell_size) / 2.0;
    let origin_z = -(maze_height as f32 * cell_size) / 2.0;

    let world_x = origin_x + exit_cell.col as f32 * cell_size;
    let world_z = origin_z + exit_cell.row as f32 * cell_size;

    let green_color = [0, 255, 0, 255]; // Bright green

    let corners = [
        [world_x, 1.0, world_z],                         // Bottom-left
        [world_x + cell_size, 1.0, world_z],             // Bottom-right
        [world_x + cell_size, 1.0, world_z + cell_size], // Top-right
        [world_x, 1.0, world_z + cell_size],             // Top-left
    ];

    let vertices = vec![
        // First triangle: 0, 1, 2
        Vertex {
            position: corners[0],
            color: green_color,
            material: 4,
        },
        Vertex {
            position: corners[1],
            color: green_color,
            material: 4,
        },
        Vertex {
            position: corners[2],
            color: green_color,
            material: 4,
        },
        // Second triangle: 0, 2, 3
        Vertex {
            position: corners[0],
            color: green_color,
            material: 4,
        },
        Vertex {
            position: corners[2],
            color: green_color,
            material: 4,
        },
        Vertex {
            position: corners[3],
            color: green_color,
            material: 4,
        },
    ];

    let center_x = world_x + cell_size / 2.0;
    let center_z = world_z + cell_size / 2.0;

    (vertices, (center_x, center_z))
}
