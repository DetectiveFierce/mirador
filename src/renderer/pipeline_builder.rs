//! # WGPU Pipeline Builder Utilities
//!
//! This module provides builder patterns and helper functions for creating WGPU render pipelines,
//! bind group layouts, and related resources. It's designed to reduce boilerplate code and make
//! pipeline creation more maintainable in the maze renderer application.
//!
//! ## Key Components
//!
//! - [`PipelineBuilder`] - Fluent API for creating render pipelines
//! - [`BindGroupLayoutBuilder`] - Fluent API for creating bind group layouts
//! - Helper functions for common vertex layouts and buffers
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use crate::renderer::pipeline_builder::{PipelineBuilder, BindGroupLayoutBuilder, create_vertex_2d_layout};
//!
//! // Create a bind group layout for texture + sampler
//! let bind_group_layout = BindGroupLayoutBuilder::new(&device)
//!     .with_label("Texture Bind Group Layout")
//!     .with_texture(0, wgpu::ShaderStages::FRAGMENT)
//!     .with_sampler(1, wgpu::ShaderStages::FRAGMENT)
//!     .build();
//!
//! // Create a render pipeline
//! let pipeline = PipelineBuilder::new(&device, surface_format)
//!     .with_label("My Pipeline")
//!     .with_shader(shader_source)
//!     .with_vertex_buffer(create_vertex_2d_layout())
//!     .with_bind_group_layout(&bind_group_layout)
//!     .with_alpha_blending()
//!     .build();
//! ```

use egui_wgpu::wgpu;
use egui_wgpu::wgpu::util::DeviceExt;

/// Builder for creating render pipelines with common patterns used in the maze renderer.
///
/// This builder provides a fluent API that reduces boilerplate code and makes pipeline
/// creation more maintainable. It sets sensible defaults while allowing customization
/// of all pipeline parameters.
///
/// ## Default Configuration
///
/// - Vertex entry point: `"vs_main"`
/// - Fragment entry point: `"fs_main"`
/// - Blend state: `REPLACE` (no blending)
/// - Cull mode: `Back` face culling
/// - Primitive topology: `TriangleList`
/// - Front face: Counter-clockwise
/// - Polygon mode: `Fill`
///
/// ## Builder Pattern Usage
///
/// The builder uses the fluent API pattern where each method returns `Self`,
/// allowing for method chaining. Call [`build()`](PipelineBuilder::build) at the end
/// to create the actual render pipeline.
///
/// ## Required Parameters
///
/// - Shader source must be provided via [`with_shader()`](PipelineBuilder::with_shader)
/// - At least one vertex buffer layout should be added for most use cases
///
/// ## Example
///
/// ```rust,no_run
/// # use egui_wgpu::wgpu;
/// # let device: wgpu::Device = unimplemented!();
/// # let surface_format = wgpu::TextureFormat::Bgra8UnormSrgb;
/// # let shader_source = "";
/// # let bind_group_layout: wgpu::BindGroupLayout = unimplemented!();
/// use crate::renderer::pipeline_builder::{PipelineBuilder, create_vertex_2d_layout};
///
/// let pipeline = PipelineBuilder::new(&device, surface_format)
///     .with_label("Custom Pipeline")
///     .with_shader(shader_source)
///     .with_vertex_buffer(create_vertex_2d_layout())
///     .with_bind_group_layout(&bind_group_layout)
///     .with_alpha_blending()
///     .with_no_culling()
///     .build();
/// ```
pub struct PipelineBuilder<'a> {
    device: &'a wgpu::Device,
    surface_format: wgpu::TextureFormat,
    label: Option<&'a str>,
    shader_source: Option<&'a str>,
    vertex_entry: Option<&'a str>,
    fragment_entry: Option<&'a str>,
    vertex_buffers: Vec<wgpu::VertexBufferLayout<'a>>,
    bind_group_layouts: Vec<&'a wgpu::BindGroupLayout>,
    blend_state: Option<wgpu::BlendState>,
    cull_mode: Option<wgpu::Face>,
    depth_stencil: Option<wgpu::DepthStencilState>,
}

