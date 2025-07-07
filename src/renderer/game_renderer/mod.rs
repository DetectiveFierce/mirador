pub mod compass;
pub mod debug;
pub mod enemy;
pub mod game_over;
pub mod stars;

use crate::game::GameState;
use crate::game::enemy::Enemy;
use crate::math::deg_to_rad;
use crate::math::mat::Mat4;
use crate::maze::parse_maze_file;
use crate::renderer::game_renderer::compass::CompassRenderer;
use crate::renderer::game_renderer::debug::DebugRenderer;
use crate::renderer::game_renderer::enemy::EnemyRenderer;
use crate::renderer::game_renderer::stars::StarRenderer;
use crate::renderer::pipeline_builder::PipelineBuilder;
use crate::renderer::primitives::{Uniforms, Vertex};
use egui_wgpu::wgpu;
use egui_wgpu::wgpu::util::DeviceExt;
/// Main renderer for the 3D maze game.
///
/// The `GameRenderer` is responsible for rendering the complete 3D maze environment,
/// including floors, walls, starfield backgrounds, and debug overlays. It manages
/// all rendering pipelines, vertex data, and GPU resources required for the game's
/// visual presentation.
///
/// ## Rendering Pipeline
///
/// The renderer uses a multi-pass approach:
/// 1. **Background Pass**: Renders animated starfield using `StarRenderer`
/// 2. **Main Pass**: Renders maze geometry (floors and walls) with depth testing
/// 3. **Debug Pass**: Optional overlay rendering for development tools
///
/// ## Memory Management
///
/// Vertex data for floors and walls is combined into a single buffer for optimal
/// GPU performance. The depth texture is dynamically resized based on the surface
/// dimensions and automatically managed to prevent memory leaks.
///
/// ## Coordinate System
///
/// The renderer uses a right-handed coordinate system with:
/// - X-axis pointing right
/// - Y-axis pointing up
/// - Z-axis pointing toward the viewer
/// - Maze positioned at world origin
///
/// ## Shader Integration
///
/// Works with `shader.wgsl` which expects:
/// ```wgsl
/// struct Uniforms {
///     matrix: mat4x4<f32>,
/// }
/// @group(0) @binding(0) var<uniform> uniforms: Uniforms;
/// ```
///
/// # Fields
///
/// - `pipeline` - Main render pipeline for maze geometry with depth testing and alpha blending
/// - `vertex_buffer` - Combined vertex buffer containing both floor and wall geometry data
/// - `vertex_count` - Total number of vertices to render from the combined buffer
/// - `uniform_buffer` - GPU buffer storing model-view-projection matrix for vertex transformations
/// - `uniform_bind_group` - WebGPU bind group linking uniform buffer to shader binding point 0
/// - `depth_texture` - Optional depth buffer for proper 3D occlusion (recreated on resize)
/// - `star_renderer` - Background renderer for animated starfield effects
/// - `debug_renderer` - Development tools for rendering bounding boxes and debug overlays
pub struct GameRenderer {
    pub pipeline: wgpu::RenderPipeline,
    /// Combined vertex buffer containing both floor and wall geometry data.
    pub vertex_buffer: wgpu::Buffer,
    /// Total number of vertices to render from the combined buffer.
    pub vertex_count: u32,
    /// GPU buffer storing model-view-projection matrix for vertex transformations.
    pub uniform_buffer: wgpu::Buffer,
    /// WebGPU bind group linking uniform buffer to shader binding point 0.
    pub uniform_bind_group: wgpu::BindGroup,
    /// Optional depth buffer for proper 3D occlusion (recreated on resize).
    pub depth_texture: Option<wgpu::Texture>,
    /// Background renderer for animated starfield effects.
    pub star_renderer: StarRenderer,
    /// Development tools for rendering bounding boxes and debug overlays.
    pub debug_renderer: DebugRenderer,
    /// Renderer for compass
    pub compass_renderer: CompassRenderer,
    pub exit_position: Option<(f32, f32)>,
    pub enemy_renderer: EnemyRenderer,
}

