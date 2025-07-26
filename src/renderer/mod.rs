//! Main renderer module.
//!
//! This module contains submodules for uniform management, vertex definitions, and the wgpu renderer
//! implementation. It provides the core rendering infrastructure for the application.

/// Game-specific rendering components and systems.
pub mod game_renderer;
/// Icon rendering and management.
pub mod icon;
/// Loading screen rendering components.
pub mod loading_renderer;
/// Pipeline building utilities for WGPU.
pub mod pipeline_builder;
/// Basic geometric primitives for rendering.
pub mod primitives;
/// Rectangle rendering utilities.
pub mod rectangle;
/// Text rendering system.
pub mod text;
/// Title screen rendering components.
pub mod title;
/// User interface rendering components.
pub mod ui;
/// Core WGPU library and utilities.
pub mod wgpu_lib;
