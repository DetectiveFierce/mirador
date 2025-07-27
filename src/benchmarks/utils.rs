//! Benchmark utility functions and helper types
//!
//! This module provides convenience functions, timers, and utilities for common
//! benchmarking tasks. It includes both manual timers and automatic scoped timers,
//! as well as functions for managing benchmark data and generating reports.
//!
//! # Key Features
//! - **Manual Timers**: `Timer` for explicit start/stop timing
//! - **Scoped Timers**: `ScopedTimer` for automatic timing based on scope
//! - **Convenience Functions**: Easy-to-use functions for common operations
//! - **File Output**: Functions to save benchmark results to files
//! - **Macros**: `benchmark!` and `debug_benchmark!` for easy code instrumentation

use chrono::{DateTime, Datelike, Local, Timelike};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use super::BenchmarkConfig;
use super::data::{BENCHMARK_DATA, PerformanceMetrics, Profiler};

/// A timer for measuring execution time of code sections
///
/// This timer provides manual control over timing operations. It starts timing
/// when created and can be stopped explicitly to record the measurement.
pub struct Timer {
    /// The name of the operation being timed
    name: String,
    /// When the timer was started
    start_time: Instant,
    /// Configuration for this timer
    config: BenchmarkConfig,
}

impl Timer {
    /// Creates a new timer with the given name
    ///
    /// # Arguments
    /// * `name` - The name of the operation to be timed
    /// * `config` - Configuration for the timer behavior
    pub fn new(name: &str, config: BenchmarkConfig) -> Self {
        Self {
            name: name.to_string(),
            start_time: Instant::now(),
            config,
        }
    }

    /// Stops the timer and records the measurement
    ///
    /// Returns the duration that elapsed since the timer was created.
    /// If the duration meets the minimum threshold, it will be recorded
    /// in the global benchmark data.
    pub fn stop(self) -> Duration {
        let duration = self.start_time.elapsed();

        if self.config.enabled && duration >= self.config.min_duration_threshold {
            BENCHMARK_DATA
                .lock()
                .unwrap()
                .record_measurement(&self.name, duration);

            if self.config.print_results {
                println!("[BENCHMARK] {}: {:?}", self.name, duration);
            }
        }

        duration
    }
}

/// A scoped timer that automatically stops when dropped
///
/// This timer automatically records timing when it goes out of scope,
/// making it ideal for timing code blocks without explicit start/stop calls.
pub struct ScopedTimer {
    /// The underlying timer
    timer: Timer,
}

impl ScopedTimer {
    /// Creates a new scoped timer
    ///
    /// # Arguments
    /// * `name` - The name of the operation to be timed
    /// * `config` - Configuration for the timer behavior
    pub fn new(name: &str, config: BenchmarkConfig) -> Self {
        Self {
            timer: Timer::new(name, config),
        }
    }
}

impl Drop for ScopedTimer {
    fn drop(&mut self) {
        let _ =
            std::mem::replace(&mut self.timer, Timer::new("", BenchmarkConfig::default())).stop();
    }
}

/// Times a closure execution with default configuration
///
/// This is a convenience function that times the execution of a closure
/// using the default benchmark configuration.
///
/// # Arguments
/// * `name` - The name of the operation being timed
/// * `f` - The closure to execute and time
///
/// # Returns
/// The result of the closure execution
pub fn time<F, R>(name: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    let config = BenchmarkConfig::default();
    let mut profiler = Profiler::new(config);
    profiler.time_closure(name, f)
}

/// Creates a scoped timer with default configuration
///
/// This is a convenience function for creating a scoped timer with
/// the default benchmark configuration.
///
/// # Arguments
/// * `name` - The name of the operation to be timed
///
/// # Returns
/// A scoped timer that will automatically record timing when dropped
pub fn scoped_timer(name: &str) -> ScopedTimer {
    ScopedTimer::new(name, BenchmarkConfig::default())
}

