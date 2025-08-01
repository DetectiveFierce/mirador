//! Spatial partitioning and collision detection system for 3D environments.
//!
//! # Overview
//!
//! This module implements a complete collision detection and resolution system for 3D environments
//! based on the Bounding Volume Hierarchy (BVH) algorithm. It provides efficient collision detection
//! between a player and maze walls, with sophisticated collision response including wall sliding.
//!
//! # Core Components
//!
//! * [`AABB`] - Axis-Aligned Bounding Box, the fundamental collision primitive
//! * [`WallFace`] - Representation of a wall surface with position and orientation
//! * [`BVHNode`] - Tree node in the bounding volume hierarchy
//! * [`BVH`] - Complete hierarchy for spatial partitioning and efficient collision queries
//! * [`CollisionSystem`] - High-level system that manages collision detection and resolution
//!
//! # How the System Works
//!
//! 1. Walls are extracted from the maze and converted to [`WallFace`] objects
//! 2. A [`BVH`] tree is constructed to spatially organize these faces
//! 3. When the player moves, potential collisions are efficiently queried using the BVH
//! 4. Collisions are resolved with physically realistic using vector projection (wall sliding)
//!
//! # Performance Considerations
//!
//! The BVH structure provides O(log n) collision detection in the average case, compared to
//! O(n) for naive approaches, making it suitable for environments with many collision objects.

use crate::game::GameAudioManager;
use crate::game::Player;
/// Axis-Aligned Bounding Box (AABB) for efficient collision detection.
///
/// An AABB is a rectangular box whose faces are aligned with the world coordinate axes.
/// It is defined by two points: the minimum and maximum corners. AABBs are used as simple
/// approximations of more complex geometry for fast overlap tests.
///
/// # Purpose
///
/// AABBs are used at multiple levels in the collision system:
/// - To approximate the bounds of wall faces
/// - To represent the player's collision volume
/// - As bounding volumes in the BVH tree nodes
///
/// # Performance Benefits
///
/// AABB-AABB intersection tests are extremely fast (just 6 comparisons), which
/// makes them ideal for the first phase of collision detection. Only when AABBs
/// overlap do we need to perform more expensive exact collision tests.
#[derive(Debug, Clone)]
pub struct AABB {
    /// The minimum corner of the bounding box as [x, y, z] coordinates.
    ///
    /// This represents the point with the smallest x, y, and z values
    /// that define the lower-left-back corner of the axis-aligned box.
    pub min: [f32; 3],
    /// The maximum corner of the bounding box as [x, y, z] coordinates.
    ///
    /// This represents the point with the largest x, y, and z values
    /// that define the upper-right-front corner of the axis-aligned box.
    pub max: [f32; 3],
}

impl AABB {
    /// Creates a new AABB from minimum and maximum corner points.
    ///
    /// # Arguments
    ///
    /// * `min` - An array of 3 f32 values [x, y, z] representing the minimum corner
    /// * `max` - An array of 3 f32 values [x, y, z] representing the maximum corner
    ///
    /// # Example
    /// ```
    /// let bbox = AABB::new([0.0, 0.0, 0.0], [1.0, 2.0, 3.0]);
    /// ```
    pub fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }

    /// Creates an AABB from a wall face defined by its four corners.
    ///
    /// This function calculates the minimum and maximum coordinates from the given points
    /// to create a box that fully contains all four corners of the wall face.
    ///
    /// # Arguments
    ///
    /// * `p1`, `p2`, `p3`, `p4` - Four corner points of the wall face
    ///
    /// # How It Works
    ///
    /// For each dimension (x, y, z), the function:
    /// 1. Finds the minimum value among all four points
    /// 2. Finds the maximum value among all four points
    /// 3. Uses these min/max values to define the AABB
    pub fn from_wall_face(p1: [f32; 3], p2: [f32; 3], p3: [f32; 3], p4: [f32; 3]) -> Self {
        let min_x = p1[0].min(p2[0]).min(p3[0]).min(p4[0]);
        let min_y = p1[1].min(p2[1]).min(p3[1]).min(p4[1]);
        let min_z = p1[2].min(p2[2]).min(p3[2]).min(p4[2]);

        let max_x = p1[0].max(p2[0]).max(p3[0]).max(p4[0]);
        let max_y = p1[1].max(p2[1]).max(p3[1]).max(p4[1]);
        let max_z = p1[2].max(p2[2]).max(p3[2]).max(p4[2]);

        Self::new([min_x, min_y, min_z], [max_x, max_y, max_z])
    }

    /// Expands this AABB to fully contain another AABB.
    ///
    /// This is used when building the BVH to create parent nodes that fully
    /// contain their children.
    ///
    /// # Arguments
    ///
    /// * `other` - Another AABB to be included in this one
    ///
    /// # Effect
    ///
    /// The current AABB is modified in-place to become the minimum bounding box
    /// that contains both the original AABB and the provided one.
    pub fn expand(&mut self, other: &AABB) {
        for i in 0..3 {
            self.min[i] = self.min[i].min(other.min[i]);
            self.max[i] = self.max[i].max(other.max[i]);
        }
    }

    /// Checks if this AABB intersects with another AABB.
    ///
    /// Two AABBs intersect if they overlap on all three axes (x, y, z).
    ///
    /// # Arguments
    ///
    /// * `other` - The AABB to test intersection against
    ///
    /// # Returns
    ///
    /// `true` if the AABBs intersect, `false` otherwise
    ///
    /// # Algorithm
    ///
    /// The function uses the "separating axis theorem" to determine if there's
    /// any axis where the boxes don't overlap. If even one such axis exists,
    /// the boxes cannot intersect.
    pub fn intersects(&self, other: &AABB) -> bool {
        for i in 0..3 {
            if self.max[i] < other.min[i] || self.min[i] > other.max[i] {
                return false;
            }
        }
        true
    }

    /// Calculates the center point of the AABB.
    ///
    /// This is used for sorting and partitioning during BVH construction,
    /// and for distance calculations during collision resolution.
    ///
    /// # Returns
    ///
    /// An array [x, y, z] representing the center point
    ///
    /// # Formula
    ///
    /// For each axis, the center is calculated as: `(min + max) * 0.5`
    pub fn center(&self) -> [f32; 3] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
            (self.min[2] + self.max[2]) * 0.5,
        ]
    }

    /// Calculates the surface area of the AABB.
    ///
    /// The surface area is used during BVH construction for optimizing the tree
    /// using the Surface Area Heuristic (SAH), which aims to minimize the total
    /// cost of traversing the tree during collision queries.
    ///
    /// # Returns
    ///
    /// The surface area as a floating point value
    ///
    /// # Formula
    ///
    /// SA = 2 * (width * height + height * depth + depth * width)
    pub fn surface_area(&self) -> f32 {
        let dx = self.max[0] - self.min[0];
        let dy = self.max[1] - self.min[1];
        let dz = self.max[2] - self.min[2];
        2.0 * (dx * dy + dy * dz + dz * dx)
    }
}

/// Represents a single wall face for collision detection.
///
/// A wall face is a quadrilateral defined by four corner points in 3D space.
/// It includes orientation information (normal vector) and a simplified
/// bounding box for quick rejection tests.
///
/// # Key Properties
///
/// * `corners` - The four corner points defining the face
/// * `normal` - The unit vector perpendicular to the face
/// * `aabb` - An axis-aligned bounding box enclosing the face
///
/// # Collision Detection
///
/// Wall faces are the primary collision primitives in the maze environment.
/// Player movement is restricted by these faces, and collision resolution
/// uses the face normal to determine sliding behavior.
#[derive(Debug, Clone)]
pub struct WallFace {
    /// The four corners of the wall face.
    pub corners: [[f32; 3]; 4],
    /// The face normal vector.
    pub normal: [f32; 3],
    /// Bounding box for this face.
    pub aabb: AABB,
}

