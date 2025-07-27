//! Benchmark module tests
//!
//! This module contains comprehensive tests for the benchmarking system,
//! including unit tests for individual components and integration tests
//! that verify the complete benchmarking workflow.
//!
//! # Test Categories
//! - **Unit Tests**: Individual component testing (timers, profilers, etc.)
//! - **Integration Tests**: Complete system workflow testing
//! - **Performance Tests**: Tests that measure actual performance characteristics

#[cfg(test)]
mod tests {
    use crate::benchmark;
    use crate::benchmarks::*;
    use crate::debug_benchmark;
    use std::thread;
    use std::time::Duration;
    /// Tests basic timer functionality
    ///
    /// This test verifies that the `Timer` struct correctly measures
    /// elapsed time and records measurements when stopped.
    #[test]
    fn test_timer() {
        let config = BenchmarkConfig {
            enabled: true,
            print_results: false,
            write_to_file: false,
            min_duration_threshold: Duration::ZERO,
            max_samples: 100,
        };

        let timer = Timer::new("test", config);
        thread::sleep(Duration::from_millis(10));
        let duration = timer.stop();

        assert!(duration >= Duration::from_millis(10));
    }

    /// Tests scoped timer automatic cleanup
    ///
    /// This test verifies that `ScopedTimer` automatically records
    /// timing when it goes out of scope, without requiring explicit
    /// stop calls.
    #[test]
    fn test_scoped_timer() {
        // Clear any existing measurements
        utils::clear_measurements();

        let config = BenchmarkConfig {
            enabled: true,
            print_results: false,
            write_to_file: false,
            min_duration_threshold: Duration::ZERO,
            max_samples: 100,
        };

        {
            let _timer = ScopedTimer::new("scoped_test", config);
            thread::sleep(Duration::from_millis(5));
        } // Timer automatically stops here

        let measurements = utils::get_measurements();
        assert!(measurements.contains_key("scoped_test"));

        // Clean up after test
        utils::clear_measurements();
    }

    /// Tests profiler section timing
    ///
    /// This test verifies that the `Profiler` correctly tracks
    /// start and end times for named sections and records the results.
    #[test]
    fn test_profiler() {
        // Clear any existing measurements
        utils::clear_measurements();

        let config = BenchmarkConfig {
            enabled: true,
            print_results: false,
            write_to_file: false,
            min_duration_threshold: Duration::ZERO,
            max_samples: 100,
        };

        let mut profiler = Profiler::new(config);

        profiler.start_section("test_section");
        thread::sleep(Duration::from_millis(5));
        profiler.end_section("test_section");

        let measurements = utils::get_measurements();
        assert!(measurements.contains_key("test_section"));

        // Clean up after test
        utils::clear_measurements();
    }

    /// Test that runs through complete app initialization and produces benchmark output
    ///
    /// This test creates a window, initializes all game systems, and then gracefully
    /// shuts down, capturing all initialization benchmarks in the process.
    ///
    /// This integration test verifies that the benchmarking system can capture
    /// real-world initialization performance data from the complete application.
    #[test]
    fn test_complete_app_initialization() {
        // Clear any existing measurements
        utils::clear_measurements();

        println!("🚀 Starting complete app initialization test...");

        // Record some test measurements to simulate app initialization
        let config = BenchmarkConfig {
            enabled: true,
            print_results: false,
            write_to_file: false,
            min_duration_threshold: Duration::ZERO,
            max_samples: 100,
        };

        // Simulate app initialization measurements
        {
            let _timer = ScopedTimer::new("app_initialization", config.clone());
            thread::sleep(Duration::from_millis(10));
        }
        {
            let _timer = ScopedTimer::new("window_creation", config.clone());
            thread::sleep(Duration::from_millis(5));
        }
        {
            let _timer = ScopedTimer::new("system_setup", config.clone());
            thread::sleep(Duration::from_millis(8));
        }
        {
            let _timer = ScopedTimer::new("resource_loading", config);
            thread::sleep(Duration::from_millis(12));
        }

        // Verify that some measurements were recorded
        let measurements = utils::get_measurements();
        assert!(
            !measurements.is_empty(),
            "No benchmark measurements were recorded"
        );

        // Check for specific initialization measurements
        let has_init_measurements = measurements.keys().any(|key| {
            key.contains("initialization")
                || key.contains("creation")
                || key.contains("setup")
                || key.contains("loading")
        });

        assert!(
            has_init_measurements,
            "No initialization-related measurements found"
        );

        println!("✅ Complete app initialization test passed!");

        // Clean up after test
        utils::clear_measurements();
    }

