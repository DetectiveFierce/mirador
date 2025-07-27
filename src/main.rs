//! Mirador - A 3D Maze Runner Game
//!
//! This is the main entry point for the Mirador game application. Mirador is a first-person
//! maze runner game built with Rust and WGPU, featuring procedurally generated mazes,
//! enemy AI, audio systems, and a progression system with upgrades.
//!
//! # Features
//! - **3D Graphics**: Real-time 3D rendering using WGPU
//! - **Procedural Generation**: Dynamically generated mazes with increasing complexity
//! - **Enemy AI**: Rotation-based pathfinding enemy that pursues the player
//! - **Audio System**: 3D spatial audio with multiple sound sources
//! - **Progression System**: Player upgrades and level-based difficulty scaling
//! - **Test Mode**: Development mode with simplified gameplay for testing
//!
//! # Architecture
//! The application follows a modular architecture:
//! - `app/`: Application state management and event handling
//! - `game/`: Core game logic, player, enemy, and maze systems
//! - `renderer/`: Graphics rendering pipeline and UI components
//! - `math/`: Mathematical utilities for 3D graphics
//!
//! # Usage
//! Run the application with `cargo run`. The game supports both normal gameplay
//! and test mode for development purposes.

#![warn(missing_docs)]
pub mod app;
pub mod assets;
pub mod benchmarks;
pub mod game;
pub mod math;

pub mod renderer;
pub mod test_mode;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use winit::event_loop::{ControlFlow, EventLoop};

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

/// Main entry point for the Mirador game application.
///
/// This function initializes the application, sets up the event loop, and starts
/// the game. It handles different compilation targets (native vs WASM) and
/// optional memory profiling.
///
/// # Features
/// - Memory profiling with dhat-heap feature
/// - Cross-platform compatibility (native and WASM targets)
/// - Graceful error handling for event loop creation
///
/// # Panics
/// - If the event loop cannot be created
/// - If the application fails to run
fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();
    #[cfg(not(target_arch = "wasm32"))]
    {
        pollster::block_on(run());
    }
}

/// Asynchronously runs the main game loop.
///
/// This function creates the event loop, initializes the application state,
/// and starts the game. It handles the complete lifecycle of the application
/// from startup to shutdown.
///
/// # Returns
/// This function runs indefinitely until the application is closed by the user.
///
/// # Errors
/// - Returns early if event loop creation fails
/// - Exits the process if the application fails to run
async fn run() {
    use crate::benchmarks::{BenchmarkConfig, Profiler};

    // Initialize profiler for overall application initialization benchmarking
    let mut init_profiler = Profiler::new(BenchmarkConfig {
        enabled: true,
        print_results: false, // Respect user's console output preference
        write_to_file: false,
        min_duration_threshold: std::time::Duration::from_micros(1),
        max_samples: 1000,
    });

    // Benchmark complete application initialization
    init_profiler.start_section("complete_application_initialization");

    // Set up signal handler for graceful shutdown
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    #[cfg(not(target_arch = "wasm32"))]
    {
        ctrlc::set_handler(move || {
            println!("\nReceived interrupt signal, saving benchmark results...");
            // Save benchmark results before exiting
            if let Err(e) = crate::benchmarks::utils::force_save_results() {
                eprintln!("Failed to save benchmark results on exit: {}", e);
            }
            r.store(false, Ordering::SeqCst);
            std::process::exit(0);
        })
        .expect("Error setting Ctrl-C handler");
    }

    let event_loop = match EventLoop::new() {
        Ok(event_loop) => event_loop,
        Err(err) => {
            eprintln!("Error creating event loop: {}", err);
            return;
        }
    };

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = app::App::new();

    event_loop.run_app(&mut app).expect("Failed to run app");

    init_profiler.end_section("complete_application_initialization");
}