impl WallFace {
    /// Creates a new wall face from four corners.
    ///
    /// The corners should be specified in a consistent winding order
    /// (either clockwise or counter-clockwise), as this determines
    /// the direction of the calculated normal vector.
    ///
    /// # Arguments
    ///
    /// * `corners` - Four corner points defining the wall face
    ///
    /// # Returns
    ///
    /// A new `WallFace` instance with calculated normal and AABB
    pub fn new(corners: [[f32; 3]; 4]) -> Self {
        let normal = Self::calculate_normal(&corners);
        let aabb = AABB::from_wall_face(corners[0], corners[1], corners[2], corners[3]);

        Self {
            corners,
            normal,
            aabb,
        }
    }

    /// Calculates the normal vector for the wall face.
    ///
    /// The normal vector is perpendicular to the plane of the wall face
    /// and is used to determine which side of the wall the player is on,
    /// as well as for calculating sliding motion during collision resolution.
    ///
    /// # Arguments
    ///
    /// * `corners` - Four corner points defining the wall face
    ///
    /// # Returns
    ///
    /// A unit-length normal vector as [x, y, z]
    ///
    /// # Algorithm
    ///
    /// 1. Calculate two edge vectors `u` and `v` from the first three corners
    /// 2. Compute the cross product `v × u` to get the normal vector
    /// 3. Normalize the result to unit length
    ///
    /// Note: The cross product order (v × u vs u × v) determines which way
    /// the normal points. The current implementation makes the normal point
    /// inward from the wall face.
    fn calculate_normal(corners: &[[f32; 3]; 4]) -> [f32; 3] {
        let u = [
            corners[1][0] - corners[0][0],
            corners[1][1] - corners[0][1],
            corners[1][2] - corners[0][2],
        ];
        let v = [
            corners[2][0] - corners[0][0],
            corners[2][1] - corners[0][1],
            corners[2][2] - corners[0][2],
        ];

        // Cross product v × u (note the order flipped to reverse the normal)
        let normal = [
            v[1] * u[2] - v[2] * u[1],
            v[2] * u[0] - v[0] * u[2],
            v[0] * u[1] - v[1] * u[0],
        ];

        // Normalize the result
        let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        [normal[0] / len, normal[1] / len, normal[2] / len]
    }
}

/// A node in the Bounding Volume Hierarchy tree structure.
///
/// # What is a BVH?
///
/// A Bounding Volume Hierarchy (BVH) is a tree structure used to organize
/// spatial data for efficient querying. Each node in the tree has an AABB
/// that contains all geometry in its subtree.
///
/// # Node Types
///
/// The BVH uses a recursive enum pattern with two variants:
///
/// * `Leaf` - A terminal node containing actual wall faces
/// * `Internal` - A branch node with two children but no faces
///
/// # Performance Characteristics
///
/// BVH traversal allows for quickly discarding large portions of the scene
/// that cannot possibly intersect with the query volume, reducing the
/// number of individual face tests required.
#[derive(Debug, Clone)]
pub enum BVHNode {
    /// Leaf node containing wall faces.
    ///
    /// Fields:
    /// - `aabb`: The bounding box that contains all faces in this leaf.
    /// - `faces`: The wall faces contained in this leaf node.
    Leaf {
        /// The bounding box that contains all faces in this leaf.
        aabb: AABB,
        /// The wall faces contained in this leaf node.
        faces: Vec<WallFace>,
    },
    /// Internal node with two children.
    ///
    /// Fields:
    /// - `aabb`: The bounding box that contains both children.
    /// - `left`: The left child node.
    /// - `right`: The right child node.
    Internal {
        /// The bounding box that contains both children.
        aabb: AABB,
        /// The left child node.
        left: Box<BVHNode>,
        /// The right child node.
        right: Box<BVHNode>,
    },
}

impl BVHNode {
    /// Gets a reference to the AABB of this node.
    ///
    /// This method provides uniform access to the node's bounding box
    /// regardless of whether it's a Leaf or Internal node.
    ///
    /// # Returns
    ///
    /// A reference to the node's AABB
    pub fn aabb(&self) -> &AABB {
        match self {
            BVHNode::Leaf { aabb, .. } => aabb,
            BVHNode::Internal { aabb, .. } => aabb,
        }
    }
}

/// Bounding Volume Hierarchy for efficient spatial partitioning and collision detection.
///
/// # What is a BVH?
///
/// A Bounding Volume Hierarchy (BVH) is a tree data structure that partitions
/// spatial data (like wall faces) into a hierarchical structure for efficient
/// queries. It's analogous to a binary search tree, but for 3D space.
///
/// # Benefits
///
/// - **Performance**: Reduces collision checks from O(n) to O(log n) on average
/// - **Flexibility**: Works with arbitrary geometry in 3D space
/// - **Precision**: Allows for exact collision detection by narrowing down candidates
///
/// # How It Works
///
/// 1. Start with all wall faces in the scene
/// 2. Recursively split them into two roughly equal groups based on spatial position
/// 3. Create a hierarchical tree where each node contains an AABB
/// 4. For collision queries, traverse only branches whose AABBs intersect the query volume
#[derive(Debug, Default, Clone)]
pub struct BVH {
    /// The root node of the BVH tree.
    ///
    /// If `None`, the BVH is empty and contains no wall faces.
    /// If `Some`, contains the root node of the BVH tree structure.
    pub root: Option<BVHNode>,
}

impl BVH {
    /// Creates a new empty Bounding Volume Hierarchy.
    ///
    /// The BVH starts with no root node and must be built
    /// using the [`build`](#method.build) method before use.
    ///
    /// # Returns
    ///
    /// A new empty `BVH` instance
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Builds the BVH from a collection of wall faces.
    ///
    /// This initializes or rebuilds the BVH with the provided wall faces.
    /// The building process recursively splits the faces into a balanced tree
    /// structure optimized for spatial queries.
    ///
    /// # Arguments
    ///
    /// * `faces` - A vector of wall faces to organize in the BVH
    ///
    /// # Process
    ///
    /// 1. If no faces are provided, the BVH is set to empty (None)
    /// 2. Otherwise, the recursive building process is initiated
    pub fn build(&mut self, faces: Vec<WallFace>) {
        // Use benchmark macro for BVH build timing
        crate::benchmark!("bvh_build", {
            if faces.is_empty() {
                self.root = None;
                return;
            }

            self.root = Some(Self::build_recursive(faces));
        });
    }

