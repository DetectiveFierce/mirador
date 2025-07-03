struct EnemyUniforms {
    model_matrix: mat4x4<f32>,
    view_proj_matrix: mat4x4<f32>,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: EnemyUniforms;

@group(0) @binding(1)
var enemy_texture: texture_2d<f32>;

@group(0) @binding(2)
var enemy_sampler: sampler;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Transform vertex position
    let world_position = uniforms.model_matrix * vec4<f32>(input.position, 1.0);
    out.clip_position = uniforms.view_proj_matrix * world_position;

    // Pass through texture coordinates
    out.tex_coords = input.tex_coords;

    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Sample the enemy texture
    let texture_color = textureSample(enemy_texture, enemy_sampler, input.tex_coords);

    // Optional: Add some subtle effects
    // You can add color tinting, transparency, or other effects here

    return texture_color;
}
