use egui_wgpu::wgpu::{
    self, BlendState, BufferUsages, ColorTargetState, ColorWrites, Device, FragmentState,
    MultisampleState, PrimitiveState, RenderPass, RenderPipeline, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, util::DeviceExt,
};
use std::mem;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
    // Add UV coordinates for the fragment shader
    uv: [f32; 2],
    // Add rectangle dimensions and corner radius
    rect_size: [f32; 2],
    corner_radius: f32,
    _padding: f32, // Ensure 16-byte alignment
}

impl Vertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x2,
                },
                // Color
                VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x4,
                },
                // UV
                VertexAttribute {
                    offset: (mem::size_of::<[f32; 2]>() + mem::size_of::<[f32; 4]>())
                        as wgpu::BufferAddress,
                    shader_location: 2,
                    format: VertexFormat::Float32x2,
                },
                // Rectangle size
                VertexAttribute {
                    offset: (mem::size_of::<[f32; 2]>()
                        + mem::size_of::<[f32; 4]>()
                        + mem::size_of::<[f32; 2]>())
                        as wgpu::BufferAddress,
                    shader_location: 3,
                    format: VertexFormat::Float32x2,
                },
                // Corner radius
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

#[derive(Debug, Clone)]
pub struct Rectangle {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: [f32; 4],
    pub corner_radius: f32,
}

impl Rectangle {
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

    pub fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }
}

pub struct RectangleRenderer {
    render_pipeline: RenderPipeline,
    rectangles: Vec<Rectangle>,
    window_width: f32,
    window_height: f32,
}

impl RectangleRenderer {
    pub fn new(device: &Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rectangle Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/rectangle.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Rectangle Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

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
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
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
            window_width: 1360.0,
            window_height: 768.0,
        }
    }

    pub fn add_rectangle(&mut self, rectangle: Rectangle) {
        self.rectangles.push(rectangle);
    }

    pub fn clear_rectangles(&mut self) {
        self.rectangles.clear();
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.window_width = width;
        self.window_height = height;
    }

    pub fn render(&mut self, device: &Device, render_pass: &mut RenderPass) {
        if self.rectangles.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.render_pipeline);

        // Create all vertices for all rectangles in one batch
        let mut all_vertices = Vec::new();
        let mut all_indices = Vec::new();

        for (rect_index, rectangle) in self.rectangles.iter().enumerate() {
            // Convert screen coordinates to normalized device coordinates
            // Note: Y-axis is flipped in screen coordinates (0,0 is top-left)
            let x = (rectangle.x / self.window_width) * 2.0 - 1.0;
            let y = 1.0 - (rectangle.y / self.window_height) * 2.0; // Flip Y-axis
            let width = (rectangle.width / self.window_width) * 2.0;
            let height = -(rectangle.height / self.window_height) * 2.0; // Negative because Y is flipped

            // Create vertices for this rectangle
            let vertices = [
                // Top-left
                Vertex {
                    position: [x, y],
                    color: rectangle.color,
                    uv: [0.0, 0.0],
                    rect_size: [rectangle.width, rectangle.height],
                    corner_radius: rectangle.corner_radius,
                    _padding: 0.0,
                },
                // Top-right
                Vertex {
                    position: [x + width, y],
                    color: rectangle.color,
                    uv: [rectangle.width, 0.0],
                    rect_size: [rectangle.width, rectangle.height],
                    corner_radius: rectangle.corner_radius,
                    _padding: 0.0,
                },
                // Bottom-right
                Vertex {
                    position: [x + width, y + height],
                    color: rectangle.color,
                    uv: [rectangle.width, rectangle.height],
                    rect_size: [rectangle.width, rectangle.height],
                    corner_radius: rectangle.corner_radius,
                    _padding: 0.0,
                },
                // Bottom-left
                Vertex {
                    position: [x, y + height],
                    color: rectangle.color,
                    uv: [0.0, rectangle.height],
                    rect_size: [rectangle.width, rectangle.height],
                    corner_radius: rectangle.corner_radius,
                    _padding: 0.0,
                },
            ];

            // Add vertices to the batch
            all_vertices.extend_from_slice(&vertices);

            // Create indices for this rectangle (offset by the current vertex count)
            let base_index = (rect_index * 4) as u16;
            let indices = [
                base_index,
                base_index + 1,
                base_index + 2,
                base_index,
                base_index + 2,
                base_index + 3,
            ];
            all_indices.extend_from_slice(&indices);
        }

        // Create vertex buffer for all rectangles
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Rectangle Vertex Buffer"),
            contents: bytemuck::cast_slice(&all_vertices),
            usage: BufferUsages::VERTEX,
        });

        // Create index buffer for all rectangles
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Rectangle Index Buffer"),
            contents: bytemuck::cast_slice(&all_indices),
            usage: BufferUsages::INDEX,
        });

        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..all_indices.len() as u32, 0, 0..1);
    }
}
