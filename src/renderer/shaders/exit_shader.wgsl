struct Uniforms {
    iTime: f32,
    iResolution: vec2<f32>,
}
@group(0) @binding(0) var<uniform> uni: Uniforms;

// Vertex shader
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    // Full-screen triangle using switch instead of array indexing
    var pos: vec2<f32>;
    switch (vertex_index) {
        case 0u: {
            pos = vec2(-1.0, -1.0);
        }
        case 1u: {
            pos = vec2(3.0, -1.0);
        }
        default: {  // case 2
            pos = vec2(-1.0, 3.0);
        }
    }
    return vec4(pos, 0.0, 1.0);
}

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
    f = f + (0.5 * noise(p_var + uni.iTime ));
    p_var = mtx * p_var * 2.02;
    f = f + (0.03125 * noise(p_var));
    p_var = mtx * p_var * 2.01;
    f = f + (0.25 * noise(p_var));
    p_var = mtx * p_var * 2.03;
    f = f + (0.125 * noise(p_var));
    p_var = mtx * p_var * 2.01;
    f = f + (0.0625 * noise(p_var));
    p_var = mtx * p_var * 2.04;
    f = f + (0.015625 * noise(p_var + sin(uni.iTime)));
    return f / 0.96875;
}

fn pattern(p: vec2<f32>) -> f32 {
    return fbm(p + fbm(p + fbm(p)));
}

// Fragment shader
@fragment
fn fs_main(@builtin(position) frag_position: vec4<f32>) -> @location(0) vec4<f32> {
    let fragCoord = frag_position.xy;
    let uv: vec2<f32> = fragCoord / uni.iResolution.x;

    // Scale UV for higher definition pattern
    let scaled_uv = uv * 8.0; // Increase this value for even finer detail

    let shade: f32 = pattern(scaled_uv);
    return vec4<f32>(colormap(shade).rgb, shade);
}