/// Prints a summary of all recorded measurements
///
/// This function displays a comprehensive summary of all benchmark
/// measurements, including FPS statistics and categorized benchmark tables.
/// The output is formatted for console display with clear sections for
/// initialization and update benchmarks.
pub fn print_summary() {
    let data = BENCHMARK_DATA.lock().unwrap();
    let measurements = data.get_measurements();

    if measurements.is_empty() {
        println!("[BENCHMARK] No measurements recorded");
        return;
    }

    println!("\n=== PERFORMANCE SUMMARY ===");

    // Print FPS statistics
    let (min_fps, avg_fps, max_fps) = get_fps_stats();
    if avg_fps > 0.0 {
        println!(
            "FPS Statistics - Min: {:.1}, Average: {:.1}, Max: {:.1}",
            min_fps, avg_fps, max_fps
        );
    } else {
        println!("FPS Statistics - No frame data recorded");
    }

    // Separate initialization benchmarks from update benchmarks
    let mut init_benchmarks: Vec<_> = Vec::new();
    let mut update_benchmarks: Vec<_> = Vec::new();

    for (name, metrics) in measurements.iter() {
        // Identify initialization benchmarks by their naming patterns
        if name.contains("initialization")
            || name.contains("creation")
            || name.contains("setup")
            || name.contains("loading")
            || name.contains("spawning")
            || name.contains("configuration")
        {
            init_benchmarks.push((name, metrics));
        } else {
            update_benchmarks.push((name, metrics));
        }
    }

    // Sort both lists by total duration (most time-consuming first)
    init_benchmarks.sort_by(|a, b| b.1.total_duration.cmp(&a.1.total_duration));
    update_benchmarks.sort_by(|a, b| b.1.total_duration.cmp(&a.1.total_duration));

    // Print initialization benchmarks table
    if !init_benchmarks.is_empty() {
        println!("\n=== INITIALIZATION BENCHMARKS ===");
        println!("This table shows the most taxing parts of program initialization:");
        println!();

        // Calculate total initialization time
        let total_init_time: f64 = init_benchmarks
            .iter()
            .map(|(_, m)| m.total_duration.as_secs_f64())
            .sum();

        // Print header - use dynamic columns based on data
        let has_multiple_counts = init_benchmarks.iter().any(|(_, m)| m.count > 1);

        // Calculate column widths based on actual content
        let (name_width, count_width, total_width, avg_width, min_width, max_width) =
            super::format::calculate_column_widths(&init_benchmarks, has_multiple_counts);

        // Print header with all columns
        println!(
            "{:<name_width$} | {:>count_width$} | {:>total_width$} | {:>avg_width$} | {:>min_width$} | {:>max_width$}",
            "Operation",
            "Count",
            "Total",
            "Avg",
            "Min",
            "Max",
            name_width = name_width,
            count_width = count_width,
            total_width = total_width,
            avg_width = avg_width,
            min_width = min_width,
            max_width = max_width
        );

        // Calculate separator line length
        let separator_length =
            name_width + count_width + total_width + avg_width + min_width + max_width + 15; // +15 for separators and spaces
        println!("{}", "-".repeat(separator_length));

        for (name, metrics) in &init_benchmarks {
            if metrics.count > 1 {
                // Show all columns for operations with count > 1
                let total_str = format!("{:?}", metrics.total_duration);
                let avg_str = format!("{:?}", metrics.avg_duration);
                let min_str = format!("{:?}", metrics.min_duration);
                let max_str = format!("{:?}", metrics.max_duration);

                println!(
                    "{:<name_width$} | {:>count_width$} | {:>total_width$} | {:>avg_width$} | {:>min_width$} | {:>max_width$}",
                    name,
                    metrics.count,
                    total_str,
                    avg_str,
                    min_str,
                    max_str,
                    name_width = name_width,
                    count_width = count_width,
                    total_width = total_width,
                    avg_width = avg_width,
                    min_width = min_width,
                    max_width = max_width
                );
            } else {
                // Show only basic columns for operations with count = 1
                let total_str = format!("{:?}", metrics.total_duration);

                println!(
                    "{:<name_width$} | {:>count_width$} | {:>total_width$}",
                    name,
                    metrics.count,
                    total_str,
                    name_width = name_width,
                    count_width = count_width,
                    total_width = total_width
                );
            }
        }

        println!();
        println!("Total initialization time: {:.3}s", total_init_time);
    }

    // Print update benchmarks table
    if !update_benchmarks.is_empty() {
        println!("\n=== UPDATE BENCHMARKS ===");
        println!("This table shows the most taxing parts of the update loop:");
        println!();

        // Calculate total update time
        let total_update_time: f64 = update_benchmarks
            .iter()
            .map(|(_, m)| m.total_duration.as_secs_f64())
            .sum();

        // Print header - use dynamic columns based on data
        let has_multiple_counts = update_benchmarks.iter().any(|(_, m)| m.count > 1);

        // Calculate column widths based on actual content
        let (name_width, count_width, total_width, avg_width, min_width, max_width) =
            super::format::calculate_column_widths(&update_benchmarks, has_multiple_counts);

        // Print header with all columns
        println!(
            "{:<name_width$} | {:>count_width$} | {:>total_width$} | {:>avg_width$} | {:>min_width$} | {:>max_width$}",
            "Operation",
            "Count",
            "Total",
            "Avg",
            "Min",
            "Max",
            name_width = name_width,
            count_width = count_width,
            total_width = total_width,
            avg_width = avg_width,
            min_width = min_width,
            max_width = max_width
        );

        // Calculate separator line length
        let separator_length =
            name_width + count_width + total_width + avg_width + min_width + max_width + 15; // +15 for separators and spaces
        println!("{}", "-".repeat(separator_length));

        for (name, metrics) in &update_benchmarks {
            if metrics.count > 1 {
                // Show all columns for operations with count > 1
                let total_str = format!("{:?}", metrics.total_duration);
                let avg_str = format!("{:?}", metrics.avg_duration);
                let min_str = format!("{:?}", metrics.min_duration);
                let max_str = format!("{:?}", metrics.max_duration);

                println!(
                    "{:<name_width$} | {:>count_width$} | {:>total_width$} | {:>avg_width$} | {:>min_width$} | {:>max_width$}",
                    name,
                    metrics.count,
                    total_str,
                    avg_str,
                    min_str,
                    max_str,
                    name_width = name_width,
                    count_width = count_width,
                    total_width = total_width,
                    avg_width = avg_width,
                    min_width = min_width,
                    max_width = max_width
                );
            } else {
                // Show only basic columns for operations with count = 1
                let total_str = format!("{:?}", metrics.total_duration);

                println!(
                    "{:<name_width$} | {:>count_width$} | {:>total_width$}",
                    name,
                    metrics.count,
                    total_str,
                    name_width = name_width,
                    count_width = count_width,
                    total_width = total_width
                );
            }
        }

        println!();
        println!("Total update time: {:.3}s", total_update_time);
    }

    println!("\n=============================\n");
}

