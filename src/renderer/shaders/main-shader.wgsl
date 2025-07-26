//! # Maze Renderer Shader
//!
//! This WGSL shader is used for rendering the 3D maze and floor in the game.
//! It supports three materials: walls, floor, and debug bounding boxes.
//! - Walls are rendered with a solid maroon color
//! - The floor uses a checkerboard pattern with tan and purple tiles
//! - Debug bounding boxes are rendered as semitransparent red wireframes
//!
//! ## Structs
//! - `VertexInput`: Input structure for the vertex shader, containing position, color, and material ID.
//! - `VertexOutput`: Output structure from the vertex shader, passed to the fragment shader. Includes
//!   clip-space position, color, world-space XZ position, and material ID.
//!
//! ## Entry Points
//! - `vs_main`: Vertex shader. Transforms vertex positions by the MVP matrix, passes color and material.
//! - `fs_main`: Fragment shader. Applies material-based coloring. Walls are solid maroon; the floor
//!   uses a checkerboard pattern based on world position.
//!
//! ## Material Logic
//! - `material == 0`: Floor cell, colored with a checkerboard pattern alternating between tan and purple.
//! - `material == 1`: Wall cell, colored maroon (`vec4<f32>(0.102, 0.027, 0.035, 1.0)`).
//! - `material == 2`: Bounding box wireframe, colored semitransparent red.

struct VertexInput {
    /// Vertex position in model space.
    @location(0) position: vec3<f32>,
    /// Vertex color (unused in current fragment logic).
    @location(1) color: vec4<f32>,
    /// Material ID: 0 = floor, 1 = wall, 2 = bounding box, 3 = ceiling, 4 = exit.
    @location(2) material: u32,
    /// Texture coordinates for texturing (used for ceiling).
    @location(3) tex_coords: vec2<f32>,
};

struct VertexOutput {
    /// Clip-space position for rasterization.
    @builtin(position) clip_position: vec4<f32>,
    /// Vertex color (passed through, not used in fragment logic).
    @location(0) fragment_color: vec4<f32>,
    /// World-space XZ position, used for floor checkerboard and portal effect.
    @location(1) world_position: vec2<f32>,
    /// Material ID: 0 = floor, 1 = wall, 2 = bounding box, 3 = ceiling, 4 = exit.
    @location(2) material: u32,
    /// Texture coordinates for texturing (used for ceiling).
    @location(3) tex_coords: vec2<f32>,
};

/// Uniforms structure now includes time for animation
struct Uniforms {
    mvp_matrix: mat4x4<f32>,
    time: f32,
    _padding: vec3<f32>, // Padding for alignment
};

/// Updated uniform binding
@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

/// Ceiling texture binding
@group(0) @binding(1)
var ceiling_texture: texture_2d<f32>;

/// Ceiling texture sampler
@group(0) @binding(2)
var ceiling_sampler: sampler;

/// Vertex shader entry point.
/// Transforms vertex position by the MVP matrix and passes through color, world XZ, and material.
@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.mvp_matrix * vec4<f32>(in.position, 1.0);
    out.fragment_color = in.color;
    out.world_position = in.position.xz;
    out.material = in.material;
    out.tex_coords = in.tex_coords;
    return out;
}

// Portal effect functions (from your portal shader)
fn colormap_red(x: f32) -> f32 {
    return 0.0; // Minimal red for green output
}

fn colormap_green(x: f32) -> f32 {
    // Smooth green gradient from dark to bright
    if (x < 0.5) {
        return x * 1.5; // Darker greens for lower values
    } else {
        return 0.75 + (x - 0.5) * 0.5; // Brighter greens for higher values
    }
}

fn colormap_blue(x: f32) -> f32 {
    return 0.0; // Minimal blue for green output
}

fn colormap(x: f32) -> vec4<f32> {
    // Add some variation to make it more organic
    let green_value = colormap_green(x);
    let slight_blue = 0.1 * (1.0 - green_value); // Subtle blue in shadows
    return vec4<f32>(
        colormap_red(x),
        green_value,
        colormap_blue(x) + slight_blue,
        1.0
    );
}

fn rand(n: vec2<f32>) -> f32 {
    return fract(sin(dot(n, vec2<f32>(12.9898, 4.1414))) * 43758.547);
}

fn noise(p: vec2<f32>) -> f32 {
    let ip: vec2<f32> = floor(p);
    var u: vec2<f32> = fract(p);
    u = u * u * (3. - 2. * u);
    let res: f32 = mix(
        mix(rand(ip), rand(ip + vec2<f32>(1., 0.)), u.x),
        mix(rand(ip + vec2<f32>(0., 1.)), rand(ip + vec2<f32>(1., 1.)), u.x),
        u.y
    );
    return res * res;
}

// Workaround for matrix initialization
fn get_mtx() -> mat2x2<f32> {
    return mat2x2<f32>(
        vec2(0.8, 0.6),
        vec2(-0.6, 0.8)
    );
}