impl<'a> PipelineBuilder<'a> {
    /// Create a new pipeline builder with default settings.
    ///
    /// # Parameters
    ///
    /// - `device` - The WGPU device used to create the pipeline
    /// - `surface_format` - The texture format of the render target (usually from surface config)
    ///
    /// # Default Settings
    ///
    /// - Vertex entry: `"vs_main"`
    /// - Fragment entry: `"fs_main"`
    /// - Blend state: `REPLACE` (opaque rendering)
    /// - Cull mode: Back face culling enabled
    /// - No depth testing
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use egui_wgpu::wgpu;
    /// # let device: wgpu::Device = unimplemented!();
    /// # let surface_format = wgpu::TextureFormat::Bgra8UnormSrgb;
    /// use crate::renderer::pipeline_builder::PipelineBuilder;
    ///
    /// let builder = PipelineBuilder::new(&device, surface_format);
    /// ```
    pub fn new(device: &'a wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        Self {
            device,
            surface_format,
            label: None,
            shader_source: None,
            vertex_entry: Some("vs_main"),
            fragment_entry: Some("fs_main"),
            vertex_buffers: Vec::new(),
            bind_group_layouts: Vec::new(),
            blend_state: Some(wgpu::BlendState::REPLACE),
            cull_mode: Some(wgpu::Face::Back),
            depth_stencil: None,
        }
    }

    /// Set the pipeline label for debugging purposes.
    ///
    /// The label will be used for the pipeline, shader module, and pipeline layout.
    /// This is helpful when debugging with graphics debugging tools.
    ///
    /// # Parameters
    ///
    /// - `label` - Debug label for the pipeline
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::pipeline_builder::PipelineBuilder;
    /// # let builder: PipelineBuilder = unimplemented!();
    /// let builder = builder.with_label("Maze Renderer Pipeline");
    /// ```
    pub fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Set the shader source code (WGSL format).
    ///
    /// This is a required parameter - the pipeline cannot be built without shader source.
    /// The source should contain both vertex and fragment shader functions.
    ///
    /// # Parameters
    ///
    /// - `source` - WGSL shader source code as a string
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::pipeline_builder::PipelineBuilder;
    /// # let builder: PipelineBuilder = unimplemented!();
    /// let shader_source = r#"
    ///     @vertex
    ///     fn vs_main(@location(0) position: vec2<f32>) -> @builtin(position) vec4<f32> {
    ///         return vec4<f32>(position, 0.0, 1.0);
    ///     }
    ///
    ///     @fragment
    ///     fn fs_main() -> @location(0) vec4<f32> {
    ///         return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    ///     }
    /// "#;
    /// let builder = builder.with_shader(shader_source);
    /// ```
    pub fn with_shader(mut self, source: &'a str) -> Self {
        self.shader_source = Some(source);
        self
    }

    /// Set the vertex shader entry point function name.
    ///
    /// Default is `"vs_main"`. Change this if your vertex shader function has a different name.
    ///
    /// # Parameters
    ///
    /// - `entry` - Name of the vertex shader function in the WGSL source
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::pipeline_builder::PipelineBuilder;
    /// # let builder: PipelineBuilder = unimplemented!();
    /// let builder = builder.with_vertex_entry("vertex_main");
    /// ```
    pub fn with_vertex_entry(mut self, entry: &'a str) -> Self {
        self.vertex_entry = Some(entry);
        self
    }

    /// Set the fragment shader entry point function name.
    ///
    /// Default is `"fs_main"`. Change this if your fragment shader function has a different name.
    ///
    /// # Parameters
    ///
    /// - `entry` - Name of the fragment shader function in the WGSL source
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::pipeline_builder::PipelineBuilder;
    /// # let builder: PipelineBuilder = unimplemented!();
    /// let builder = builder.with_fragment_entry("fragment_main");
    /// ```
    pub fn with_fragment_entry(mut self, entry: &'a str) -> Self {
        self.fragment_entry = Some(entry);
        self
    }

    /// Add a vertex buffer layout to the pipeline.
    ///
    /// This defines how vertex data is laid out in memory and how it maps to shader inputs.
    /// You can call this method multiple times to add multiple vertex buffers.
    ///
    /// # Parameters
    ///
    /// - `layout` - Vertex buffer layout describing the vertex attributes
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::pipeline_builder::{PipelineBuilder, create_vertex_2d_layout};
    /// # let builder: PipelineBuilder = unimplemented!();
    /// let builder = builder.with_vertex_buffer(create_vertex_2d_layout());
    /// ```
    pub fn with_vertex_buffer(mut self, layout: wgpu::VertexBufferLayout<'a>) -> Self {
        self.vertex_buffers.push(layout);
        self
    }

    /// Add a bind group layout to the pipeline.
    ///
    /// Bind group layouts define what resources (textures, samplers, uniform buffers)
    /// the shader can access. You can call this method multiple times to add multiple
    /// bind group layouts.
    ///
    /// # Parameters
    ///
    /// - `layout` - Bind group layout for shader resources
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::pipeline_builder::PipelineBuilder;
    /// # let builder: PipelineBuilder = unimplemented!();
    /// # let bind_group_layout: &wgpu::BindGroupLayout = unimplemented!();
    /// let builder = builder.with_bind_group_layout(bind_group_layout);
    /// ```
    pub fn with_bind_group_layout(mut self, layout: &'a wgpu::BindGroupLayout) -> Self {
        self.bind_group_layouts.push(layout);
        self
    }

    /// Set a custom blend state for color blending.
    ///
    /// This overrides the default `REPLACE` blend state. Use this for custom
    /// blending operations.
    ///
    /// # Parameters
    ///
    /// - `blend` - Custom blend state configuration
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use egui_wgpu::wgpu;
    /// # use crate::renderer::pipeline_builder::PipelineBuilder;
    /// # let builder: PipelineBuilder = unimplemented!();
    /// let custom_blend = wgpu::BlendState {
    ///     color: wgpu::BlendComponent {
    ///         src_factor: wgpu::BlendFactor::SrcAlpha,
    ///         dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
    ///         operation: wgpu::BlendOperation::Add,
    ///     },
    ///     alpha: wgpu::BlendComponent::REPLACE,
    /// };
    /// let builder = builder.with_blend_state(custom_blend);
    /// ```
    pub fn with_blend_state(mut self, blend: wgpu::BlendState) -> Self {
        self.blend_state = Some(blend);
        self
    }

    /// Enable standard alpha blending.
    ///
    /// This is a convenience method that sets up standard alpha blending:
    /// - Color: `(SrcAlpha * src) + (OneMinusSrcAlpha * dst)`
    /// - Alpha: Replace with source alpha
    ///
    /// Useful for rendering transparent or semi-transparent objects.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::pipeline_builder::PipelineBuilder;
    /// # let builder: PipelineBuilder = unimplemented!();
    /// let builder = builder.with_alpha_blending();
    /// ```
    pub fn with_alpha_blending(mut self) -> Self {
        self.blend_state = Some(wgpu::BlendState::ALPHA_BLENDING);
        self
    }

    /// Disable face culling.
    ///
    /// By default, back faces are culled. Use this method to render both
    /// front and back faces of triangles.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::pipeline_builder::PipelineBuilder;
    /// # let builder: PipelineBuilder = unimplemented!();
    /// let builder = builder.with_no_culling();
    /// ```
    pub fn with_no_culling(mut self) -> Self {
        self.cull_mode = None;
        self
    }

    /// Set depth and stencil testing configuration.
    ///
    /// Use this to enable depth testing, depth writing, or stencil operations.
    /// By default, no depth or stencil testing is performed.
    ///
    /// # Parameters
    ///
    /// - `depth_stencil` - Depth and stencil state configuration
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use egui_wgpu::wgpu;
    /// # use crate::renderer::pipeline_builder::PipelineBuilder;
    /// # let builder: PipelineBuilder = unimplemented!();
    /// let depth_stencil = wgpu::DepthStencilState {
    ///     format: wgpu::TextureFormat::Depth32Float,
    ///     depth_write_enabled: true,
    ///     depth_compare: wgpu::CompareFunction::Less,
    ///     stencil: wgpu::StencilState::default(),
    ///     bias: wgpu::DepthBiasState::default(),
    /// };
    /// let builder = builder.with_depth_stencil(depth_stencil);
    /// ```
    pub fn with_depth_stencil(mut self, depth_stencil: wgpu::DepthStencilState) -> Self {
        self.depth_stencil = Some(depth_stencil);
        self
    }

    /// Build the render pipeline with the configured parameters.
    ///
    /// This consumes the builder and creates the actual WGPU render pipeline.
    /// The shader source must have been provided via [`with_shader()`](PipelineBuilder::with_shader)
    /// or this method will panic.
    ///
    /// # Panics
    ///
    /// Panics if no shader source was provided.
    ///
    /// # Returns
    ///
    /// A configured `wgpu::RenderPipeline` ready for use in rendering.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::pipeline_builder::PipelineBuilder;
    /// # let builder: PipelineBuilder = unimplemented!();
    /// let pipeline = builder.build();
    /// ```
    pub fn build(self) -> wgpu::RenderPipeline {
        let shader_source = self.shader_source.expect("Shader source must be provided");

        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: self.label,
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: self.label,
                bind_group_layouts: &self.bind_group_layouts,
                push_constant_ranges: &[],
            });

        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: self.label,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: self.vertex_entry,
                    buffers: &self.vertex_buffers,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: self.fragment_entry,
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.surface_format,
                        blend: self.blend_state,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: self.cull_mode,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: self.depth_stencil,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            })
    }
}