    /// Recursively builds the BVH tree structure.
    ///
    /// This is the core algorithm for BVH construction. It splits faces into
    /// two groups based on their spatial distribution, and recursively builds
    /// a tree until reaching small enough leaf nodes.
    ///
    /// # Arguments
    ///
    /// * `faces` - Wall faces to organize in this subtree
    ///
    /// # Returns
    ///
    /// A new `BVHNode` representing the root of this subtree
    ///
    /// # Algorithm
    ///
    /// 1. If the number of faces is small (≤4), create a leaf node
    /// 2. Otherwise:
    ///    - Find the best axis to split on
    ///    - Sort faces along that axis
    ///    - Split faces into two roughly equal groups
    ///    - Recursively build left and right subtrees
    ///    - Create an internal node with the combined AABB
    ///
    /// # Termination
    ///
    /// The recursion terminates when a node contains 4 or fewer faces,
    /// balancing tree depth against the cost of brute-force checking.
    fn build_recursive(mut faces: Vec<WallFace>) -> BVHNode {
        if faces.len() <= 4 {
            // Create leaf node
            let mut aabb = faces[0].aabb.clone();
            for face in faces.iter().skip(1) {
                aabb.expand(&face.aabb);
            }
            return BVHNode::Leaf { aabb, faces };
        }

        // Find the best split axis and position
        let (split_axis, _split_pos) = Self::find_best_split(&faces);

        // Sort faces by their center position along the split axis
        faces.sort_by(|a, b| {
            let a_center = a.aabb.center()[split_axis];
            let b_center = b.aabb.center()[split_axis];
            a_center
                .partial_cmp(&b_center)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Split into two groups
        let mid = faces.len() / 2;
        let left_faces = faces.drain(..mid).collect();
        let right_faces = faces;

        // Recursively build children
        let left_child = Self::build_recursive(left_faces);
        let right_child = Self::build_recursive(right_faces);

        // Create parent AABB
        let mut aabb = left_child.aabb().clone();
        aabb.expand(right_child.aabb());

        BVHNode::Internal {
            aabb,
            left: Box::new(left_child),
            right: Box::new(right_child),
        }
    }

    /// Finds the best axis and position to split the faces.
    ///
    /// This function implements a heuristic approach to determining how
    /// to split the faces for optimal BVH performance. It evaluates
    /// potential splits along all three axes and chooses the most balanced one.
    ///
    /// # Arguments
    ///
    /// * `faces` - The wall faces to analyze for splitting
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// - The axis index to split on (0=X, 1=Y, 2=Z)
    /// - The position along that axis (though median split is used instead)
    ///
    /// # Optimization Goal
    ///
    /// The objective is to create a balanced tree that minimizes traversal
    /// cost during collision queries. A balanced split means each child node
    /// contains roughly the same number of faces.
    ///
    /// # Performance Note
    ///
    /// While this implementation tries several split positions per axis,
    /// it ultimately uses a simple median split. More advanced BVH implementations
    /// might use the Surface Area Heuristic (SAH) for even better performance.
    fn find_best_split(faces: &[WallFace]) -> (usize, f32) {
        let mut best_axis = 0;
        let mut best_cost = f32::INFINITY;

        // Try each axis
        for axis in 0..3 {
            // Find the range of centers along this axis
            let mut min_center = f32::INFINITY;
            let mut max_center = f32::NEG_INFINITY;

            for face in faces {
                let center = face.aabb.center()[axis];
                min_center = min_center.min(center);
                max_center = max_center.max(center);
            }

            // Try several split positions
            for i in 1..8 {
                let t = i as f32 / 8.0;
                let split_pos = min_center + t * (max_center - min_center);

                // Calculate cost of this split
                let cost = Self::evaluate_split_cost(faces, axis, split_pos);
                if cost < best_cost {
                    best_cost = cost;
                    best_axis = axis;
                }
            }
        }

        (best_axis, 0.0) // We'll use median split instead of optimal position
    }

    /// Evaluates the cost of a potential split.
    ///
    /// This function determines how well a particular split position
    /// balances the distribution of faces between the two child nodes.
    ///
    /// # Arguments
    ///
    /// * `faces` - The wall faces being split
    /// * `axis` - The axis along which to evaluate the split (0=X, 1=Y, 2=Z)
    /// * `split_pos` - The position along the axis where the split would occur
    ///
    /// # Returns
    ///
    /// A cost value where lower is better. The current implementation
    /// returns the absolute difference between the number of faces on each side,
    /// favoring even distributions.
    ///
    /// # Use in BVH Construction
    ///
    /// The split with the lowest cost is selected when building
    /// the BVH, leading to a more balanced tree.
    fn evaluate_split_cost(faces: &[WallFace], axis: usize, split_pos: f32) -> f32 {
        let mut left_count = 0;
        let mut right_count = 0;

        for face in faces {
            let center = face.aabb.center()[axis];
            if center < split_pos {
                left_count += 1;
            } else {
                right_count += 1;
            }
        }

        // Prefer balanced splits
        ((left_count as f32) - (right_count as f32)).abs()
    }

    /// Queries the BVH for potential collisions with a player AABB.
    ///
    /// This is the main access point for collision detection. It traverses
    /// the BVH tree to find all wall faces whose AABBs intersect the player's AABB.
    ///
    /// # Arguments
    ///
    /// * `player_aabb` - The player's axis-aligned bounding box
    ///
    /// # Returns
    ///
    /// A vector of references to wall faces that potentially collide with the player
    ///
    /// # Performance
    ///
    /// This query is much more efficient than checking against every wall face
    /// in the scene. For a balanced BVH, the average complexity is O(log n + k)
    /// where n is the total number of faces and k is the number of potentially
    /// colliding faces.
    pub fn query_collisions(&self, player_aabb: &AABB) -> Vec<&WallFace> {
        // Use benchmark macro for BVH query timing
        crate::benchmark!("bvh_query_collisions", {
            let mut results = Vec::new();
            if let Some(ref root) = self.root {
                Self::query_recursive(root, player_aabb, &mut results);
            }
            results
        })
    }

    /// Recursively queries the BVH tree for potential collisions.
    ///
    /// This method traverses the BVH tree, pruning branches that cannot
    /// possibly contain collisions, and collecting wall faces from
    /// leaf nodes that might intersect with the query volume.
    ///
    /// # Arguments
    ///
    /// * `node` - The current BVH node being examined
    /// * `query_aabb` - The AABB to test against (typically the player's bounds)
    /// * `results` - A mutable vector to collect potentially colliding wall faces
    ///
    /// # Algorithm
    ///
    /// 1. If the node's AABB doesn't intersect the query AABB, return immediately
    /// 2. If this is a leaf node, test each face's AABB against the query AABB
    /// 3. If this is an internal node, recursively visit both children
    ///
    /// # Efficiency
    ///
    /// This algorithm avoids checking branches of the tree that cannot
    /// contain collisions, significantly reducing the number of tests needed
    /// compared to a brute-force approach.
    fn query_recursive<'a>(node: &'a BVHNode, query_aabb: &AABB, results: &mut Vec<&'a WallFace>) {
        if !node.aabb().intersects(query_aabb) {
            return;
        }

        match node {
            BVHNode::Leaf { faces, .. } => {
                for face in faces {
                    if face.aabb.intersects(query_aabb) {
                        results.push(face);
                    }
                }
            }
            BVHNode::Internal { left, right, .. } => {
                Self::query_recursive(left, query_aabb, results);
                Self::query_recursive(right, query_aabb, results);
            }
        }
    }
}

/// High-level collision detection and response system.
///
/// The `CollisionSystem` ties together all collision components into a complete
/// system that can detect and respond to collisions between a player and the
/// environment. It maintains the spatial acceleration structure (BVH) and
/// handles the physics of collision response.
///
/// # Core Responsibilities
///
/// * Building and maintaining the BVH from maze geometry
/// * Detecting potential collisions using the BVH
/// * Resolving collisions with physically-based responses
/// * Supporting wall sliding for smooth player movement
///
/// # Fields
///
/// - `bvh`: The bounding volume hierarchy used for fast spatial queries of wall faces.
/// - `player_radius`: The radius of the player's cylindrical collision shape (in world units).
/// - `player_height`: The height of the player's cylindrical collision shape (in world units).
/// - `maze_dimensions`: The dimensions of the maze as a tuple (width, height), in grid cells.
/// # How To Use
///
/// 1. Create a `CollisionSystem` with appropriate player dimensions
/// 2. Build the BVH from maze geometry using `build_from_maze()`
/// 3. For each player movement, call `check_and_resolve_collision()`
#[derive(Debug, Default, Clone)]

pub struct CollisionSystem {
    /// The bounding volume hierarchy for spatial partitioning of wall faces.
    pub bvh: BVH,
    /// The radius of the player's collision cylinder.
    pub player_radius: f32,
    /// The height of the player's collision cylinder.
    pub player_height: f32,
    /// The dimensions of the maze grid (width, height).
    pub maze_dimensions: (usize, usize),
}

impl CollisionSystem {
    /// Creates a new collision system with specified player dimensions.
    ///
    /// # Arguments
    ///
    /// * `player_radius` - The radius of the player's cylindrical collision shape
    /// * `player_height` - The height of the player's cylindrical collision shape
    ///
    /// # Returns
    ///
    /// A new `CollisionSystem` instance ready to be initialized with maze geometry
    ///
    /// # Note
    ///
    /// The collision system treats the player as a vertical cylinder with the
    /// specified radius and height. This approximation provides a good balance
    /// between accuracy and computational efficiency.
    pub fn new(player_radius: f32, player_height: f32) -> Self {
        Self {
            bvh: BVH::new(),
            player_radius,
            player_height,
            maze_dimensions: (0, 0),
        }
    }

