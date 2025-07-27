//! Benchmark result formatting utilities
//!
//! This module provides functions for formatting benchmark data into readable
//! tables and reports. It handles column width calculations and output formatting
//! to ensure benchmark results are displayed in a clean, organized manner.

use super::data::PerformanceMetrics;

/// Helper function to calculate column widths for benchmark table formatting
///
/// This function analyzes a collection of benchmark results to determine
/// appropriate column widths for tabular output. It ensures all data fits
/// properly while maintaining readability.
///
/// # Arguments
/// * `benchmarks` - A slice of tuples containing benchmark names and their metrics
/// * `has_multiple_counts` - Whether to include detailed statistics columns
///
/// # Returns
/// A tuple of column widths in the order: (name, count, total, avg, min, max)
pub fn calculate_column_widths(
    benchmarks: &[(&String, &PerformanceMetrics)],
    has_multiple_counts: bool,
) -> (usize, usize, usize, usize, usize, usize) {
    // Minimum widths for each column
    let mut name_width = 30;
    let mut count_width = 6;
    let mut total_width = 12;
    let mut avg_width = 10;
    let mut min_width = 10;
    let mut max_width = 10;

    for (name, metrics) in benchmarks {
        // Update name width
        name_width = name_width.max(name.len());

        // Update count width
        count_width = count_width.max(format!("{}", metrics.count).len());

        // Update duration widths by formatting them as strings
        let total_str = format!("{:?}", metrics.total_duration);
        total_width = total_width.max(total_str.len());

        if has_multiple_counts {
            let avg_str = format!("{:?}", metrics.avg_duration);
            avg_width = avg_width.max(avg_str.len());

            let min_str = format!("{:?}", metrics.min_duration);
            min_width = min_width.max(min_str.len());

            let max_str = format!("{:?}", metrics.max_duration);
            max_width = max_width.max(max_str.len());
        }
    }

    // Add some padding
    name_width = name_width.max(30);
    count_width = count_width.max(6);
    total_width = total_width.max(15);
    avg_width = avg_width.max(15);
    min_width = min_width.max(15);
    max_width = max_width.max(15);

    (
        name_width,
        count_width,
        total_width,
        avg_width,
        min_width,
        max_width,
    )
}