/// Clears all recorded measurements
///
/// This function removes all stored benchmark data, resetting the
/// benchmark system to an empty state.
pub fn clear_measurements() {
    BENCHMARK_DATA.lock().unwrap().clear();
}

/// Gets all recorded measurements
///
/// Returns a copy of all currently stored benchmark measurements.
///
/// # Returns
/// A HashMap containing all benchmark measurements indexed by operation name
pub fn get_measurements() -> HashMap<String, PerformanceMetrics> {
    BENCHMARK_DATA.lock().unwrap().get_measurements()
}

/// Writes benchmark results to a file in the debug-analytics/benchmarks directory
///
/// This function creates a comprehensive benchmark report file with timestamped
/// filename. The report includes FPS statistics, initialization benchmarks,
/// and update benchmarks in a formatted table structure.
///
/// # Returns
/// `Ok(())` on success, or an `io::Error` if file operations fail
pub fn write_results_to_file(source: &str) -> io::Result<()> {
    let measurements = get_measurements();
    if measurements.is_empty() {
        println!("[BENCHMARK] No measurements to write");
        return Ok(());
    }

    // Create the benchmarks directory
    let benchmarks_dir = Path::new("debug-analytics/benchmarks");
    if let Err(e) = fs::create_dir_all(benchmarks_dir) {
        eprintln!("[BENCHMARK] Failed to create benchmarks directory: {}", e);
        return Err(e);
    }

    // Generate filename with timestamp
    let now: DateTime<Local> = Local::now();
    let filename = format!(
        "{:02}:{:02}{}-{:02}-{:02}-{:04}.txt",
        now.hour12().1,
        now.minute(),
        if now.hour12().0 { "pm" } else { "am" },
        now.month(),
        now.day(),
        now.year()
    );

    let file_path = benchmarks_dir.join(filename);

    // Open file for writing
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&file_path)?;

    // Write header with source label
    writeln!(file, "Mirador Performance Benchmark Results")?;
    writeln!(file, "Generated: {}", now.format("%Y-%m-%d %H:%M:%S"))?;
    writeln!(file, "Source: {}", source)?;
    writeln!(file, "{}", "=".repeat(60))?;
    writeln!(file)?;

    // Write performance summary section
    writeln!(file, "=== PERFORMANCE SUMMARY ===")?;

    // Write FPS statistics
    let (min_fps, avg_fps, max_fps) = get_fps_stats();
    if avg_fps > 0.0 {
        writeln!(
            file,
            "FPS Statistics - Min: {:.1}, Average: {:.1}, Max: {:.1}",
            min_fps, avg_fps, max_fps
        )?;
    } else {
        writeln!(file, "FPS Statistics - No frame data recorded")?;
    }
    writeln!(file)?;

    // Separate initialization benchmarks from update benchmarks
    let mut init_benchmarks: Vec<_> = Vec::new();
    let mut update_benchmarks: Vec<_> = Vec::new();

    for (name, metrics) in measurements.iter() {
        // Identify initialization benchmarks by their naming patterns
        if name.contains("initialization")
            || name.contains("creation")
            || name.contains("setup")
            || name.contains("loading")
            || name.contains("spawning")
            || name.contains("configuration")
        {
            init_benchmarks.push((name, metrics));
        } else {
            update_benchmarks.push((name, metrics));
        }
    }

    // Sort both lists by total duration (most time-consuming first)
    init_benchmarks.sort_by(|a, b| b.1.total_duration.cmp(&a.1.total_duration));
    update_benchmarks.sort_by(|a, b| b.1.total_duration.cmp(&a.1.total_duration));

    // Write initialization benchmarks table
    if !init_benchmarks.is_empty() {
        writeln!(file, "=== INITIALIZATION BENCHMARKS ===")?;
        writeln!(
            file,
            "This table shows the most taxing parts of program initialization:"
        )?;
        writeln!(file)?;

        // Calculate total initialization time
        let total_init_time: f64 = init_benchmarks
            .iter()
            .map(|(_, m)| m.total_duration.as_secs_f64())
            .sum();

        // Print header - use dynamic columns based on data
        let has_multiple_counts = init_benchmarks.iter().any(|(_, m)| m.count > 1);

        // Calculate column widths based on actual content
        let (name_width, count_width, total_width, avg_width, min_width, max_width) =
            super::format::calculate_column_widths(&init_benchmarks, has_multiple_counts);

        // Print header with all columns
        writeln!(
            file,
            "{:<name_width$} | {:>count_width$} | {:>total_width$} | {:>avg_width$} | {:>min_width$} | {:>max_width$}",
            "Operation",
            "Count",
            "Total",
            "Avg",
            "Min",
            "Max",
            name_width = name_width,
            count_width = count_width,
            total_width = total_width,
            avg_width = avg_width,
            min_width = min_width,
            max_width = max_width
        )?;

        // Calculate separator line length
        let separator_length =
            name_width + count_width + total_width + avg_width + min_width + max_width + 15; // +15 for separators and spaces
        writeln!(file, "{}", "-".repeat(separator_length))?;

        for (name, metrics) in &init_benchmarks {
            if metrics.count > 1 {
                // Show all columns for operations with count > 1
                let total_str = format!("{:?}", metrics.total_duration);
                let avg_str = format!("{:?}", metrics.avg_duration);
                let min_str = format!("{:?}", metrics.min_duration);
                let max_str = format!("{:?}", metrics.max_duration);

                writeln!(
                    file,
                    "{:<name_width$} | {:>count_width$} | {:>total_width$} | {:>avg_width$} | {:>min_width$} | {:>max_width$}",
                    name,
                    metrics.count,
                    total_str,
                    avg_str,
                    min_str,
                    max_str,
                    name_width = name_width,
                    count_width = count_width,
                    total_width = total_width,
                    avg_width = avg_width,
                    min_width = min_width,
                    max_width = max_width
                )?;
            } else {
                // Show only basic columns for operations with count = 1
                let total_str = format!("{:?}", metrics.total_duration);

                writeln!(
                    file,
                    "{:<name_width$} | {:>count_width$} | {:>total_width$}",
                    name,
                    metrics.count,
                    total_str,
                    name_width = name_width,
                    count_width = count_width,
                    total_width = total_width
                )?;
            }
        }

        writeln!(file)?;
        writeln!(file, "Total initialization time: {:.3}s", total_init_time)?;
    }

    // Write update benchmarks table
    if !update_benchmarks.is_empty() {
        writeln!(file, "\n=== UPDATE BENCHMARKS ===")?;
        writeln!(
            file,
            "This table shows the most taxing parts of the update loop:"
        )?;
        writeln!(file)?;

        // Calculate total update time
        let total_update_time: f64 = update_benchmarks
            .iter()
            .map(|(_, m)| m.total_duration.as_secs_f64())
            .sum();

        // Print header - use dynamic columns based on data
        let has_multiple_counts = update_benchmarks.iter().any(|(_, m)| m.count > 1);

        // Calculate column widths based on actual content
        let (name_width, count_width, total_width, avg_width, min_width, max_width) =
            super::format::calculate_column_widths(&update_benchmarks, has_multiple_counts);

        // Print header with all columns
        writeln!(
            file,
            "{:<name_width$} | {:>count_width$} | {:>total_width$} | {:>avg_width$} | {:>min_width$} | {:>max_width$}",
            "Operation",
            "Count",
            "Total",
            "Avg",
            "Min",
            "Max",
            name_width = name_width,
            count_width = count_width,
            total_width = total_width,
            avg_width = avg_width,
            min_width = min_width,
            max_width = max_width
        )?;

        // Calculate separator line length
        let separator_length =
            name_width + count_width + total_width + avg_width + min_width + max_width + 15; // +15 for separators and spaces
        writeln!(file, "{}", "-".repeat(separator_length))?;

        for (name, metrics) in &update_benchmarks {
            if metrics.count > 1 {
                // Show all columns for operations with count > 1
                let total_str = format!("{:?}", metrics.total_duration);
                let avg_str = format!("{:?}", metrics.avg_duration);
                let min_str = format!("{:?}", metrics.min_duration);
                let max_str = format!("{:?}", metrics.max_duration);

                writeln!(
                    file,
                    "{:<name_width$} | {:>count_width$} | {:>total_width$} | {:>avg_width$} | {:>min_width$} | {:>max_width$}",
                    name,
                    metrics.count,
                    total_str,
                    avg_str,
                    min_str,
                    max_str,
                    name_width = name_width,
                    count_width = count_width,
                    total_width = total_width,
                    avg_width = avg_width,
                    min_width = min_width,
                    max_width = max_width
                )?;
            } else {
                // Show only basic columns for operations with count = 1
                let total_str = format!("{:?}", metrics.total_duration);

                writeln!(
                    file,
                    "{:<name_width$} | {:>count_width$} | {:>total_width$}",
                    name,
                    metrics.count,
                    total_str,
                    name_width = name_width,
                    count_width = count_width,
                    total_width = total_width
                )?;
            }
        }

        writeln!(file)?;
        writeln!(file, "Total update time: {:.3}s", total_update_time)?;

        // Find high average time operations
        let high_avg_ops: Vec<_> = update_benchmarks
            .iter()
            .filter(|(_, metrics)| metrics.avg_duration.as_millis() > 1)
            .take(3)
            .collect();

        if !high_avg_ops.is_empty() {
            writeln!(file, "\n⚠️  Operations with high average times (>1ms):")?;
            for (name, metrics) in high_avg_ops {
                writeln!(file, "   • {}: {:?} average", name, metrics.avg_duration)?;
            }
        }

        // Find frequently called operations
        let frequent_ops: Vec<_> = update_benchmarks
            .iter()
            .filter(|(_, metrics)| metrics.count > 1000)
            .take(3)
            .collect();

        if !frequent_ops.is_empty() {
            writeln!(
                file,
                "\n🔄 Most frequently called operations (>1000 calls):"
            )?;
            for (name, metrics) in frequent_ops {
                writeln!(file, "   • {}: {} calls", name, metrics.count)?;
            }
        }
    }

    writeln!(file)?;
    writeln!(file, "{}", "=".repeat(60))?;
    writeln!(file, "End of benchmark report")?;

    println!("[BENCHMARK] Results written to: {}", file_path.display());
    println!(
        "[BENCHMARK] Full path: {}",
        file_path.canonicalize().unwrap_or(file_path).display()
    );
    Ok(())
}