impl GameRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        let uniforms = Uniforms::new();
        let uniform_buffer = uniforms.create_buffer(device);
        let (uniform_bind_group, uniform_bind_group_layout) =
            uniforms.create_bind_group(&uniform_buffer, device);

        let pipeline = PipelineBuilder::new(device, surface_config.format)
            .with_label("Main Pipeline")
            .with_shader(include_str!("../shaders/main-shader.wgsl"))
            .with_vertex_buffer(Vertex::desc())
            .with_bind_group_layout(&uniform_bind_group_layout)
            .with_blend_state(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::OVER,
            })
            .with_no_culling()
            .with_depth_stencil(wgpu::DepthStencilState {
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                format: wgpu::TextureFormat::Depth24Plus,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
            .build();

        // Load wall grid from file
        let (maze_grid, exit_cell) = parse_maze_file("src/maze/saved-mazes/test.mz");

        let (mut floor_vertices, _exit_position) =
            Vertex::create_floor_vertices(&maze_grid, exit_cell);

        // Generate wall geometry
        let mut wall_vertices = Vertex::create_wall_vertices(&maze_grid);

        // Append wall vertices to floor
        floor_vertices.append(&mut wall_vertices);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Combined Vertex Buffer"),
            contents: bytemuck::cast_slice(&floor_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let star_renderer = stars::create_star_renderer(device, surface_config, 100);

        let debug_renderer = DebugRenderer {
            debug_render_bounding_boxes: false,
            debug_vertex_buffer: None,
            debug_vertex_count: 0,
        };

        let compass_renderer = CompassRenderer::new(device, queue, surface_config);
        let enemy = Enemy::new([-1370.0, 50.0, 1370.0], 100.0);
        let enemy_renderer = EnemyRenderer::new(enemy, device, queue, surface_config);
        Self {
            pipeline,
            vertex_buffer,
            vertex_count: floor_vertices.len() as u32,
            uniform_buffer,
            uniform_bind_group,
            depth_texture: None,
            star_renderer,
            debug_renderer,
            compass_renderer,
            exit_position: None,
            enemy_renderer,
        }
    }

    pub fn update_depth_texture(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> wgpu::TextureView {
        if self.depth_texture.is_none()
            || self.depth_texture.as_ref().unwrap().width() != width
            || self.depth_texture.as_ref().unwrap().height() != height
        {
            if let Some(depth_texture) = self.depth_texture.take() {
                // Manually drop the texture to free up resources
                drop(depth_texture);
            }

            self.depth_texture = Some(device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth24Plus,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }));
        }
        self.depth_texture
            .as_ref()
            .unwrap()
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn render_game(
        &mut self,
        queue: &wgpu::Queue,
        game_state: &GameState,
        pass: &mut wgpu::RenderPass,
        aspect: f32,
    ) {
        // Calculate view and projection matrices once
        let view_matrix = game_state.player.get_view_matrix();
        let projection_matrix = Mat4::perspective(
            deg_to_rad(game_state.player.fov),
            aspect,
            0.1,    // zNear
            2000.0, // zFar
        );
        let view_proj_matrix = view_matrix.multiply(&projection_matrix);

        // ==============================================
        // 1. RENDER MAZE/FLOOR FIRST
        // ==============================================
        {
            // Model Matrix for floor - identity since floor is at world origin
            let model_matrix = Mat4::identity();

            // Combine matrices: Projection * View * Model
            let final_mvp_matrix = model_matrix.multiply(&view_proj_matrix);

            let uniforms = Uniforms {
                matrix: final_mvp_matrix.into(),
            };

            // Upload uniform values for the maze/floor
            queue.write_buffer(&self.uniform_buffer, 0, uniforms.as_bytes());

            // Render the maze/floor
            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            pass.draw(0..self.vertex_count, 0..1);

            // Debug rendering for maze/floor
            if self.debug_renderer.debug_render_bounding_boxes
                && self.debug_renderer.debug_vertex_count > 0
            {
                if let Some(debug_buffer) = &self.debug_renderer.debug_vertex_buffer {
                    pass.set_vertex_buffer(0, debug_buffer.slice(..));
                    pass.draw(0..self.debug_renderer.debug_vertex_count as u32, 0..1);
                }
            }
        }

        // ==============================================
        // 2. RENDER ENEMIES
        // ==============================================
        {
            // Update enemy transform with the combined view-projection matrix
            self.enemy_renderer.update(
                queue,
                game_state,
                view_proj_matrix.0, // Pass the view-projection matrix
            );

            // Actually render the enemy
            self.enemy_renderer.render(pass); // You might need to pass the actual window here
        }
    }
}
