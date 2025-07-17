//! Update logic for Mirador App.
//!
//! Contains update and game logic methods for the App struct.
use crate::game::GameTimer;
use crate::game::enemy::place_enemy_standard;
use crate::game::maze::parse_maze_file;
use crate::game::player::Player;
use crate::game::{self, CurrentScreen, TimerConfig};
use crate::math::coordinates::maze_to_world;
use crate::renderer::loading_renderer::LoadingRenderer;
use crate::renderer::primitives::Vertex;
use crate::test_mode::setup_test_environment;
use std::time::Duration;
use std::time::Instant;
use wgpu;
use wgpu::util::DeviceExt;

use super::event_handler::App;

impl App {
    // handle_redraw
    pub fn handle_redraw(&mut self) {
        let window = self
            .window
            .as_ref()
            .expect("Window must be initialized before use");
        if window.is_minimized().unwrap_or(false) {
            println!("Window is minimized");
            return;
        }

        let state = self
            .state
            .as_mut()
            .expect("State must be initialized before use");

        if state.game_state.current_screen == CurrentScreen::Loading {
            state
                .game_state
                .audio_manager
                .pause_enemy_audio("enemy")
                .expect("Failed to pause enemy audio");
            state.handle_loading_screen(window);
        } else if state.game_state.current_screen == CurrentScreen::Title {
            crate::renderer::title::handle_title(state, window);
            return;
        } else {
            state.game_state.player.update_cell(
                &state
                    .wgpu_renderer
                    .loading_screen_renderer
                    .maze
                    .lock()
                    .unwrap()
                    .walls,
                state.game_state.is_test_mode,
            );
        }

        // Update game state and UI
        state.key_state.update(&mut state.game_state);
        state.update_game_ui(window);
        state
            .game_state
            .audio_manager
            .set_listener_position(state.game_state.player.position)
            .expect("Failed to set listener position");
        state
            .game_state
            .audio_manager
            .update_enemy_position("enemy", state.game_state.enemy.pathfinder.position)
            .expect("Failed to update enemy position");

        // Prepare rendering commands
        let mut encoder = state
            .wgpu_renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        // Update canvas surface
        let (surface_view, surface_texture) = match state.wgpu_renderer.update_canvas(
            window,
            &mut encoder,
            &state.game_state,
            &mut state.text_renderer,
        ) {
            Ok(result) => result,
            Err(err) => {
                eprintln!("Failed to update canvas: {}", err);
                #[cfg(debug_assertions)]
                eprintln!("Backtrace: {:?}", std::backtrace::Backtrace::capture());
                return;
            }
        };

        // --- Debug Info Panel ---
        if state.pause_menu.is_debug_panel_visible() {
            let window_size = &state.wgpu_renderer.surface_config;

            // Add timer information to debug text
            let timer_info = if let Some(timer) = &state.game_state.game_ui.timer {
                let remaining = timer.get_remaining_time();
                let total = timer.config.duration;
                let remaining_secs = remaining.as_secs_f64();
                let total_secs = total.as_secs_f64();
                let progress = if total_secs > 0.0 {
                    remaining_secs / total_secs
                } else {
                    0.0
                };
                format!(
                    "Window Size: {} x {}\nTimer: {:.2}s / {:.2}s ({:.1}%)",
                    window_size.width,
                    window_size.height,
                    remaining_secs,
                    total_secs,
                    progress * 100.0
                )
            } else {
                format!(
                    "Window Size: {} x {}\nTimer: Not active",
                    window_size.width, window_size.height
                )
            };

            let style = crate::renderer::text::TextStyle {
                font_family: "HankenGrotesk".to_string(),
                font_size: 22.0,
                line_height: 26.0,
                color: glyphon::Color::rgb(220, 40, 40),
                weight: glyphon::Weight::BOLD,
                style: glyphon::Style::Normal,
            };
            let pos = crate::renderer::text::TextPosition {
                x: window_size.width as f32 - 320.0,
                y: 20.0,
                max_width: Some(300.0),
                max_height: Some(80.0), // Increased height for two lines
            };
            state.text_renderer.create_text_buffer(
                "debug_info",
                &timer_info,
                Some(style),
                Some(pos),
            );
        } else {
            // Hide debug info by making it transparent if it exists
            if let Some(buf) = state.text_renderer.text_buffers.get_mut("debug_info") {
                buf.visible = false;
            }
        }
        // Prepare and render text BEFORE pause menu overlay
        if let Err(e) = state.text_renderer.prepare(
            &state.wgpu_renderer.device,
            &state.wgpu_renderer.queue,
            &state.wgpu_renderer.surface_config,
        ) {
            println!("Failed to prepare text renderer: {}", e);
        }
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                label: Some("text render pass"),
                occlusion_query_set: None,
            });
            if let Err(e) = state.text_renderer.render(&mut render_pass) {
                println!("Failed to render text: {}", e);
            }
        }
        // --- End Game UI ---

        // If paused, render the pause menu on top
        if state.game_state.current_screen == CurrentScreen::Pause {
            if !state.pause_menu.is_visible() {
                state.pause_menu.show(state.game_state.is_test_mode);
            }

            // Create a render pass for the pause menu
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                label: Some("pause menu render pass"),
                occlusion_query_set: None,
            });

            // --- Add semi-transparent grey overlay ---
            let overlay_color = [0.08, 0.09, 0.11, 0.88]; // darker, neutral semi-transparent grey
            let (w, h) = (
                state.wgpu_renderer.surface_config.width as f32,
                state.wgpu_renderer.surface_config.height as f32,
            );
            state
                .pause_menu
                .button_manager
                .rectangle_renderer
                .add_rectangle(crate::renderer::rectangle::Rectangle::new(
                    0.0,
                    0.0,
                    w,
                    h,
                    overlay_color,
                ));
            state
                .pause_menu
                .button_manager
                .rectangle_renderer
                .render(&state.wgpu_renderer.device, &mut render_pass);
            // --- End overlay ---

            // Prepare pause menu for rendering (text)
            if let Err(e) = state.pause_menu.prepare(
                &state.wgpu_renderer.device,
                &state.wgpu_renderer.queue,
                &state.wgpu_renderer.surface_config,
            ) {
                println!("Failed to prepare pause menu: {}", e);
            }

            // Render the pause menu (rectangles + text)
            if let Err(e) = state
                .pause_menu
                .render(&state.wgpu_renderer.device, &mut render_pass)
            {
                println!("Failed to render pause menu: {}", e);
            }
        } else {
            if state.pause_menu.is_visible() {
                state.pause_menu.hide();
            }
            // Explicitly clear rectangles if menu is not visible
            state
                .pause_menu
                .button_manager
                .rectangle_renderer
                .clear_rectangles();
        }

        window.request_redraw();

        // Submit commands and present
        state.wgpu_renderer.queue.submit(Some(encoder.finish()));

        // Present the surface texture and ensure it's properly handled
        surface_texture.present();

        // Poll the device to process any pending operations
        // This helps ensure resources are properly cleaned up and prevents
        // the "SurfaceSemaphores still in use" error during cleanup
        state.wgpu_renderer.device.poll(wgpu::Maintain::Poll);

        // Update enemy pathfinding
        state.game_state.enemy.update(
            state.game_state.player.position,
            state.game_state.delta_time,
            state.game_state.game_ui.level as u32,
            |from, to| {
                state
                    .game_state
                    .collision_system
                    .cylinder_intersects_geometry(from, to, 5.0)
            },
        );

        // Handle title screen animation if needed
        if state.game_state.current_screen == CurrentScreen::Loading {
            state.game_state.game_ui.stop_timer();
            self.handle_maze_generation();
        } else if state.game_state.current_screen == CurrentScreen::NewGame {
            state.text_renderer.hide_game_over_display();
            self.new_level(true);
        } else if state.game_state.current_screen == CurrentScreen::Game
            && Some(state.game_state.player.current_cell) == state.game_state.exit_cell
        {
            state.game_state.enemy.pathfinder.position = [0.0, 30.0, 0.0];
            state.game_state.enemy.pathfinder.locked = true;
            self.new_level(false);
        } else if state.game_state.current_screen == CurrentScreen::Game {
            state
                .game_state
                .audio_manager
                .resume_enemy_audio("enemy")
                .expect("Failed to resume enemy audio");
            if !state.game_state.is_test_mode {
                state.game_state.enemy.pathfinder.locked = false;
            }
        }
    }

    // handle_frame_timing
    pub fn handle_frame_timing(&mut self, current_time: Instant) {
        if let Some(state) = self.state.as_mut() {
            let duration = current_time.duration_since(state.game_state.last_fps_time);

            state.elapsed_time = current_time.duration_since(state.start_time);
            state.game_state.frame_count += 1;

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

            if state
                .wgpu_renderer
                .game_renderer
                .debug_renderer
                .debug_render_bounding_boxes
            {
                state
                    .wgpu_renderer
                    .game_renderer
                    .debug_renderer
                    .update_debug_vertices(
                        &state.wgpu_renderer.device,
                        &state.game_state.collision_system,
                    );
            }
        }
    }

    // handle_maze_generation
    pub fn handle_maze_generation(&mut self) {
        if let Some(state) = self.state.as_mut() {
            // If in test mode, skip maze generation entirely and go directly to game
            if state.game_state.is_test_mode {
                println!("Test mode enabled - skipping maze generation and going to game");
                // Mark generation as complete immediately
                state
                    .wgpu_renderer
                    .loading_screen_renderer
                    .generator
                    .generation_complete = true;
                // Set up test environment immediately
                setup_test_environment(&mut state.game_state, &mut state.wgpu_renderer);
                // Set a dummy maze path to prevent re-entry
                state.game_state.maze_path = Some(std::path::PathBuf::from("test_mode"));
                // Go directly to game screen
                state.game_state.current_screen = CurrentScreen::Game;
                return;
            }

            let renderer = &mut state.wgpu_renderer.loading_screen_renderer;

            // Calculate update timing
            let speed = if renderer.generator.fast_mode {
                Duration::from_millis(10) / 20
            } else {
                Duration::from_millis(10)
            };

            // Skip if not time to update or already complete
            if renderer.last_update.elapsed() < speed || renderer.generator.is_complete() {
                return;
            }

            // Process animation steps
            let steps = if renderer.generator.fast_mode {
                300
            } else {
                100
            };
            for _ in 0..steps {
                if !renderer.generator.step() {
                    break;
                }
            }

            // Report progress
            let (current, total) = renderer.generator.get_progress();

            if current % 50 == 0 || renderer.generator.is_complete() {
                println!(
                    "Progress: {}/{} ({:.1}%)",
                    current,
                    total,
                    (current as f32 * 100.0 / total.max(1) as f32)
                );
            }
            if renderer.generator.is_complete() && state.game_state.maze_path.is_none() {
                println!("Maze generation complete! Saving to file...");

                // Play completion sound
                state
                    .game_state
                    .audio_manager
                    .complete()
                    .expect("Failed to play complete sound!");

                // Handle completion
                if renderer.generator.is_complete() {
                    println!("Maze generation complete! Saving to file...");
                    let maze_lock = renderer.maze.lock().unwrap();
                    state.game_state.maze_path = maze_lock.save_to_file().map_or_else(
                        |err| {
                            eprintln!("Failed to save maze: {}", err);
                            std::process::exit(1);
                        },
                        Some,
                    );

                    // Generate geometry if maze was saved successfully
                    if let Some(maze_path) = &state.game_state.maze_path {
                        let (maze_grid, exit_cell) = parse_maze_file(maze_path.to_str().unwrap());
                        let (mut floor_vertices, exit_position) = Vertex::create_floor_vertices(
                            &maze_grid,
                            exit_cell,
                            state.game_state.is_test_mode,
                        );

                        state.wgpu_renderer.game_renderer.exit_position = Some(exit_position);

                        floor_vertices.append(&mut Vertex::create_wall_vertices(
                            &maze_grid,
                            state.game_state.is_test_mode,
                        ));

                        state.wgpu_renderer.game_renderer.vertex_buffer = state
                            .wgpu_renderer
                            .device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("Combined Vertex Buffer"),
                                contents: bytemuck::cast_slice(&floor_vertices),
                                usage: wgpu::BufferUsages::VERTEX,
                            });

                        // Update vertex count so the renderer knows how many vertices to draw
                        state.wgpu_renderer.game_renderer.vertex_count =
                            floor_vertices.len() as u32;

                        if let Some(exit_cell_position) = exit_cell {
                            state.game_state.exit_cell = Some(exit_cell_position);
                            state.game_state.enemy = place_enemy_standard(
                                maze_to_world(
                                    &exit_cell_position,
                                    maze_lock.get_dimensions(),
                                    30.0,
                                    state.game_state.is_test_mode,
                                ),
                                state.game_state.player.position,
                                state.game_state.game_ui.level,
                                |from, to| {
                                    state
                                        .game_state
                                        .collision_system
                                        .cylinder_intersects_geometry(from, to, 5.0)
                                },
                            );
                        }

                        state
                            .game_state
                            .collision_system
                            .build_from_maze(&maze_grid, state.game_state.is_test_mode);

                        // Spawn the player at the bottom-left corner of the maze
                        state
                            .game_state
                            .player
                            .spawn_at_maze_entrance(&maze_grid, state.game_state.is_test_mode);
                        // (No automatic transition to Game here)
                    }
                }
            }
        }
    }

    // new_level
    pub fn new_level(&mut self, game_over: bool) {
        let state = self
            .state
            .as_mut()
            .expect("State must be initialized before use");
        state.game_state.current_screen = CurrentScreen::Loading;
        state.game_state.maze_path = None;
        state.wgpu_renderer.loading_screen_renderer = LoadingRenderer::new(
            &state.wgpu_renderer.device,
            &state.wgpu_renderer.surface_config,
        );

        // Clear previous level state
        state.game_state.player = Player::new();
        state.game_state.enemy.pathfinder.position = [0.0, 30.0, 0.0];
        state.game_state.enemy.pathfinder.locked = true;
        state.game_state.exit_cell = None; // Clear exit cell to prevent accidental win condition

        // Stop and reset timer
        if let Some(timer) = &mut state.game_state.game_ui.timer {
            timer.stop();
            timer.reset();
        }

        if game_over {
            state.game_state.set_level(1);
            state.game_state.set_score(0);
            state.game_state.game_ui.timer = Some(GameTimer::new(TimerConfig::default()));
            game::update_game_ui(
                &mut state.text_renderer,
                &mut state.game_state.game_ui,
                &state.game_state.current_screen,
                self.window
                    .as_ref()
                    .expect("Window must be initialized before use"),
            );
            // Ensure clean state for new game
            state.game_state.exit_cell = None;
        } else {
            let current_level = state.game_state.game_ui.level;

            // Calculate completion time and performance metrics
            let (completion_time, _) = if let Some(timer) = &state.game_state.game_ui.timer {
                let remaining_time = timer.get_remaining_time().as_secs_f32();
                let total_time = timer.config.duration.as_secs_f32();
                let completion_time = total_time - remaining_time;

                // Performance-based time bonus calculation
                // Optimal time: 15 seconds, Average time: 25 seconds, Slow time: 35+ seconds
                let time_bonus = if completion_time <= 15.0_f32 {
                    // Exceptional performance: 15-25 seconds added
                    let performance_ratio = (15.0 - completion_time).max(0.0_f32) / 15.0_f32;
                    15.0_f32 + (performance_ratio * 10.0_f32)
                } else if completion_time <= 25.0_f32 {
                    // Good performance: 8-15 seconds added
                    let performance_ratio = (25.0 - completion_time) / 10.0_f32;
                    8.0_f32 + (performance_ratio * 7.0_f32)
                } else if completion_time <= 35.0_f32 {
                    // Average performance: 3-8 seconds added
                    let performance_ratio = (35.0 - completion_time) / 10.0_f32;
                    3.0_f32 + (performance_ratio * 5.0_f32)
                } else {
                    // Slow completion: 1-3 seconds added
                    let performance_ratio = (45.0 - completion_time).max(0.0_f32) / 10.0_f32;
                    1.0_f32 + (performance_ratio * 2.0_f32)
                };

                (completion_time, time_bonus)
            } else {
                (30.0_f32, 3.0_f32) // Fallback values
            };

            // Enhanced scoring system
            let base_score = 150 * current_level as u32; // Increased base score

            // Speed bonus calculation
            let speed_bonus = if completion_time <= 15.0_f32 {
                // Exceptional: 3x to 5x multiplier
                let multiplier = 3.0_f32 + ((15.0_f32 - completion_time) / 15.0_f32) * 2.0_f32;
                (base_score as f32 * multiplier) as u32
            } else if completion_time <= 25.0_f32 {
                // Good: 1.5x to 3x multiplier
                let multiplier = 1.5_f32 + ((25.0_f32 - completion_time) / 10.0_f32) * 1.5_f32;
                (base_score as f32 * multiplier) as u32
            } else if completion_time <= 35.0_f32 {
                // Average: 0.5x to 1.5x multiplier
                let multiplier = 0.5_f32 + ((35.0_f32 - completion_time) / 10.0_f32) * 1.0_f32;
                (base_score as f32 * multiplier) as u32
            } else {
                // Slow: 0.1x to 0.5x multiplier
                let multiplier =
                    0.1_f32 + ((45.0_f32 - completion_time).max(0.0_f32) / 10.0_f32) * 0.4_f32;
                (base_score as f32 * multiplier) as u32
            };

            // Level progression bonus (small bonus for reaching higher levels)
            let level_bonus = if current_level > 5 {
                (current_level - 5) as u32 * 50
            } else {
                0
            };

            // Consecutive level bonus (reward for sustained performance)
            let consecutive_bonus = if completion_time <= 20.0_f32 {
                // Only give consecutive bonus for good performance
                current_level as u32 * 25
            } else {
                0
            };

            let total_score = base_score + speed_bonus + level_bonus + consecutive_bonus;

            // Update score and level
            state
                .game_state
                .set_score(state.game_state.game_ui.score + total_score);
            state.game_state.set_level(current_level + 1);

            // Enhanced time management: Not supported in new timer, so skip add_time/subtract_time/prev_time
        }
    }
}
