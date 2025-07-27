use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::BenchmarkConfig;

#[allow(missing_docs)]

/// Frame rate counter for monitoring rendering performance
pub struct FrameRateCounter {
    /// Vector storing the duration of each recorded frame
    pub frame_times: Vec<Duration>,
    /// Maximum number of frame time samples to keep in memory
    max_samples: usize,
    /// Timestamp of the last recorded frame
    last_frame_time: Option<Instant>,
}

impl FrameRateCounter {
    /// Creates a new frame rate counter
    pub fn new(max_samples: usize) -> Self {
        Self {
            frame_times: Vec::with_capacity(max_samples),
            max_samples,
            last_frame_time: None,
        }
    }

    /// Records a frame and returns the current FPS
    pub fn record_frame(&mut self) -> f64 {
        let now = Instant::now();

        if let Some(last_time) = self.last_frame_time {
            let frame_time = now.duration_since(last_time);
            self.frame_times.push(frame_time);

            // Keep only the most recent samples
            if self.frame_times.len() > self.max_samples {
                self.frame_times.remove(0);
            }
        }

        self.last_frame_time = Some(now);
        self.get_fps()
    }

    /// Gets the current average FPS
    pub fn get_fps(&self) -> f64 {
        if self.frame_times.is_empty() {
            return 0.0;
        }

        let total_time: Duration = self.frame_times.iter().sum();
        let avg_frame_time = total_time / self.frame_times.len() as u32;

        if avg_frame_time.as_secs_f64() > 0.0 {
            1.0 / avg_frame_time.as_secs_f64()
        } else {
            0.0
        }
    }

    /// Gets the minimum frame time (worst performance)
    pub fn get_min_frame_time(&self) -> Duration {
        self.frame_times
            .iter()
            .min()
            .copied()
            .unwrap_or(Duration::ZERO)
    }

    /// Gets the maximum frame time (best performance)
    pub fn get_max_frame_time(&self) -> Duration {
        self.frame_times
            .iter()
            .max()
            .copied()
            .unwrap_or(Duration::ZERO)
    }
}

/// Memory usage tracker
pub struct MemoryTracker {
    initial_memory: Option<usize>,
}

impl MemoryTracker {
    /// Creates a new memory tracker
    pub fn new() -> Self {
        Self {
            initial_memory: None,
        }
    }

    /// Records the initial memory usage
    pub fn record_initial(&mut self) {
        self.initial_memory = Some(Self::get_current_memory_usage());
    }

    /// Gets the current memory usage in bytes
    pub fn get_current_memory_usage() -> usize {
        // This is a simplified implementation
        // In a real application, you might want to use platform-specific APIs
        // or a crate like `memory-stats` for more accurate measurements
        std::mem::size_of::<Self>()
    }

    /// Gets the memory usage difference since initial recording
    pub fn get_memory_delta(&self) -> Option<isize> {
        self.initial_memory.map(|initial| {
            let current = Self::get_current_memory_usage();
            current as isize - initial as isize
        })
    }
}

/// Performance profiler for identifying hot paths
pub struct Profiler {
    active_timers: HashMap<String, Instant>,
    config: BenchmarkConfig,
}

impl Profiler {
    /// Creates a new profiler
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            active_timers: HashMap::new(),
            config,
        }
    }

    /// Starts timing a section
    pub fn start_section(&mut self, name: &str) {
        if self.config.enabled {
            self.active_timers.insert(name.to_string(), Instant::now());
        }
    }

    /// Ends timing a section
    pub fn end_section(&mut self, name: &str) {
        if self.config.enabled {
            if let Some(start_time) = self.active_timers.remove(name) {
                let duration = start_time.elapsed();
                BENCHMARK_DATA
                    .lock()
                    .unwrap()
                    .record_measurement(name, duration);

                if self.config.print_results {
                    println!("[PROFILER] {}: {:?}", name, duration);
                }
            }
        }
    }

    /// Times a closure execution
    pub fn time_closure<F, R>(&mut self, name: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        if self.config.enabled {
            self.start_section(name);
            let result = f();
            self.end_section(name);
            result
        } else {
            f()
        }
    }
}

