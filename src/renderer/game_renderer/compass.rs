//! Compass rendering system for directional navigation.
//!
//! This module provides a complete compass rendering system that displays
//! a directional indicator pointing toward the maze exit. The compass uses
//! a base texture with an animated needle that rotates to show the correct
//! direction relative to the player's orientation.
//!
//! # Features
//!
//! - **Directional Navigation**: Points toward the maze exit from player position
//! - **Player Orientation**: Accounts for player's current facing direction
//! - **Smooth Animation**: Interpolated needle rotation for fluid movement
//! - **Multiple Needle Frames**: 12-directional needle sprites for precise indication
//! - **Screen Positioning**: Configurable position and size via uniforms
//!
//! # Usage
//!
//! ```rust
//! // Create compass renderer
//! let compass = CompassRenderer::new(device, queue, surface_config);
//!
//! // Update direction based on player and exit positions
//! compass.update_compass_with_yaw(player_pos, exit_pos, player_yaw);
//!
//! // Render compass overlay
//! compass.render(&mut render_pass, window);
//! ```
//!
//! # Texture Requirements
//!
//! The compass system expects the following texture files:
//! - `assets/compass/gold-compass.png` - Base compass background
//! - `assets/compass/needle/needle-0.png` through `needle-11.png` - Needle sprites
//!
//! # Coordinate System
//!
//! The compass uses normalized screen coordinates (0.0 to 1.0) for positioning
//! and world coordinates for direction calculations. The needle rotation is
//! calculated relative to the player's forward direction.

use crate::renderer::pipeline_builder::BindGroupLayoutBuilder;
use crate::renderer::pipeline_builder::PipelineBuilder;
use crate::renderer::pipeline_builder::create_uniform_buffer;
use wgpu;
use wgpu::util::DeviceExt;
use crate::assets;
use image;

/// Uniform data for compass positioning and sizing.
///
/// This struct contains the data sent to the GPU shader to control
/// the compass position and size on screen. The data is packed to
/// match GPU memory alignment requirements.
///
/// # Memory Layout
///
/// - `screen_position`: Normalized screen coordinates [x, y] (0.0 to 1.0)
/// - `compass_size`: Size as fraction of screen [width, height] (0.0 to 1.0)
/// - `_padding`: Ensures proper GPU memory alignment
///
/// # Default Values
///
/// - Position: Bottom-right corner (0.85, 0.85)
/// - Size: 12% of screen (0.12, 0.12)
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CompassUniforms {
    /// Screen position in normalized coordinates (0.0 to 1.0).
    /// Origin is at bottom-left, with (1.0, 1.0) at top-right.
    screen_position: [f32; 2],

    /// Compass size as fraction of screen dimensions.
    /// Values of 0.12 mean 12% of screen width/height.
    compass_size: [f32; 2],

    /// Padding for GPU memory alignment requirements.
    _padding: [f32; 4],
}

