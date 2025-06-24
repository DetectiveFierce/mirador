use crate::game::GameState;
use crate::keys::GameKey;
use crate::keys::KeyState;
use crate::{egui_lib::EguiRenderer, sliders::UiState, wgpu_lib::WgpuRenderer};
use egui_wgpu::wgpu;
use std::sync::Arc;
use std::time::Instant;
use winit::keyboard;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, DeviceId, ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

pub struct AppState {
    pub wgpu_renderer: WgpuRenderer,
    pub egui_renderer: EguiRenderer,
    pub ui: UiState,
    pub game_state: GameState,
    pub key_state: KeyState, // Add key state tracking
}

impl AppState {
    async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        window: &Window,
        width: u32,
        height: u32,
    ) -> Self {
        window.set_cursor_visible(false);

        let wgpu_renderer = WgpuRenderer::new(instance, surface, width, height).await;
        let egui_renderer = EguiRenderer::new(
            &wgpu_renderer.device,
            wgpu_renderer.surface_config.format,
            None,
            1,
            window,
        );

        Self {
            wgpu_renderer,
            egui_renderer,
            ui: UiState::new(),
            game_state: GameState::new(),
            key_state: KeyState::new(), // Initialize key state
        }
    }

    fn resize_surface(&mut self, width: u32, height: u32) {
        self.wgpu_renderer.surface_config.width = width;
        self.wgpu_renderer.surface_config.height = height;
        self.wgpu_renderer.surface.configure(
            &self.wgpu_renderer.device,
            &self.wgpu_renderer.surface_config,
        );
    }

    fn lock_cursor(&mut self, window: &Window) {
        if let Err(e) = window.set_cursor_grab(winit::window::CursorGrabMode::Locked) {
            eprintln!("Failed to lock cursor: {}", e);
        }
    }

    fn center_mouse(&mut self, window: &Window) {
        self.lock_cursor(window);
        window.set_cursor_visible(false);
        let window_size = window.inner_size().to_logical::<f64>(window.scale_factor());

        let center_x = window_size.width / 2.0;
        let center_y = window_size.height / 2.0;
        if let Err(e) =
            window.set_cursor_position(winit::dpi::LogicalPosition::new(center_x, center_y))
        {
            eprintln!("Failed to center cursor: {}", e);
        }
    }

    fn handle_mouse_motion(&mut self, delta: (f64, f64), window: &Window) {
        self.game_state
            .player
            .handle_mouse_movement(delta.0, delta.1);
        self.center_mouse(window);
    }

    // Convert winit key to our game key enum
    fn winit_key_to_game_key(key: &keyboard::Key) -> Option<GameKey> {
        match key.to_text() {
            Some("w") | Some("ArrowUp") => Some(GameKey::MoveForward),
            Some("s") | Some("ArrowDown") => Some(GameKey::MoveBackward),
            Some("a") | Some("ArrowLeft") => Some(GameKey::MoveLeft),
            Some("d") | Some("ArrowRight") => Some(GameKey::MoveRight),
            Some("c") => Some(GameKey::ToggleSliders),
            Some("q") => Some(GameKey::Quit),
            _ => None,
        }
    }

    // Process all currently pressed keys for movement
    fn process_movement(&mut self) {
        let delta_time = self.game_state.delta_time;

        if self.key_state.is_pressed(GameKey::MoveForward) {
            self.game_state.player.move_forward(delta_time);
        }
        if self.key_state.is_pressed(GameKey::MoveBackward) {
            self.game_state.player.move_backward(delta_time);
        }
        if self.key_state.is_pressed(GameKey::MoveLeft) {
            self.game_state.player.move_left(delta_time);
        }
        if self.key_state.is_pressed(GameKey::MoveRight) {
            self.game_state.player.move_right(delta_time);
        }
    }
}

pub struct App {
    instance: wgpu::Instance,
    state: Option<AppState>,
    window: Option<Arc<Window>>,
}

impl App {
    pub fn new() -> Self {
        let instance = egui_wgpu::wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        Self {
            instance,
            state: None,
            window: None,
        }
    }

    async fn set_window(&mut self, window: Window) {
        let window = Arc::new(window);
        let initial_width = 1360;
        let initial_height = 768;

        let _ = window.request_inner_size(PhysicalSize::new(initial_width, initial_height));

        let surface = self
            .instance
            .create_surface(window.clone())
            .expect("Failed to create surface!");

        let state = AppState::new(
            &self.instance,
            surface,
            &window,
            initial_width,
            initial_height, // Fixed: was initial_width twice
        )
        .await;

        self.window.get_or_insert(window);
        self.state.get_or_insert(state);
    }

