//! # Rectangle Renderer for Menu Creation
//!
//! This module provides the core primitives for creating UI menus using WebGPU.
//! It defines a flexible rectangle rendering system that can be used to build
//! various menu components like buttons, panels, dropdowns, and other UI elements.
//!
//! ## Core Components
//!
//! - `Vertex`: Defines the data structure for each vertex in a rectangle
//! - `Rectangle`: Represents a menu primitive with position, size, color, and styling
//! - `RectangleRenderer`: Manages the rendering pipeline and batch rendering of rectangles
//!
//! ## Usage for Menu Creation
//!
//! This system is designed to be the foundation for menu systems where:
//! - Each menu item can be represented as a `Rectangle`
//! - Multiple rectangles can be batched and rendered efficiently
//! - Rounded corners provide modern UI aesthetics
//! - Alpha blending enables overlays and transparency effects

use std::mem;
use wgpu::{
    self, BlendState, BufferUsages, ColorTargetState, ColorWrites, Device, FragmentState,
    MultisampleState, PrimitiveState, RenderPass, RenderPipeline, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, util::DeviceExt,
};

/// Vertex data structure for rectangle rendering in menu systems.
///
/// Each vertex contains all the information needed to render a portion of a rectangle
/// in the GPU shader. This design allows for efficient batch rendering of multiple
/// menu elements while supporting advanced features like rounded corners.
///
/// ## Memory Layout
///
/// The struct uses `#[repr(C)]` to ensure consistent memory layout across platforms,
/// which is crucial for GPU buffer compatibility. The total size is 48 bytes per vertex
/// with proper 16-byte alignment.
///
/// ## Usage in Menu Creation
///
/// For a typical menu button, four vertices are created (one for each corner)
/// with the same rectangle data but different UV coordinates to enable
/// proper texture mapping and corner radius calculations.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    /// Screen position in normalized device coordinates (-1.0 to 1.0)
    /// Used by the vertex shader to position the rectangle on screen
    position: [f32; 2],

    /// RGBA color values (0.0 to 1.0) for the rectangle
    /// Enables different colored menu items (e.g., hover states, active buttons)
    color: [f32; 4],

    /// UV coordinates for texture mapping and distance calculations
    /// Used in the fragment shader to determine pixel position within the rectangle
    /// Essential for rounded corner calculations
    uv: [f32; 2],

    /// Original rectangle dimensions in screen pixels
    /// Passed to fragment shader for accurate corner radius calculations
    /// regardless of the rectangle's screen position
    rect_size: [f32; 2],

    /// Corner radius in pixels for rounded rectangles
    /// Enables modern UI aesthetics for menu buttons and panels
    corner_radius: f32,

    /// Padding to ensure 16-byte alignment required by GPU buffers
    _padding: f32,
}

impl Vertex {
    /// Defines the vertex buffer layout for the GPU pipeline.
    ///
    /// This describes how vertex data is organized in memory and maps
    /// each field to a shader input location. The GPU uses this information
    /// to correctly interpret the vertex buffer data.
    ///
    /// ## Shader Locations
    ///
    /// - Location 0: Position (vec2)
    /// - Location 1: Color (vec4)  
    /// - Location 2: UV coordinates (vec2)
    /// - Location 3: Rectangle size (vec2)
    /// - Location 4: Corner radius (float)
    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position attribute - where the vertex appears on screen
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x2,
                },
                // Color attribute - RGBA color for this menu element
                VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x4,
                },
                // UV attribute - texture coordinates for corner radius calculations
                VertexAttribute {
                    offset: (mem::size_of::<[f32; 2]>() + mem::size_of::<[f32; 4]>())
                        as wgpu::BufferAddress,
                    shader_location: 2,
                    format: VertexFormat::Float32x2,
                },
                // Rectangle size - original dimensions for distance calculations
                VertexAttribute {
                    offset: (mem::size_of::<[f32; 2]>()
                        + mem::size_of::<[f32; 4]>()
                        + mem::size_of::<[f32; 2]>())
                        as wgpu::BufferAddress,
                    shader_location: 3,
                    format: VertexFormat::Float32x2,
                },
                // Corner radius - enables rounded rectangle rendering
                VertexAttribute {
                    offset: (mem::size_of::<[f32; 2]>()
                        + mem::size_of::<[f32; 4]>()
                        + mem::size_of::<[f32; 2]>()
                        + mem::size_of::<[f32; 2]>())
                        as wgpu::BufferAddress,
                    shader_location: 4,
                    format: VertexFormat::Float32,
                },
            ],
        }
    }
}

