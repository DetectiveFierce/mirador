//! Debug rendering utilities for visualizing collision systems.
//!
//! This module provides tools for rendering debug elements like bounding boxes,
//! which are useful for understanding and debugging the collision system.

use crate::game::collision::CollisionSystem;
use crate::game::collision::{AABB, BVH, BVHNode};
use crate::renderer::primitives::Vertex;
use wgpu;
use wgpu::util::DeviceExt;
/// Material ID for debug bounding boxes
pub const BOUNDING_BOX_MATERIAL: u32 = 2;

pub struct DebugRenderer {
    /// Whether to render bounding boxes for debugging.
    pub debug_render_bounding_boxes: bool,
    /// Vertex buffer for the debug renderer
    pub debug_vertex_buffer: Option<wgpu::Buffer>,
    pub debug_vertex_count: usize,
}

impl DebugRenderer {
    pub fn update_debug_vertices(
        &mut self,
        device: &wgpu::Device,
        collision_system: &CollisionSystem,
    ) {
        // Skip if debug rendering is disabled
        if !self.debug_render_bounding_boxes {
            self.debug_vertex_count = 0;
            return;
        }

        // Collect only wall face AABBs, not the entire BVH hierarchy
        let debug_vertices = collect_wall_face_debug_vertices(&collision_system.bvh);

        // Create or update the debug vertex buffer
        self.debug_vertex_count = debug_vertices.len();
        if self.debug_vertex_count > 0 {
            let debug_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Debug Vertex Buffer"),
                contents: bytemuck::cast_slice(&debug_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            self.debug_vertex_buffer = Some(debug_buffer);
        }
    }
}

/// Generates vertices for rendering an AABB as a solid semitransparent box.
///
/// # Arguments
///
/// * `aabb` - The Axis-Aligned Bounding Box to visualize
///
/// # Returns
///
/// A vector of vertices defining the triangles of the bounding box
pub fn create_aabb_box_vertices(aabb: &AABB) -> Vec<Vertex> {
    let color: [u8; 4] = [255, 0, 0, 77]; // Semitransparent red
    let min = aabb.min;
    let max = aabb.max;

    let mut vertices = Vec::new();

    // Front face (Z+)
    vertices.extend_from_slice(&[
        Vertex {
            position: [min[0], min[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0], // Debug doesn't use texture coordinates
        },
        Vertex {
            position: [max[0], min[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [max[0], max[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [min[0], min[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [max[0], max[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [min[0], max[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
    ]);

    // Back face (Z-)
    vertices.extend_from_slice(&[
        Vertex {
            position: [max[0], min[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [min[0], min[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [min[0], max[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [max[0], min[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [min[0], max[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [max[0], max[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
    ]);

    // Right face (X+)
    vertices.extend_from_slice(&[
        Vertex {
            position: [max[0], min[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [max[0], max[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [max[0], max[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [max[0], min[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [max[0], max[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [max[0], min[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
    ]);

    // Left face (X-)
    vertices.extend_from_slice(&[
        Vertex {
            position: [min[0], min[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [min[0], max[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [min[0], max[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [min[0], min[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [min[0], max[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [min[0], min[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
    ]);

    // Top face (Y+)
    vertices.extend_from_slice(&[
        Vertex {
            position: [min[0], max[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [max[0], max[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [max[0], max[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [min[0], max[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [max[0], max[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [min[0], max[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
    ]);

    // Bottom face (Y-)
    vertices.extend_from_slice(&[
        Vertex {
            position: [min[0], min[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [max[0], min[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [max[0], min[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [min[0], min[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [max[0], min[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [min[0], min[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
            tex_coords: [0.0, 0.0],
        },
    ]);

    vertices
}

/// Collects all wall face AABB debug vertices from the BVH tree.
///
/// This traverses the BVH to find all leaf nodes and generates visualization
/// vertices only for the actual wall faces, not for the BVH structure itself.
///
/// # Arguments
///
/// * `bvh` - Reference to the collision system BVH
///
/// # Returns
///
/// A vector of vertices representing the wall face bounding boxes
pub fn collect_wall_face_debug_vertices(bvh: &BVH) -> Vec<Vertex> {
    let mut vertices = Vec::new();

    if let Some(root) = &bvh.root {
        collect_wall_faces_recursive(root, &mut vertices);
    }

    vertices
}

/// Helper function to recursively collect wall face AABBs from BVH nodes
fn collect_wall_faces_recursive(node: &BVHNode, vertices: &mut Vec<Vertex>) {
    match node {
        BVHNode::Internal { left, right, .. } => {
            // Just traverse the tree, don't visualize internal nodes
            collect_wall_faces_recursive(left, vertices);
            collect_wall_faces_recursive(right, vertices);
        }
        BVHNode::Leaf { faces, .. } => {
            // Visualize only the actual wall faces
            for face in faces {
                vertices.extend(create_aabb_box_vertices(&face.aabb));
            }
        }
    }
}