    /// Builds the collision BVH from maze geometry.
    ///
    /// This method processes a 2D maze grid and constructs a 3D collision
    /// environment with walls of appropriate height. It extracts wall faces
    /// from the maze structure and organizes them in a BVH.
    ///
    /// # Arguments
    ///
    /// * `maze_grid` - A 2D grid where `true` represents walls and `false` represents open spaces
    /// * `is_test_mode` - Whether test mode is enabled (affects collision geometry)
    ///
    /// # Process
    ///
    /// 1. Extract wall faces from the maze grid
    /// 2. Build the BVH from these faces
    ///
    /// # Usage Example
    ///
    /// ```
    /// let mut collision_system = CollisionSystem::new(0.4, 1.8);
    /// collision_system.build_from_maze(&maze.grid, false);
    /// ```
    pub fn build_from_maze(&mut self, maze_grid: &[Vec<bool>], is_test_mode: bool) {
        // Use benchmark macro for collision system build timing
        crate::benchmark!("collision_system_build", {
            // Store maze dimensions
            self.maze_dimensions = (maze_grid[0].len(), maze_grid.len());
            let wall_faces = self.extract_wall_faces_from_maze(maze_grid, is_test_mode);
            self.bvh.build(wall_faces);
        });
    }

    /// Extracts wall faces from the maze grid for collision detection.
    ///
    /// This method converts a 2D maze representation into 3D wall faces with
    /// proper orientation. It's analogous to mesh generation, but specifically
    /// for collision detection rather than rendering.
    ///
    /// # Arguments
    ///
    /// * `maze_grid` - A 2D grid where `true` represents walls and `false` represents open spaces
    /// * `is_test_mode` - Whether test mode is enabled (affects collision geometry)
    ///
    /// # Returns
    ///
    /// A vector of `WallFace` objects representing all the collidable walls in the maze
    ///
    /// # Algorithm
    ///
    /// 1. Calculate cell size to scale the maze to match the floor
    /// 2. For each wall cell in the grid:
    ///    - Create wall faces for each exposed side (N, S, E, W)
    ///    - Generate both front and back faces for proper collision from both sides
    ///    - Set appropriate orientation (normal) for each face
    ///
    /// # Optimization
    ///
    /// The method only creates wall faces where there's a transition between
    /// wall and non-wall cells, avoiding redundant interior faces.
    fn extract_wall_faces_from_maze(
        &self,
        maze_grid: &[Vec<bool>],
        is_test_mode: bool,
    ) -> Vec<WallFace> {
        let mut faces = Vec::new();
        let maze_width = maze_grid[0].len();
        let maze_height = maze_grid.len();
        let maze_dimensions = (maze_width, maze_height);

        // Use coordinate API to calculate sizes and origins
        let cell_size =
            crate::math::coordinates::calculate_cell_size(maze_dimensions, is_test_mode);
        let wall_height = cell_size;

        // Calculate the world origin offset (bottom-left corner of the maze)
        let origin_x = -(maze_width as f32 * cell_size) / 2.0;
        let origin_z = -(maze_height as f32 * cell_size) / 2.0;

        if is_test_mode {
            // Test mode: only create collision faces for perimeter walls
            // Top wall (row 0)
            for x in 0..maze_width {
                if maze_grid[0][x] {
                    let wx = origin_x + x as f32 * cell_size;
                    let wz = origin_z + 0.0 * cell_size;
                    faces.push(self.create_z_facing_wall_face(
                        wx,
                        wz,
                        cell_size,
                        wall_height,
                        false,
                    ));
                    faces.push(self.create_z_facing_wall_face(
                        wx,
                        wz,
                        cell_size,
                        wall_height,
                        true,
                    ));
                }
            }

            // Bottom wall (row maze_height-1)
            for x in 0..maze_width {
                if maze_grid[maze_height - 1][x] {
                    let wx = origin_x + x as f32 * cell_size;
                    let wz = origin_z + (maze_height - 1) as f32 * cell_size;
                    faces.push(self.create_z_facing_wall_face(
                        wx,
                        wz + cell_size,
                        cell_size,
                        wall_height,
                        false,
                    ));
                    faces.push(self.create_z_facing_wall_face(
                        wx,
                        wz + cell_size,
                        cell_size,
                        wall_height,
                        true,
                    ));
                }
            }

            // Left wall (column 0)
            for z in 0..maze_height {
                if maze_grid[z][0] {
                    let wx = origin_x + 0.0 * cell_size;
                    let wz = origin_z + z as f32 * cell_size;
                    faces.push(self.create_x_facing_wall_face(
                        wx,
                        wz,
                        cell_size,
                        wall_height,
                        false,
                    ));
                    faces.push(self.create_x_facing_wall_face(
                        wx,
                        wz,
                        cell_size,
                        wall_height,
                        true,
                    ));
                }
            }

            // Right wall (column maze_width-1)
            for z in 0..maze_height {
                if maze_grid[z][maze_width - 1] {
                    let wx = origin_x + (maze_width - 1) as f32 * cell_size;
                    let wz = origin_z + z as f32 * cell_size;
                    faces.push(self.create_x_facing_wall_face(
                        wx + cell_size,
                        wz,
                        cell_size,
                        wall_height,
                        false,
                    ));
                    faces.push(self.create_x_facing_wall_face(
                        wx + cell_size,
                        wz,
                        cell_size,
                        wall_height,
                        true,
                    ));
                }
            }
        } else {
            // Normal mode: create collision faces for all walls
            for (z, row) in maze_grid.iter().enumerate() {
                for (x, &is_wall) in row.iter().enumerate() {
                    if is_wall {
                        let wx = origin_x + x as f32 * cell_size;
                        let wz = origin_z + z as f32 * cell_size;

                        // Create wall faces for each direction
                        if z == 0 || !maze_grid[z - 1][x] {
                            faces.push(self.create_z_facing_wall_face(
                                wx,
                                wz,
                                cell_size,
                                wall_height,
                                false,
                            ));
                            faces.push(self.create_z_facing_wall_face(
                                wx,
                                wz,
                                cell_size,
                                wall_height,
                                true,
                            ));
                        }

                        // X-facing walls (both front and back)
                        if x == 0 || !maze_grid[z][x - 1] {
                            faces.push(self.create_x_facing_wall_face(
                                wx,
                                wz,
                                cell_size,
                                wall_height,
                                false,
                            ));
                            faces.push(self.create_x_facing_wall_face(
                                wx,
                                wz,
                                cell_size,
                                wall_height,
                                true,
                            ));
                        }
                        if z == maze_height - 1 {
                            faces.push(self.create_z_facing_wall_face(
                                wx,
                                wz + cell_size,
                                cell_size,
                                wall_height,
                                false,
                            ));
                            faces.push(self.create_z_facing_wall_face(
                                wx,
                                wz + cell_size,
                                cell_size,
                                wall_height,
                                true,
                            ));
                        }
                        if x == maze_width - 1 {
                            faces.push(self.create_x_facing_wall_face(
                                wx + cell_size,
                                wz,
                                cell_size,
                                wall_height,
                                false,
                            ));
                            faces.push(self.create_x_facing_wall_face(
                                wx + cell_size,
                                wz,
                                cell_size,
                                wall_height,
                                true,
                            ));
                        }
                    }
                }
            }
        }

        faces
    }