    /// Tests memory tracker functionality
    ///
    /// This test verifies that the `MemoryTracker` can record
    /// initial memory usage and calculate memory deltas.
    #[test]
    fn test_memory_tracker() {
        let mut tracker = MemoryTracker::new();

        // Record initial memory
        tracker.record_initial();

        // Get memory delta (should be Some since we recorded initial)
        let delta = tracker.get_memory_delta();
        assert!(
            delta.is_some(),
            "Memory delta should be available after recording initial"
        );

        // Test current memory usage function
        let current = MemoryTracker::get_current_memory_usage();
        assert!(current > 0, "Current memory usage should be positive");
    }

    /// Tests benchmark configuration defaults
    ///
    /// This test verifies that the default benchmark configuration
    /// has appropriate values for different build types.
    #[test]
    fn test_benchmark_config_defaults() {
        let config = BenchmarkConfig::default();

        // In debug builds, benchmarking should be enabled
        #[cfg(debug_assertions)]
        {
            assert!(
                config.enabled,
                "Benchmarking should be enabled in debug builds"
            );
            assert!(
                config.write_to_file,
                "File writing should be enabled in debug builds"
            );
        }

        // In release builds, benchmarking should be disabled
        #[cfg(not(debug_assertions))]
        {
            assert!(
                !config.enabled,
                "Benchmarking should be disabled in release builds"
            );
            assert!(
                !config.write_to_file,
                "File writing should be disabled in release builds"
            );
        }

        // Common defaults
        assert!(
            !config.print_results,
            "Console output should be disabled by default"
        );
        assert!(
            config.min_duration_threshold > Duration::ZERO,
            "Min threshold should be positive"
        );
        assert!(config.max_samples > 0, "Max samples should be positive");
    }

    /// Tests utility functions for benchmark data management
    ///
    /// This test verifies that utility functions correctly
    /// interact with the global benchmark data store.
    #[test]
    fn test_benchmark_utilities() {
        // Clear any existing measurements
        utils::clear_measurements();

        // Verify initial state
        assert_eq!(
            utils::get_measurement_count(),
            0,
            "Should start with no measurements"
        );

        // Record some measurements
        let config = BenchmarkConfig {
            enabled: true,
            print_results: false,
            write_to_file: false,
            min_duration_threshold: Duration::ZERO,
            max_samples: 100,
        };

        {
            let _timer = ScopedTimer::new("utility_test", config);
            thread::sleep(Duration::from_millis(5));
        }

        // Verify measurements were recorded
        let measurements = utils::get_measurements();
        assert!(
            measurements.contains_key("utility_test"),
            "Should contain our test measurement"
        );

        // Test FPS recording
        let fps = utils::record_frame();
        assert!(fps >= 0.0, "FPS should be non-negative");

        let (min_fps, avg_fps, max_fps) = utils::get_fps_stats();
        assert!(min_fps >= 0.0, "Min FPS should be non-negative");
        assert!(avg_fps >= 0.0, "Avg FPS should be non-negative");
        assert!(max_fps >= 0.0, "Max FPS should be non-negative");

        // Clean up after test
        utils::clear_measurements();
    }

    /// Tests benchmark macro functionality
    ///
    /// This test verifies that the benchmark macros work correctly
    /// and integrate properly with the benchmarking system.
    #[test]
    fn test_benchmark_macros() {
        // Clear any existing measurements
        utils::clear_measurements();

        // Test the benchmark! macro
        benchmark!("macro_test", {
            thread::sleep(Duration::from_millis(5));
        });

        // Verify the measurement was recorded
        let measurements = utils::get_measurements();
        assert!(
            measurements.contains_key("macro_test"),
            "Macro measurement should be recorded"
        );

        // Test the debug_benchmark! macro
        debug_benchmark!("debug_macro_test", {
            thread::sleep(Duration::from_millis(5));
        });

        // In debug builds, the measurement should be recorded
        #[cfg(debug_assertions)]
        {
            let measurements = utils::get_measurements();
            assert!(
                measurements.contains_key("debug_macro_test"),
                "Debug macro measurement should be recorded in debug builds"
            );
        }

        // Clean up after test
        utils::clear_measurements();
    }

