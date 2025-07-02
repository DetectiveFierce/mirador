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
    let scaled_pos = input.position * 0.15; // Make it smaller
    let positioned = scaled_pos + vec2<f32>(0.7, -0.7); // Bottom-right in NDC

    out.clip_position = vec4<f32>(positioned.x, positioned.y, 0.0, 1.0);
    out.tex_coords = input.tex_coords;

    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Test with solid color first
    //return vec4<f32>(1.0, 0.0, 0.0, 0.8); // Semi-transparent red

    // Once you see the red square, uncomment this:
    let color = textureSample(compass_texture, compass_sampler, input.tex_coords);
    return color;
}
