// Enemy billboard shader - rotates sprite around Y-axis to face player

struct EnemyUniforms {
    view_proj_matrix: mat4x4<f32>,
    enemy_position: vec3<f32>,
    enemy_size: f32,
    player_position: vec3<f32>,
    _padding: f32,
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
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Calculate direction from enemy to player (for Y-axis rotation)
    let to_player = uniforms.player_position - uniforms.enemy_position;
    let rotation_angle = atan2(to_player.x, to_player.z);

    // Create rotation matrix around Y-axis (fixed signs)
    let cos_y = cos(rotation_angle);
    let sin_y = sin(rotation_angle);
    let rotation_matrix = mat3x3<f32>(
        cos_y,  0.0, -sin_y,  // Changed: sin_y to -sin_y
        0.0,    1.0, 0.0,
        sin_y,  0.0, cos_y    // Changed: -sin_y to sin_y
    );

    // Scale the vertex by enemy size
    let scaled_position = model.position * uniforms.enemy_size;

    // Apply rotation to the scaled position
    let rotated_position = rotation_matrix * scaled_position;

    // Translate to enemy's world position
    let world_position = rotated_position + uniforms.enemy_position;

    // Transform to clip space
    out.clip_position = uniforms.view_proj_matrix * vec4<f32>(world_position, 1.0);
    out.tex_coords = model.tex_coords;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texture_color = textureSample(enemy_texture, enemy_sampler, in.tex_coords);

    // Discard transparent pixels (alpha testing)
    if (texture_color.a < 0.1) {
        discard;
    }

    return texture_color;
}