/// Clears measurements and optionally writes them to file before clearing
///
/// This function saves any existing measurements to a file before clearing
/// them, ensuring no data is lost when clearing the benchmark state.
///
/// # Returns
/// `Ok(())` on success, or an `io::Error` if file operations fail
pub fn clear_and_save_measurements() -> io::Result<()> {
    if !get_measurements().is_empty() {
        write_results_to_file_default()?;
    }
    clear_measurements();
    Ok(())
}

/// Forces a save of benchmark results regardless of configuration
///
/// This function bypasses normal configuration settings to ensure
/// benchmark results are written to a file immediately.
///
/// # Returns
/// `Ok(())` on success, or an `io::Error` if file operations fail
pub fn force_save_results() -> io::Result<()> {
    println!("[BENCHMARK] Force saving results...");
    write_results_to_file_default()
}

/// Writes benchmark results to file with default source (Debug Run)
///
/// This is a convenience function that calls write_results_to_file with
/// the default "Debug Run" source label.
///
/// # Returns
/// `Ok(())` on success, or an `io::Error` if file operations fail
pub fn write_results_to_file_default() -> io::Result<()> {
    write_results_to_file("Debug Run")
}

/// Gets the current number of measurements
///
/// Returns the total count of unique benchmark operations that have
/// been recorded.
///
/// # Returns
/// The number of unique benchmark measurements currently stored
pub fn get_measurement_count() -> usize {
    get_measurements().len()
}

