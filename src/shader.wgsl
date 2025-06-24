// shader.wgsl

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) fragment_color: vec4<f32>,
    @location(1) world_position: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> mvp_matrix: mat4x4<f32>;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = mvp_matrix * vec4<f32>(in.position, 1.0);
    out.fragment_color = in.color;
    out.world_position = in.position.xz; // Use XZ plane for ground
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tan = vec4<f32>(0.941, 0.875, 0.62, 1.0);
    let purple = vec4<f32>(0.545, 0.455, 0.51, 1.0);

    let tile_size = 20.0;
    let grid = vec2<i32>(floor(in.world_position / tile_size));
    let checker = (grid.x + grid.y) % 2 != 0;

    return select(tan, purple, checker);
}
