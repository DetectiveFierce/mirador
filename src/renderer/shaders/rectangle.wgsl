struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) rect_size: vec2<f32>,
    @location(4) corner_radius: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) rect_size: vec2<f32>,
    @location(3) corner_radius: f32,
}

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(vertex.position, 0.0, 1.0);
    out.color = vertex.color;
    out.uv = vertex.uv;
    out.rect_size = vertex.rect_size;
    out.corner_radius = vertex.corner_radius;
    return out;
}

// Signed distance function for a rounded rectangle
fn sdf_rounded_rect(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let half_size = size * 0.5;
    let d = abs(p - half_size) - half_size + radius;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0) - radius;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // If corner radius is 0, just return the color (no rounding)
    if (in.corner_radius <= 0.0) {
        return in.color;
    }
    
    // Calculate the signed distance from the current fragment to the rounded rectangle edge
    let distance = sdf_rounded_rect(in.uv, in.rect_size, in.corner_radius);
    
    // Use smoothstep for anti-aliasing
    let alpha = 1.0 - smoothstep(-1.0, 1.0, distance);
    
    // Apply the alpha to the color
    var output_color = in.color;
    output_color.a *= alpha;
    
    return output_color;
}