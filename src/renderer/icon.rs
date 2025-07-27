use wgpu::{
    BindGroup, BindGroupLayout, BufferUsages, ColorTargetState, ColorWrites, Device,
    FragmentState, MultisampleState, PrimitiveState, RenderPass, RenderPipeline,
    SamplerBindingType, ShaderStages, Texture, TextureFormat, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, util::DeviceExt,
};
use image;
use std::collections::HashMap;
use std::mem;

/// Vertex data structure for rendering icon quads.
///
/// Each vertex contains position coordinates in normalized device coordinates (-1 to 1)
/// and UV texture coordinates (0 to 1) for texture sampling.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct IconVertex {
    /// Position in normalized device coordinates [x, y]
    position: [f32; 2],
    /// Texture coordinates [u, v] where (0,0) is top-left and (1,1) is bottom-right
    uv: [f32; 2],
}

impl IconVertex {
    /// Returns the vertex buffer layout descriptor for the graphics pipeline.
    ///
    /// Defines how vertex data is laid out in memory and which shader locations
    /// correspond to which vertex attributes.
    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: mem::size_of::<IconVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position attribute (location 0)
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x2,
                },
                // UV coordinate attribute (location 1)
                VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x2,
                },
            ],
        }
    }
}

/// Represents a 2D icon with position, size, and texture information.
///
/// Icons are positioned using screen coordinates where (0,0) is the top-left corner.
/// The texture_id corresponds to a loaded texture that will be used for rendering.
#[derive(Debug, Clone)]
pub struct Icon {
    /// X position in screen coordinates (pixels from left edge)
    pub x: f32,
    /// Y position in screen coordinates (pixels from top edge)
    pub y: f32,
    /// Width in screen coordinates (pixels)
    pub width: f32,
    /// Height in screen coordinates (pixels)
    pub height: f32,
    /// Identifier for the texture to use when rendering this icon
    pub texture_id: String,
}

impl Icon {
    /// Creates a new icon with the specified position, size, and texture.
    ///
    /// # Arguments
    /// * `x` - X position in screen coordinates
    /// * `y` - Y position in screen coordinates  
    /// * `width` - Width in pixels
    /// * `height` - Height in pixels
    /// * `texture_id` - String identifier for the texture to use
    ///
    /// # Returns
    /// A new `Icon` instance
    pub fn new(x: f32, y: f32, width: f32, height: f32, texture_id: String) -> Self {
        Self {
            x,
            y,
            width,
            height,
            texture_id,
        }
    }
}

/// High-performance batch renderer for 2D icons using WGPU.
///
/// This renderer optimizes performance through several techniques:
/// - **Texture Batching**: Groups icons by texture to minimize bind group changes
/// - **Buffer Caching**: Reuses vertex and index buffers when icon counts haven't changed
/// - **Instanced Rendering**: Renders multiple icons with the same texture in a single draw call
///
/// The renderer converts screen coordinates to normalized device coordinates automatically
/// and handles texture loading and management.
pub struct IconRenderer {
    /// The graphics pipeline used for rendering icons
    render_pipeline: RenderPipeline,
    /// Layout descriptor for texture and sampler bind groups
    bind_group_layout: BindGroupLayout,
    /// Collection of icons to be rendered
    icons: Vec<Icon>,
    /// Cache of loaded textures, views, and bind groups keyed by texture ID
    textures: HashMap<String, (Texture, BindGroup)>,
    /// Current window width in pixels (used for coordinate conversion)
    window_width: f32,
    /// Current window height in pixels (used for coordinate conversion)
    window_height: f32,
    /// Cached vertex buffers per texture to avoid recreation
    cached_vertex_buffers: HashMap<String, wgpu::Buffer>,
    /// Cached index buffers per texture to avoid recreation
    cached_index_buffers: HashMap<String, wgpu::Buffer>,
    /// Number of icons for each texture in the cached buffers
    cached_icon_counts: HashMap<String, usize>,
}

