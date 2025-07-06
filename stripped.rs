use crate::game::Player;

#[derive(Debug, Clone)]
pub struct AABB {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl AABB {
    pub fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }

    pub fn from_wall_face(p1: [f32; 3], p2: [f32; 3], p3: [f32; 3], p4: [f32; 3]) -> Self {
        let min_x = p1[0].min(p2[0]).min(p3[0]).min(p4[0]);
        let min_y = p1[1].min(p2[1]).min(p3[1]).min(p4[1]);
        let min_z = p1[2].min(p2[2]).min(p3[2]).min(p4[2]);

        let max_x = p1[0].max(p2[0]).max(p3[0]).max(p4[0]);
        let max_y = p1[1].max(p2[1]).max(p3[1]).max(p4[1]);
        let max_z = p1[2].max(p2[2]).max(p3[2]).max(p4[2]);

        Self::new([min_x, min_y, min_z], [max_x, max_y, max_z])
    }

    pub fn expand(&mut self, other: &AABB) {
        for i in 0..3 {
            self.min[i] = self.min[i].min(other.min[i]);
            self.max[i] = self.max[i].max(other.max[i]);
        }
    }

    pub fn intersects(&self, other: &AABB) -> bool {
        for i in 0..3 {
            if self.max[i] < other.min[i] || self.min[i] > other.max[i] {
                return false;
            }
        }
        true
    }

    pub fn center(&self) -> [f32; 3] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
            (self.min[2] + self.max[2]) * 0.5,
        ]
    }

    pub fn surface_area(&self) -> f32 {
        let dx = self.max[0] - self.min[0];
        let dy = self.max[1] - self.min[1];
        let dz = self.max[2] - self.min[2];
        2.0 * (dx * dy + dy * dz + dz * dx)
    }
}

#[derive(Debug, Clone)]
pub struct WallFace {
    pub corners: [[f32; 3]; 4],
    pub normal: [f32; 3],
    pub aabb: AABB,
}

impl WallFace {
    pub fn new(corners: [[f32; 3]; 4]) -> Self {
        let normal = Self::calculate_normal(&corners);
        let aabb = AABB::from_wall_face(corners[0], corners[1], corners[2], corners[3]);

        Self {
            corners,
            normal,
            aabb,
        }
    }

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

#[derive(Debug, Clone)]
pub enum BVHNode {
    Leaf {
        aabb: AABB,
        faces: Vec<WallFace>,
    },
    Internal {
        aabb: AABB,
        left: Box<BVHNode>,
        right: Box<BVHNode>,
    },
}

impl BVHNode {
    pub fn aabb(&self) -> &AABB {
        match self {
            BVHNode::Leaf { aabb, .. } => aabb,
            BVHNode::Internal { aabb, .. } => aabb,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct BVH {
    pub root: Option<BVHNode>,
}

impl BVH {
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn build(&mut self, faces: Vec<WallFace>) {
        if faces.is_empty() {
            self.root = None;
            return;
        }

        self.root = Some(Self::build_recursive(faces));
    }

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
            a_center.partial_cmp(&b_center).unwrap()
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

    pub fn query_collisions(&self, player_aabb: &AABB) -> Vec<&WallFace> {
        let mut results = Vec::new();
        if let Some(ref root) = self.root {
            Self::query_recursive(root, player_aabb, &mut results);
        }
        results
    }

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

#[derive(Debug, Default, Clone)]
pub struct CollisionSystem {
    pub bvh: BVH,
    pub player_radius: f32,
    pub player_height: f32,
    pub maze_dimensions: (usize, usize),
}

impl CollisionSystem {
    pub fn new(player_radius: f32, player_height: f32) -> Self {
        Self {
            bvh: BVH::new(),
            player_radius,
            player_height,
            maze_dimensions: (0, 0),
        }
    }

    pub fn build_from_maze(&mut self, maze_grid: &[Vec<bool>]) {
        // Store maze dimensions
        self.maze_dimensions = (maze_grid[0].len(), maze_grid.len());
        let wall_faces = self.extract_wall_faces_from_maze(maze_grid);
        self.bvh.build(wall_faces);
    }

    fn extract_wall_faces_from_maze(&self, maze_grid: &[Vec<bool>]) -> Vec<WallFace> {
        let mut faces = Vec::new();
        let maze_width = maze_grid[0].len();
        let maze_height = maze_grid.len();
        let maze_dimensions = (maze_width, maze_height);

        // Use coordinate API to calculate sizes and origins
        let cell_size = crate::math::coordinates::calculate_cell_size(maze_dimensions);
        let wall_height = cell_size;

        // Calculate the world origin offset (bottom-left corner of the maze)
        let origin_x = -(maze_width as f32 * cell_size) / 2.0;
        let origin_z = -(maze_height as f32 * cell_size) / 2.0;

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

        faces
    }

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

    pub fn check_and_resolve_collision(
        &self,
        current_pos: [f32; 3],
        desired_pos: [f32; 3],
    ) -> [f32; 3] {
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

            resolved_pos =
                self.resolve_wall_collision(current_pos, resolved_pos, movement, closest_face);

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
    }

    fn resolve_wall_collision(
        &self,
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
}

// Integration with Player struct
impl Player {
    pub fn move_with_collision(
        &mut self,
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
        self.position = collision_system.check_and_resolve_collision(current_pos, desired_pos);
    }
}

impl CollisionSystem {
    pub fn cylinder_intersects_geometry(
        &self,
        start: [f32; 3],
        end: [f32; 3],
        radius: f32,
    ) -> bool {
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
    }

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