/// Represents a rectangular primitive for menu creation.
///
/// This is the fundamental building block for menu systems. Each `Rectangle`
/// can represent various menu components:
///
/// - Menu buttons and items
/// - Background panels and containers
/// - Separators and dividers
/// - Hover and selection highlights
/// - Dropdown menus and tooltips
///
/// ## Coordinate System
///
/// Uses screen-space coordinates where (0,0) is the top-left corner,
/// which is natural for UI layout systems.
///
/// ## Examples
///
/// ```rust
/// // Create a menu button
/// let button = Rectangle::new(100.0, 50.0, 200.0, 40.0, [0.2, 0.4, 0.8, 1.0])
///     .with_corner_radius(8.0);
///
/// // Create a menu panel background
/// let panel = Rectangle::new(0.0, 0.0, 300.0, 400.0, [0.1, 0.1, 0.1, 0.9])
///     .with_corner_radius(12.0);
/// ```
#[derive(Debug, Clone)]
pub struct Rectangle {
    /// X coordinate of the rectangle's top-left corner in screen pixels
    pub x: f32,

    /// Y coordinate of the rectangle's top-left corner in screen pixels
    pub y: f32,

    /// Width of the rectangle in screen pixels
    pub width: f32,

    /// Height of the rectangle in screen pixels  
    pub height: f32,

    /// RGBA color values (0.0 to 1.0) for the rectangle
    /// Alpha channel enables transparency for overlays and hover effects
    pub color: [f32; 4],

    /// Corner radius in pixels for rounded rectangles
    /// Set to 0.0 for sharp corners, or positive values for rounded corners
    pub corner_radius: f32,
}

impl Rectangle {
    /// Creates a new rectangle with sharp corners.
    ///
    /// This is the primary constructor for menu primitives. The rectangle
    /// is positioned using screen coordinates where (0,0) is top-left.
    ///
    /// ## Parameters
    ///
    /// - `x`, `y`: Top-left corner position in screen pixels
    /// - `width`, `height`: Dimensions in screen pixels
    /// - `color`: RGBA color array with values from 0.0 to 1.0
    ///
    /// ## Returns
    ///
    /// A new `Rectangle` with sharp corners (corner_radius = 0.0)
    pub fn new(x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) -> Self {
        Self {
            x,
            y,
            width,
            height,
            color,
            corner_radius: 0.0,
        }
    }

    /// Sets the corner radius for rounded rectangles.
    ///
    /// This builder method enables the creation of modern-looking menu
    /// elements with rounded corners. The radius is applied to all four
    /// corners equally.
    ///
    /// ## Parameters
    ///
    /// - `radius`: Corner radius in pixels. Should be <= min(width, height) / 2
    ///
    /// ## Returns
    ///
    /// The modified rectangle with the specified corner radius
    ///
    /// ## Usage
    ///
    /// ```rust
    /// let rounded_button = Rectangle::new(x, y, w, h, color)
    ///     .with_corner_radius(8.0);  // 8-pixel corner radius
    /// ```
    pub fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }
}

/// High-performance rectangle renderer for menu systems.
///
/// This renderer is optimized for UI applications where multiple rectangles
/// (menu items, buttons, panels) need to be drawn efficiently. It uses
/// batch rendering to minimize GPU draw calls and supports advanced
/// features like transparency and rounded corners.
///
/// ## Key Features
///
/// - **Batch Rendering**: All rectangles are rendered in a single draw call
/// - **Alpha Blending**: Supports transparent and semi-transparent elements
/// - **Rounded Corners**: Hardware-accelerated rounded rectangle rendering
/// - **Efficient Memory Usage**: Vertices and indices are created dynamically
///
/// ## Usage Pattern
///
/// 1. Create the renderer with GPU device and surface format
/// 2. Add rectangles representing menu elements
/// 3. Call render() to draw all rectangles in a batch
/// 4. Clear rectangles for the next frame if needed
///
/// ## Performance Characteristics
///
/// - O(n) vertex generation where n is the number of rectangles
/// - Single GPU draw call regardless of rectangle count
/// - Memory allocation only occurs during rendering
pub struct RectangleRenderer {
    /// WebGPU render pipeline configured for rectangle rendering
    /// Includes vertex and fragment shaders, blending state, and vertex layout
    render_pipeline: RenderPipeline,

    /// Collection of rectangles to be rendered
    /// Represents all menu elements that will be drawn in the next render call
    rectangles: Vec<Rectangle>,

