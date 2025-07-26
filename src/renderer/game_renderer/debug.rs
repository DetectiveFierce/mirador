//! Debug rendering utilities for visualizing collision systems.
//!
//! This module provides tools for rendering debug elements like bounding boxes,
//! which are useful for understanding and debugging the collision system.
//!
//! # Overview
//!
//! The debug renderer visualizes the collision system's internal structures,
//! particularly the Axis-Aligned Bounding Boxes (AABBs) used for efficient
//! collision detection. This is essential for debugging collision issues,
//! understanding spatial partitioning, and verifying collision detection accuracy.
//!
//! # Features
//!
//! - **Bounding Box Visualization**: Renders semitransparent red boxes around
//!   collision objects for easy visual identification
//! - **Wall Face Highlighting**: Specifically visualizes wall faces from the
//!   BVH (Bounding Volume Hierarchy) collision system
//! - **Performance Optimized**: Only renders when debug mode is enabled
//! - **GPU-Efficient**: Uses vertex buffers and instanced rendering for
//!   optimal performance
//!
//! # Usage
//!
//! ```rust
//! // Enable debug rendering
//! debug_renderer.debug_render_bounding_boxes = true;
//!
//! // Update debug vertices when collision system changes
//! debug_renderer.update_debug_vertices(device, collision_system);
//!
//! // Render debug elements (call after main game rendering)
//! debug_renderer.render(&mut render_pass);
//! ```
//!
//! # Material System
//!
//! Debug elements use a dedicated material ID (`BOUNDING_BOX_MATERIAL`) to
//! ensure they render with the correct shader and blending settings.

use crate::game::collision::CollisionSystem;
use crate::game::collision::{AABB, BVH, BVHNode};
use crate::renderer::primitives::Vertex;
use wgpu;
use wgpu::util::DeviceExt;

/// Material ID for debug bounding boxes.
///
/// This constant defines the material identifier used by the debug renderer
/// to distinguish debug elements from regular game objects in the shader.
/// The shader uses this ID to apply appropriate rendering settings for
/// debug visualization.
pub const BOUNDING_BOX_MATERIAL: u32 = 2;

/// Debug renderer for visualizing collision system internals.
///
/// This struct manages the rendering of debug elements such as bounding boxes
/// and collision volumes. It provides a visual representation of the collision
/// system's internal data structures, making it easier to debug collision
/// detection issues and understand spatial partitioning.
///
/// # Performance Considerations
///
/// - Debug rendering is disabled by default to avoid performance impact
/// - Vertex buffers are only created when debug rendering is enabled
/// - The renderer uses efficient GPU instancing for multiple debug elements
///
/// # Thread Safety
///
/// This struct is not thread-safe and should only be accessed from the
/// main rendering thread.
///
/// # Example
///
/// ```rust
/// # use crate::renderer::game_renderer::debug::DebugRenderer;
/// # let device: wgpu::Device = unimplemented!();
/// # let collision_system: CollisionSystem = unimplemented!();
///
/// let mut debug_renderer = DebugRenderer {
///     debug_render_bounding_boxes: true,
///     debug_vertex_buffer: None,
///     debug_vertex_count: 0,
/// };
///
/// // Update debug visualization
/// debug_renderer.update_debug_vertices(&device, &collision_system);
/// ```
pub struct DebugRenderer {
    /// Whether to render bounding boxes for debugging.
    ///
    /// When `true`, the debug renderer will generate and render bounding box
    /// vertices for all collision objects. When `false`, no debug rendering
    /// occurs, improving performance.
    ///
    /// # Default Value
    ///
    /// This field defaults to `false` to avoid performance impact in release builds.
    pub debug_render_bounding_boxes: bool,

    /// Vertex buffer for the debug renderer.
    ///
    /// Contains the vertex data for all debug elements (bounding boxes, etc.).
    /// This buffer is only created when debug rendering is enabled and vertices
    /// are available.
    ///
    /// # GPU Memory Management
    ///
    /// The buffer is automatically created and destroyed based on debug state
    /// to minimize GPU memory usage when debug rendering is disabled.
    pub debug_vertex_buffer: Option<wgpu::Buffer>,

    /// Number of vertices in the debug vertex buffer.
    ///
    /// This count is used during rendering to determine how many vertices
    /// to draw. It's updated whenever the debug vertices are regenerated.
    pub debug_vertex_count: usize,
}

