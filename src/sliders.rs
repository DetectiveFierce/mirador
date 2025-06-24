use crate::app::AppState;
use crate::egui_lib;
use std::sync::Arc;
use winit::window::Window;

pub struct UiState {
    pub show_sliders: bool,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub start_time: std::time::Instant,
    pub elapsed_time: f32,
}

impl UiState {
    pub fn new() -> Self {
        UiState {
            show_sliders: true,
            r: 0.003,
            g: 0.0003,
            b: 0.007,
            start_time: std::time::Instant::now(),
            elapsed_time: 0.0,
            // FOV in radians, ~100 degrees
        }
    }
}

fn custom_slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    speed: f64,
    decimals: usize,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(
            egui::DragValue::new(value)
                .speed(speed)
                .fixed_decimals(decimals)
                .range(range.clone())
                .suffix(""),
        );
        ui.add(egui::Slider::new(value, range).show_value(false));
    });
}

impl AppState {
    pub fn update_ui(&mut self, window: &Arc<Window>) {
        {
            self.egui_renderer.begin_frame(window);
            if self.ui.show_sliders {
                egui::Window::new("Debug Sliders:")
                    .default_open(true)
                    .collapsible(false)
                    .default_size(egui::Vec2::new(200.0, 0.0))
                    .show(self.egui_renderer.context(), |ui| {
                        ui.spacing_mut().slider_width = 100.0;
                        match egui_lib::ui_theme() {
                            Ok(visuals) => ui.ctx().set_visuals(visuals),
                            Err(e) => {
                                eprintln!("Failed to load custom theme: {}", e);
                                // Optionally fall back to default dark/light theme
                                ui.ctx().set_visuals(egui::Visuals::dark()); // or Visuals::light()
                            }
                        }
                        ui.vertical(|ui| {
                            ui.label("Background Color");
                            custom_slider(ui, "Red", &mut self.ui.r, 0.0..=1.0, 0.01, 2);
                            custom_slider(ui, "Green", &mut self.ui.g, 0.0..=1.0, 0.01, 2);
                            custom_slider(ui, "Blue", &mut self.ui.b, 0.0..=1.0, 0.01, 2);

                            ui.separator();
                            // FOV: Smaller FOV means a more zoomed-in view, larger means wider.
                            // If you go too high, distortion increases. Too low, you might crop the F.
                            // A reasonable range to keep the F visible without extreme distortion.
                            custom_slider(
                                ui,
                                "FOV",
                                &mut self.game_state.player.fov,
                                1.0..=179.0, // Constrained to physically meaningful range
                                1.0,
                                2,
                            );

                            // custom_slider(
                            //     ui,
                            //     "Pitch",
                            //     &mut self.game_state.player.pitch,
                            //     1.0..=359.0,
                            //     1.0,
                            //     2,
                            // );
                            // custom_slider(
                            //     ui,
                            //     "Yaw",
                            //     &mut self.game_state.player.yaw,
                            //     1.0..=359.0,
                            //     1.0,
                            //     2,
                            // );
                        })
                    });
            }
        }
    }
}