/// Compass renderer for directional navigation overlay.
///
/// This struct manages the complete compass rendering system, including
/// texture loading, shader pipeline setup, and directional calculations.
/// The compass provides visual feedback to help players navigate toward
/// the maze exit.
///
/// # Rendering Pipeline
///
/// The compass uses a two-pass rendering approach:
/// 1. **Base Pass**: Renders the compass background texture
/// 2. **Needle Pass**: Renders the directional needle on top
///
/// # Texture Management
///
/// - **Base Texture**: Static compass background (gold-compass.png)
/// - **Needle Textures**: 12 directional needle sprites (needle-0.png to needle-11.png)
/// - **Bind Groups**: Separate bind groups for base and each needle direction
///
/// # Smoothing System
///
/// The compass uses exponential smoothing to prevent jarring needle movements:
/// - `smoothing_factor`: Controls responsiveness (0.01 = very smooth, 1.0 = instant)
/// - `smoothed_compass_angle`: Current interpolated angle for needle selection
///
/// # Performance Characteristics
///
/// - **GPU Memory**: Pre-allocated textures and bind groups
/// - **CPU Usage**: Minimal per-frame calculations
/// - **Rendering**: Two draw calls per frame (base + needle)
///
/// # Thread Safety
///
/// This struct is not thread-safe and should only be accessed from the
/// main rendering thread.
///
/// # Example
///
/// ```rust
/// # use crate::renderer::game_renderer::compass::CompassRenderer;
/// # let device: wgpu::Device = unimplemented!();
/// # let queue: wgpu::Queue = unimplemented!();
/// # let surface_config: wgpu::SurfaceConfiguration = unimplemented!();
///
/// let compass = CompassRenderer::new(&device, &queue, &surface_config);
///
/// // Update direction
/// compass.update_compass_with_yaw(player_pos, exit_pos, player_yaw);
///
/// // Render
/// compass.render(&mut render_pass, window);
/// ```
pub struct CompassRenderer {
    /// WGPU render pipeline for compass rendering.
    ///
    /// Handles vertex processing, fragment shading, and blending for
    /// the compass overlay. Uses alpha blending for semitransparent rendering.
    pipeline: wgpu::RenderPipeline,

    /// Vertex buffer containing compass quad geometry.
    ///
    /// Contains a simple quad (-1 to 1) that gets positioned and scaled
    /// via uniforms in the vertex shader.
    vertex_buffer: wgpu::Buffer,

    /// Uniform buffer for compass position and size data.
    ///
    /// Contains `CompassUniforms` struct that controls where and how
    /// large the compass appears on screen.
    uniform_buffer: wgpu::Buffer,

    /// Bind group for the compass base texture.
    ///
    /// Contains the background texture, sampler, and uniform buffer
    /// for rendering the compass base.
    base_bind_group: wgpu::BindGroup,

    /// Bind groups for each needle direction texture.
    ///
    /// Array of 12 bind groups, one for each needle sprite (0-11).
    /// Each bind group contains the needle texture, sampler, and uniform buffer.
    needle_bind_groups: Vec<wgpu::BindGroup>,

    /// Current needle sprite index (0-11).
    ///
    /// Determines which needle texture is currently rendered.
    /// Updated based on calculated direction to exit.
    current_needle_index: usize,

    /// Current smoothed compass angle in radians.
    ///
    /// This is the interpolated angle used for needle selection.
    /// Ranges from 0 to 2π and is updated with smoothing applied.
    smoothed_compass_angle: f32,

    /// Smoothing factor for compass movement (0.01 to 1.0).
    ///
    /// Controls how quickly the needle responds to direction changes.
    /// Lower values = smoother but slower response.
    /// Higher values = faster but potentially jittery response.
    smoothing_factor: f32,
}

impl CompassRenderer {
    /// Creates a new `CompassRenderer` instance and initializes all GPU resources required for compass rendering.
    ///
    /// This function loads the compass base texture and all 12 needle textures, creates the uniform buffer,
    /// sets up the bind group layout for textures, samplers, and uniforms, and builds the render pipeline.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        // Load compass base texture
        let base_texture = Self::load_compass_base_texture(device, queue);

        // Load all needle textures
        let needle_textures = Self::load_needle_textures(device, queue);

        let uniforms = CompassUniforms {
            screen_position: [0.85, 0.85], // Bottom-right corner (normalized coordinates)
            compass_size: [0.12, 0.12],    // 12% of screen size
            _padding: [0.0; 4],
        };

        let uniform_buffer = create_uniform_buffer(device, &uniforms, "Compass Uniform Buffer");

        // Create bind group layout for texture + sampler + uniforms
        let bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label("Compass Bind Group Layout")
            .with_uniform_buffer(0, wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT)
            .with_texture(1, wgpu::ShaderStages::FRAGMENT)
            .with_sampler(2, wgpu::ShaderStages::FRAGMENT)
            .build();