/// Builder for creating bind group layouts with common resource patterns.
///
/// This builder simplifies the creation of bind group layouts by providing
/// convenience methods for common resource types like textures, samplers,
/// and uniform buffers.
///
/// ## Supported Resource Types
///
/// - 2D textures with float sampling
/// - Filtering samplers
/// - Uniform buffers
///
/// ## Usage Pattern
///
/// Use the fluent API to add bindings, then call [`build()`](BindGroupLayoutBuilder::build)
/// to create the layout:
///
/// ```rust,no_run
/// # use egui_wgpu::wgpu;
/// # let device: wgpu::Device = unimplemented!();
/// use crate::renderer::pipeline_builder::BindGroupLayoutBuilder;
///
/// let layout = BindGroupLayoutBuilder::new(&device)
///     .with_label("My Bind Group Layout")
///     .with_texture(0, wgpu::ShaderStages::FRAGMENT)
///     .with_sampler(1, wgpu::ShaderStages::FRAGMENT)
///     .with_uniform_buffer(2, wgpu::ShaderStages::VERTEX)
///     .build();
/// ```
pub struct BindGroupLayoutBuilder<'a> {
    device: &'a wgpu::Device,
    entries: Vec<wgpu::BindGroupLayoutEntry>,
    label: Option<&'a str>,
}

