struct TimerBarUniforms {
    progress: f32,
    time: f32,
    resolution: vec2<f32>,
    _padding: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: TimerBarUniforms;

// Vertex shader
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    // Full-screen triangle
    var pos: vec2<f32>;
    switch (vertex_index) {
        case 0u: { pos = vec2(-1.0, -1.0); }
        case 1u: { pos = vec2(3.0, -1.0); }
        default: { pos = vec2(-1.0, 3.0); }
    }
    return vec4<f32>(pos, 0.0, 1.0);
}

// Enhanced contrast colormap with warmer highlights and cooler shadows
fn colormap_red(x: f32) -> f32 {
    // Enhance contrast curve and add warmth to highlights
    let contrast_x = pow(x, 0.8); // Slight gamma adjustment for more punch
    
    if (contrast_x < 0.0) {
        return 15.0 / 255.0; // Darker, cooler shadows
    } else if (contrast_x < 0.2) {
        return (150.0 * contrast_x + 15.0) / 255.0;
    } else if (contrast_x < 0.5) {
        return (320.0 * contrast_x + 25.0) / 255.0;
    } else if (contrast_x < 0.8) {
        return (280.0 * contrast_x + 45.0) / 255.0;
    } else {
        // Much brighter, warmer highlights
        return (400.0 * contrast_x + 150.0) / 255.0;
    }
}

fn colormap_green(x: f32) -> f32 {
    let contrast_x = pow(x, 0.8);
    
    if (contrast_x < 0.3) {
        return 0.0; // Keep shadows very dark
    } else if (contrast_x < 0.5) {
        return (60.0 * contrast_x - 18.0) / 255.0;
    } else if (contrast_x < 0.7) {
        return (120.0 * contrast_x - 48.0) / 255.0;
    } else if (contrast_x <= 1.0) {
        // Warmer highlights with more green
        return (200.0 * contrast_x - 88.0) / 255.0;
    } else {
        return 1.0;
    }
}

fn colormap_blue(x: f32) -> f32 {
    let contrast_x = pow(x, 0.8);
    
    if (contrast_x < 0.0) {
        return 95.0 / 255.0; // Cooler, bluer shadows
    } else if (contrast_x < 0.15) {
        return (400.0 * contrast_x + 95.0) / 255.0;
    } else if (contrast_x < 0.4) {
        return (320.0 * contrast_x + 107.0) / 255.0;
    } else if (contrast_x < 0.7) {
        return (250.0 * contrast_x + 135.0) / 255.0;
    } else {
        // Reduce blue in highlights for warmer tone
        return (80.0 * contrast_x + 254.0) / 255.0;
    }
}

fn colormap(x: f32) -> vec4<f32> {
    // Apply additional contrast curve
    let enhanced_x = pow(clamp(x, 0.0, 1.0), 0.9);
    
    // Get base color
    let base_color = vec3<f32>(
        colormap_red(enhanced_x), 
        colormap_green(enhanced_x), 
        colormap_blue(enhanced_x)
    );
    
    // Additional contrast and temperature adjustment
    let contrast_factor = 1.4; // Increase overall contrast
    let mid_point = 0.5;
    
    // Apply S-curve for more dramatic contrast
    var adjusted_color = (base_color - mid_point) * contrast_factor + mid_point;
    
    // Temperature shift: cooler shadows, warmer highlights
    let temp_factor = (enhanced_x - 0.5) * 0.3; // -0.15 to +0.15
    adjusted_color.r += temp_factor * 0.4; // More red in highlights
    adjusted_color.g += temp_factor * 0.2; // Slight green boost in highlights
    adjusted_color.b -= temp_factor * 0.3; // Less blue in highlights, more in shadows
    
    return vec4<f32>(clamp(adjusted_color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}

fn rand(n: vec2<f32>) -> f32 {
    return fract(sin(dot(n, vec2<f32>(12.9898, 4.1414))) * 43758.5453);
}

fn noise(p: vec2<f32>) -> f32 {
    let ip: vec2<f32> = floor(p);
    var u: vec2<f32> = fract(p);
    u = u * u * (3.0 - 2.0 * u);
    let res: f32 = mix(
        mix(rand(ip), rand(ip + vec2<f32>(1.0, 0.0)), u.x),
        mix(rand(ip + vec2<f32>(0.0, 1.0)), rand(ip + vec2<f32>(1.0, 1.0)), u.x),
        u.y
    );
    return res * res;
}

fn get_mtx() -> mat2x2<f32> {
    return mat2x2<f32>(
        vec2(0.80, 0.60),
        vec2(-0.60, 0.80)
    );
}

fn fbm(p: vec2<f32>) -> f32 {
    var p_var = p;
    var f: f32 = 0.0;
    let mtx = get_mtx();
    
    // More violent shifting - increased time multipliers and chaos
    let violent_time = uniforms.time * 2.5;
    let chaos_factor = sin(uniforms.time * 3.7) * 0.3;
    
    f = f + (0.500000 * noise(p_var + violent_time + chaos_factor));
    p_var = mtx * p_var * 2.02;
    f = f + (0.031250 * noise(p_var + sin(violent_time * 1.4)));
    p_var = mtx * p_var * 2.01;
    f = f + (0.250000 * noise(p_var + violent_time * 0.7));
    p_var = mtx * p_var * 2.03;
    f = f + (0.125000 * noise(p_var + sin(violent_time * 2.1)));
    p_var = mtx * p_var * 2.01;
    f = f + (0.062500 * noise(p_var + violent_time * 1.3));
    p_var = mtx * p_var * 2.04;
    f = f + (0.015625 * noise(p_var + sin(violent_time * 1.8) + chaos_factor));
    
    return f / 0.96875;
}

fn pattern(p: vec2<f32>) -> f32 {
    // Add more violent distortion with additional chaos layers
    let chaos1 = sin(uniforms.time * 4.2) * 0.2;
    let chaos2 = sin(uniforms.time * 2.8) * 0.15;
    
    return fbm(p + fbm(p + fbm(p + vec2<f32>(chaos1, chaos2))));
}

@fragment
fn fs_main(@builtin(position) frag_position: vec4<f32>) -> @location(0) vec4<f32> {
    let fragCoord = frag_position.xy;
    let res = uniforms.resolution;
    
    // Timer bar dimensions
    let bar_width = res.x / 3.0;
    let bar_height = res.y * 0.03;
    let margin_top = res.y * 0.04;
    let bar_left = (res.x - bar_width) / 2.0;
    let bar_right = bar_left + bar_width;
    let bar_top = margin_top;
    let bar_bottom = margin_top + bar_height;
    
    // Only draw inside the bar
    if (fragCoord.x < bar_left || fragCoord.x > bar_right || fragCoord.y < bar_top || fragCoord.y > bar_bottom) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    
    // Progress mask
    let rel_x = (fragCoord.x - bar_left) / bar_width;
    let progress_mask = step(rel_x, uniforms.progress);
    
    // UV coordinates - scale like the reference shader
    let uv = vec2<f32>(rel_x, (fragCoord.y - bar_top) / bar_height);
    let scaled_uv = uv * vec2<f32>(16.0, 1.0);
    
    let shade: f32 = pattern(scaled_uv);
    let animated_color = colormap(shade);
    let bg_color = vec4<f32>(0.12, 0.12, 0.14, 1.0); // Slightly cooler, darker background
    
    // Apply progress mask
    let final_color = mix(bg_color, animated_color, progress_mask);
    return vec4<f32>(final_color.rgb, 1.0);
}