    /// Creates a Z-facing wall face (perpendicular to Z axis).
    ///
    /// This helper method generates a wall face that is perpendicular to the Z axis,
    /// with the ability to flip its normal vector to face either direction.
    ///
    /// # Arguments
    ///
    /// * `x`, `z` - The base position of the wall in the XZ plane
    /// * `size` - The width of the wall along the X axis
    /// * `height` - The height of the wall along the Y axis
    /// * `reverse_normal` - Whether to reverse the normal vector direction
    ///
    /// # Returns
    ///
    /// A `WallFace` object oriented perpendicular to the Z axis
    ///
    /// # Normal Direction
    ///
    /// * When `reverse_normal` is `false`, the normal points in the negative Z direction
    /// * When `reverse_normal` is `true`, the normal points in the positive Z direction
    fn create_z_facing_wall_face(
        &self,
        x: f32,
        z: f32,
        size: f32,
        height: f32,
        reverse_normal: bool,
    ) -> WallFace {
        let corners = if reverse_normal {
            [
                [x, 0.0, z],           // Bottom-left
                [x + size, 0.0, z],    // Bottom-right
                [x + size, height, z], // Top-right
                [x, height, z],        // Top-left
            ]
        } else {
            [
                [x, height, z],        // Top-left
                [x + size, height, z], // Top-right
                [x + size, 0.0, z],    // Bottom-right
                [x, 0.0, z],           // Bottom-left
            ]
        };

        WallFace::new(corners)
    }

    /// Creates an X-facing wall face (perpendicular to X axis).
    ///
    /// This helper method generates a wall face that is perpendicular to the X axis,
    /// with the ability to flip its normal vector to face either direction.
    ///
    /// # Arguments
    ///
    /// * `x`, `z` - The base position of the wall in the XZ plane
    /// * `size` - The depth of the wall along the Z axis
    /// * `height` - The height of the wall along the Y axis
    /// * `reverse_normal` - Whether to reverse the normal vector direction
    ///
    /// # Returns
    ///
    /// A `WallFace` object oriented perpendicular to the X axis
    ///
    /// # Normal Direction
    ///
    /// * When `reverse_normal` is `false`, the normal points in the negative X direction
    /// * When `reverse_normal` is `true`, the normal points in the positive X direction
    fn create_x_facing_wall_face(
        &self,
        x: f32,
        z: f32,
        size: f32,
        height: f32,
        reverse_normal: bool,
    ) -> WallFace {
        let corners = if reverse_normal {
            [
                [x, 0.0, z],           // Bottom-near
                [x, 0.0, z + size],    // Bottom-far
                [x, height, z + size], // Top-far
                [x, height, z],        // Top-near
            ]
        } else {
            [
                [x, height, z],        // Top-near
                [x, height, z + size], // Top-far
                [x, 0.0, z + size],    // Bottom-far
                [x, 0.0, z],           // Bottom-near
            ]
        };

        WallFace::new(corners)
    }

    /// Checks for collisions and resolves them with realistic wall sliding.
    ///
    /// This is the main method for collision detection and resolution. It determines
    /// if a proposed movement would cause a collision, and if so, modifies the movement
    /// to slide along walls instead of penetrating them.
    ///
    /// # Arguments
    ///
    /// * `current_pos` - The player's current position as [x, y, z]
    /// * `desired_pos` - The position the player wants to move to
    ///
    /// # Returns
    ///
    /// The final position after collision resolution
    ///
    /// # Algorithm
    ///
    /// 1. Create an AABB for the player at the desired position
    /// 2. Query the BVH to find potential wall collisions
    /// 3. If no collisions are detected, allow the movement
    /// 4. Otherwise, resolve each collision sequentially
    ///    - Calculate the movement vector
    ///    - Project the movement to slide along the wall
    ///    - Update the position for the next collision check
    ///
    /// # Physical Realism
    ///
    /// The sliding behavior ensures that players can smoothly move along walls
    /// rather than being abruptly stopped. This creates more natural movement
    /// through confined spaces.
    pub fn check_and_resolve_collision(
        &self,
        audio_manager: &mut GameAudioManager,
        current_pos: [f32; 3],
        desired_pos: [f32; 3],
    ) -> [f32; 3] {
        // Use benchmark macro for collision detection timing
        crate::benchmark!("collision_detection_and_resolution", {
            // Create player AABB
            let player_aabb = AABB::new(
                [
                    desired_pos[0] - self.player_radius,
                    desired_pos[1],
                    desired_pos[2] - self.player_radius,
                ],
                [
                    desired_pos[0] + self.player_radius,
                    desired_pos[1] + self.player_height,
                    desired_pos[2] + self.player_radius,
                ],
            );

            // Query BVH for potential collisions
            let potential_collisions = self.bvh.query_collisions(&player_aabb);

            if potential_collisions.is_empty() {
                return desired_pos;
            }

            // NEW: Check if player is stuck between opposing faces
            if self.is_stuck_between_faces(&potential_collisions, current_pos) {
                // Return to a safe position
                return self.find_safe_position(current_pos);
            }

            // Perform collision resolution with wall sliding
            let mut resolved_pos = desired_pos;
            let max_iterations = 5;

            for _ in 0..max_iterations {
                // Create player AABB at current resolved position
                let player_aabb = AABB::new(
                    [
                        resolved_pos[0] - self.player_radius,
                        resolved_pos[1],
                        resolved_pos[2] - self.player_radius,
                    ],
                    [
                        resolved_pos[0] + self.player_radius,
                        resolved_pos[1] + self.player_height,
                        resolved_pos[2] + self.player_radius,
                    ],
                );

                // Check for collisions at this position
                let potential_collisions = self.bvh.query_collisions(&player_aabb);
                if potential_collisions.is_empty() {
                    break; // No collisions, we're done
                }

                // Resolve the closest collision first
                // (This requires adding a distance calculation)
                let mut closest_face = &potential_collisions[0];
                let mut closest_distance = f32::MAX;

                for face in &potential_collisions {
                    let face_center = face.aabb.center();
                    let distance = ((face_center[0] - resolved_pos[0]).powi(2)
                        + (face_center[1] - resolved_pos[1]).powi(2)
                        + (face_center[2] - resolved_pos[2]).powi(2))
                    .sqrt();

                    if distance < closest_distance {
                        closest_distance = distance;
                        closest_face = face;
                    }
                }

                // Resolve only the closest collision
                let movement = [
                    resolved_pos[0] - current_pos[0],
                    resolved_pos[1] - current_pos[1],
                    resolved_pos[2] - current_pos[2],
                ];

                resolved_pos = self.resolve_wall_collision(
                    audio_manager,
                    current_pos,
                    resolved_pos,
                    movement,
                    closest_face,
                );

                // If position didn't change significantly, we're stuck - break out
                let epsilon = 0.0001;
                let position_changed = (resolved_pos[0] - current_pos[0]).abs() > epsilon
                    || (resolved_pos[1] - current_pos[1]).abs() > epsilon
                    || (resolved_pos[2] - current_pos[2]).abs() > epsilon;

                if !position_changed {
                    break;
                }
            }

            resolved_pos
        })
    }

    /// Detects if the player is stuck between opposing wall faces.
    ///
    /// This method identifies situations where the player is positioned between
    /// two wall faces with opposite normal vectors, which can cause the player
    /// to become trapped and unable to move.
    ///
    /// # Arguments
    ///
    /// * `faces` - A slice of wall faces that potentially collide with the player
    /// * `pos` - The player's current position
    ///
    /// # Returns
    ///
    /// `true` if the player is stuck between opposing faces, `false` otherwise
    ///
    /// # Algorithm
    ///
    /// 1. Iterate through all pairs of faces
    /// 2. Check if their normal vectors are roughly opposite (dot product < -0.95)
    /// 3. If so, calculate the distance to each face
    /// 4. If the player is within radius of both faces, they are stuck
    ///
    /// # Use Case
    ///
    /// This detection is used to trigger special handling when the player
    /// becomes trapped in a corner or narrow passage, allowing the system
    /// to find an escape route.
    fn is_stuck_between_faces(&self, faces: &[&WallFace], pos: [f32; 3]) -> bool {
        // Check if there are opposing faces (normals pointing in opposite directions)
        for i in 0..faces.len() {
            for j in i + 1..faces.len() {
                let face1 = faces[i];
                let face2 = faces[j];

                // Check if normals are roughly opposite
                let dot_product = face1.normal[0] * face2.normal[0]
                    + face1.normal[1] * face2.normal[1]
                    + face1.normal[2] * face2.normal[2];

                if dot_product < -0.95 {
                    // Roughly opposite normals
                    // Check if player is between these faces
                    let dist1 = self.distance_to_face(pos, face1);
                    let dist2 = self.distance_to_face(pos, face2);

                    if dist1 < self.player_radius && dist2 < self.player_radius {
                        return true; // Stuck between opposing faces
                    }
                }
            }
        }
        false
    }

