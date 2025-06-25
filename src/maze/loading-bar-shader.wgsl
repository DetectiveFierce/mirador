
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@location(0) position: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.uv = (position + 1.0) * 0.5;
    return out;
}

struct Uniforms {
    progress: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Very thin bar at the top (1% of screen height)
    let bar_height = 0.01;
    let bar_y_max = 1.0; // Top of the screen
    let bar_y_min = bar_y_max - bar_height;

    // Check if we're in the loading bar's vertical area
    if (input.uv.y >= bar_y_min && input.uv.y <= bar_y_max) {
        // Background of the loading bar (transparent)
        var color = vec3<f32>(0.0, 0.0, 0.0);

        // Calculate progress across full width
        let progress_x = uniforms.progress;
        if (input.uv.x <= progress_x) {
            // Bright green progress indicator
            color = vec3<f32>(0.0, 1.0, 0.0);
        }
        return vec4<f32>(color, 1.0);
    }

    // Transparent elsewhere
    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}
