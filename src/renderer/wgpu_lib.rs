use crate::background::stars::{self, StarRenderer};
use crate::game::player::Player;
use crate::math::{deg_to_rad, mat::Mat4};
use crate::maze::parse_maze_file;
use crate::renderer::uniform::Uniforms;
use crate::renderer::vertex::Vertex;
use crate::ui::sliders::UiState;
use egui_wgpu::ScreenDescriptor;
use egui_wgpu::wgpu;
use egui_wgpu::wgpu::util::DeviceExt;
use egui_wgpu::wgpu::{SurfaceTexture, TextureView};
pub struct WgpuRenderer {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub num_vertices: u32,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub depth_texture: Option<wgpu::Texture>,
    pub background: StarRenderer,
}

impl WgpuRenderer {
    pub async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
    ) -> Self {
        let power_pref = wgpu::PowerPreference::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: power_pref,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let features = wgpu::Features::empty();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: features,
                    required_limits: Default::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let selected_format = wgpu::TextureFormat::Bgra8UnormSrgb;
        let swapchain_format = swapchain_capabilities
            .formats
            .iter()
            .find(|d| **d == selected_format)
            .expect("failed to select proper surface texture format!");

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: *swapchain_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 0,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &surface_config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Main Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let uniforms = Uniforms::new();
        let uniform_buffer = uniforms.create_buffer(&device);
        let (uniform_bind_group, uniform_bind_group_layout) =
            uniforms.create_bind_group(&uniform_buffer, &device);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Main Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: *swapchain_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                format: wgpu::TextureFormat::Depth24Plus,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let (mut floor_vertices, _floor_vertex_count) = Vertex::create_floor_vertices();

        // Load wall grid from file
        let maze_grid = parse_maze_file("src/mazes/test.mz");

        // Generate wall geometry
        let mut wall_vertices = Vertex::create_wall_vertices(&maze_grid);

        // Append wall vertices to floor
        floor_vertices.append(&mut wall_vertices);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Combined Vertex Buffer"),
            contents: bytemuck::cast_slice(&floor_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let star_renderer = stars::create_star_renderer(&device, &surface_config, 100);

        Self {
            surface,
            device,
            queue,
            surface_config,
            uniform_buffer,
            uniform_bind_group,
            pipeline,
            vertex_buffer,
            num_vertices: floor_vertices.len() as u32,
            depth_texture: None,
            background: star_renderer,
        }
    }

    pub fn update_canvas(
        &mut self,
        window: &winit::window::Window,
        ui_state: &UiState,
        encoder: &mut wgpu::CommandEncoder,
        start_time: std::time::Instant,
        player: &Player,
    ) -> Result<(TextureView, ScreenDescriptor, SurfaceTexture), String> {
        let surface_texture_obj = self.surface.get_current_texture();

        let surface_texture = match surface_texture_obj {
            Err(wgpu::SurfaceError::Outdated) => {
                // Ignoring outdated to allow resizing and minimization
                println!("wgpu surface outdated");
                return Err("wgpu surface outdated".to_string());
            }
            Err(_) => {
                surface_texture_obj.expect("Failed to acquire next swap chain texture");
                return Err("Failed to acquire next swap chain texture".to_string());
            }
            Ok(surface_texture) => surface_texture,
        };

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.surface_config.width, self.surface_config.height],
            pixels_per_point: window.scale_factor() as f32,
        };
        // If we don't have a depth texture OR if its size is different
        // from the canvasTexture when make a new depth texture
        let (width, height) = (self.surface_config.width, self.surface_config.height);

        let depth_texture_view = {
            if self.depth_texture.is_none()
                || self.depth_texture.as_ref().unwrap().width() != width
                || self.depth_texture.as_ref().unwrap().height() != height
            {
                if let Some(depth_texture) = self.depth_texture.take() {
                    // Manually drop the texture to free up resources
                    drop(depth_texture);
                }

                self.depth_texture = Some(self.device.create_texture(&wgpu::TextureDescriptor {
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
        };

        {
            let clear_pass_desc = wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: ui_state.r as f64,
                            g: ui_state.g as f64,
                            b: ui_state.b as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            };
            let clear_pass = encoder.begin_render_pass(&clear_pass_desc);
            // No draw calls needed - just clears

            drop(clear_pass)
        }

        {
            let elapsed_time = start_time.elapsed().as_secs_f32();
            self.background
                .update_background_color(&self.queue, [ui_state.r, ui_state.g, ui_state.b, 1.0]);
            self.background.update_time(&self.queue, elapsed_time);
            self.queue.write_buffer(
                &self.background.time_buffer,
                0,
                bytemuck::cast_slice(&[elapsed_time]),
            );

            let star_pass_desc = wgpu::RenderPassDescriptor {
                label: Some("Star Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Preserve cleared background
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None, // No depth testing for stars
                occlusion_query_set: None,
                timestamp_writes: None,
            };
            let mut star_pass = encoder.begin_render_pass(&star_pass_desc);
            star_pass.set_pipeline(&self.background.pipeline);
            star_pass.set_bind_group(0, &self.background.uniform_bind_group, &[]);
            star_pass.set_vertex_buffer(0, self.background.vertex_buffer.slice(..));
            star_pass.set_index_buffer(
                self.background.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            star_pass.draw_indexed(0..self.background.num_indices, 0, 0..1);
            drop(star_pass);
        };

        let aspect = width as f32 / height as f32;

        // Step 1: Model Matrix - Just identity since the floor is at world origin
        let model_matrix = Mat4::identity();

        // Step 2: View Matrix - Based on player's camera position and orientation
        let view_matrix = player.get_view_matrix();

        // Step 3: Projection Matrix - Using FOV from UI state
        let projection_matrix = Mat4::perspective(
            deg_to_rad(player.fov),
            aspect,
            0.1,    // zNear
            2000.0, // zFar
        );

        // Step 4: Combine matrices: Projection * View * Model
        let final_mvp_matrix = projection_matrix
            .multiply(&view_matrix)
            .multiply(&model_matrix);

        let uniforms = Uniforms {
            matrix: final_mvp_matrix.into(), // Access the inner `[[f32; 4]; 4]` array
        };
        // upload the uniform values to the uniform buffer
        self.queue
            .write_buffer(&self.uniform_buffer, 0, uniforms.as_bytes());

        let main_pass_desc = wgpu::RenderPassDescriptor {
            label: Some("Main Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        };
        let mut main_pass = encoder.begin_render_pass(&main_pass_desc);
        main_pass.set_pipeline(&self.pipeline);
        main_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        main_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        main_pass.draw(0..self.num_vertices, 0..1);

        Ok((surface_view, screen_descriptor, surface_texture))
    }
}