    /// Calculates the signed distance from a point to a wall face.
    ///
    /// This method computes the perpendicular distance from a given point
    /// to a wall face using vector projection onto the face's normal vector.
    ///
    /// # Arguments
    ///
    /// * `pos` - The point to measure distance from
    /// * `face` - The wall face to measure distance to
    ///
    /// # Returns
    ///
    /// The absolute distance from the point to the face plane
    ///
    /// # Mathematical Method
    ///
    /// 1. Calculate the vector from the point to the face center
    /// 2. Project this vector onto the face's normal vector using dot product
    /// 3. Return the absolute value of the projection
    ///
    /// # Note
    ///
    /// This method returns the absolute distance, so it doesn't distinguish
    /// between being on the front or back side of the face.
    fn distance_to_face(&self, pos: [f32; 3], face: &WallFace) -> f32 {
        let face_center = face.aabb.center();
        let to_face = [
            face_center[0] - pos[0],
            face_center[1] - pos[1],
            face_center[2] - pos[2],
        ];

        // Project onto face normal to get distance
        let distance =
            to_face[0] * face.normal[0] + to_face[1] * face.normal[1] + to_face[2] * face.normal[2];

        distance.abs()
    }

    /// Attempts to find a safe position when the player is stuck.
    ///
    /// This method is called when the player becomes trapped between opposing
    /// wall faces. It tries to move the player in different directions to find
    /// a position that doesn't collide with any walls.
    ///
    /// # Arguments
    ///
    /// * `current_pos` - The player's current stuck position
    ///
    /// # Returns
    ///
    /// A safe position if found, otherwise returns the current position
    ///
    /// # Algorithm
    ///
    /// 1. Try moving in four cardinal directions (+X, -X, +Z, -Z)
    /// 2. Use a small escape distance (half the player radius)
    /// 3. Test each potential position for collisions
    /// 4. Return the first safe position found
    /// 5. If no safe position is found, return the current position
    ///
    /// # Use Case
    ///
    /// This method provides a fallback mechanism to prevent the player from
    /// becoming permanently stuck in corners or narrow passages.
    fn find_safe_position(&self, current_pos: [f32; 3]) -> [f32; 3] {
        // Try moving in different directions to find a safe spot
        let escape_distance = self.player_radius / 2.0;
        let directions = [
            [1.0, 0.0, 0.0],  // +X
            [-1.0, 0.0, 0.0], // -X
            [0.0, 0.0, 1.0],  // +Z
            [0.0, 0.0, -1.0], // -Z
        ];

        for direction in &directions {
            let test_pos = [
                current_pos[0] + direction[0] * escape_distance,
                current_pos[1] + direction[1] * escape_distance,
                current_pos[2] + direction[2] * escape_distance,
            ];

            // Test if this position is safe
            let test_aabb = AABB::new(
                [
                    test_pos[0] - self.player_radius,
                    test_pos[1],
                    test_pos[2] - self.player_radius,
                ],
                [
                    test_pos[0] + self.player_radius,
                    test_pos[1] + self.player_height,
                    test_pos[2] + self.player_radius,
                ],
            );

            if self.bvh.query_collisions(&test_aabb).is_empty() {
                return test_pos;
            }
        }

        // If no safe position found, return current position
        current_pos
    }
    /// Resolves collision with a single wall using vector projection for sliding.
    ///
    /// This method handles the physics of collision response between the player
    /// and a wall face. It projects the player's movement vector onto the wall
    /// plane, allowing for realistic sliding along surfaces.
    ///
    /// # Arguments
    ///
    /// * `current_pos` - The player's current position
    /// * `desired_pos` - The position the player wants to move to
    /// * `movement` - The movement vector (desired_pos - current_pos)
    /// * `wall_face` - Reference to the wall face being collided with
    ///
    /// # Returns
    ///
    /// The resolved position after handling the collision
    ///
    /// # Vector Mathematics
    ///
    /// 1. Determine the effective wall normal based on which side the player is on
    /// 2. Calculate the dot product of the movement vector and the normal
    /// 3. If moving into the wall (negative dot product), remove the component
    ///    of movement that's in the direction of the normal
    /// 4. The result is a "slide" vector parallel to the wall surface
    ///
    /// # Visual Explanation
    ///
    /// ```text
    ///                   Wall
    ///                    |
    ///                    |
    ///  wall              |
    /// normal <-----------|
    ///                 ↑  |
    ///                 ┊  |
    ///          slide  ┊  |
    ///         movement┊  |
    ///                 ┊  |
    /// original --------->|
    /// movement           |
    ///                    |
    /// ```
    fn resolve_wall_collision(
        &self,
        audio_manager: &mut GameAudioManager,
        current_pos: [f32; 3],
        desired_pos: [f32; 3],
        movement: [f32; 3],
        wall_face: &WallFace,
    ) -> [f32; 3] {
        let normal = wall_face.normal;

        // Calculate ray from previous position to desired position
        let movement_dir = [
            desired_pos[0] - current_pos[0],
            desired_pos[1] - current_pos[1],
            desired_pos[2] - current_pos[2],
        ];

        // Dot product between movement and wall normal tells us direction of approach
        let approach_dot =
            movement_dir[0] * normal[0] + movement_dir[1] * normal[1] + movement_dir[2] * normal[2];

        // Use the normal that opposes the player's movement
        let effective_normal = if approach_dot < 0.0 {
            normal
        } else {
            [-normal[0], -normal[1], -normal[2]]
        };

        // Calculate penetration
        let movement_dot = movement[0] * effective_normal[0]
            + movement[1] * effective_normal[1]
            + movement[2] * effective_normal[2];

        // Only resolve if moving into the wall
        if movement_dot < 0.0 {
            audio_manager
                .wall_hit()
                .expect("Failed to play wall hit sound");
            let slide_movement = [
                movement[0] - movement_dot * effective_normal[0],
                movement[1] - movement_dot * effective_normal[1],
                movement[2] - movement_dot * effective_normal[2],
            ];

            return [
                // Add a small buffer ("skin") to prevent getting exactly flushrn [
                current_pos[0] + slide_movement[0],
                current_pos[1] + slide_movement[1],
                current_pos[2] + slide_movement[2],
            ];
        }

        desired_pos
    }

    /// Tests if a swept cylinder intersects with any geometry in the scene.
    ///
    /// This method performs collision detection for a cylinder that moves along
    /// a line segment from start to end. It's useful for testing movement paths
    /// or line-of-sight calculations.
    ///
    /// # Arguments
    ///
    /// * `start` - The starting position of the cylinder center
    /// * `end` - The ending position of the cylinder center
    /// * `radius` - The radius of the cylinder
    ///
    /// # Returns
    ///
    /// `true` if the cylinder intersects any wall face, `false` otherwise
    ///
    /// # Algorithm
    ///
    /// 1. Create an expanded AABB that encompasses the entire cylinder path
    /// 2. Query the BVH for potential wall faces that might intersect
    /// 3. For each potential face, perform detailed cylinder-face intersection test
    /// 4. Return true if any intersection is found
    ///
    /// # Performance
    ///
    /// This method uses the BVH for efficient broad-phase collision detection,
    /// then performs detailed tests only on potentially intersecting faces.
    pub fn cylinder_intersects_geometry(
        &self,
        start: [f32; 3],
        end: [f32; 3],
        radius: f32,
    ) -> bool {
        // Use benchmark macro for cylinder intersection timing
        crate::benchmark!("cylinder_intersects_geometry", {
            // Create expanded AABB for the cylinder path
            let cylinder_aabb = AABB::new(
                [
                    start[0].min(end[0]) - radius,
                    start[1].min(end[1]) - radius,
                    start[2].min(end[2]) - radius,
                ],
                [
                    start[0].max(end[0]) + radius,
                    start[1].max(end[1]) + radius,
                    start[2].max(end[2]) + radius,
                ],
            );

            let potential_faces = self.bvh.query_collisions(&cylinder_aabb);

            for face in &potential_faces {
                if self.cylinder_intersects_wall_face(start, end, radius, face) {
                    return true;
                }
            }

            false
        })
    }