impl<'a> BindGroupLayoutBuilder<'a> {
    /// Create a new bind group layout builder.
    ///
    /// # Parameters
    ///
    /// - `device` - The WGPU device used to create the bind group layout
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use egui_wgpu::wgpu;
    /// # let device: wgpu::Device = unimplemented!();
    /// use crate::renderer::pipeline_builder::BindGroupLayoutBuilder;
    ///
    /// let builder = BindGroupLayoutBuilder::new(&device);
    /// ```
    pub fn new(device: &'a wgpu::Device) -> Self {
        Self {
            device,
            entries: Vec::new(),
            label: None,
        }
    }

    /// Set the bind group layout label for debugging.
    ///
    /// # Parameters
    ///
    /// - `label` - Debug label for the bind group layout
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::pipeline_builder::BindGroupLayoutBuilder;
    /// # let builder: BindGroupLayoutBuilder = unimplemented!();
    /// let builder = builder.with_label("Texture Resources");
    /// ```
    pub fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Add a 2D texture binding.
    ///
    /// Creates a binding for a 2D texture with float sampling support.
    /// The texture can be filtered and is not multisampled.
    ///
    /// # Parameters
    ///
    /// - `binding` - Binding index in the shader (e.g., `@binding(0)`)
    /// - `visibility` - Which shader stages can access this texture
    ///
    /// # Shader Usage
    ///
    /// In WGSL, access this texture with:
    /// ```wgsl
    /// @group(0) @binding(0) var my_texture: texture_2d<f32>;
    /// ```
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use egui_wgpu::wgpu;
    /// # use crate::renderer::pipeline_builder::BindGroupLayoutBuilder;
    /// # let builder: BindGroupLayoutBuilder = unimplemented!();
    /// let builder = builder.with_texture(0, wgpu::ShaderStages::FRAGMENT);
    /// ```
    pub fn with_texture(mut self, binding: u32, visibility: wgpu::ShaderStages) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            },
            count: None,
        });
        self
    }

    /// Add a filtering sampler binding.
    ///
    /// Creates a binding for a sampler that supports filtering (linear interpolation).
    /// Use this with textures that need smooth sampling.
    ///
    /// # Parameters
    ///
    /// - `binding` - Binding index in the shader (e.g., `@binding(1)`)
    /// - `visibility` - Which shader stages can access this sampler
    ///
    /// # Shader Usage
    ///
    /// In WGSL, access this sampler with:
    /// ```wgsl
    /// @group(0) @binding(1) var my_sampler: sampler;
    /// ```
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use egui_wgpu::wgpu;
    /// # use crate::renderer::pipeline_builder::BindGroupLayoutBuilder;
    /// # let builder: BindGroupLayoutBuilder = unimplemented!();
    /// let builder = builder.with_sampler(1, wgpu::ShaderStages::FRAGMENT);
    /// ```
    pub fn with_sampler(mut self, binding: u32, visibility: wgpu::ShaderStages) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });
        self
    }

    /// Add a uniform buffer binding.
    ///
    /// Creates a binding for a uniform buffer that can be read by shaders.
    /// Uniform buffers are typically used for passing small amounts of data
    /// that remain constant during a draw call.
    ///
    /// # Parameters
    ///
    /// - `binding` - Binding index in the shader (e.g., `@binding(2)`)
    /// - `visibility` - Which shader stages can access this uniform buffer
    ///
    /// # Shader Usage
    ///
    /// In WGSL, access this uniform buffer with:
    /// ```wgsl
    /// struct MyUniforms {
    ///     value: f32,
    /// }
    /// @group(0) @binding(2) var<uniform> uniforms: MyUniforms;
    /// ```
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use egui_wgpu::wgpu;
    /// # use crate::renderer::pipeline_builder::BindGroupLayoutBuilder;
    /// # let builder: BindGroupLayoutBuilder = unimplemented!();
    /// let builder = builder.with_uniform_buffer(2, wgpu::ShaderStages::VERTEX_FRAGMENT);
    /// ```
    pub fn with_uniform_buffer(mut self, binding: u32, visibility: wgpu::ShaderStages) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        self
    }

    /// Build the bind group layout.
    ///
    /// This consumes the builder and creates the actual WGPU bind group layout
    /// with all the configured bindings.
    ///
    /// # Returns
    ///
    /// A `wgpu::BindGroupLayout` that can be used to create bind groups and
    /// pipeline layouts.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use crate::renderer::pipeline_builder::BindGroupLayoutBuilder;
    /// # let builder: BindGroupLayoutBuilder = unimplemented!();
    /// let layout = builder.build();
    /// ```
    pub fn build(self) -> wgpu::BindGroupLayout {
        self.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &self.entries,
                label: self.label,
            })
    }
}

