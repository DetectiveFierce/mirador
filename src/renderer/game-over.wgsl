// game-over.wgsl
struct GameOverUniforms {
    time: f32,
}

@group(0) @binding(0) var<uniform> uniforms: GameOverUniforms;

// Vertex shader
struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.clip_position = vec4<f32>(input.position, 0.0, 1.0);
    out.tex_coords = (input.position + 1.0) * 0.5; // Convert from [-1,1] to [0,1]

    return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Create a semitransparent red overlay
    let overlay_color = vec3<f32>(0.8, 0.1, 0.1); // Dark red

    // Add a subtle pulsing effect using time
    let pulse = sin(uniforms.time * 2.0) * 0.1 + 0.9;
    let alpha = 0.7 * pulse; // Semi-transparent with pulsing

    // Optional: Add a subtle vignette effect
    let center = vec2<f32>(0.5, 0.5);
    let dist = distance(in.tex_coords, center);
    let vignette = 1.0 - smoothstep(0.3, 0.8, dist);

    let final_alpha = alpha * (0.5 + 0.5 * vignette);

    return vec4<f32>(overlay_color * pulse, final_alpha);
}