    /// Performs cylinder-wall face intersection test.
    ///
    /// This method tests whether a cylinder swept along a line segment intersects with a specific wall face.
    /// The cylinder is defined by its radius and the line segment represents its center path.
    ///
    /// # Arguments
    ///
    /// * `start` - Cylinder center start point
    /// * `end` - Cylinder center end point
    /// * `radius` - Cylinder radius
    /// * `face` - The wall face to test against
    ///
    /// # Returns
    ///
    /// `true` if the cylinder intersects the face, `false` otherwise.
    fn cylinder_intersects_wall_face(
        &self,
        start: [f32; 3],
        end: [f32; 3],
        radius: f32,
        face: &WallFace,
    ) -> bool {
        // Get the face's AABB bounds
        let min_bounds = face.aabb.min;
        let max_bounds = face.aabb.max;

        // Determine which axis the face is aligned with based on its normal
        let normal = face.normal;
        let epsilon = 0.0001;

        // Check if this is an X-facing wall (normal in X direction)
        if normal[0].abs() > epsilon {
            let wall_x = if normal[0] > 0.0 {
                min_bounds[0]
            } else {
                max_bounds[0]
            };

            // Check if cylinder path crosses this X plane (accounting for radius)
            let start_distance = start[0] - wall_x;
            let end_distance = end[0] - wall_x;

            // If both ends are on the same side and farther than radius, no intersection
            if start_distance * end_distance > 0.0
                && start_distance.abs() > radius
                && end_distance.abs() > radius
            {
                return false;
            }

            // Calculate the closest approach of the cylinder center to the plane
            let line_dir = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
            let line_length_sq =
                line_dir[0] * line_dir[0] + line_dir[1] * line_dir[1] + line_dir[2] * line_dir[2];

            if line_length_sq < epsilon * epsilon {
                // Start and end are the same point
                if start_distance.abs() <= radius {
                    // Check if the cylinder at this point intersects the face bounds
                    return self.point_in_expanded_face_bounds(start, radius, face, 0); // X-axis
                }
                return false;
            }

            let t = if line_dir[0].abs() > epsilon {
                (wall_x - start[0]) / line_dir[0]
            } else {
                // Line is parallel to the plane
                if start_distance.abs() <= radius {
                    // Check if cylinder intersects face bounds along its entire path
                    return self
                        .cylinder_intersects_face_bounds_parallel(start, end, radius, face, 0); // X-axis
                }
                return false;
            };

            // Find the point on the line closest to the plane
            let closest_point = if t < 0.0 {
                start
            } else if t > 1.0 {
                end
            } else {
                [
                    start[0] + t * line_dir[0],
                    start[1] + t * line_dir[1],
                    start[2] + t * line_dir[2],
                ]
            };

            let distance_to_plane = (closest_point[0] - wall_x).abs();

            if distance_to_plane <= radius {
                // Check if the intersection area overlaps with the face bounds
                return self.point_in_expanded_face_bounds(closest_point, radius, face, 0); // X-axis
            }
        }

        // Check if this is a Z-facing wall (normal in Z direction)
        if normal[2].abs() > epsilon {
            let wall_z = if normal[2] > 0.0 {
                min_bounds[2]
            } else {
                max_bounds[2]
            };

            let start_distance = start[2] - wall_z;
            let end_distance = end[2] - wall_z;

            if start_distance * end_distance > 0.0
                && start_distance.abs() > radius
                && end_distance.abs() > radius
            {
                return false;
            }

            let line_dir = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
            let line_length_sq =
                line_dir[0] * line_dir[0] + line_dir[1] * line_dir[1] + line_dir[2] * line_dir[2];

            if line_length_sq < epsilon * epsilon {
                if start_distance.abs() <= radius {
                    return self.point_in_expanded_face_bounds(start, radius, face, 2); // Z-axis
                }
                return false;
            }

            let t = if line_dir[2].abs() > epsilon {
                (wall_z - start[2]) / line_dir[2]
            } else {
                if start_distance.abs() <= radius {
                    return self
                        .cylinder_intersects_face_bounds_parallel(start, end, radius, face, 2); // Z-axis
                }
                return false;
            };

            let closest_point = if t < 0.0 {
                start
            } else if t > 1.0 {
                end
            } else {
                [
                    start[0] + t * line_dir[0],
                    start[1] + t * line_dir[1],
                    start[2] + t * line_dir[2],
                ]
            };

            let distance_to_plane = (closest_point[2] - wall_z).abs();

            if distance_to_plane <= radius {
                return self.point_in_expanded_face_bounds(closest_point, radius, face, 2); // Z-axis
            }
        }

        // Check if this is a Y-facing wall (normal in Y direction) - for floors/ceilings
        if normal[1].abs() > epsilon {
            let wall_y = if normal[1] > 0.0 {
                min_bounds[1]
            } else {
                max_bounds[1]
            };

            let start_distance = start[1] - wall_y;
            let end_distance = end[1] - wall_y;

            if start_distance * end_distance > 0.0
                && start_distance.abs() > radius
                && end_distance.abs() > radius
            {
                return false;
            }

            let line_dir = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
            let line_length_sq =
                line_dir[0] * line_dir[0] + line_dir[1] * line_dir[1] + line_dir[2] * line_dir[2];

            if line_length_sq < epsilon * epsilon {
                if start_distance.abs() <= radius {
                    return self.point_in_expanded_face_bounds(start, radius, face, 1); // Y-axis
                }
                return false;
            }

            let t = if line_dir[1].abs() > epsilon {
                (wall_y - start[1]) / line_dir[1]
            } else {
                if start_distance.abs() <= radius {
                    return self
                        .cylinder_intersects_face_bounds_parallel(start, end, radius, face, 1); // Y-axis
                }
                return false;
            };

            let closest_point = if t < 0.0 {
                start
            } else if t > 1.0 {
                end
            } else {
                [
                    start[0] + t * line_dir[0],
                    start[1] + t * line_dir[1],
                    start[2] + t * line_dir[2],
                ]
            };

            let distance_to_plane = (closest_point[1] - wall_y).abs();

            if distance_to_plane <= radius {
                return self.point_in_expanded_face_bounds(closest_point, radius, face, 1); // Y-axis
            }
        }

        false
    }

