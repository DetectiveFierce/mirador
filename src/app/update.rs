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
    /// Handles the main rendering loop and game state updates.
    ///
    /// This method is called every frame and orchestrates the complete rendering pipeline.
    /// It handles different game screens (loading, title, game, pause, upgrade menu),
    /// updates game state, renders the scene, and manages UI overlays.
    ///
    /// # Rendering Pipeline
    /// 1. **Screen-Specific Logic**: Handles different game screens appropriately
    /// 2. **Game State Updates**: Updates player, enemy, audio, and UI systems
    /// 3. **Rendering**: Creates command encoder and renders the scene
    /// 4. **UI Overlays**: Renders pause menu, upgrade menu, and debug information
    /// 5. **Frame Submission**: Submits commands and presents the frame
    ///
    /// # Screen Handling
    /// - **Loading**: Handles maze generation and loading screen rendering
    /// - **Title**: Renders title screen and handles transitions
    /// - **Game**: Updates player movement, enemy AI, and game logic
    /// - **Pause**: Renders pause menu overlay
    /// - **UpgradeMenu**: Handles upgrade selection and menu interactions
    ///
    /// # Error Handling
    /// - Logs errors for canvas update failures
    /// - Continues execution even if some systems fail
    /// - Provides debug backtraces in debug builds
    ///
    /// # Performance
    /// - Skips rendering if window is minimized
    /// - Uses efficient command encoding for GPU operations
    /// - Manages GPU resource cleanup and polling
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

        // Start timing the entire frame
        state.profiler.start_section("total_frame");

        if state.game_state.current_screen == CurrentScreen::Loading {
            state
                .game_state
                .audio_manager
                .pause_enemy_audio("enemy")
                .expect("Failed to pause enemy audio");
            state.handle_loading_screen(window);
        } else if state.game_state.current_screen == CurrentScreen::Title {
            crate::renderer::title::handle_title(state, window);
            state.upgrade_menu.upgrade_manager.player_upgrades.clear();
            state.game_state.player = crate::game::player::Player::new();
            state.game_state.enemy = crate::game::enemy::Enemy::new([0.0, 30.0, 0.0], 150.0);
            return;
        } else if state.game_state.current_screen == CurrentScreen::UpgradeMenu {
            // Handle upgrade menu - just update it, rendering is handled separately
            state.upgrade_menu.update();
            // Pass player and game_state to handle_input if needed (if input is handled here)

            // Check if upgrade menu is no longer visible (upgrade was selected)
            if !state.upgrade_menu.is_visible() {
                println!("Upgrade menu is no longer visible, transitioning to loading screen...");
                // Continue to next level
                state.game_state.current_screen = CurrentScreen::Loading;
                // Ensure upgrade menu is completely hidden
                state.upgrade_menu.hide();
                // Recapture mouse when leaving upgrade menu
                state.game_state.capture_mouse = true;
                if let Some(window) = self.window.as_ref() {
                    state.triage_mouse(window);
                    // Force a redraw to ensure the transition is visible
                    window.request_redraw();
                }
                {
                    // Limit the mutable borrow of self/state to this block
                    self.new_level(false);
                }
                // Upgrade effects are already applied in handle_input when the upgrade was selected
                return;
            } else {
                state.game_state.capture_mouse = false;
            }
            // Don't return early - let the normal rendering pipeline continue
        } else {
            state.game_state.player.update_cell(
                &state
                    .wgpu_renderer
                    .loading_screen_renderer
                    .maze
                    .lock()
                    .expect("Failed to lock maze")
                    .walls,
                state.game_state.is_test_mode,
            );
        }

        // Update game state and UI
        state.profiler.start_section("game_state_update");
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
        state.profiler.end_section("game_state_update");

        // Update audio manager to process any pending audio operations
        state.profiler.start_section("audio_update");
        if let Err(e) = state.game_state.audio_manager.update() {
            println!("Failed to update audio manager: {:?}", e);
        }
        state.profiler.end_section("audio_update");

        // Prepare rendering commands
        state.profiler.start_section("command_encoder_creation");
        let mut encoder = state
            .wgpu_renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        state.profiler.end_section("command_encoder_creation");

        // Update canvas surface
        state.profiler.start_section("canvas_update");
        let (surface_view, surface_texture) = match state.wgpu_renderer.update_canvas(
            window,
            &mut encoder,
            &state.game_state,
            &mut state.text_renderer,
            state.start_time,
        ) {
            Ok(result) => result,
            Err(err) => {
                eprintln!("Failed to update canvas: {}", err);
                #[cfg(debug_assertions)]
                eprintln!("Backtrace: {:?}", std::backtrace::Backtrace::capture());
                return;
            }
        };
        state.profiler.end_section("canvas_update");

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
                font_family: "Hanken Grotesk".to_string(),
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
        state.profiler.start_section("text_preparation");
        if let Err(e) = state.text_renderer.prepare(
            &state.wgpu_renderer.device,
            &state.wgpu_renderer.queue,
            &state.wgpu_renderer.surface_config,
        ) {
            println!("Failed to prepare text renderer: {}", e);
        }
        state.profiler.end_section("text_preparation");
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
            state.profiler.start_section("text_rendering");
            if let Err(e) = state.text_renderer.render(&mut render_pass) {
                println!("Failed to render text: {}", e);
            }
            state.profiler.end_section("text_rendering");
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

        // If in upgrade menu, render the upgrade menu on top
        if state.game_state.current_screen == CurrentScreen::UpgradeMenu {
            // Prepare the upgrade menu
            if let Err(e) = state.upgrade_menu.prepare(
                &state.wgpu_renderer.device,
                &state.wgpu_renderer.queue,
                &state.wgpu_renderer.surface_config,
            ) {
                println!("Failed to prepare upgrade menu: {}", e);
            }

            // Create a render pass for the upgrade menu
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
                label: Some("upgrade menu render pass"),
                occlusion_query_set: None,
            });

            // --- Add semi-transparent overlay for upgrade menu ---
            let overlay_color = [0.08, 0.09, 0.11, 0.88]; // darker, neutral semi-transparent grey
            let (w, h) = (
                state.wgpu_renderer.surface_config.width as f32,
                state.wgpu_renderer.surface_config.height as f32,
            );
            state
                .upgrade_menu
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
                .upgrade_menu
                .button_manager
                .rectangle_renderer
                .render(&state.wgpu_renderer.device, &mut render_pass);

            // Render the upgrade menu (rectangles + text)
            if let Err(e) = state
                .upgrade_menu
                .render(&state.wgpu_renderer.device, &mut render_pass)
            {
                println!("Failed to render upgrade menu: {}", e);
            }
        } else {
            if state.upgrade_menu.is_visible() {
                state.upgrade_menu.hide();
            }
            // Explicitly clear rectangles if menu is not visible
            state
                .upgrade_menu
                .button_manager
                .rectangle_renderer
                .clear_rectangles();
        }

        window.request_redraw();

        // Submit commands and present
        state.profiler.start_section("command_submission");
        state.wgpu_renderer.queue.submit(Some(encoder.finish()));
        state.profiler.end_section("command_submission");

        // Present the surface texture and ensure it's properly handled
        state.profiler.start_section("surface_presentation");
        surface_texture.present();
        state.profiler.end_section("surface_presentation");

        // Poll the device to process any pending operations
        // This helps ensure resources are properly cleaned up and prevents
        // the "SurfaceSemaphores still in use" error during cleanup
        state.profiler.start_section("device_polling");
        state.wgpu_renderer.device.poll(wgpu::Maintain::Poll);
        state.profiler.end_section("device_polling");

        // Manage enemy locked state based on timer and test mode
        if state.game_state.current_screen == CurrentScreen::Game {
            let was_locked = state.game_state.enemy.pathfinder.locked;
            if state.game_state.is_test_mode {
                // Always keep enemy locked in test mode
                state.game_state.enemy.pathfinder.locked = true;
            } else if state.game_state.game_ui.timer.is_some() {
                // In normal mode, unlock enemy only when timer is running (not paused)
                if let Some(timer) = &state.game_state.game_ui.timer {
                    if timer.is_running && timer.paused_at.is_none() {
                        state.game_state.enemy.pathfinder.locked = false;
                    } else {
                        // Lock enemy when timer is paused or stopped
                        state.game_state.enemy.pathfinder.locked = true;
                    }
                }
            } else {
                // Lock enemy when no timer exists
                state.game_state.enemy.pathfinder.locked = true;
            }

            // Debug: Print when enemy lock state changes
            if was_locked != state.game_state.enemy.pathfinder.locked {
                println!(
                    "Enemy lock state changed: {} -> {}",
                    was_locked, state.game_state.enemy.pathfinder.locked
                );
            }
        }

        // Update enemy pathfinding
        state.profiler.start_section("enemy_pathfinding");
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
        state.profiler.end_section("enemy_pathfinding");

        // Handle title screen animation if needed
        if state.game_state.current_screen == CurrentScreen::Loading {
            state.game_state.game_ui.stop_timer();
            let _ = state; // Release the borrow
            self.handle_maze_generation();
            return; // Exit early to avoid the borrow checker issue
        } else if state.game_state.current_screen == CurrentScreen::NewGame {
            state.text_renderer.hide_game_over_display();
            state.upgrade_menu.upgrade_manager.player_upgrades.clear();
            state.game_state.player = crate::game::player::Player::new();
            state.game_state.enemy = crate::game::enemy::Enemy::new([0.0, 30.0, 0.0], 150.0);
            let _ = state; // Release the borrow
            self.new_level(true);
            return; // Exit early to avoid the borrow checker issue
        } else if state.game_state.current_screen == CurrentScreen::Game
            && Some(state.game_state.player.current_cell) == state.game_state.exit_cell
        {
            // Transition to ExitReached screen
            state.game_state.current_screen = CurrentScreen::ExitReached;
            state.game_state.exit_reached_timer = 0.0;
            state.game_state.enemy.pathfinder.position = [0.0, 30.0, 0.0];
            state.game_state.enemy.pathfinder.locked = true;
            if !state.game_state.beeper_rise_played {
                let _ = state.game_state.audio_manager.play_beeper_rise();
                state.game_state.beeper_rise_played = true;
            }
        } else if state.game_state.current_screen == CurrentScreen::ExitReached {
            // Handle exit reached upward movement
            state.game_state.exit_reached_timer += state.game_state.delta_time;

            // Move player upward for 3 seconds
            if state.game_state.exit_reached_timer < 1.0 {
                state.game_state.player.move_up(state.game_state.delta_time);
            } else {
                // After 3 seconds, transition to appropriate next screen
                let current_level = state.game_state.game_ui.level;
                if current_level > 0 && current_level % 3 == 0 {
                    // Show upgrade menu
                    state.game_state.current_screen = CurrentScreen::UpgradeMenu;
                    state.upgrade_menu.show();
                } else {
                    // Continue to next level
                    // Store the state we need before calling new_level
                    let should_continue = true;
                    let _ = state; // Release the borrow
                    if should_continue {
                        self.new_level(false);
                    }
                    return; // Exit early to avoid the borrow checker issue
                }
            }
        } else if state.game_state.current_screen == CurrentScreen::Game {
            state
                .game_state
                .audio_manager
                .resume_enemy_audio("enemy")
                .expect("Failed to resume enemy audio");
        }

        // End timing the entire frame and record FPS
        state.profiler.end_section("total_frame");

        // Record frame time for performance analysis
        crate::benchmark!("frame_time", {
            // This is just a marker - the actual timing is done by the profiler
        });

        state.fps_counter.record_frame();

        // Record frame in global benchmark system for FPS statistics
        crate::benchmarks::utils::record_frame();

        // Print performance summary every 1000 frames (approximately every 16 seconds at 60 FPS)
        if state.fps_counter.frame_times.len() % 1000 == 0 {
            // Note: We can't call print_summary here due to borrow checker constraints
            // The summary will be available when the game exits or when explicitly called
        }

        // Save benchmark results every 5000 frames (approximately every 83 seconds at 60 FPS)
        // This ensures we don't lose data if the program crashes
        if state.fps_counter.frame_times.len() % 5000 == 0
            && state.fps_counter.frame_times.len() > 0
        {
            // Use force_save_results to avoid borrow checker issues
            if let Err(e) = crate::benchmarks::utils::force_save_results() {
                eprintln!(
                    "[BENCHMARK] Failed to save benchmark results during periodic save: {}",
                    e
                );
            }
        }
    }

    /// Updates frame timing and performance metrics.
    ///
    /// This method calculates delta time between frames, updates FPS counter,
    /// and manages performance-related state. It's called every frame to ensure
    /// smooth gameplay and accurate timing.
    ///
    /// # Arguments
    /// - `current_time`: The current frame timestamp
    ///
    /// # Calculations
    /// - **Delta Time**: Time elapsed since the last frame
    /// - **FPS**: Frames per second, updated every second
    /// - **Frame Count**: Total frames rendered since start
    /// - **Elapsed Time**: Total time since application start
    ///
    /// # Performance Monitoring
    /// - Updates debug renderer vertices if bounding box rendering is enabled
    /// - Maintains accurate timing for game systems
    /// - Provides timing data for performance analysis
    ///
    /// # Side Effects
    /// - Updates `game_state.delta_time` for use by other systems
    /// - Updates FPS counter and frame timing state
    /// - Triggers debug renderer updates when needed
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

    /// Handles procedural maze generation and loading screen logic.
    ///
    /// This method manages the maze generation process, which can be either
    /// animated (showing generation steps) or instant (fast mode). It handles
    /// both normal maze generation and test mode maze creation.
    ///
    /// # Generation Process
    /// 1. **Test Mode Check**: Skips generation and goes directly to game in test mode
    /// 2. **Animation Control**: Manages generation speed and progress reporting
    /// 3. **Completion Handling**: Saves maze to file and generates geometry
    /// 4. **State Setup**: Initializes player, enemy, and collision systems
    ///
    /// # Animation Modes
    /// - **Normal Mode**: Shows generation steps with configurable speed
    /// - **Fast Mode**: Completes generation quickly for faster gameplay
    /// - **Test Mode**: Uses predefined test maze instead of generation
    ///
    /// # Progress Reporting
    /// - Logs generation progress every 50 steps
    /// - Shows completion percentage
    /// - Automatically completes generation when 70%+ complete
    ///
    /// # Maze Completion
    /// - Saves maze to file with timestamp
    /// - Generates floor, wall, and ceiling geometry
    /// - Places player at maze entrance
    /// - Places enemy based on exit position
    /// - Builds collision system from maze data
    ///
    /// # Audio Integration
    /// - Plays completion sound when generation finishes
    /// - Manages audio state during generation process
    pub fn handle_maze_generation(&mut self) {
        if let Some(state) = self.state.as_mut() {
            // Start timing maze generation
            state.profiler.start_section("maze_generation");
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
                2000
            } else {
                100
            };
            state.profiler.start_section("maze_generation_steps");
            for _ in 0..steps {
                if !renderer.generator.step() {
                    break;
                }
            }
            state.profiler.end_section("maze_generation_steps");

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

            // Complete generation all at once if less than 10% remains
            let progress_ratio = current as f32 / total.max(1) as f32;
            if progress_ratio > 0.7 && !renderer.generator.is_complete() {
                while !renderer.generator.is_complete() {
                    renderer.generator.step();
                }
            }

            if renderer.generator.is_complete() && state.game_state.maze_path.is_none() {
                println!("Maze generation complete! Saving to file...");

                // Play completion sound
                state
                    .game_state
                    .audio_manager
                    .complete()
                    .expect("Failed to play complete sound!");

                // --- DEBUG PRINT: Applied Upgrades and Stats (print once, with complete sound) ---
                println!("\n=== [DEBUG] Applied Upgrades and Influenced Stats ===");
                use crate::game::upgrades::AvailableUpgrade;
                let mgr = &state.upgrade_menu.upgrade_manager;
                let player = &state.game_state.player;
                for upgrade in [
                    AvailableUpgrade::SpeedUp,
                    AvailableUpgrade::Dash,
                    AvailableUpgrade::TallBoots,
                    AvailableUpgrade::SlowTime,
                    AvailableUpgrade::SilentStep,
                    AvailableUpgrade::HeadStart,
                    AvailableUpgrade::Unknown,
                ] {
                    let count = mgr.get_upgrade_count(&upgrade);
                    if count > 0 {
                        let name = upgrade.to_upgrade().name;
                        let stat = match upgrade {
                            AvailableUpgrade::SpeedUp => {
                                format!("base_speed: {:.2}", player.base_speed)
                            }
                            AvailableUpgrade::Dash => {
                                format!("max_stamina: {:.2}", player.max_stamina)
                            }
                            AvailableUpgrade::TallBoots => {
                                format!("height: {:.2}", player.position[1])
                            }
                            AvailableUpgrade::SlowTime => format!("timer: {}s", player.max_stamina), // Timer is in game_state, but not directly accessible here; placeholder
                            AvailableUpgrade::SilentStep => "enemy pathfinding penalty".to_string(),
                            AvailableUpgrade::HeadStart => "enemy lock time".to_string(),
                            AvailableUpgrade::Unknown => "???".to_string(),
                        };
                        println!("- {} (x{}): {}", name, count, stat);
                    }
                }
                println!("===============================================\n");
                // Handle completion
                if renderer.generator.is_complete() {
                    println!("Maze generation complete! Saving to file...");
                    state.profiler.start_section("maze_completion_processing");

                    let maze_lock = renderer.maze.lock().expect("Failed to lock maze");
                    state.game_state.maze_path = maze_lock.save_to_file().map_or_else(
                        |err| {
                            eprintln!("Failed to save maze: {}", err);
                            std::process::exit(1);
                        },
                        Some,
                    );

                    // Generate geometry if maze was saved successfully
                    if let Some(maze_path) = &state.game_state.maze_path {
                        state.profiler.start_section("maze_geometry_generation");
                        let (maze_grid, exit_cell) = parse_maze_file(
                            maze_path
                                .to_str()
                                .expect("Failed to convert path to string"),
                        );
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

                        // Add ceiling vertices
                        floor_vertices.append(&mut Vertex::create_ceiling_vertices(
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
                        state.profiler.end_section("maze_geometry_generation");

                        if let Some(exit_cell_position) = exit_cell {
                            state.profiler.start_section("enemy_placement");
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
                            state.profiler.end_section("enemy_placement");
                        }

                        state.profiler.start_section("collision_system_build");
                        state
                            .game_state
                            .collision_system
                            .build_from_maze(&maze_grid, state.game_state.is_test_mode);
                        state.profiler.end_section("collision_system_build");

                        // Spawn the player at the bottom-left corner of the maze
                        state
                            .game_state
                            .player
                            .spawn_at_maze_entrance(&maze_grid, state.game_state.is_test_mode);
                        // (No automatic transition to Game here)
                    }

                    state.profiler.end_section("maze_completion_processing");
                }
            }
        }

        // End timing maze generation
        if let Some(state) = self.state.as_mut() {
            state.profiler.end_section("maze_generation");
        }
    }

    /// Initializes a new level or restarts the game.
    ///
    /// This method handles the transition to a new level or game restart. It manages
    /// player state, enemy positioning, timer configuration, and scoring systems.
    /// The method implements a sophisticated scoring system based on completion time
    /// and performance metrics.
    ///
    /// # Arguments
    /// - `game_over`: Whether this is a game restart (true) or level progression (false)
    ///
    /// # Game Restart (game_over = true)
    /// - Resets player to starting position and stats
    /// - Resets level to 1 and score to 0
    /// - Initializes new timer with default configuration
    /// - Restarts background music
    /// - Clears all upgrade effects
    ///
    /// # Level Progression (game_over = false)
    /// - Preserves player stats and upgrades
    /// - Resets only position and orientation
    /// - Calculates performance-based scoring
    /// - Updates level counter
    /// - Maintains upgrade effects
    ///
    /// # Scoring System
    /// The scoring system rewards performance with multiple components:
    /// - **Base Score**: 150 points per level
    /// - **Speed Bonus**: Multiplier based on completion time
    ///   - Exceptional (≤15s): 3x-5x multiplier
    ///   - Good (≤25s): 1.5x-3x multiplier
    ///   - Average (≤35s): 0.5x-1.5x multiplier
    ///   - Slow (>35s): 0.1x-0.5x multiplier
    /// - **Level Bonus**: Additional points for higher levels (level > 5)
    /// - **Consecutive Bonus**: Reward for sustained good performance
    ///
    /// # State Management
    /// - Resets maze path to trigger new generation
    /// - Clears exit cell and timer state
    /// - Resets enemy position and locks enemy
    /// - Manages audio state transitions
    ///
    /// # Player State
    /// - **Game Restart**: Complete reset of all stats
    /// - **Level Progression**: Preserves upgrades, resets position only
    /// - **Height Adjustment**: Accounts for TallBoots upgrades
    /// - **Stamina Reset**: Refills stamina for new level
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
        if game_over {
            state.game_state.player = Player::new();
        } else {
            // Only reset position (x/z), orientation, and cell, not stats or height
            let player = &mut state.game_state.player;
            player.position[0] = 0.0;
            player.position[2] = 0.0;
            // Set height based on TallBoots upgrades
            let tall_boots_count = state
                .upgrade_menu
                .upgrade_manager
                .get_upgrade_count(&crate::game::upgrades::AvailableUpgrade::TallBoots);
            player.position[1] = crate::math::coordinates::constants::PLAYER_HEIGHT
                + 5.0 * (tall_boots_count as f32);
            player.pitch = 3.0;
            player.yaw = 316.0;
            player.fov = 100.0;
            player.current_cell = crate::game::maze::generator::Cell::default();
            // Optionally, reset stamina to max for new level:
            player.stamina = player.max_stamina;
            // (Do not reset base_speed, max_stamina, regen rates, etc.)
        }
        state.game_state.enemy.pathfinder.position = [0.0, 30.0, 0.0];
        state.game_state.enemy.pathfinder.locked = true;
        state.game_state.exit_cell = None; // Clear exit cell to prevent accidental win condition
        state.game_state.exit_reached_timer = 0.0; // Reset exit reached timer
        state.game_state.beeper_rise_played = false; // Reset beeper rise played flag

        // Stop and reset timer
        if let Some(timer) = &mut state.game_state.game_ui.timer {
            timer.stop();
            timer.reset();
        }

        if game_over {
            state.game_state.set_level(1);
            state.game_state.set_score(0);
            state.game_state.game_ui.timer = Some(GameTimer::new(TimerConfig::default()));

            // Restart background music for new game
            state
                .game_state
                .audio_manager
                .restart_background_music()
                .expect("Failed to restart background music");

            // Set game audio volumes after restarting background music
            state
                .game_state
                .audio_manager
                .set_game_volumes()
                .expect("Failed to set game volumes");

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