/// Create a vertex buffer layout for 2D positions.
///
/// This creates a vertex buffer layout for simple 2D vertex data consisting
/// of `[f32; 2]` position arrays. The layout maps to shader location 0.
///
/// ## Memory Layout
///
/// Each vertex consists of:
/// - Position: `vec2<f32>` at shader location 0
///
/// ## Shader Compatibility
///
/// Use this layout with vertex shaders that have:
/// ```wgsl
/// @vertex
/// fn vs_main(@location(0) position: vec2<f32>) -> ... {
///     // vertex shader code
/// }
/// ```
///
/// ## Usage Example
///
/// ```rust,no_run
/// use crate::renderer::pipeline_builder::{PipelineBuilder, create_vertex_2d_layout};
/// # let device: egui_wgpu::wgpu::Device = unimplemented!();
/// # let surface_format = egui_wgpu::wgpu::TextureFormat::Bgra8UnormSrgb;
/// # let shader_source = "";
///
/// let pipeline = PipelineBuilder::new(&device, surface_format)
///     .with_shader(shader_source)
///     .with_vertex_buffer(create_vertex_2d_layout())
///     .build();
/// ```
///
/// # Returns
///
/// A `wgpu::VertexBufferLayout` configured for 2D position data.
pub fn create_vertex_2d_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[wgpu::VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x2,
        }],
    }
}

