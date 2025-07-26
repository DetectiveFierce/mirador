//! Application module for Mirador.
//!
//! This module contains the core application logic and state management for the Mirador game.
//! It provides the main application structure, event handling, and update loop that orchestrates
//! all game systems.
//!
//! # Module Structure
//!
//! - [`app_state`]: Contains the [`AppState`] struct which holds all application state
//! - [`event_handler`]: Contains the [`App`] struct and event handling logic
//! - [`update`]: Contains the main game update loop and rendering logic
//!
//! # Architecture
//!
//! The application follows a state-driven architecture where:
//! - [`App`] manages the application lifecycle and event routing
//! - [`AppState`] holds all mutable game state and resources
//! - The update loop processes game logic and renders frames
//!
//! # Event Flow
//!
//! 1. **Input Events**: Window and device events are captured by the event handler
//! 2. **State Updates**: Events are processed and game state is updated
//! 3. **Rendering**: The current state is rendered to the screen
//! 4. **Audio**: Audio systems are updated based on game state
//!
//! # Threading Model
//!
//! The application runs on a single thread with an async event loop. All game systems
//! are updated synchronously in the main thread to ensure consistent state and
//! avoid complex synchronization issues.

pub mod app_state;
pub mod event_handler;
pub mod update;

pub use app_state::AppState;
pub use event_handler::App;