        // Create sampler for all textures
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create bind group for base texture
        let base_texture_view = base_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let base_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&base_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("Compass Base Bind Group"),
        });

        // Create bind groups for each needle texture
        let needle_bind_groups: Vec<wgpu::BindGroup> = needle_textures
            .iter()
            .enumerate()
            .map(|(i, texture)| {
                let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: uniform_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                    ],
                    label: Some(&format!("Compass Needle Bind Group {}", i)),
                })
            })
            .collect();

        // Create vertex buffer layout for position + tex_coords
        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: 4 * 4, // 4 floats * 4 bytes each
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2, // position
                },
                wgpu::VertexAttribute {
                    offset: 2 * 4, // 2 floats * 4 bytes
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2, // tex_coords
                },
            ],
        };

        let pipeline = PipelineBuilder::new(device, surface_config.format)
            .with_label("Compass Pipeline")
            .with_shader(include_str!("../shaders/compass.wgsl"))
            .with_vertex_buffer(vertex_buffer_layout)
            .with_bind_group_layout(&bind_group_layout)
            .with_alpha_blending()
            .build();

        let vertex_buffer = Self::create_compass_vertices(device);

        Self {
            pipeline,
            vertex_buffer,
            uniform_buffer,
            base_bind_group,
            needle_bind_groups,
            current_needle_index: 0,

            smoothed_compass_angle: 0.0,
            smoothing_factor: 0.8, // Higher = more responsive, lower = smoother
        }
    }

    fn load_compass_base_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
        // Load image from embedded assets
        let img = match image::load_from_memory(assets::GOLD_COMPASS) {
            Ok(img) => img.to_rgba8(),
            Err(e) => {
                eprintln!("Failed to load compass base texture from embedded assets: {}", e);
                // Create a fallback texture (solid color or default compass)
                image::RgbaImage::new(64, 64)
            }
        };

        let dimensions = img.dimensions();
        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Compass Base Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &img,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        texture
    }

    fn load_needle_textures(device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<wgpu::Texture> {
        let mut textures = Vec::new();

        // Load needle textures from embedded assets
        for (i, needle_data) in assets::compass_needles().iter().enumerate() {
            // Load image from embedded assets
            let img = match image::load_from_memory(needle_data) {
                Ok(img) => img.to_rgba8(),
                Err(e) => {
                    eprintln!("Failed to load needle texture {} from embedded assets: {}", i, e);
                    // Create a fallback texture (transparent or simple needle)
                    image::RgbaImage::new(64, 64)
                }
            };

            let dimensions = img.dimensions();
            let texture_size = wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            };

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("Compass Needle Texture {}", i)),
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    aspect: wgpu::TextureAspect::All,
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                &img,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * dimensions.0),
                    rows_per_image: Some(dimensions.1),
                },
                texture_size,
            );

            textures.push(texture);
        }

        textures
    }

    fn create_compass_vertices(device: &wgpu::Device) -> wgpu::Buffer {
        // Create a quad for the compass (will be positioned via uniforms in shader)
        // Raw vertex data: [x, y, u, v] for each vertex
        let vertices: &[f32] = &[
            // Triangle 1
            -1.0, -1.0, 0.0, 1.0, // Bottom-left
            1.0, -1.0, 1.0, 1.0, // Bottom-right
            -1.0, 1.0, 0.0, 0.0, // Top-left
            // Triangle 2
            1.0, -1.0, 1.0, 1.0, // Bottom-right
            1.0, 1.0, 1.0, 0.0, // Top-right
            -1.0, 1.0, 0.0, 0.0, // Top-left
        ];

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Compass Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        })
    }

    /// Updates the compass position and size on screen.
    ///
    /// This method uploads new position and size data to the GPU uniform buffer,
    /// allowing dynamic repositioning and resizing of the compass overlay.
    /// The position and size are specified in normalized screen coordinates.
    ///
    /// # Coordinate System
    ///
    /// - **Position**: (0.0, 0.0) = bottom-left, (1.0, 1.0) = top-right
    /// - **Size**: Values represent fraction of screen (0.12 = 12% of screen)
    /// - **Origin**: Position is relative to the compass center
    ///
    /// # Performance Notes
    ///
    /// This operation involves a GPU buffer write. It's typically called when
    /// the window is resized or when the UI layout changes.
    ///
    /// # Parameters
    ///
    /// - `queue` - WGPU queue for buffer uploads
    /// - `screen_position` - Normalized screen position [x, y] (0.0 to 1.0)
    /// - `compass_size` - Normalized size [width, height] (0.0 to 1.0)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use crate::renderer::game_renderer::compass::CompassRenderer;
    /// # let compass: CompassRenderer = unimplemented!();
    /// # let queue: wgpu::Queue = unimplemented!();
    ///
    /// // Position in bottom-right corner, 12% of screen size
    /// compass.update_uniforms(&queue, [0.85, 0.85], [0.12, 0.12]);
    ///
    /// // Position in top-left corner, 8% of screen size
    /// compass.update_uniforms(&queue, [0.08, 0.92], [0.08, 0.08]);
    /// ```
    pub fn update_uniforms(
        &self,
        queue: &wgpu::Queue,
        screen_position: [f32; 2],
        compass_size: [f32; 2],
    ) {
        let uniforms = CompassUniforms {
            screen_position,
            compass_size,
            _padding: [0.0; 4],
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Renders the compass overlay to the current render pass.
    ///
    /// This method performs a two-pass rendering approach:
    /// 1. Renders the compass base texture (background)
    /// 2. Renders the current needle texture on top
    ///
    /// The compass is rendered as an overlay using alpha blending,
    /// allowing it to appear on top of the game scene while maintaining
    /// transparency for visual integration.
    ///
    /// # Render State Requirements
    ///
    /// This method assumes:
    /// - A render pass is active and configured for alpha blending
    /// - The game scene has already been rendered
    /// - The viewport is set to the full screen dimensions
    ///
    /// # Performance Characteristics
    ///
    /// - **Draw Calls**: 2 draw calls per frame (base + needle)
    /// - **GPU Memory**: Minimal - only vertex buffer and bind group switches
    /// - **CPU Usage**: Negligible - only pipeline and bind group setup
    ///
    /// # Parameters
    ///
    /// - `render_pass` - Active render pass to render into
    /// - `window` - Window reference (currently unused, reserved for future use)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use crate::renderer::game_renderer::compass::CompassRenderer;
    /// # let compass: CompassRenderer = unimplemented!();
    /// # let mut render_pass: wgpu::RenderPass = unimplemented!();
    /// # let window: &winit::window::Window = unimplemented!();
    ///
    /// // Render game scene first
    /// // ... render background, maze, player, etc ...
    ///
    /// // Render compass overlay on top
    /// compass.render(&mut render_pass, window);
    /// ```
    pub fn render(&self, render_pass: &mut wgpu::RenderPass, _window: &winit::window::Window) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        // First render the compass base
        render_pass.set_bind_group(0, &self.base_bind_group, &[]);
        render_pass.draw(0..6, 0..1);

        // Then render the needle on top
        render_pass.set_bind_group(0, &self.needle_bind_groups[self.current_needle_index], &[]);
        render_pass.draw(0..6, 0..1);
    }

    /// Calculate which needle image to show based on player and exit positions
    pub fn update_compass_direction(&mut self, player_pos: (f32, f32), exit_pos: (f32, f32)) {
        // Vector from player to exit
        let direction_vector = (player_pos.0 - exit_pos.0, player_pos.1 - exit_pos.1);

        // Skip if too close (avoid jitter when on top of exit)
        let distance_sq =
            direction_vector.0 * direction_vector.0 + direction_vector.1 * direction_vector.1;
        if distance_sq < 0.0001 {
            return;
        }

        // Calculate angle to exit in world space
        let mut target_angle = direction_vector.1.atan2(direction_vector.0); // [-π, π]

        // Normalize to [0, 2π]
        if target_angle < 0.0 {
            target_angle += 2.0 * std::f32::consts::PI;
        }

        // Smooth angle update (exponential smoothing)
        let alpha = self.smoothing_factor; // Lower = slower/smoother
        let mut delta = target_angle - self.smoothed_compass_angle;

        // Wrap to [-π, π] for shortest rotation
        if delta > std::f32::consts::PI {
            delta -= 2.0 * std::f32::consts::PI;
        } else if delta < -std::f32::consts::PI {
            delta += 2.0 * std::f32::consts::PI;
        }

        self.smoothed_compass_angle += alpha * delta;

        // Re-wrap smoothed angle to [0, 2π]
        while self.smoothed_compass_angle < 0.0 {
            self.smoothed_compass_angle += 2.0 * std::f32::consts::PI;
        }
        while self.smoothed_compass_angle >= 2.0 * std::f32::consts::PI {
            self.smoothed_compass_angle -= 2.0 * std::f32::consts::PI;
        }

        // Map to needle frame (0–11, since we have 12 needles indexed 1-12)
        let new_index = ((self.smoothed_compass_angle / (2.0 * std::f32::consts::PI)) * 12.0)
            .floor() as usize
            % 12;

        self.current_needle_index = new_index;
    }

    /// Updates the compass to point toward the exit from the player's current position.
    ///
    /// This function calculates the direction from the player to the exit cell and
    /// adjusts for the player's current orientation (yaw) so that the compass always
    /// indicates the direction the player should move to reach the exit.
    ///
    /// # Direction Calculation
    ///
    /// The method uses a sophisticated approach to calculate the compass direction:
    /// 1. **Vector Calculation**: Computes direction vector from player to exit
    /// 2. **Player Orientation**: Accounts for player's current facing direction
    /// 3. **Coordinate Transformation**: Converts world direction to player-relative direction
    /// 4. **Smoothing**: Applies exponential smoothing to prevent jarring movements
    /// 5. **Needle Selection**: Maps smoothed angle to appropriate needle sprite (0-11)
    ///
    /// # Coordinate Systems
    ///
    /// - **World Coordinates**: Player and exit positions in maze space
    /// - **Player Coordinates**: Direction relative to player's forward vector
    /// - **Compass Coordinates**: Angle mapped to 12-directional needle sprites
    ///
    /// # Smoothing Behavior
    ///
    /// The compass uses exponential smoothing to create fluid needle movement:
    /// - **Shortest Path**: Always takes the shortest angular distance
    /// - **Configurable Response**: Smoothing factor controls responsiveness
    /// - **Wrapping**: Properly handles angle wrapping around 0°/360°
    ///
    /// # Performance Notes
    ///
    /// This method performs minimal calculations and is safe to call every frame.
    /// The smoothing calculations are CPU-efficient and don't involve GPU operations.
    ///
    /// # Parameters
    ///
    /// * `player_pos` - The player's position as (x, z) coordinates in world space
    /// * `exit_pos` - The exit's position as (x, z) coordinates in world space
    /// * `player_yaw_degrees` - The player's current yaw angle in degrees (0-360)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use crate::renderer::game_renderer::compass::CompassRenderer;
    /// # let mut compass: CompassRenderer = unimplemented!();
    ///
    /// // Update compass direction
    /// compass.update_compass_with_yaw(
    ///     (player.x, player.z),      // Player position
    ///     (exit.x, exit.z),          // Exit position
    ///     player.yaw_degrees         // Player facing direction
    /// );
    /// ```
    pub fn update_compass_with_yaw(
        &mut self,
        player_pos: (f32, f32), // (x, z) coordinates
        exit_pos: (f32, f32),   // (x, z) coordinates
        player_yaw_degrees: f32,
    ) {
        // Calculate vector from player to exit
        let dx = exit_pos.0 - player_pos.0; // Change in X
        let dz = exit_pos.1 - player_pos.1; // Change in Z

        let distance_sq = dx * dx + dz * dz;

        // Skip if too close to exit
        if distance_sq < 0.0001 {
            return;
        }

        // Calculate direction to exit using the same trig approach as player movement
        // First, get forward vector based on player's yaw (same as in move_forward)
        let forward_x = player_yaw_degrees.to_radians().sin();
        let forward_z = player_yaw_degrees.to_radians().cos();

        // Get right vector (same as in move_right)
        let right_x = player_yaw_degrees.to_radians().cos();
        let right_z = player_yaw_degrees.to_radians().sin();

        // Normalize the direction vector to the exit
        let length = distance_sq.sqrt();
        let dir_x = dx / length;
        let dir_z = dz / length;

        // Calculate dot products to determine the angle
        let forward_dot = -forward_x * dir_x - forward_z * dir_z; // Dot product with forward vector
        let right_dot = right_x * dir_x - right_z * dir_z; // Dot product with right vector

        // Calculate angle using atan2
        let mut target_compass_angle = right_dot.atan2(forward_dot);

        // Normalize to [-π, π]
        target_compass_angle = self.normalize_angle(target_compass_angle);

        // Initialize smoothed angle on first update
        if self.smoothed_compass_angle.is_nan() {
            self.smoothed_compass_angle = target_compass_angle;
        }

        // Calculate the shortest angular distance for smooth interpolation
        let angle_diff =
            self.shortest_angle_diff(target_compass_angle, self.smoothed_compass_angle);

        // Apply smoothing
        self.smoothed_compass_angle += angle_diff * self.smoothing_factor;

        // Normalize the smoothed angle
        self.smoothed_compass_angle = self.normalize_angle(self.smoothed_compass_angle);

        // Convert to needle index (0-11 for 12 needle sprites)
        // Convert from [-π, π] to [0, 2π] for easier indexing
        let angle_for_index = if self.smoothed_compass_angle < 0.0 {
            self.smoothed_compass_angle + 2.0 * std::f32::consts::PI
        } else {
            self.smoothed_compass_angle
        };

        // Convert to 12-segment index (each segment is 30° = π/6 radians)
        // Add half a segment (π/12) for proper rounding to nearest segment
        let needle_index = ((angle_for_index + std::f32::consts::PI / 12.0)
            / (std::f32::consts::PI / 6.0))
            .floor() as usize
            % 12;

        self.current_needle_index = needle_index;
    }

    /// Normalize angle to [-π, π]
    fn normalize_angle(&self, mut angle: f32) -> f32 {
        while angle > std::f32::consts::PI {
            angle -= 2.0 * std::f32::consts::PI;
        }
        while angle < -std::f32::consts::PI {
            angle += 2.0 * std::f32::consts::PI;
        }
        angle
    }

    /// Calculate shortest angular difference between two angles
    fn shortest_angle_diff(&self, target: f32, current: f32) -> f32 {
        let mut diff = target - current;

        // Wrap to shortest path
        if diff > std::f32::consts::PI {
            diff -= 2.0 * std::f32::consts::PI;
        } else if diff < -std::f32::consts::PI {
            diff += 2.0 * std::f32::consts::PI;
        }

        diff
    }

    /// Alternative update with configurable smoothing
    pub fn update_compass_with_smoothing(
        &mut self,
        player_pos: (f32, f32),
        exit_pos: (f32, f32),
        player_yaw_degrees: f32,
        smoothing: f32, // 0.0 = very smooth, 1.0 = instant response
    ) {
        let old_smoothing = self.smoothing_factor;
        self.smoothing_factor = smoothing.clamp(0.01, 1.0);

        self.update_compass_with_yaw(player_pos, exit_pos, player_yaw_degrees);

        self.smoothing_factor = old_smoothing;
    }

    /// For debugging - get current compass angle in degrees
    pub fn get_compass_angle_degrees(&self) -> f32 {
        self.smoothed_compass_angle.to_degrees()
    }

    /// Set smoothing factor (0.0 = very smooth, 1.0 = instant)
    pub fn set_smoothing_factor(&mut self, factor: f32) {
        self.smoothing_factor = factor.clamp(0.01, 1.0);
    }
}