/// Create a vertex buffer containing a fullscreen quad.
///
/// This creates a vertex buffer with 6 vertices forming two triangles that
/// cover the entire screen in normalized device coordinates (-1 to 1).
/// The quad is suitable for fullscreen effects, post-processing, or
/// screen-space rendering.
///
/// ## Vertex Data
///
/// The buffer contains 6 vertices in triangle list format:
/// - Triangle 1: (-1,-1), (1,-1), (1,1)
/// - Triangle 2: (-1,-1), (1,1), (-1,1)
///
/// This covers the entire NDC space from (-1,-1) to (1,1).
///
/// ## Usage with Shaders
///
/// Vertex shaders can use these positions directly:
/// ```wgsl
/// @vertex
/// fn vs_main(@location(0) position: vec2<f32>) -> @builtin(position) vec4<f32> {
///     return vec4<f32>(position, 0.0, 1.0);
/// }
/// ```
///
/// For UV coordinates, convert from NDC:
/// ```wgsl
/// let uv = position * 0.5 + 0.5; // Convert from [-1,1] to [0,1]
/// ```
///
/// ## Rendering
///
/// Draw with:
/// ```rust,no_run
/// # let render_pass: &mut egui_wgpu::wgpu::RenderPass = unimplemented!();
/// # let vertex_buffer: egui_wgpu::wgpu::Buffer = unimplemented!();
/// render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
/// render_pass.draw(0..6, 0..1); // Draw 6 vertices as triangle list
/// ```
///
/// # Parameters
///
/// - `device` - The WGPU device used to create the buffer
///
/// # Returns
///
/// A `wgpu::Buffer` containing the fullscreen quad vertices, ready for use
/// as a vertex buffer.
///
/// # Example
///
/// ```rust,no_run
/// use crate::renderer::pipeline_builder::create_fullscreen_vertices;
/// # let device: egui_wgpu::wgpu::Device = unimplemented!();
///
/// let vertex_buffer = create_fullscreen_vertices(&device);
///
/// // In render loop:
/// # let render_pass: &mut egui_wgpu::wgpu::RenderPass = unimplemented!();
/// render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
/// render_pass.draw(0..6, 0..1);
/// ```
pub fn create_fullscreen_vertices(device: &wgpu::Device) -> wgpu::Buffer {
    let vertices: &[f32] = &[
        -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0,
    ];

    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Fullscreen Quad Vertex Buffer"),
        contents: bytemuck::cast_slice(vertices),
        usage: wgpu::BufferUsages::VERTEX,
    })
}

/// Helper for creating uniform buffers
///
pub fn create_uniform_buffer<T: bytemuck::Pod>(
    device: &wgpu::Device,
    data: &T,
    label: &str,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(std::slice::from_ref(data)),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    })
}
