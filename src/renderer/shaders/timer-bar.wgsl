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

// Capsule SDF function for a horizontal capsule (rectangle with semicircular ends)
fn capsule_sdf(p: vec2<f32>, width: f32, height: f32) -> f32 {
    let radius = height * 0.5;
    let half_width = width * 0.5;
    
    // Clamp the point to the line segment (the "spine" of the capsule)
    let spine_half_length = max(0.0, half_width - radius);
    let clamped_x = clamp(p.x, -spine_half_length, spine_half_length);
    
    // Distance from point to the closest point on the spine
    let closest_point = vec2<f32>(clamped_x, 0.0);
    
    // Return distance to the circle centered at that point
    return length(p - closest_point) - radius;
}

@fragment
fn fs_main(@builtin(position) frag_position: vec4<f32>) -> @location(0) vec4<f32> {
    let fragCoord = frag_position.xy;
    let res = uniforms.resolution;
    
    // --- Bar dimensions: 1/3 width, 4% height ---
    let bar_width = res.x / 3.0;
    let bar_height = res.y * 0.04;
    let margin_top = res.y * 0.04;
    let bar_left = (res.x - bar_width) / 2.0;
    let bar_top = margin_top;
    
    // --- Convert to local coordinates centered at bar center ---
    let bar_center = vec2<f32>(bar_left + bar_width * 0.5, bar_top + bar_height * 0.5);
    let local_pos = fragCoord - bar_center;
    
    // --- Capsule SDF for mask ---
    let capsule_dist = capsule_sdf(local_pos, bar_width, bar_height);
    let edge_softness = 0.5; // Make edge very sharp to remove border
    let mask = 1.0 - smoothstep(0.0, edge_softness, capsule_dist);
    
    // --- Shadow calculation ---
    // Create a shadow that extends well beyond the visible area
    let shadow_offset = vec2<f32>(0.0, 3.0); // Slight downward offset
    let shadow_pos = local_pos - shadow_offset;
    let shadow_dist = capsule_sdf(shadow_pos, bar_width, bar_height);
    
    // Shadow parameters for smooth falloff
    let shadow_max_distance = bar_height * 4.0; // Shadow extends 4x bar height
    let shadow_intensity = 0.85; // Maximum shadow darkness
    
    // Create smooth shadow falloff using exponential decay
    let shadow_falloff = exp(-max(0.0, shadow_dist) / (bar_height * 0.8));
    let shadow_alpha = shadow_intensity * shadow_falloff;
    
    // Additional gaussian-like falloff for ultra-smooth edges
    let distance_factor = clamp(shadow_dist / shadow_max_distance, 0.0, 1.0);
    let gaussian_falloff = exp(-distance_factor * distance_factor * 8.0);
    let final_shadow_alpha = shadow_alpha * gaussian_falloff;
    
    // Shadow color (dark with slight blue tint)
    let shadow_color = vec3<f32>(0.0, 0.01, 0.04); // deeper, less blue
    
    // --- Check if we need to render anything ---
    if (mask < 0.01 && final_shadow_alpha < 0.01) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    
    // --- Progress fill calculation (only if we're in the bar area) ---
    var bar_color = vec4<f32>(0.12, 0.12, 0.14, 1.0); // Default background

    if (mask > 0.01) {
        // Map to [0,1] in bar local space for UV calculations
        let rel_x = (fragCoord.x - bar_left) / bar_width;
        let rel_y = (fragCoord.y - bar_top) / bar_height;
        let uv = vec2<f32>(rel_x, rel_y);

        // --- Flat right edge progress fill ---
        // Only draw fill if progress > 0
        var final_progress_mask = 0.0;
        if (uniforms.progress > 0.0) {
            // Compute the left-capped rectangle SDF
            let fill_width = bar_width * uniforms.progress;
            let fill_left = bar_left;
            let fill_right = bar_left + fill_width;
            let fill_center = vec2<f32>(fill_left + fill_width * 0.5, bar_center.y);
            let fill_local_pos = vec2<f32>(fragCoord.x - fill_center.x, fragCoord.y - bar_center.y);
            let radius = bar_height * 0.5;
            let half_width = fill_width * 0.5;
            // SDF for left semicircle
            let left_cap_center = vec2<f32>(-half_width + radius, 0.0);
            let left_cap_dist = length(fill_local_pos - left_cap_center) - radius;
            // SDF for rectangle (flat right edge)
            let rect_right = half_width;
            let rect_left = -half_width + radius;
            let rect_dist = max(abs(fill_local_pos.y) - radius, fill_local_pos.x - rect_right);
            let in_rect = step(rect_left, fill_local_pos.x) * step(fill_local_pos.x, rect_right);
            // Combine: inside if in left cap or in rectangle
            let fill_sdf = min(left_cap_dist, rect_dist);
            let edge_softness = 0.5;
            let fill_mask = 1.0 - smoothstep(0.0, edge_softness, fill_sdf);
            // Only allow fill to the right of the left cap
            let right_clip = step(fill_local_pos.x, rect_right);
            final_progress_mask = mask * fill_mask * right_clip;
        }

        // --- Timer bar color ---
        let scaled_uv = uv * vec2<f32>(16.0, 1.0);
        let shade: f32 = pattern(scaled_uv);
        let animated_color = colormap(shade);

        // --- Glass highlight overlay (for both filled and depleted areas) ---
        let highlight_curve = pow(clamp(1.0 - abs((rel_x - 0.22) * 2.0), 0.0, 1.0), 3.0);
        let highlight_band = smoothstep(0.10, 0.0, rel_y - 0.18);
        let highlight = highlight_curve * highlight_band * 0.65;
        let highlight2_curve = pow(clamp(1.0 - abs((rel_x - 0.78) * 2.0), 0.0, 1.0), 2.5);
        let highlight2_band = smoothstep(0.10, 0.0, 0.82 - rel_y);
        let highlight2 = highlight2_curve * highlight2_band * 0.25;
        let glass_highlight = vec3<f32>(1.0, 1.0, 1.0) * (highlight + highlight2);

        // --- Glass tint and inner shadow (for both areas) ---
        let glass_tint = vec3<f32>(0.75, 0.90, 1.0) * 0.18;
        let shadow = smoothstep(0.0, 0.18 * bar_height, capsule_dist);
        let inner_shadow = vec3<f32>(0.0, 0.05, 0.10) * shadow * 0.45;

        // --- Compose filled and depleted area colors ---
        let filled_rgb = animated_color.rgb + glass_highlight + glass_tint - inner_shadow;
        let depleted_rgb = glass_highlight + glass_tint - inner_shadow;
        let depleted_alpha = 0.22; // glassy transparency for depleted area

        // Mix between filled and depleted area
        let rgb = mix(depleted_rgb, filled_rgb, final_progress_mask);
        let alpha = mix(depleted_alpha, 1.0, final_progress_mask);
        bar_color = vec4<f32>(rgb, alpha);
    }
    
    // --- Endcap shadow for realism ---
    // Compute how far along the bar's axis this pixel is (0=center, -1/1=ends)
    let axis_pos = clamp(local_pos.x / ((bar_width - bar_height) * 0.5), -1.0, 1.0);
    // Shadow strength is stronger near the ends (rounded parts)
    let end_shadow_strength = pow(abs(axis_pos), 2.2);
    let end_shadow = vec3<f32>(0.0, 0.0, 0.0) * end_shadow_strength * 0.22;
    // Blend end shadow into bar color (subtle)
    bar_color = vec4<f32>(bar_color.rgb - end_shadow, bar_color.a);
    
    // --- Tube border and rim lighting ---
    let border_width = bar_height * 0.18;
    let border_mask = 1.0 - smoothstep(border_width * 0.5, border_width, abs(capsule_dist));
    // Approximate normal (gradient of SDF)
    let eps = 0.5;
    let grad_x = capsule_sdf(local_pos + vec2<f32>(eps, 0.0), bar_width, bar_height) - capsule_sdf(local_pos - vec2<f32>(eps, 0.0), bar_width, bar_height);
    let grad_y = capsule_sdf(local_pos + vec2<f32>(0.0, eps), bar_width, bar_height) - capsule_sdf(local_pos - vec2<f32>(0.0, eps), bar_width, bar_height);
    let normal = normalize(vec2<f32>(grad_x, grad_y));
    // Rim lighting: lighter at top, darker at bottom
    let rim_light = clamp(normal.y, 0.0, 1.0);
    let rim_shadow = clamp(-normal.y, 0.0, 1.0);
    let border_light = vec3<f32>(0.22, 0.24, 0.32) + rim_light * 0.10; // subtle light
    let border_dark = vec3<f32>(0.08, 0.09, 0.13) + rim_shadow * 0.08; // subtle shadow
    let border_color = mix(border_light, border_dark, rim_shadow);
    // Blend border into bar color (subtle)
    bar_color = vec4<f32>(mix(bar_color.rgb, border_color, border_mask * 0.32), bar_color.a);
    
    // --- Combine shadow and bar ---
    if (mask > 0.01) {
        // We're in the bar area - show the bar only (no shadow blending)
        return bar_color;
    } else {
        // We're in the shadow area only
        return vec4<f32>(shadow_color, final_shadow_alpha);
    }
}