// shader.wgsl
// Vertex shader input/output structures
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>, // vec4 for u8 conversion in Rust
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) fragment_color: vec4<f32>,
};

// Single uniform buffer containing the combined MVP matrix
@group(0) @binding(0)
var<uniform> mvp_matrix: mat4x4<f32>;

// Vertex Shader
@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    // Apply the combined Model-View-Projection matrix directly
    out.clip_position = mvp_matrix * vec4(in.position, 1.0);
    out.fragment_color = in.color;
    return out;
}

// Fragment Shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.fragment_color;
}
