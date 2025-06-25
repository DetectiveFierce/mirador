//! # Maze Renderer Shader
//!
//! This WGSL shader is used for rendering the 3D maze and floor in the game.
//! It supports two materials: walls and floor. Walls are rendered with a solid maroon color,
//! while the floor uses a checkerboard pattern with tan and purple tiles.
//!
//! ## Structs
//! - `VertexInput`: Input structure for the vertex shader, containing position, color, and material ID.
//! - `VertexOutput`: Output structure from the vertex shader, passed to the fragment shader. Includes
//!   clip-space position, color, world-space XZ position, and material ID.
//!
//! ## Entry Points
//! - `vs_main`: Vertex shader. Transforms vertex positions by the MVP matrix, passes color and material.
//! - `fs_main`: Fragment shader. Applies material-based coloring. Walls are solid maroon; the floor
//!   uses a checkerboard pattern based on world position.
//!
//! ## Material Logic
//! - `material == 1`: Wall cell, colored maroon (`vec4<f32>(0.102, 0.027, 0.035, 1.0)`).
//! - `material == 0`: Floor cell, colored with a checkerboard pattern alternating between tan and purple.

struct VertexInput {
    /// Vertex position in model space.
    @location(0) position: vec3<f32>,
    /// Vertex color (unused in current fragment logic).
    @location(1) color: vec4<f32>,
    /// Material ID: 0 = floor, 1 = wall.
    @location(2) material: u32,
};

struct VertexOutput {
    /// Clip-space position for rasterization.
    @builtin(position) clip_position: vec4<f32>,
    /// Vertex color (passed through, not used in fragment logic).
    @location(0) fragment_color: vec4<f32>,
    /// World-space XZ position, used for floor checkerboard.
    @location(1) world_position: vec2<f32>,
    /// Material ID: 0 = floor, 1 = wall.
    @location(2) material: u32,
};

/// Model-View-Projection matrix uniform.
@group(0) @binding(0)
var<uniform> mvp_matrix: mat4x4<f32>;

/// Vertex shader entry point.
/// Transforms vertex position by the MVP matrix and passes through color, world XZ, and material.
@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = mvp_matrix * vec4<f32>(in.position, 1.0);
    out.fragment_color = in.color;
    out.world_position = in.position.xz;
    out.material = in.material;
    return out;
}

/// Fragment shader entry point.
/// Applies material-based coloring: walls are maroon, floor is a checkerboard of tan and purple.
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Material-based coloring
    if (in.material == 1u) {
        return vec4<f32>(0.102, 0.027, 0.035, 1.0); // Wall: Maroon
    }

    // Floor: checkerboard
    let tan = vec4<f32>(0.941, 0.875, 0.62, 1.0);
    let purple = vec4<f32>(0.545, 0.455, 0.51, 1.0);

    let tile_size = 20.0;
    let grid = vec2<i32>(floor(in.world_position / tile_size));
    let checker = (grid.x + grid.y) % 2 != 0;

    return select(tan, purple, checker);
}
