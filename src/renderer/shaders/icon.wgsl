@group(0) @binding(0)
var t_icon: texture_2d<f32>;
@group(0) @binding(1)
var s_icon: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(vertex.position, 0.0, 1.0);
    out.uv = vertex.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t_icon, s_icon, in.uv);
    
    // Only apply antialiasing if the texture has some alpha (not fully transparent)
    if (tex_color.a > 0.0) {
        // Calculate distance from center (assuming circular icon)
        let center = vec2<f32>(0.5, 0.5);
        let distance = length(in.uv - center);
        
        // Create smooth antialiased edge for the circular boundary
        let circle_edge_width = 0.012; // Wider edge for smoother circular boundary
        let circle_alpha = smoothstep(0.5 + circle_edge_width, 0.5 - circle_edge_width, distance);
        
        // Apply enhanced anti-aliasing to the icon's internal edges
        // Sample neighboring pixels to detect edges with multiple scales
        let pixel_size = 1.0 / 256.0; // Assuming 256x256 texture, adjust if different
        
        // Sample 8 neighboring pixels for more comprehensive edge detection
        let uv_offset = vec2<f32>(pixel_size, 0.0);
        let uv_offset_diag = vec2<f32>(pixel_size, pixel_size);
        
        // Horizontal and vertical samples
        let sample_right = textureSample(t_icon, s_icon, in.uv + uv_offset).a;
        let sample_left = textureSample(t_icon, s_icon, in.uv - uv_offset).a;
        let sample_up = textureSample(t_icon, s_icon, in.uv + vec2<f32>(0.0, pixel_size)).a;
        let sample_down = textureSample(t_icon, s_icon, in.uv - vec2<f32>(0.0, pixel_size)).a;
        
        // Diagonal samples for better edge detection
        let sample_ur = textureSample(t_icon, s_icon, in.uv + uv_offset_diag).a;
        let sample_ul = textureSample(t_icon, s_icon, in.uv + vec2<f32>(-pixel_size, pixel_size)).a;
        let sample_dr = textureSample(t_icon, s_icon, in.uv + vec2<f32>(pixel_size, -pixel_size)).a;
        let sample_dl = textureSample(t_icon, s_icon, in.uv - uv_offset_diag).a;
        
        // Calculate gradient magnitude for edge detection (more comprehensive)
        let grad_x = abs(sample_right - sample_left);
        let grad_y = abs(sample_up - sample_down);
        let grad_diag1 = abs(sample_ur - sample_dl);
        let grad_diag2 = abs(sample_ul - sample_dr);
        
        // Combine gradients for more accurate edge detection
        let gradient_magnitude = sqrt(grad_x * grad_x + grad_y * grad_y + 0.5 * (grad_diag1 * grad_diag1 + grad_diag2 * grad_diag2));
        
        // Apply enhanced edge smoothing with multiple smoothing levels
        let edge_smoothing_strong = smoothstep(0.05, 0.15, gradient_magnitude);
        let edge_smoothing_medium = smoothstep(0.1, 0.25, gradient_magnitude);
        let edge_smoothing_weak = smoothstep(0.15, 0.35, gradient_magnitude);
        
        // Multi-level smoothing for more natural transitions
        let smoothed_alpha_strong = mix(tex_color.a, 0.3, edge_smoothing_strong * 0.4);
        let smoothed_alpha_medium = mix(smoothed_alpha_strong, 0.5, edge_smoothing_medium * 0.3);
        let smoothed_alpha = mix(smoothed_alpha_medium, 0.7, edge_smoothing_weak * 0.2);
        
        // Combine circular boundary with enhanced internal edge smoothing
        let final_alpha = min(smoothed_alpha, circle_alpha);
        
        return vec4<f32>(tex_color.rgb, final_alpha);
    }
    
    // Return original texture if it's fully transparent
    return tex_color;
} 