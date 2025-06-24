use egui::Context;
use egui_wgpu::wgpu::{CommandEncoder, Device, Queue, StoreOp, TextureFormat, TextureView};
use egui_wgpu::{Renderer, ScreenDescriptor, wgpu};
use egui_winit::State;
use winit::event::WindowEvent;
use winit::window::Window;

pub struct EguiRenderer {
    state: State,
    renderer: Renderer,
    frame_started: bool,
}

impl EguiRenderer {
    pub fn context(&self) -> &Context {
        self.state.egui_ctx()
    }

    pub fn new(
        device: &Device,
        output_color_format: TextureFormat,
        output_depth_format: Option<TextureFormat>,
        msaa_samples: u32,
        window: &Window,
    ) -> EguiRenderer {
        let egui_context = Context::default();

        let egui_state = egui_winit::State::new(
            egui_context,
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            Some(2 * 1024), // default dimension is 2048
        );
        let egui_renderer = Renderer::new(
            device,
            output_color_format,
            output_depth_format,
            msaa_samples,
            true,
        );

        EguiRenderer {
            state: egui_state,
            renderer: egui_renderer,
            frame_started: false,
        }
    }

    pub fn handle_input(&mut self, window: &Window, event: &WindowEvent) {
        let _ = self.state.on_window_event(window, event);
    }

    pub fn ppp(&mut self, v: f32) {
        self.context().set_pixels_per_point(v);
    }

    pub fn begin_frame(&mut self, window: &Window) {
        let raw_input = self.state.take_egui_input(window);
        self.state.egui_ctx().begin_pass(raw_input);
        self.frame_started = true;
    }

    pub fn end_frame_and_draw(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        window_surface_view: &TextureView,
        screen_descriptor: ScreenDescriptor,
    ) {
        if !self.frame_started {
            panic!("begin_frame must be called before end_frame_and_draw can be called!");
        }

        self.ppp(screen_descriptor.pixels_per_point);

        let full_output = self.state.egui_ctx().end_pass();

        self.state
            .handle_platform_output(window, full_output.platform_output);

        let tris = self
            .state
            .egui_ctx()
            .tessellate(full_output.shapes, self.state.egui_ctx().pixels_per_point());
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }
        self.renderer
            .update_buffers(device, queue, encoder, &tris, &screen_descriptor);
        let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: window_surface_view,
                resolve_target: None,
                ops: egui_wgpu::wgpu::Operations {
                    load: egui_wgpu::wgpu::LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            label: Some("egui main render pass"),
            occlusion_query_set: None,
        });

        self.renderer
            .render(&mut rpass.forget_lifetime(), &tris, &screen_descriptor);
        for x in &full_output.textures_delta.free {
            self.renderer.free_texture(x)
        }

        self.frame_started = false;
    }
}

use egui::{
    Color32, CornerRadius, Shadow, Stroke,
    style::{
        HandleShape::Circle, NumericColorSpace::GammaByte, Selection, TextCursorStyle, Visuals,
        WidgetVisuals, Widgets,
    },
};

pub fn ui_theme() -> Result<Visuals, String> {
    // Helper macro to reduce repetition
    macro_rules! color {
        ($hex:expr) => {
            Color32::from_hex($hex).map_err(|e| format!("Invalid color {}: {:?}", $hex, e))?
        };
    }

    Ok(Visuals {
        dark_mode: true,
        override_text_color: None,
        widgets: Widgets {
            noninteractive: WidgetVisuals {
                bg_fill: color!("#182C00"),      // background
                weak_bg_fill: color!("#293911"), // element.background
                bg_stroke: Stroke {
                    width: 1.0,
                    color: color!("#182C00"), // border
                },
                corner_radius: CornerRadius::same(2),
                fg_stroke: Stroke {
                    width: 1.0,
                    color: color!("#92b161"), // text
                },
                expansion: 0.0,
            },
            inactive: WidgetVisuals {
                bg_fill: color!("#293911"),      // element.background
                weak_bg_fill: color!("#293911"), // element.background
                bg_stroke: Stroke {
                    width: 0.0,
                    color: Color32::TRANSPARENT,
                },
                corner_radius: CornerRadius::same(2),
                fg_stroke: Stroke {
                    width: 1.0,
                    color: color!("#92b161"), // text
                },
                expansion: 0.0,
            },
            hovered: WidgetVisuals {
                bg_fill: color!("#8aa513"),      // element.hover
                weak_bg_fill: color!("#8aa513"), // element.hover
                bg_stroke: Stroke {
                    width: 1.0,
                    color: color!("#182C00"), // border
                },
                corner_radius: CornerRadius::same(3),
                fg_stroke: Stroke {
                    width: 1.5,
                    color: color!("#92b161"), // text
                },
                expansion: 1.0,
            },
            active: WidgetVisuals {
                bg_fill: color!("#3a5412"),      // element.selected
                weak_bg_fill: color!("#3a5412"), // element.selected
                bg_stroke: Stroke {
                    width: 1.0,
                    color: color!("#182C00"), // border
                },
                corner_radius: CornerRadius::same(2),
                fg_stroke: Stroke {
                    width: 2.0,
                    color: color!("#92b161"), // text
                },
                expansion: 1.0,
            },
            open: WidgetVisuals {
                bg_fill: color!("#182C00"),      // background
                weak_bg_fill: color!("#293911"), // element.background
                bg_stroke: Stroke {
                    width: 1.0,
                    color: color!("#182C00"), // border
                },
                corner_radius: CornerRadius::same(2),
                fg_stroke: Stroke {
                    width: 1.0,
                    color: color!("#92b161"), // text
                },
                expansion: 0.0,
            },
        },
        selection: Selection {
            bg_fill: color!("#566dda3d"), // players[0].selection
            stroke: Stroke {
                width: 1.0,
                color: color!("#566ddaff"), // players[0].cursor
            },
        },
        hyperlink_color: color!("#6A7EC8"), // terminal.ansi.blue
        faint_bg_color: Color32::TRANSPARENT,
        extreme_bg_color: color!("#0d1303"), // terminal.background
        code_bg_color: color!("#1a2d00"),    // editor.background
        warn_fg_color: color!("#B3B42B"),    // terminal.ansi.yellow
        error_fg_color: color!("#C4265E"),   // terminal.ansi.red
        window_corner_radius: CornerRadius::same(6),
        window_shadow: Shadow {
            offset: [10, 20],
            blur: 15,
            spread: 0,
            color: color!("#00000060"),
        },
        window_fill: color!("#182C00"), // background
        window_stroke: Stroke {
            width: 1.0,
            color: color!("#182C00"), // border
        },
        window_highlight_topmost: true,
        menu_corner_radius: CornerRadius::same(6),
        panel_fill: color!("#182C00"), // background
        popup_shadow: Shadow {
            offset: [6, 10],
            blur: 8,
            spread: 0,
            color: color!("#00000060"),
        },
        resize_corner_size: 12.0,
        text_cursor: TextCursorStyle {
            stroke: Stroke {
                width: 2.0,
                color: color!("#566ddaff"), // players[0].cursor
            },
            preview: false,
            blink: true,
            on_duration: 0.5,
            off_duration: 0.5,
        },
        clip_rect_margin: 3.0,
        button_frame: true,
        collapsing_header_frame: false,
        indent_has_left_vline: true,
        striped: false,
        slider_trailing_fill: false,
        handle_shape: Circle,
        interact_cursor: None,
        image_loading_spinners: true,
        numeric_color_space: GammaByte,
    })
}