    /// Current window width in pixels
    /// Used for coordinate transformation from screen space to NDC
    window_width: f32,

    /// Current window height in pixels  
    /// Used for coordinate transformation from screen space to NDC
    window_height: f32,
}

impl RectangleRenderer {
    /// Creates a new rectangle renderer for menu systems.
    ///
    /// Initializes the WebGPU rendering pipeline with shaders optimized
    /// for rectangle rendering. The pipeline is configured with:
    ///
    /// - Alpha blending for transparency effects
    /// - Counter-clockwise front face orientation
    /// - Triangle list primitive topology
    /// - Front face culling for performance
    ///
    /// ## Parameters
    ///
    /// - `device`: WebGPU device for creating GPU resources
    /// - `surface_format`: The texture format of the render target
    ///
    /// ## Returns
    ///
    /// A new `RectangleRenderer` ready to render menu elements
    ///
    /// ## Shader Requirements
    ///
    /// Expects a WGSL shader file at "shaders/rectangle.wgsl" with:
    /// - `vs_main` vertex shader entry point
    /// - `fs_main` fragment shader entry point
    /// - Support for the vertex attributes defined in `Vertex::desc()`
    pub fn new(device: &Device, surface_format: wgpu::TextureFormat) -> Self {
        // Load the rectangle shader that handles rounded corner rendering
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rectangle Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/rectangle.wgsl").into()),
        });

        // Create pipeline layout (no bind groups needed for basic rectangles)
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Rectangle Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        // Configure the rendering pipeline for efficient rectangle rendering
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rectangle Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    // Enable alpha blending for transparent menu elements
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                // Cull front faces for performance (we render rectangles from inside)
                cull_mode: Some(wgpu::Face::Front),
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
            rectangles: Vec::new(),
            // Default window size - should be updated via resize()
            window_width: 1360.0,
            window_height: 768.0,
        }
    }

    /// Adds a rectangle to be rendered in the next draw call.
    ///
    /// This method is used to build up a collection of menu elements
    /// that will all be rendered together for optimal performance.
    ///
    /// ## Parameters
    ///
    /// - `rectangle`: The rectangle primitive to add to the render queue
    ///
    /// ## Usage Examples
    ///
    /// ```rust
    /// // Add a menu button
    /// renderer.add_rectangle(
    ///     Rectangle::new(100.0, 50.0, 200.0, 40.0, [0.2, 0.4, 0.8, 1.0])
    ///         .with_corner_radius(8.0)
    /// );
    ///
    /// // Add a hover highlight
    /// renderer.add_rectangle(
    ///     Rectangle::new(95.0, 45.0, 210.0, 50.0, [1.0, 1.0, 1.0, 0.1])
    ///         .with_corner_radius(10.0)
    /// );
    /// ```
    pub fn add_rectangle(&mut self, rectangle: Rectangle) {
        self.rectangles.push(rectangle);
    }

    /// Clears all rectangles from the render queue.
    ///
    /// This is typically called at the beginning of each frame to
    /// remove the previous frame's menu elements before adding
    /// new ones.
    ///
    /// ## Usage
    ///
    /// ```rust
    /// // Start of frame
    /// renderer.clear_rectangles();
    ///
    /// // Add current frame's menu elements
    /// for menu_item in menu.items() {
    ///     renderer.add_rectangle(menu_item.to_rectangle());
    /// }
    ///
    /// // Render all elements
    /// renderer.render(&device, &mut render_pass);
    /// ```
    pub fn clear_rectangles(&mut self) {
        self.rectangles.clear();
    }

    /// Updates the window dimensions for coordinate transformation.
    ///
    /// This method must be called whenever the window is resized to
    /// ensure rectangles are positioned correctly. The renderer uses
    /// these dimensions to convert from screen coordinates to
    /// normalized device coordinates (NDC).
    ///
    /// ## Parameters
    ///
    /// - `width`: New window width in pixels
    /// - `height`: New window height in pixels
    ///
    /// ## Important
    ///
    /// Call this method in your window resize handler to maintain
    /// correct menu positioning and proportions.
    pub fn resize(&mut self, width: f32, height: f32) {
        self.window_width = width;
        self.window_height = height;
    }

    /// Renders all queued rectangles in a single optimized draw call.
    ///
    /// This method performs batch rendering of all rectangles that have
    /// been added since the last clear. It:
    ///
    /// 1. Converts screen coordinates to normalized device coordinates
    /// 2. Generates vertices for each rectangle (4 vertices per rectangle)
    /// 3. Creates indices for triangle rendering (2 triangles per rectangle)
    /// 4. Uploads vertex and index data to the GPU
    /// 5. Executes a single indexed draw call
    ///
    /// ## Performance Notes
    ///
    /// - All rectangles are rendered in one draw call for maximum performance
    /// - Vertex and index buffers are created dynamically each frame
    /// - Memory allocation is proportional to the number of rectangles
    /// - GPU memory is automatically reclaimed after rendering
    ///
    /// ## Coordinate Transformation
    ///
    /// The method handles the conversion from screen space (0,0 at top-left)
    /// to NDC space (-1,-1 to 1,1 with origin at center), including proper
    /// Y-axis flipping for correct rendering.
    ///
    /// ## Parameters
    ///
    /// - `device`: WebGPU device for creating GPU buffers
    /// - `render_pass`: Active render pass to submit draw commands to
    pub fn render(&mut self, device: &Device, render_pass: &mut RenderPass) {
        // Early return if no rectangles to render
        if self.rectangles.is_empty() {
            return;
        }

        // Set the rendering pipeline for rectangle rendering
        render_pass.set_pipeline(&self.render_pipeline);

        // Batch all rectangle data for efficient GPU upload
        let mut all_vertices = Vec::new();
        let mut all_indices = Vec::new();

        // Process each rectangle in the render queue
        for (rect_index, rectangle) in self.rectangles.iter().enumerate() {
            // Transform screen coordinates to normalized device coordinates (NDC)
            // Screen space: (0,0) = top-left, positive Y = down
            // NDC space: (-1,-1) = bottom-left, (1,1) = top-right
            let x = (rectangle.x / self.window_width) * 2.0 - 1.0;
            let y = 1.0 - (rectangle.y / self.window_height) * 2.0; // Flip Y-axis
            let width = (rectangle.width / self.window_width) * 2.0;
            let height = -(rectangle.height / self.window_height) * 2.0; // Negative due to Y-flip

            // Create the four vertices for this rectangle
            // Each vertex contains position, color, UV coords, size, and corner radius
            let vertices = [
                // Top-left vertex
                Vertex {
                    position: [x, y],
                    color: rectangle.color,
                    uv: [0.0, 0.0], // UV coordinates for fragment shader distance calculations
                    rect_size: [rectangle.width, rectangle.height],
                    corner_radius: rectangle.corner_radius,
                    _padding: 0.0,
                },
                // Top-right vertex
                Vertex {
                    position: [x + width, y],
                    color: rectangle.color,
                    uv: [rectangle.width, 0.0],
                    rect_size: [rectangle.width, rectangle.height],
                    corner_radius: rectangle.corner_radius,
                    _padding: 0.0,
                },
                // Bottom-right vertex
                Vertex {
                    position: [x + width, y + height],
                    color: rectangle.color,
                    uv: [rectangle.width, rectangle.height],
                    rect_size: [rectangle.width, rectangle.height],
                    corner_radius: rectangle.corner_radius,
                    _padding: 0.0,
                },
                // Bottom-left vertex
                Vertex {
                    position: [x, y + height],
                    color: rectangle.color,
                    uv: [0.0, rectangle.height],
                    rect_size: [rectangle.width, rectangle.height],
                    corner_radius: rectangle.corner_radius,
                    _padding: 0.0,
                },
            ];

            // Add vertices to the batch buffer
            all_vertices.extend_from_slice(&vertices);

            // Create triangle indices for this rectangle
            // Two triangles per rectangle: (0,1,2) and (0,2,3)
            let base_index = (rect_index * 4) as u16;
            let indices = [
                base_index,
                base_index + 1,
                base_index + 2, // First triangle
                base_index,
                base_index + 2,
                base_index + 3, // Second triangle
            ];
            all_indices.extend_from_slice(&indices);
        }

        // Create GPU vertex buffer with all rectangle vertices
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Rectangle Vertex Buffer"),
            contents: bytemuck::cast_slice(&all_vertices),
            usage: BufferUsages::VERTEX,
        });

        // Create GPU index buffer with all triangle indices
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Rectangle Index Buffer"),
            contents: bytemuck::cast_slice(&all_indices),
            usage: BufferUsages::INDEX,
        });

        // Bind buffers and execute the draw call
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        // Draw all rectangles in a single indexed draw call
        // This renders all menu elements with optimal GPU performance
        render_pass.draw_indexed(0..all_indices.len() as u32, 0, 0..1);
    }
}