    /// Checks if a point (expanded by radius) intersects with face bounds.
    ///
    /// This helper method determines if a cylindrical volume centered at a point
    /// intersects with the bounds of a wall face. It's used in cylinder-wall
    /// intersection tests to determine if the cylinder overlaps the face's area.
    ///
    /// # Arguments
    ///
    /// * `point` - The center point of the cylinder
    /// * `radius` - The radius of the cylinder
    /// * `face` - The wall face to check against
    /// * `normal_axis` - The axis perpendicular to the face (0=X, 1=Y, 2=Z)
    ///
    /// # Returns
    ///
    /// `true` if the expanded point intersects the face bounds, `false` otherwise
    ///
    /// # Algorithm
    ///
    /// For each face orientation (X, Y, or Z-facing):
    /// 1. Expand the point by the radius in the two non-normal axes
    /// 2. Check if this expanded area overlaps with the face's AABB bounds
    /// 3. Return true if there's any overlap
    ///
    /// # Use Case
    ///
    /// This method is called during cylinder-wall intersection tests to determine
    /// if a cylinder at a specific point would intersect with a wall face's area.
    fn point_in_expanded_face_bounds(
        &self,
        point: [f32; 3],
        radius: f32,
        face: &WallFace,
        normal_axis: usize,
    ) -> bool {
        let min_bounds = face.aabb.min;
        let max_bounds = face.aabb.max;

        match normal_axis {
            0 => {
                // X-facing wall: check Y and Z bounds
                point[1] + radius >= min_bounds[1]
                    && point[1] - radius <= max_bounds[1]
                    && point[2] + radius >= min_bounds[2]
                    && point[2] - radius <= max_bounds[2]
            }
            1 => {
                // Y-facing wall: check X and Z bounds
                point[0] + radius >= min_bounds[0]
                    && point[0] - radius <= max_bounds[0]
                    && point[2] + radius >= min_bounds[2]
                    && point[2] - radius <= max_bounds[2]
            }
            2 => {
                // Z-facing wall: check X and Y bounds
                point[0] + radius >= min_bounds[0]
                    && point[0] - radius <= max_bounds[0]
                    && point[1] + radius >= min_bounds[1]
                    && point[1] - radius <= max_bounds[1]
            }
            _ => false,
        }
    }

    /// Tests cylinder intersection when the path is parallel to the face.
    ///
    /// This helper method handles the special case where a cylinder's movement
    /// path is parallel to a wall face. In this case, the cylinder sweeps out
    /// a rectangular volume that can be tested against the face bounds.
    ///
    /// # Arguments
    ///
    /// * `start` - Start point of the cylinder center path
    /// * `end` - End point of the cylinder center path
    /// * `radius` - Radius of the cylinder
    /// * `face` - The wall face to test against
    /// * `normal_axis` - The axis perpendicular to the face (0=X, 1=Y, 2=Z)
    ///
    /// # Returns
    ///
    /// `true` if the cylinder path intersects the face bounds, `false` otherwise
    ///
    /// # Algorithm
    ///
    /// 1. Expand the start and end points by the cylinder radius
    /// 2. Create a bounding box for the entire swept volume
    /// 3. Test if this bounding box overlaps with the face's AABB
    /// 4. The test is performed in the two axes perpendicular to the face normal
    ///
    /// # Use Case
    ///
    /// This method is called when the cylinder movement path is parallel to a
    /// wall face, which requires a different intersection test than the general
    /// case.
    fn cylinder_intersects_face_bounds_parallel(
        &self,
        start: [f32; 3],
        end: [f32; 3],
        radius: f32,
        face: &WallFace,
        normal_axis: usize,
    ) -> bool {
        let min_bounds = face.aabb.min;
        let max_bounds = face.aabb.max;

        match normal_axis {
            0 => {
                // X-facing wall: check if cylinder path intersects Y-Z bounds
                let start_y_min = start[1] - radius;
                let start_y_max = start[1] + radius;
                let end_y_min = end[1] - radius;
                let end_y_max = end[1] + radius;

                let start_z_min = start[2] - radius;
                let start_z_max = start[2] + radius;
                let end_z_min = end[2] - radius;
                let end_z_max = end[2] + radius;

                let path_y_min = start_y_min.min(end_y_min);
                let path_y_max = start_y_max.max(end_y_max);
                let path_z_min = start_z_min.min(end_z_min);
                let path_z_max = start_z_max.max(end_z_max);

                path_y_max >= min_bounds[1]
                    && path_y_min <= max_bounds[1]
                    && path_z_max >= min_bounds[2]
                    && path_z_min <= max_bounds[2]
            }
            1 => {
                // Y-facing wall: check if cylinder path intersects X-Z bounds
                let start_x_min = start[0] - radius;
                let start_x_max = start[0] + radius;
                let end_x_min = end[0] - radius;
                let end_x_max = end[0] + radius;

                let start_z_min = start[2] - radius;
                let start_z_max = start[2] + radius;
                let end_z_min = end[2] - radius;
                let end_z_max = end[2] + radius;

                let path_x_min = start_x_min.min(end_x_min);
                let path_x_max = start_x_max.max(end_x_max);
                let path_z_min = start_z_min.min(end_z_min);
                let path_z_max = start_z_max.max(end_z_max);

                path_x_max >= min_bounds[0]
                    && path_x_min <= max_bounds[0]
                    && path_z_max >= min_bounds[2]
                    && path_z_min <= max_bounds[2]
            }
            2 => {
                // Z-facing wall: check if cylinder path intersects X-Y bounds
                let start_x_min = start[0] - radius;
                let start_x_max = start[0] + radius;
                let end_x_min = end[0] - radius;
                let end_x_max = end[0] + radius;

                let start_y_min = start[1] - radius;
                let start_y_max = start[1] + radius;
                let end_y_min = end[1] - radius;
                let end_y_max = end[1] + radius;

                let path_x_min = start_x_min.min(end_x_min);
                let path_x_max = start_x_max.max(end_x_max);
                let path_y_min = start_y_min.min(end_y_min);
                let path_y_max = start_y_max.max(end_y_max);

                path_x_max >= min_bounds[0]
                    && path_x_min <= max_bounds[0]
                    && path_y_max >= min_bounds[1]
                    && path_y_min <= max_bounds[1]
            }
            _ => false,
        }
    }
}

// Integration with Player struct
impl Player {
    /// Enhanced movement with collision detection and resolution.
    ///
    /// This method extends the basic player movement with physics-based
    /// collision handling. It calculates the desired position based on
    /// input controls, then uses the collision system to ensure the player
    /// doesn't pass through walls.
    ///
    /// # Arguments
    ///
    /// * `collision_system` - Reference to the collision system
    /// * `delta_time` - Time elapsed since last frame (for consistent movement)
    /// * `forward`, `backward`, `left`, `right` - Movement control flags
    ///
    /// # Movement Process
    ///
    /// 1. Start with the player's current position
    /// 2. Calculate desired position based on input and player orientation
    /// 3. Check for collisions using the collision system
    /// 4. Update the player's position with collision-resolved coordinates
    ///
    /// # Physics Integration
    ///
    /// The movement respects the physics of the environment by preventing
    /// penetration into walls and allowing for realistic sliding along surfaces.
    #[allow(clippy::too_many_arguments)]
    pub fn move_with_collision(
        &mut self,
        audio_manager: &mut GameAudioManager,
        collision_system: &CollisionSystem,
        delta_time: f32,
        forward: bool,
        backward: bool,
        left: bool,
        right: bool,
    ) {
        let current_pos = self.position;
        let mut desired_pos = current_pos;

        // Apply movement based on input
        if forward {
            let forward_x = self.yaw.to_radians().sin();
            let forward_z = self.yaw.to_radians().cos();
            desired_pos[0] -= forward_x * self.speed * delta_time;
            desired_pos[2] -= forward_z * self.speed * delta_time;
        }
        if backward {
            let forward_x = self.yaw.to_radians().sin();
            let forward_z = self.yaw.to_radians().cos();
            desired_pos[0] += forward_x * self.speed * delta_time;
            desired_pos[2] += forward_z * self.speed * delta_time;
        }
        if left {
            let right_x = self.yaw.to_radians().cos();
            let right_z = self.yaw.to_radians().sin();
            desired_pos[0] -= right_x * self.speed * delta_time;
            desired_pos[2] += right_z * self.speed * delta_time;
        }
        if right {
            let right_x = self.yaw.to_radians().cos();
            let right_z = self.yaw.to_radians().sin();
            desired_pos[0] += right_x * self.speed * delta_time;
            desired_pos[2] -= right_z * self.speed * delta_time;
        }

        // Resolve collisions and update position
        self.position =
            collision_system.check_and_resolve_collision(audio_manager, current_pos, desired_pos);
    }
}