impl DebugRenderer {
    /// Updates the debug vertex buffer with current collision system data.
    ///
    /// This method regenerates the vertex buffer containing all debug elements
    /// based on the current state of the collision system. It traverses the
    /// BVH tree and creates visualization vertices for wall faces and other
    /// collision objects.
    ///
    /// # Performance Notes
    ///
    /// - This operation involves GPU buffer creation/updates
    /// - Should be called when the collision system changes significantly
    /// - Automatically skips processing if debug rendering is disabled
    ///
    /// # GPU Resource Management
    ///
    /// The method creates a new vertex buffer each time it's called, allowing
    /// the old buffer to be garbage collected. This approach is suitable for
    /// debug rendering where performance is less critical than correctness.
    ///
    /// # Parameters
    ///
    /// - `device` - WGPU device for creating GPU resources
    /// - `collision_system` - Current collision system state to visualize
    ///
    /// # Example
    ///
    /// ```rust
    /// # use crate::renderer::game_renderer::debug::DebugRenderer;
    /// # let mut debug_renderer: DebugRenderer = unimplemented!();
    /// # let device: wgpu::Device = unimplemented!();
    /// # let collision_system: CollisionSystem = unimplemented!();
    ///
    /// // Update debug visualization after collision system changes
    /// debug_renderer.update_debug_vertices(&device, &collision_system);
    /// ```
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
/// This function creates a complete 3D box representation of an Axis-Aligned
/// Bounding Box using triangles. The box is rendered as a semitransparent
/// red volume to make it easily distinguishable from regular game objects.
///
/// # Visual Properties
///
/// - **Color**: Semitransparent red (`[255, 0, 0, 77]`)
/// - **Material**: Uses `BOUNDING_BOX_MATERIAL` for shader identification
/// - **Geometry**: Complete 3D box with all 6 faces (36 vertices total)
/// - **Transparency**: Alpha value of 77 (30% opacity) for overlay effect
///
/// # Vertex Layout
///
/// Each vertex contains:
/// - `position`: 3D coordinates in world space
/// - `color`: RGBA color values (8-bit per channel)
/// - `material`: Material ID for shader routing
/// - `tex_coords`: Texture coordinates (unused for debug, set to `[0.0, 0.0]`)
///
/// # Face Order
///
/// The function generates faces in this order:
/// 1. Front face (Z+)
/// 2. Back face (Z-)
/// 3. Right face (X+)
/// 4. Left face (X-)
/// 5. Top face (Y+)
/// 6. Bottom face (Y-)
///
/// # Parameters
///
/// * `aabb` - The Axis-Aligned Bounding Box to visualize
///
/// # Returns
///
/// A vector of 36 vertices defining the triangles of the bounding box
///
/// # Example
///
/// ```rust
/// # use crate::game::collision::AABB;
/// # use crate::renderer::game_renderer::debug::create_aabb_box_vertices;
/// # use crate::math::vec::Vec3;
///
/// let aabb = AABB {
///     min: Vec3::new(0.0, 0.0, 0.0),
///     max: Vec3::new(1.0, 1.0, 1.0),
/// };
///
/// let vertices = create_aabb_box_vertices(&aabb);
/// assert_eq!(vertices.len(), 36); // 6 faces × 6 vertices per face
/// ```
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
/// This function traverses the Bounding Volume Hierarchy (BVH) to find all
/// leaf nodes containing wall faces and generates visualization vertices
/// for their bounding boxes. It specifically focuses on wall faces rather
/// than the entire BVH structure to provide targeted debugging information.
///
/// # BVH Traversal Strategy
///
/// The function uses a depth-first traversal approach:
/// - **Internal nodes**: Recursively traverse left and right children
/// - **Leaf nodes**: Generate bounding box vertices for all contained faces
/// - **Skipping**: Internal BVH nodes are not visualized to reduce clutter
///
/// # Performance Characteristics
///
/// - **Time Complexity**: O(n) where n is the number of BVH nodes
/// - **Memory Usage**: Proportional to the number of wall faces
/// - **GPU Impact**: Creates vertices for all wall faces simultaneously
///
/// # Use Cases
///
/// - Debugging collision detection accuracy
/// - Visualizing spatial partitioning efficiency
/// - Identifying problematic collision areas
/// - Understanding maze geometry structure
///
/// # Parameters
///
/// * `bvh` - Reference to the collision system BVH
///
/// # Returns
///
/// A vector of vertices representing the wall face bounding boxes
///
/// # Example
///
/// ```rust
/// # use crate::game::collision::BVH;
/// # use crate::renderer::game_renderer::debug::collect_wall_face_debug_vertices;
///
/// let bvh: BVH = /* collision system BVH */;
/// let debug_vertices = collect_wall_face_debug_vertices(&bvh);
///
/// // Use vertices for debug rendering
/// if !debug_vertices.is_empty() {
///     // Create vertex buffer and render...
/// }
/// ```
pub fn collect_wall_face_debug_vertices(bvh: &BVH) -> Vec<Vertex> {
    let mut vertices = Vec::new();

    if let Some(root) = &bvh.root {
        collect_wall_faces_recursive(root, &mut vertices);
    }

    vertices
}

/// Helper function to recursively collect wall face AABBs from BVH nodes.
///
/// This internal function performs the actual traversal of the BVH tree
/// structure. It recursively visits all nodes and generates bounding box
/// vertices only for leaf nodes containing wall faces.
///
/// # Recursion Strategy
///
/// - **Base Case**: Leaf nodes - generate vertices for all contained faces
/// - **Recursive Case**: Internal nodes - traverse both children
/// - **Termination**: Automatically terminates when all nodes are visited
///
/// # Memory Management
///
/// The function accumulates vertices in the provided mutable vector to
/// avoid excessive memory allocations during traversal.
///
/// # Parameters
///
/// * `node` - Current BVH node to process
/// * `vertices` - Mutable reference to accumulate vertices
///
/// # Implementation Details
///
/// The function pattern matches on the BVH node type:
/// - `BVHNode::Internal`: Contains left and right child nodes
/// - `BVHNode::Leaf`: Contains actual collision faces
///
/// # Example
///
/// ```rust
/// # use crate::game::collision::BVHNode;
/// # use crate::renderer::game_renderer::debug::collect_wall_faces_recursive;
///
/// let mut vertices = Vec::new();
/// let node: &BVHNode = /* BVH node */;
/// collect_wall_faces_recursive(node, &mut vertices);
/// ```
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
