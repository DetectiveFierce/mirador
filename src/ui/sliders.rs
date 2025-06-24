use crate::app::AppState;
use crate::ui::egui_lib;
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

fn _custom_slider(
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
                    .default_size(egui::Vec2::new(225.0, 0.0))
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
                            let pos = self.game_state.player.position;
                            ui.label(format!(
                                "Position:  x: {:.2},  y: {:.2},  z: {:.2}",
                                pos[0], pos[1], pos[2]
                            ));
                            ui.label(format!(
                                "Pitch: {:.2}   Yaw: {:.2}   Speed: {:.2}",
                                self.game_state.player.pitch,
                                self.game_state.player.yaw,
                                self.game_state.player.speed
                            ));
                            // Display key state (assuming Debug is implemented for KeyState)
                            ui.label(format!("KeyState: {:?}", self.key_state.pressed_keys));

                            ui.label(format!("FPS: {}", self.game_state.current_fps));
                        })
                    });
            }
        }
    }
}
