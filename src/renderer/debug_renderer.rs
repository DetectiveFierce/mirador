//! Debug rendering utilities for visualizing collision systems.
//!
//! This module provides tools for rendering debug elements like bounding boxes,
//! which are useful for understanding and debugging the collision system.

use crate::game::collision::{AABB, BVH, BVHNode};
use crate::renderer::vertex::Vertex;

/// Material ID for debug bounding boxes
pub const BOUNDING_BOX_MATERIAL: u32 = 2;

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
        },
        Vertex {
            position: [max[0], min[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [max[0], max[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [min[0], min[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [max[0], max[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [min[0], max[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
    ]);

    // Back face (Z-)
    vertices.extend_from_slice(&[
        Vertex {
            position: [max[0], min[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [min[0], min[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [min[0], max[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [max[0], min[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [min[0], max[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [max[0], max[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
    ]);

    // Right face (X+)
    vertices.extend_from_slice(&[
        Vertex {
            position: [max[0], min[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [max[0], max[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [max[0], max[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [max[0], min[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [max[0], max[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [max[0], min[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
    ]);

    // Left face (X-)
    vertices.extend_from_slice(&[
        Vertex {
            position: [min[0], min[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [min[0], max[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [min[0], max[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [min[0], min[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [min[0], max[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [min[0], min[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
    ]);

    // Top face (Y+)
    vertices.extend_from_slice(&[
        Vertex {
            position: [min[0], max[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [max[0], max[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [max[0], max[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [min[0], max[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [max[0], max[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [min[0], max[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
    ]);

    // Bottom face (Y-)
    vertices.extend_from_slice(&[
        Vertex {
            position: [min[0], min[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [max[0], min[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [max[0], min[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [min[0], min[1], min[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [max[0], min[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
        },
        Vertex {
            position: [min[0], min[1], max[2]],
            color,
            material: BOUNDING_BOX_MATERIAL,
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
