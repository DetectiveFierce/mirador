use crate::maze::parse_maze_file;
use crate::renderer::vertex::Vertex;
use crate::{
    game::{
        GameState,
        keys::{GameKey, KeyState, winit_key_to_game_key},
    },
    renderer::wgpu_lib::WgpuRenderer,
    ui::{egui_lib::EguiRenderer, sliders::UiState},
};
use egui_wgpu::wgpu;
use egui_wgpu::wgpu::util::DeviceExt;
use std::time::Duration;
use std::{sync::Arc, time::Instant};
use winit::event::MouseButton;
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

    fn center_mouse(&mut self, window: &Window) {
        if self.game_state.capture_mouse {
            if let Err(e) = window.set_cursor_grab(winit::window::CursorGrabMode::Locked) {
                eprintln!("Failed to lock cursor: {}", e);
            }
            window.set_cursor_visible(false);
            let window_size = window.inner_size().to_logical::<f64>(window.scale_factor());

            let center_x = window_size.width / 2.0;
            let center_y = window_size.height / 2.0;
            if let Err(e) =
                window.set_cursor_position(winit::dpi::LogicalPosition::new(center_x, center_y))
            {
                eprintln!("Failed to center cursor: {}", e);
            }
        } else if !self.game_state.capture_mouse {
            if let Err(e) = window.set_cursor_grab(winit::window::CursorGrabMode::None) {
                eprintln!("Failed to unlock cursor: {}", e);
            }
            window.set_cursor_visible(true);
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
            initial_height,
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
        let window = match &self.window {
            Some(window) => window,
            None => {
                panic!("Window must be initialized before use");
            }
        };

        // Attempt to handle minimizing window
        if let Some(min) = window.is_minimized() {
            if min {
                println!("Window is minimized");
                return;
            }
        }

        let state = match &mut self.state {
            Some(state) => state,
            None => {
                panic!("State must be initialized before use");
            }
        };

        // Process movement based on currently pressed keys
        state.key_state.update(&mut state.game_state);

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
                state.game_state.title_screen,
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

        if state.game_state.title_screen {
            let animation_speed = Duration::from_millis(10);
            let fast_mode_speed = animation_speed / 20; // 20x faster in fast mode

            let speed = if state
                .wgpu_renderer
                .title_screen_renderer
                .generator
                .fast_mode
            {
                fast_mode_speed
            } else {
                animation_speed
            };

            // Animation timing
            if state
                .wgpu_renderer
                .title_screen_renderer
                .last_update
                .elapsed()
                >= speed
                && !state
                    .wgpu_renderer
                    .title_screen_renderer
                    .generator
                    .is_complete()
            {
                // Process multiple steps when in fast mode for better performance
                let steps_per_frame = if state
                    .wgpu_renderer
                    .title_screen_renderer
                    .generator
                    .fast_mode
                {
                    30
                } else {
                    10
                };

                for _ in 0..steps_per_frame {
                    if !state.wgpu_renderer.title_screen_renderer.generator.step() {
                        break;
                    }
                }

                let (current, total) = state
                    .wgpu_renderer
                    .title_screen_renderer
                    .generator
                    .get_progress();
                if current % 50 == 0
                    || state
                        .wgpu_renderer
                        .title_screen_renderer
                        .generator
                        .is_complete()
                {
                    let mode_indicator = "";

                    println!(
                        "Progress: {}/{} ({:.1}%){}",
                        current,
                        total,
                        (current as f32 * 100.0 / total.max(1) as f32),
                        mode_indicator
                    );

                    if state
                        .wgpu_renderer
                        .title_screen_renderer
                        .generator
                        .is_complete()
                    {
                        println!("Maze generation complete! Saving to file...");
                        let maze_lock = state
                            .wgpu_renderer
                            .title_screen_renderer
                            .maze
                            .lock()
                            .unwrap();
                        state.game_state.maze_path = match maze_lock.save_to_file() {
                            Ok(path) => Some(path),
                            Err(err) => {
                                eprintln!("Failed to save maze: {}", err);
                                std::process::exit(1);
                            }
                        };

                        let (mut floor_vertices, _floor_vertex_count) =
                            Vertex::create_floor_vertices();

                        if state.game_state.maze_path.is_some() {
                            let maze_grid = parse_maze_file(
                                state
                                    .game_state
                                    .maze_path
                                    .as_mut()
                                    .unwrap()
                                    .to_str()
                                    .unwrap(),
                            );
                            // Generate wall geometry
                            let mut wall_vertices = Vertex::create_wall_vertices(&maze_grid);
                            // Append wall vertices to floor
                            floor_vertices.append(&mut wall_vertices);

                            let vertex_buffer = state.wgpu_renderer.device.create_buffer_init(
                                &wgpu::util::BufferInitDescriptor {
                                    label: Some("Combined Vertex Buffer"),
                                    contents: bytemuck::cast_slice(&floor_vertices),
                                    usage: wgpu::BufferUsages::VERTEX,
                                },
                            );
                            state.wgpu_renderer.vertex_buffer = vertex_buffer;
                        }
                    }
                }
            }
        }
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
                    state
                        .game_state
                        .player
                        .handle_mouse_movement(delta.0, delta.1);
                    state.center_mouse(window);
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

                if let Some(game_key) = winit_key_to_game_key(&key) {
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

            WindowEvent::MouseInput { state, button, .. } => match state {
                ElementState::Pressed => {
                    if let Some(app_state) = self.state.as_mut() {
                        match button {
                            MouseButton::Left => {
                                app_state.key_state.press_key(GameKey::MouseButtonLeft);
                            }
                            MouseButton::Right => {
                                app_state.key_state.press_key(GameKey::MouseButtonRight);
                            }
                            _ => {}
                        }
                    }
                }
                ElementState::Released => {
                    if let Some(app_state) = self.state.as_mut() {
                        match button {
                            MouseButton::Left => {
                                app_state.key_state.release_key(GameKey::MouseButtonLeft);
                            }
                            MouseButton::Right => {
                                app_state.key_state.release_key(GameKey::MouseButtonRight);
                            }
                            _ => {}
                        }
                    }
                }
            },

            WindowEvent::RedrawRequested => {
                self.handle_redraw();
                if let Some(state) = self.state.as_mut() {
                    if state.game_state.title_screen {
                        let progress = state
                            .wgpu_renderer
                            .title_screen_renderer
                            .generator
                            .get_progress_ratio();
                        let (maze_width, maze_height) = {
                            let maze_lock = state
                                .wgpu_renderer
                                .title_screen_renderer
                                .maze
                                .lock()
                                .unwrap();
                            maze_lock.get_dimensions()
                        };

                        state
                            .wgpu_renderer
                            .title_screen_renderer
                            .update_loading_bar(&state.wgpu_renderer.queue, progress);

                        let maze_data = {
                            let maze_lock = state
                                .wgpu_renderer
                                .title_screen_renderer
                                .maze
                                .lock()
                                .unwrap();
                            maze_lock.get_render_data(
                                &state
                                    .wgpu_renderer
                                    .title_screen_renderer
                                    .generator
                                    .connected_cells,
                            )
                        };
                        state.wgpu_renderer.title_screen_renderer.update_texture(
                            &state.wgpu_renderer.queue,
                            &maze_data,
                            maze_width,
                            maze_height,
                        );
                        state.wgpu_renderer.title_screen_renderer.last_update = Instant::now();
                    }
                    state.ui.elapsed_time += 1.0;
                    state.game_state.frame_count += 1;
                    let current_time = Instant::now();
                    let duration = current_time.duration_since(state.game_state.last_fps_time);

                    if duration.as_secs_f32() >= 1.0 {
                        state.game_state.current_fps = state.game_state.frame_count;
                        state.game_state.frame_count = 0;
                        state.game_state.last_fps_time = current_time;
                    }

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
