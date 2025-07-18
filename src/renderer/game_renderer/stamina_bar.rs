use crate::renderer::pipeline_builder::{
    BindGroupLayoutBuilder, PipelineBuilder, create_uniform_buffer,
};
use std::time::Instant;
use wgpu;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct StaminaBarUniforms {
    pub progress: f32,
    pub time: f32,
    pub resolution: [f32; 2],
    pub _padding: [f32; 2],
}

pub struct StaminaBarRenderer {
    pub pipeline: wgpu::RenderPipeline,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub start_time: Instant,
}

impl StaminaBarRenderer {
    pub fn new(device: &wgpu::Device, surface_config: &wgpu::SurfaceConfiguration) -> Self {
        let uniforms = StaminaBarUniforms {
            progress: 1.0,
            time: 0.0,
            resolution: [800.0, 600.0],
            _padding: [0.0; 2],
        };
        let uniform_buffer = create_uniform_buffer(device, &uniforms, "Stamina Bar Uniform Buffer");
        let bind_group_layout = BindGroupLayoutBuilder::new(device)
            .with_label("Stamina Bar Bind Group Layout")
            .with_uniform_buffer(0, wgpu::ShaderStages::VERTEX_FRAGMENT)
            .build();
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("Stamina Bar Bind Group"),
        });
        let pipeline = PipelineBuilder::new(device, surface_config.format)
            .with_label("Stamina Bar Pipeline")
            .with_shader(include_str!("../shaders/loading-bar.wgsl"))
            .with_bind_group_layout(&bind_group_layout)
            .with_alpha_blending()
            .build();
        Self {
            pipeline,
            uniform_buffer,
            bind_group,
            start_time: Instant::now(),
        }
    }
    pub fn update_uniforms(
        &self,
        queue: &wgpu::Queue,
        progress: f32,
        resolution: [f32; 2],
        time: f32,
    ) {
        let uniforms = StaminaBarUniforms {
            progress: progress.clamp(0.0, 1.0), // Do NOT invert, so bar shrinks as stamina decreases
            time,
            resolution,
            _padding: [0.0; 2],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }
    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