impl IconRenderer {
    /// Creates a new IconRenderer with the specified device and surface format.
    ///
    /// This method sets up the complete graphics pipeline including:
    /// - Shader module loading
    /// - Bind group layout for textures and samplers
    /// - Render pipeline with alpha blending enabled
    /// - Default window dimensions
    ///
    /// # Arguments
    /// * `device` - The WGPU device for creating graphics resources
    /// * `surface_format` - The texture format of the render target
    ///
    /// # Returns
    /// A new `IconRenderer` instance ready for use
    pub fn new(device: &Device, surface_format: TextureFormat) -> Self {
        // Load the icon shader from an embedded WGSL file
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Icon Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/icon.wgsl").into()),
        });

        // Create bind group layout for texture and sampler
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Icon Bind Group Layout"),
            entries: &[
                // Texture binding (binding 0)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler binding (binding 1)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Create the pipeline layout
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Icon Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        // Create the render pipeline with alpha blending
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Icon Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[IconVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    // Enable alpha blending for transparent icons
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            render_pipeline,
            bind_group_layout,
            icons: Vec::new(),
            textures: HashMap::new(),
            window_width: 1360.0,
            window_height: 768.0,
            cached_vertex_buffers: HashMap::new(),
            cached_index_buffers: HashMap::new(),
            cached_icon_counts: HashMap::new(),
        }
    }

    /// Loads a texture from embedded assets and creates associated GPU resources.
    ///
    /// This method:
    /// 1. Loads an image from embedded assets using the provided data
    /// 2. Converts it to RGBA8 format
    /// 3. Creates a WGPU texture and uploads the image data
    /// 4. Creates a texture view and sampler
    /// 5. Creates a bind group for use in rendering
    /// 6. Caches all resources for later use
    ///
    /// # Arguments
    /// * `device` - The WGPU device for creating resources
    /// * `queue` - The WGPU queue for uploading texture data
    /// * `texture_data` - The embedded texture data
    /// * `texture_id` - Unique identifier for this texture
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the image cannot be loaded
    ///
    /// # Errors
    /// Returns an error if:
    /// - The image data cannot be decoded
    /// - GPU resource creation fails
    pub fn load_texture_from_data(
        &mut self,
        device: &Device,
        queue: &wgpu::Queue,
        texture_data: &[u8],
        texture_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Load and convert image to RGBA8
        let img = image::load_from_memory(texture_data)?;
        let rgba = img.to_rgba8();
        let dimensions = rgba.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        // Create GPU texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("Icon texture: {}", texture_id)),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload image data to GPU
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        // Create texture view and sampler
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some(&format!("Icon bind group: {}", texture_id)),
        });

        // Cache the texture and bind group
        self.textures.insert(texture_id.to_string(), (texture, bind_group));

        Ok(())
    }

    /// Loads a texture from a file path and creates associated GPU resources.
    ///
    /// This method:
    /// 1. Loads an image from the specified file path
    /// 2. Converts it to RGBA8 format
    /// 3. Creates a WGPU texture and uploads the image data
    /// 4. Creates a texture view and sampler
    /// 5. Creates a bind group for use in rendering
    /// 6. Caches all resources for later use
    ///
    /// # Arguments
    /// * `device` - The WGPU device for creating resources
    /// * `queue` - The WGPU queue for uploading texture data
    /// * `path` - File system path to the image file
    /// * `texture_id` - Unique identifier for this texture
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the image cannot be loaded
    ///
    /// # Errors
    /// Returns an error if:
    /// - The image file cannot be found or read
    /// - The image format is not supported
    /// - GPU resource creation fails
    pub fn load_texture(
        &mut self,
        device: &Device,
        queue: &wgpu::Queue,
        path: &str,
        texture_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Load and convert image to RGBA8
        let img = image::open(std::path::Path::new(path))?;
        let rgba = img.to_rgba8();
        let dimensions = rgba.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        // Create GPU texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("Icon texture: {}", texture_id)),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload image data to GPU
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        // Create texture view and sampler
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some(&format!("Icon bind group: {}", texture_id)),
        });

        // Cache the texture and bind group
        self.textures.insert(texture_id.to_string(), (texture, bind_group));

        Ok(())
    }

    /// Adds an icon to the render queue.
    ///
    /// Icons added via this method will be rendered on the next call to `render()`.
    /// The icon's texture must have been previously loaded via `load_texture()`.
    ///
    /// # Arguments
    /// * `icon` - The icon to add to the render queue
    pub fn add_icon(&mut self, icon: Icon) {
        self.icons.push(icon);
    }

    /// Removes all icons from the render queue and clears cached buffers.
    ///
    /// This method also invalidates all cached vertex and index buffers,
    /// forcing them to be recreated on the next render call if new icons are added.
    pub fn clear_icons(&mut self) {
        self.icons.clear();
        // Clear cached buffers when icons are cleared
        self.cached_vertex_buffers.clear();
        self.cached_index_buffers.clear();
        self.cached_icon_counts.clear();
    }

    /// Updates the window dimensions and invalidates cached buffers.
    ///
    /// This method should be called whenever the render target size changes.
    /// It clears all cached buffers since the vertex positions need to be
    /// recalculated for the new screen dimensions.
    ///
    /// # Arguments
    /// * `width` - New window width in pixels
    /// * `height` - New window height in pixels
    pub fn resize(&mut self, width: f32, height: f32) {
        self.window_width = width;
        self.window_height = height;
        // Clear cached buffers when window is resized
        self.cached_vertex_buffers.clear();
        self.cached_index_buffers.clear();
        self.cached_icon_counts.clear();
    }

    /// Renders all queued icons to the specified render pass.
    ///
    /// This method implements several optimizations:
    ///
    /// ## Texture Batching
    /// Icons are grouped by texture ID to minimize the number of bind group changes,
    /// which is an expensive GPU operation.
    ///
    /// ## Buffer Caching
    /// Vertex and index buffers are cached and reused when the number of icons
    /// per texture hasn't changed since the last render.
    ///
    /// ## Coordinate Conversion
    /// Screen coordinates are automatically converted to normalized device coordinates
    /// (-1 to 1 range) required by the vertex shader.
    ///
    /// # Arguments
    /// * `device` - The WGPU device for creating new buffers if needed
    /// * `render_pass` - The active render pass to submit draw commands to
    ///
    /// # Performance Notes
    /// - Best performance is achieved when icons using the same texture are rendered together
    /// - Buffer recreation only occurs when icon counts change
    /// - Empty icon lists result in immediate return with no GPU work
    pub fn render(&mut self, device: &Device, render_pass: &mut RenderPass) {
        if self.icons.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.render_pipeline);

        // Group icons by texture to minimize bind group changes
        let mut icons_by_texture: HashMap<String, Vec<&Icon>> = HashMap::new();
        for icon in &self.icons {
            icons_by_texture
                .entry(icon.texture_id.clone())
                .or_insert_with(Vec::new)
                .push(icon);
        }

        // Render each texture batch
        for (texture_id, icons) in icons_by_texture {
            if let Some((_texture, bind_group)) = self.textures.get(&texture_id) {
                render_pass.set_bind_group(0, bind_group, &[]);

                // Check if we can reuse cached buffers for this texture
                let cached_count = self.cached_icon_counts.get(&texture_id).unwrap_or(&0);
                let need_new_buffers = *cached_count != icons.len();

                if need_new_buffers {
                    // Create vertices for all icons using this texture
                    let mut all_vertices = Vec::new();
                    let mut all_indices = Vec::new();

                    for (icon_index, icon) in icons.iter().enumerate() {
                        // Convert screen coordinates to normalized device coordinates
                        let x = (icon.x / self.window_width) * 2.0 - 1.0;
                        let y = (icon.y / self.window_height) * 2.0 - 1.0;
                        let width = (icon.width / self.window_width) * 2.0;
                        let height = (icon.height / self.window_height) * 2.0;

                        // Create quad vertices for this icon
                        let vertices = [
                            // Top-left
                            IconVertex {
                                position: [x, y],
                                uv: [0.0, 1.0], // Flip V coordinate for correct texture orientation
                            },
                            // Top-right
                            IconVertex {
                                position: [x + width, y],
                                uv: [1.0, 1.0],
                            },
                            // Bottom-right
                            IconVertex {
                                position: [x + width, y + height],
                                uv: [1.0, 0.0],
                            },
                            // Bottom-left
                            IconVertex {
                                position: [x, y + height],
                                uv: [0.0, 0.0],
                            },
                        ];

                        // Add vertices to the batch
                        all_vertices.extend_from_slice(&vertices);

                        // Create triangle indices for this quad (two triangles)
                        let base_index = (icon_index * 4) as u16;
                        let indices = [
                            // First triangle (top-left, top-right, bottom-right)
                            base_index,
                            base_index + 1,
                            base_index + 2,
                            // Second triangle (top-left, bottom-right, bottom-left)
                            base_index,
                            base_index + 2,
                            base_index + 3,
                        ];
                        all_indices.extend_from_slice(&indices);
                    }

                    // Create new vertex buffer for all icons using this texture
                    let vertex_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Icon Vertex Buffer"),
                            contents: bytemuck::cast_slice(&all_vertices),
                            usage: BufferUsages::VERTEX,
                        });

                    // Create new index buffer for all icons using this texture
                    let index_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Icon Index Buffer"),
                            contents: bytemuck::cast_slice(&all_indices),
                            usage: BufferUsages::INDEX,
                        });

                    // Cache the new buffers
                    self.cached_vertex_buffers
                        .insert(texture_id.clone(), vertex_buffer);
                    self.cached_index_buffers
                        .insert(texture_id.clone(), index_buffer);
                    self.cached_icon_counts
                        .insert(texture_id.clone(), icons.len());
                }

                // Use cached buffers for rendering
                if let (Some(vertex_buffer), Some(index_buffer)) = (
                    self.cached_vertex_buffers.get(&texture_id),
                    self.cached_index_buffers.get(&texture_id),
                ) {
                    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    // Draw all icons for this texture (6 indices per icon: 2 triangles × 3 vertices each)
                    render_pass.draw_indexed(0..(icons.len() * 6) as u32, 0, 0..1);
                }
            }
        }
    }
}