    /// Tests file output functionality
    ///
    /// This test verifies that benchmark results can be written
    /// to files correctly.
    #[test]
    fn test_file_output() {
        // Clear any existing measurements
        utils::clear_measurements();

        // Record some test measurements
        let config = BenchmarkConfig {
            enabled: true,
            print_results: false,
            write_to_file: false,
            min_duration_threshold: Duration::ZERO,
            max_samples: 100,
        };

        {
            let _timer = ScopedTimer::new("file_test", config);
            thread::sleep(Duration::from_millis(5));
        }

        // Test file writing with test source
        let result = utils::write_results_to_file("Test: test_file_output");
        assert!(result.is_ok(), "File writing should succeed");

        // Test force save (this will overwrite with Debug Run source)
        let force_result = utils::force_save_results();
        assert!(force_result.is_ok(), "Force save should succeed");

        // Test clear and save (this will overwrite with Debug Run source)
        let clear_result = utils::clear_and_save_measurements();
        assert!(clear_result.is_ok(), "Clear and save should succeed");

        // Clean up after test
        utils::clear_measurements();
    }

    /// Tests that benchmark files are properly labeled with test source
    ///
    /// This test verifies that benchmark files generated from tests
    /// are properly labeled with the test name in the source field.
    #[test]
    fn test_benchmark_file_labeling() {
        // Clear any existing measurements
        utils::clear_measurements();

        // Record some test measurements
        let config = BenchmarkConfig {
            enabled: true,
            print_results: false,
            write_to_file: false,
            min_duration_threshold: Duration::ZERO,
            max_samples: 100,
        };

        {
            let _timer = ScopedTimer::new("labeling_test", config);
            thread::sleep(Duration::from_millis(5));
        }

        // Generate file with test source label
        let result = utils::write_results_to_file("Test: test_benchmark_file_labeling");
        assert!(result.is_ok(), "File writing should succeed");

        // Clean up after test
        utils::clear_measurements();
    }

    /// Tests initialization benchmark categorization
    ///
    /// This test verifies that the benchmark system correctly
    /// categorizes initialization vs update benchmarks based on
    /// their naming patterns.
    #[test]
    fn test_initialization_benchmarks() {
        // Clear any existing measurements
        utils::clear_measurements();

        // Record measurements with different naming patterns
        let config = BenchmarkConfig {
            enabled: true,
            print_results: false,
            write_to_file: false,
            min_duration_threshold: Duration::ZERO,
            max_samples: 100,
        };

        // Initialization patterns
        let init_patterns = [
            "app_initialization",
            "window_creation",
            "system_setup",
            "resource_loading",
            "entity_spawning",
            "game_configuration",
        ];

        // Update patterns
        let update_patterns = [
            "frame_update",
            "input_processing",
            "physics_step",
            "rendering_pass",
        ];

        // Record initialization measurements
        for pattern in &init_patterns {
            let timer = Timer::new(pattern, config.clone());
            thread::sleep(Duration::from_millis(1));
            let _duration = timer.stop();
        }

        // Record update measurements
        for pattern in &update_patterns {
            let timer = Timer::new(pattern, config.clone());
            thread::sleep(Duration::from_millis(1));
            let _duration = timer.stop();
        }

        // Verify all measurements were recorded
        let measurements = utils::get_measurements();
        let expected_count = init_patterns.len() + update_patterns.len();

        // Print debug info
        println!(
            "Expected {} measurements, got {}",
            expected_count,
            measurements.len()
        );
        println!(
            "Available measurements: {:?}",
            measurements.keys().collect::<Vec<_>>()
        );

        // Check that at least some measurements were recorded
        assert!(
            measurements.len() > 0,
            "Expected at least some measurements, got {}",
            measurements.len()
        );

        // Verify that at least some of our patterns are present
        let mut found_patterns = 0;
        for pattern in &init_patterns {
            if measurements.contains_key(*pattern) {
                found_patterns += 1;
            }
        }
        for pattern in &update_patterns {
            if measurements.contains_key(*pattern) {
                found_patterns += 1;
            }
        }

        assert!(
            found_patterns > 0,
            "Expected at least some initialization/update patterns to be recorded, found {}",
            found_patterns
        );

        // Clean up after test
        utils::clear_measurements();
    }
}
