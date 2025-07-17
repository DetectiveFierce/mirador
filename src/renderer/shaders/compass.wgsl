struct CompassUniforms {
    screen_position: vec2<f32>,
    compass_size: vec2<f32>,
    _padding: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: CompassUniforms;

@group(0) @binding(1)
var compass_texture: texture_2d<f32>;

@group(0) @binding(2)
var compass_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Simplified: just put a small quad in bottom-right corner
    // Scale the quad down and position it
    let scaled_pos = input.position * 0.25; // Make it smaller
    let positioned = scaled_pos + vec2<f32>(0.7, -0.7); // Bottom-right in NDC

    out.clip_position = vec4<f32>(positioned.x, positioned.y, 0.0, 1.0);
    out.tex_coords = input.tex_coords;

    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let tex_coords = input.tex_coords;
    let color = textureSample(compass_texture, compass_sampler, tex_coords);

    // Define the shadow parameters
    let center = vec2<f32>(0.5, 0.4); // Center of the texture
    let dist = distance(tex_coords, center);

    // Make the shadow directional: stronger below the center
    let direction_bias = clamp((tex_coords.y - center.y) * 1.5 + 0.2, 0.0, 1.0);

    // Sharper falloff and darker center
    let shadow_radius = 0.5;        // Slightly wider shadow
    let shadow_inner = 0.35;        // Starts fading earlier
    let shadow_fade = smoothstep(shadow_inner, shadow_radius, dist);

    // Stronger shadow alpha at center, faster fade out
    let shadow_alpha = (1.0 - shadow_fade) * direction_bias * (1.0 - color.a);

    // Increased darkness at center (was 0.4)
    let shadow_color = vec4<f32>(0.0, 0.0, 0.0, shadow_alpha * 0.8); 

    return shadow_color + color;
}