/// Records a frame for FPS tracking
///
/// This function records the current frame time and updates FPS statistics.
/// It should be called once per frame for accurate FPS measurement.
///
/// # Returns
/// The current average FPS
pub fn record_frame() -> f64 {
    BENCHMARK_DATA.lock().unwrap().record_frame()
}

/// Gets FPS statistics (min, average, max)
///
/// Returns the current FPS statistics based on recorded frame times.
///
/// # Returns
/// A tuple containing (min_fps, avg_fps, max_fps)
pub fn get_fps_stats() -> (f64, f64, f64) {
    BENCHMARK_DATA.lock().unwrap().get_fps_stats()
}

/// Macro for easy timing of code blocks
///
/// This macro provides a convenient way to time code blocks without
/// explicit timer management. It automatically creates a scoped timer
/// that records timing when the block exits.
///
/// # Example
/// ```rust
/// benchmark!("my_operation", {
///     // Code to be timed
///     expensive_operation();
/// });
/// ```
#[macro_export]
macro_rules! benchmark {
    ($name:expr, $block:expr) => {{
        use crate::benchmarks::{BenchmarkConfig, ScopedTimer};
        let _timer = ScopedTimer::new($name, BenchmarkConfig::default());
        $block
    }};
}

/// Macro for conditional benchmarking (only in debug builds)
///
/// This macro provides conditional benchmarking that only activates
/// in debug builds. In release builds, the code block executes normally
/// without any timing overhead.
///
/// # Example
/// ```rust
/// debug_benchmark!("debug_only_operation", {
///     // Code that will only be timed in debug builds
///     debug_operation();
/// });
/// ```
#[macro_export]
macro_rules! debug_benchmark {
    ($name:expr, $block:expr) => {{
        #[cfg(debug_assertions)]
        {
            use crate::benchmarks::{BenchmarkConfig, ScopedTimer};
            let _timer = ScopedTimer::new($name, BenchmarkConfig::default());
            $block
        }
        #[cfg(not(debug_assertions))]
        {
            $block
        }
    }};
}