fn fbm(p: vec2<f32>) -> f32 {
    var p_var = p;
    var f: f32 = 0.;
    let mtx = get_mtx(); // Initialize matrix here
    f = f + (0.5 * noise(p_var + uniforms.time));
    p_var = mtx * p_var * 2.02;
    f = f + (0.03125 * noise(p_var));
    p_var = mtx * p_var * 2.01;
    f = f + (0.25 * noise(p_var));
    p_var = mtx * p_var * 2.03;
    f = f + (0.125 * noise(p_var));
    p_var = mtx * p_var * 2.01;
    f = f + (0.0625 * noise(p_var));
    p_var = mtx * p_var * 2.04;
    f = f + (0.015625 * noise(p_var + sin(uniforms.time)));
    return f / 0.96875;
}

fn pattern(p: vec2<f32>) -> f32 {
    return fbm(p + fbm(p + fbm(p)));
}

/// Fragment shader entry point.
/// Applies material-based coloring: walls are maroon, floor is a checkerboard, exit gets portal effect.
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Material-based coloring
    if (in.material == 1u) {
        // Wall: Maroon
        return vec4<f32>(0.102, 0.027, 0.035, 1.0);
    } else if (in.material == 2u) {
        // Bounding box: Semitransparent red
        return vec4<f32>(1.0, 0.0, 0.0, 0.3);
    } else if (in.material == 3u) {
        // Ceiling: Use tiled texture with high-contrast maroon/mauve recoloring
        let original_color = textureSample(ceiling_texture, ceiling_sampler, in.tex_coords);
        
        // Extract and analyze original channels
        let green_value = original_color.g;
        let blue_value = original_color.b;
        let red_value = original_color.r;
        
        // Calculate dominant channel and create dramatic contrast
        let dominant_channel = max(max(green_value, blue_value), red_value);
        let channel_ratio = green_value / (blue_value + 0.001); // Avoid division by zero
        
        // Create dramatic value variation based on original texture
        let dark_tiles = dominant_channel < 0.4;
        let medium_tiles = dominant_channel >= 0.4 && dominant_channel < 0.7;
        let light_tiles = dominant_channel >= 0.7;
        
        var final_color: vec4<f32>;
        
        if (dark_tiles) {
            // Dark tiles become very deep maroon/burgundy - almost black
            let dark_intensity = dominant_channel * 0.8; // Very low amplification
            final_color = vec4<f32>(
                0.02 + dark_intensity * 0.08, // Very deep red: 0.02-0.1
                dark_intensity * 0.01,        // Almost no green: 0.0-0.008
                dark_intensity * 0.02,        // Almost no blue: 0.0-0.016
                original_color.a
            );
        } else if (light_tiles) {
            // Light tiles become very dark mauve - just above starfield
            let light_intensity = (dominant_channel - 0.7) * 3.33; // 0.7-1.0 -> 0.0-1.0
            final_color = vec4<f32>(
                0.08 + light_intensity * 0.12,  // Very dark red: 0.08-0.2
                0.02 + light_intensity * 0.08,  // Very low green: 0.02-0.1
                0.05 + light_intensity * 0.1,   // Very dark blue: 0.05-0.15
                original_color.a
            );
        } else {
            // Medium tiles - determine if more maroon or mauve based on original
            let medium_intensity = (dominant_channel - 0.4) * 3.33; // 0.4-0.7 -> 0.0-1.0
            
            if (channel_ratio > 1.2) {
                // Originally more green - make very dark maroon
                final_color = vec4<f32>(
                    0.04 + medium_intensity * 0.08, // Very dark red: 0.04-0.12
                    medium_intensity * 0.015,       // Almost no green: 0.0-0.015
                    medium_intensity * 0.02,        // Almost no blue: 0.0-0.02
                    original_color.a
                );
            } else {
                // Originally more blue - make very dark mauve
                final_color = vec4<f32>(
                    0.05 + medium_intensity * 0.08,  // Very dark red: 0.05-0.13
                    medium_intensity * 0.02,         // Almost no green: 0.0-0.02
                    0.03 + medium_intensity * 0.08,  // Very dark blue: 0.03-0.11
                    original_color.a
                );
            }
        }
        
        // Add dramatic highlights and shadows based on original texture detail
        let texture_detail = abs(green_value - blue_value) + abs(red_value - green_value);
        if (texture_detail > 0.1) {
            // High detail areas get enhanced contrast
            final_color.r = clamp(final_color.r + texture_detail * 0.2, 0.0, 1.0);
            final_color.g = clamp(final_color.g + texture_detail * 0.1, 0.0, 1.0);
            final_color.b = clamp(final_color.b + texture_detail * 0.15, 0.0, 1.0);
        }
        
        return final_color;
    } else if (in.material == 4u) {
        // Exit cell: Animated portal effect
        // Scale the world position to get appropriate texture coordinates
        let portal_scale = 0.05; // Adjust this to control the portal pattern size
        let uv = in.world_position * portal_scale;
        let shade = pattern(uv);
        return colormap(shade);
    }

    // Floor: checkerboard
    let tan = vec4<f32>(0.941, 0.875, 0.62, 1.0);
    let purple = vec4<f32>(0.545, 0.455, 0.51, 1.0);
    let tile_size = 20.0;
    let grid = vec2<i32>(floor(in.world_position / tile_size));
    let checker = (grid.x + grid.y) % 2 != 0;
    return select(tan, purple, checker);
}
