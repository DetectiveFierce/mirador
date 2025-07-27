//! Benchmarking and Performance Analysis Library
//!
//! This module provides utilities for measuring and analyzing performance bottlenecks
//! in the Mirador game. It includes timing measurements, performance counters,
//! and profiling tools to identify where the program spends the most time.
//!
//! # Features
//! - **Timing Measurements**: Precise timing for code sections and functions
//! - **Performance Counters**: Track frame rates, memory usage, and other metrics
//! - **Profiling Tools**: Identify hot paths and performance bottlenecks
//! - **Conditional Compilation**: Benchmarks can be disabled in release builds
//! - **Minimal Overhead**: Designed to have minimal impact on performance when not active

use std::time::Duration;

/// Configuration for benchmarking features
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Whether benchmarking is enabled
    pub enabled: bool,
    /// Whether to print results to console
    pub print_results: bool,
    /// Whether to write results to file
    pub write_to_file: bool,
    /// Minimum duration to log (filters out very fast operations)
    pub min_duration_threshold: Duration,
    /// Maximum number of samples to keep in memory
    pub max_samples: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            enabled: cfg!(debug_assertions),
            print_results: false, // Disable console output
            write_to_file: cfg!(debug_assertions),
            min_duration_threshold: Duration::from_micros(100),
            max_samples: 1000,
        }
    }
}

/// Data structures and types for storing benchmark measurements
///
/// This module contains the core data types used for collecting and storing
/// performance measurements, including frame rate counters, memory trackers,
/// and performance metrics.
pub mod data;

/// Utilities for formatting and displaying benchmark results
///
/// This module provides functions for formatting benchmark data into readable
/// tables and reports, including column width calculations and output formatting.
pub mod format;

/// Utility functions and helper types for benchmarking operations
///
/// This module contains convenience functions, timers, and utilities for
/// common benchmarking tasks like timing closures, printing summaries,
/// and managing benchmark data.
pub mod utils;

#[cfg(test)]
mod tests;

// Re-export main types for convenience
pub use data::{FrameRateCounter, MemoryTracker, PerformanceMetrics, Profiler};
pub use utils::*;