/// Performance metrics for a specific operation
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Number of times this operation was measured
    pub count: usize,
    /// Total duration of all measurements combined
    pub total_duration: Duration,
    /// Shortest duration recorded for this operation
    pub min_duration: Duration,
    /// Longest duration recorded for this operation
    pub max_duration: Duration,
    /// Average duration across all measurements
    pub avg_duration: Duration,
}

impl PerformanceMetrics {
    fn new() -> Self {
        Self {
            count: 0,
            total_duration: Duration::ZERO,
            min_duration: Duration::MAX,
            max_duration: Duration::ZERO,
            avg_duration: Duration::ZERO,
        }
    }

    fn update(&mut self, duration: Duration) {
        self.count += 1;
        self.total_duration += duration;
        self.min_duration = self.min_duration.min(duration);
        self.max_duration = self.max_duration.max(duration);
        self.avg_duration = self.total_duration / self.count as u32;
    }
}
lazy_static::lazy_static! {
    /// Centralized benchmark data storage
    pub static ref BENCHMARK_DATA: Arc<Mutex<BenchmarkData>> = Arc::new(Mutex::new(BenchmarkData::new()));
}

/// Central storage for all benchmark measurements
pub struct BenchmarkData {
    measurements: HashMap<String, PerformanceMetrics>,
    config: BenchmarkConfig,
    fps_counter: FrameRateCounter,
}

impl BenchmarkData {
    fn new() -> Self {
        Self {
            measurements: HashMap::new(),
            config: BenchmarkConfig::default(),
            fps_counter: FrameRateCounter::new(1000), // Store up to 1000 frame samples
        }
    }

    /// Records a new measurement for the specified operation
    ///
    /// # Arguments
    /// * `name` - The name of the operation being measured
    /// * `duration` - The duration of the operation
    pub fn record_measurement(&mut self, name: &str, duration: Duration) {
        let metrics = self
            .measurements
            .entry(name.to_string())
            .or_insert_with(PerformanceMetrics::new);
        metrics.update(duration);

        // Limit samples if configured
        if self.config.max_samples > 0 && self.measurements.len() > self.config.max_samples {
            // Remove oldest entries (simple strategy - could be improved)
            let keys_to_remove: Vec<_> = self
                .measurements
                .keys()
                .take(self.measurements.len() - self.config.max_samples)
                .cloned()
                .collect();
            for key in keys_to_remove {
                self.measurements.remove(&key);
            }
        }
    }

    /// Returns a copy of all recorded measurements
    pub fn get_measurements(&self) -> HashMap<String, PerformanceMetrics> {
        self.measurements.clone()
    }

    /// Clears all recorded measurements
    pub fn clear(&mut self) {
        self.measurements.clear();
    }

    /// Records a frame and returns the current FPS
    pub fn record_frame(&mut self) -> f64 {
        self.fps_counter.record_frame()
    }

    /// Returns FPS statistics as (min_fps, avg_fps, max_fps)
    pub fn get_fps_stats(&self) -> (f64, f64, f64) {
        let min_frame_time = self.fps_counter.get_min_frame_time();
        let max_frame_time = self.fps_counter.get_max_frame_time();

        // Minimum frame time (fastest frame) corresponds to maximum FPS
        let max_fps = if min_frame_time.as_secs_f64() > 0.0 {
            1.0 / min_frame_time.as_secs_f64()
        } else {
            0.0
        };

        // Maximum frame time (slowest frame) corresponds to minimum FPS
        let min_fps = if max_frame_time.as_secs_f64() > 0.0 {
            1.0 / max_frame_time.as_secs_f64()
        } else {
            0.0
        };

        let avg_fps = self.fps_counter.get_fps();

        (min_fps, avg_fps, max_fps)
    }
}