    fn handle_resized(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            let state = match &mut self.state {
                Some(state) => state,
                None => {
                    panic!("State must be initialized before use");
                }
            };
            state.resize_surface(width, height);
        }
    }

    fn handle_redraw(&mut self) {
        // Attempt to handle minimizing window
        if let Some(window) = self.window.as_ref() {
            if let Some(min) = window.is_minimized() {
                if min {
                    println!("Window is minimized");
                    return;
                }
            }
        }

        let state = match &mut self.state {
            Some(state) => state,
            None => {
                panic!("State must be initialized before use");
            }
        };

        let window = match &self.window {
            Some(window) => window,
            None => {
                panic!("Window must be initialized before use");
            }
        };

        // Process movement based on currently pressed keys
        state.process_movement();

        // All of the UI code is handled by this 'update_ui' function which is defined in the gui module
        state.update_ui(window);

        let mut encoder = state
            .wgpu_renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        // Beyond the UI, the meat of the rendering code is handled by this 'update_canvas' function which is defined in the wgpu_lib module
        let (surface_view, screen_descriptor, surface_texture) =
            match state.wgpu_renderer.update_canvas(
                window,
                &state.ui,
                &mut encoder,
                state.ui.start_time,
                &state.game_state.player,
            ) {
                Ok((surface_view, screen_descriptor, surface_texture)) => {
                    (surface_view, screen_descriptor, surface_texture)
                }
                Err(err) => {
                    panic!("Failed to update canvas: {}", err);
                }
            };

        state.egui_renderer.end_frame_and_draw(
            &state.wgpu_renderer.device,
            &state.wgpu_renderer.queue,
            &mut encoder,
            window,
            &surface_view,
            screen_descriptor,
        );

        state.wgpu_renderer.queue.submit(Some(encoder.finish()));
        surface_texture.present();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = match event_loop.create_window(Window::default_attributes()) {
            Ok(window) => window,
            Err(err) => {
                panic!("Failed to create window: {}", err);
            }
        };
        pollster::block_on(self.set_window(window));
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                if let Some(state) = self.state.as_mut() {
                    let window = match &mut self.window {
                        Some(window) => window,
                        None => return,
                    };
                    state.handle_mouse_motion(delta, window.as_ref());
                }
            }
            DeviceEvent::MouseWheel { delta } => match delta {
                winit::event::MouseScrollDelta::LineDelta(x, y) => {
                    println!("Mouse wheel line delta: ({}, {})", x, y);
                }
                winit::event::MouseScrollDelta::PixelDelta(pos) => {
                    println!("Mouse wheel pixel delta: ({}, {})", pos.x, pos.y);
                }
            },
            DeviceEvent::Button { button, state } => {
                println!("Mouse button {}: {:?}", button, state);
            }
            DeviceEvent::Key(key_input) => {
                println!("Device key event: {:?}", key_input);
            }
            _ => {}
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let state = match self.state.as_mut() {
            Some(state) => state,
            None => {
                panic!("State not initialized");
            }
        };
        let window = match self.window.as_ref() {
            Some(window) => window,
            None => {
                panic!("Window not initialized");
            }
        };

        state.egui_renderer.handle_input(window, &event);

        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: key,
                        state: key_state,
                        repeat,
                        ..
                    },
                ..
            } => {
                // Ignore repeat events to avoid OS key repeat behavior
                if repeat {
                    return;
                }

                if let Some(game_key) = AppState::winit_key_to_game_key(&key) {
                    match key_state {
                        ElementState::Pressed => {
                            state.key_state.press_key(game_key);

                            // Handle non-movement keys immediately on press
                            match game_key {
                                GameKey::Quit => event_loop.exit(),
                                GameKey::ToggleSliders => {
                                    state.ui.show_sliders = !state.ui.show_sliders;
                                }
                                _ => {} // Movement keys are handled in process_movement
                            }
                        }
                        ElementState::Released => {
                            state.key_state.release_key(game_key);
                        }
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                self.handle_redraw();
                if let Some(state) = self.state.as_mut() {
                    state.ui.elapsed_time += 1.0;
                    let current_time = Instant::now();
                    let delta_time = current_time
                        .duration_since(state.game_state.last_frame_time)
                        .as_secs_f32();
                    state.game_state.delta_time = delta_time;
                    state.game_state.last_frame_time = current_time;
                } else {
                    eprintln!("Warning: Cannot update elapsed time - state not initialized");
                }
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                } else {
                    eprintln!("Warning: Cannot request redraw - window not available");
                }
            }
            WindowEvent::Resized(new_size) => {
                self.handle_resized(new_size.width, new_size.height);
            }
            _ => (),
        }
    }
}
