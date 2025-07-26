//! Mirador - A 3D Maze Runner Game
//!
//! This is the main entry point for the Mirador game application. Mirador is a first-person
//! maze runner game built with Rust and WGPU, featuring procedurally generated mazes,
//! enemy AI, audio systems, and a progression system with upgrades.
//!
//! # Features
//! - **3D Graphics**: Real-time 3D rendering using WGPU
//! - **Procedural Generation**: Dynamically generated mazes with increasing complexity
//! - **Enemy AI**: A* pathfinding enemy that pursues the player
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
pub mod game;
pub mod math;

pub mod renderer;
pub mod test_mode;

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
}
