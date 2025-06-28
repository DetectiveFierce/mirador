struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) properties: vec4<f32>, // size, brightness, tex_x, tex_y
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) brightness: f32,
    @location(2) star_position: vec2<f32>, // Pass through for twinkling calculation
}

// Uniforms
@group(0) @binding(0)
var<uniform> time: f32;

@group(0) @binding(1)
var<uniform> background_color: vec4<f32>;

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(vertex.position, 0.0, 1.0);
    out.tex_coords = vertex.properties.zw; // tex_x, tex_y
    out.brightness = vertex.properties.y;  // brightness
    out.star_position = vertex.position;   // Use position as seed for twinkling
    return out;
}

// Simple hash function to generate pseudo-random values from position
fn hash(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    let p3_dot = p3.x + p3.y + p3.z;
    return fract((p3.x + p3.y) * p3_dot);
}

// Function to create a four-pointed star shape with sharp points and deep concave sides
fn star_shape(uv: vec2<f32>) -> f32 {
    // Transform UV coordinates to center around origin
    let centered_uv = uv - vec2<f32>(0.5, 0.5);

    // Use diamond distance for sharp four-pointed star
    let abs_uv = abs(centered_uv);

    // Create the classic four-pointed star using diamond distance
    // This creates sharp points at 45-degree angles
    let diamond_dist = abs_uv.x + abs_uv.y;

    // Create the concave curves by using a different metric
    // This creates the "pinched" waist effect
    let concave_dist = max(abs_uv.x, abs_uv.y);

    // Combine both distances to create the star shape
    // The star is defined by the intersection of these two shapes
    let star_boundary = diamond_dist - concave_dist * 0.7;

    // Create smaller star with very sharp points
    let star_size = 0.12; // Much smaller stars
    let star_alpha = 1.0 - smoothstep(0.0, 0.005, star_boundary - star_size); // Very sharp falloff

    // Add minimal glow for visibility
    let glow_size = 0.14;
    let glow_alpha = (1.0 - smoothstep(0.0, 0.015, star_boundary - glow_size)) * 0.2;

    // Sharp center core
    let core_size = 0.06;
    let core_alpha = 1.0 - smoothstep(0.0, 0.002, star_boundary - core_size);

    return max(max(star_alpha, glow_alpha), core_alpha);
}

// Function to calculate opposite color
fn opposite_color(bg_color: vec3<f32>) -> vec3<f32> {
    // Calculate luminance to determine if background is light or dark
    let luminance = dot(bg_color, vec3<f32>(0.299, 0.587, 0.114));

    // Simple inversion for high contrast
    let inverted = vec3<f32>(1.0) - bg_color;

    // For better visibility, we can also boost contrast
    let contrast_boost = 1.5;
    let contrasted = clamp(inverted * contrast_boost, vec3<f32>(0.0), vec3<f32>(1.0));

    // Ensure minimum brightness difference
    let min_diff = 0.3;
    if (luminance > 0.5) {
        // Background is light, make star dark
        return min(contrasted, vec3<f32>(1.0 - min_diff));
    } else {
        // Background is dark, make star light
        return max(contrasted, vec3<f32>(min_diff));
    }
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Create four-pointed star shape
    let star_alpha = star_shape(in.tex_coords);

    // Early exit if pixel is outside star shape
    if (star_alpha <= 0.01) {
        discard;
    }

    // Generate unique twinkling parameters for each star based on position
    let star_seed = hash(in.star_position);
    let star_seed2 = hash(in.star_position + vec2<f32>(1.234, 5.678));
    let star_seed3 = hash(in.star_position + vec2<f32>(9.101, 1.121));

    // Create different twinkling rates for each star
    let twinkle_speed1 = 0.5 + star_seed * 3.0;      // 0.5 to 3.5 cycles per time unit
    let twinkle_speed2 = 0.3 + star_seed2 * 2.0;     // 0.3 to 2.3 cycles per time unit
    let twinkle_speed3 = 0.8 + star_seed3 * 4.0;     // 0.8 to 4.8 cycles per time unit

    // Create multiple sine wave components for complex twinkling
    let twinkle1 = sin(time * twinkle_speed1 * 0.25) * 0.5 + 0.5;
    let twinkle2 = sin(time * twinkle_speed2 + star_seed * 6.28) * 0.3 + 0.7;
    let twinkle3 = sin(time * twinkle_speed3 + star_seed2 * 6.28) * 0.2 + 0.8;

    // Combine twinkling effects
    let combined_twinkle = twinkle1 * twinkle2 * twinkle3;

    // Occasionally add a bright flash
    let flash_rate = 0.1 + star_seed3 * 0.2; // Very slow flash rate
    let flash = sin(time * flash_rate + star_seed * 6.28);
    let flash_intensity = max(0.0, flash * flash * flash * flash); // Sharp peaks

    // Calculate final brightness
    let base_brightness = in.brightness * combined_twinkle;
    let final_brightness = base_brightness + flash_intensity * 0.5;

    // Calculate star color as opposite of background
    let star_color = opposite_color(background_color.rgb);

    // Apply brightness to the star color
    let final_color = star_color * final_brightness;

    // Apply twinkling to alpha as well
    let final_alpha = star_alpha * combined_twinkle * in.brightness;

    return vec4<f32>(final_color, final_alpha);
}
