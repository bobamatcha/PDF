//! Percentile calculations for benchmark results.
//!
//! This module provides utilities for computing percentiles and statistical
//! summaries from sample data.

/// Calculate percentile value from a slice of f64 samples.
///
/// Uses linear interpolation between nearest ranks for accurate percentile estimation.
///
/// # Arguments
///
/// * `samples` - Slice of samples (will be sorted internally)
/// * `p` - Percentile to calculate (0.0 to 100.0)
///
/// # Returns
///
/// * `Some(value)` - The percentile value
/// * `None` - If samples is empty or p is invalid
///
/// # Examples
///
/// ```
/// use benchmark_harness::stats::percentiles::percentile;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let p50 = percentile(&data, 50.0);
/// assert_eq!(p50, Some(3.0));
/// ```
pub fn percentile(samples: &[f64], p: f64) -> Option<f64> {
    if samples.is_empty() || !(0.0..=100.0).contains(&p) {
        return None;
    }

    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    if sorted.len() == 1 {
        return Some(sorted[0]);
    }

    // Calculate the rank using linear interpolation
    let rank = (p / 100.0) * (sorted.len() - 1) as f64;
    let lower_index = rank.floor() as usize;
    let upper_index = rank.ceil() as usize;

    if lower_index == upper_index {
        Some(sorted[lower_index])
    } else {
        let lower_value = sorted[lower_index];
        let upper_value = sorted[upper_index];
        let fraction = rank - lower_index as f64;
        Some(lower_value + fraction * (upper_value - lower_value))
    }
}

/// Calculate common percentiles in one pass.
///
/// Provides a comprehensive statistical summary including min, max, quartiles,
/// mean, standard deviation, and common percentiles (p95, p99).
#[derive(Debug, Clone)]
pub struct PercentileSummary {
    pub min: f64,
    pub p25: f64,
    pub p50: f64, // median
    pub p75: f64,
    pub p95: f64,
    pub p99: f64,
    pub max: f64,
    pub mean: f64,
    pub std_dev: f64,
    pub count: usize,
}

impl PercentileSummary {
    /// Compute summary statistics from samples.
    ///
    /// # Arguments
    ///
    /// * `samples` - Slice of samples to analyze
    ///
    /// # Returns
    ///
    /// * `Some(summary)` - Statistical summary
    /// * `None` - If samples is empty
    ///
    /// # Examples
    ///
    /// ```
    /// use benchmark_harness::stats::percentiles::PercentileSummary;
    ///
    /// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    /// let summary = PercentileSummary::from_samples(&data).unwrap();
    /// assert_eq!(summary.count, 10);
    /// assert_eq!(summary.mean, 5.5);
    /// assert_eq!(summary.min, 1.0);
    /// assert_eq!(summary.max, 10.0);
    /// ```
    pub fn from_samples(samples: &[f64]) -> Option<Self> {
        if samples.is_empty() {
            return None;
        }

        let mut sorted = samples.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let count = sorted.len();
        let min = sorted[0];
        let max = sorted[count - 1];

        // Calculate mean
        let sum: f64 = sorted.iter().sum();
        let mean = sum / count as f64;

        // Calculate standard deviation (sample std dev)
        let variance = if count > 1 {
            let squared_diffs: f64 = sorted.iter().map(|&x| (x - mean).powi(2)).sum();
            squared_diffs / (count - 1) as f64
        } else {
            0.0
        };
        let std_dev = variance.sqrt();

        // Calculate percentiles
        let p25 = percentile(&sorted, 25.0)?;
        let p50 = percentile(&sorted, 50.0)?;
        let p75 = percentile(&sorted, 75.0)?;
        let p95 = percentile(&sorted, 95.0)?;
        let p99 = percentile(&sorted, 99.0)?;

        Some(PercentileSummary {
            min,
            p25,
            p50,
            p75,
            p95,
            p99,
            max,
            mean,
            std_dev,
            count,
        })
    }

    /// Coefficient of variation (std_dev / mean).
    ///
    /// Returns a relative measure of variability. Lower values indicate
    /// less variability relative to the mean.
    ///
    /// # Returns
    ///
    /// The coefficient of variation, or `f64::INFINITY` if mean is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use benchmark_harness::stats::percentiles::PercentileSummary;
    ///
    /// let data = vec![10.0, 12.0, 14.0, 16.0, 18.0];
    /// let summary = PercentileSummary::from_samples(&data).unwrap();
    /// let cv = summary.coefficient_of_variation();
    /// assert!(cv > 0.0 && cv < 1.0);
    /// ```
    pub fn coefficient_of_variation(&self) -> f64 {
        if self.mean == 0.0 {
            f64::INFINITY
        } else {
            self.std_dev / self.mean
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentile_empty_samples() {
        assert_eq!(percentile(&[], 50.0), None);
    }

    #[test]
    fn test_percentile_single_sample() {
        let samples = vec![42.0];
        assert_eq!(percentile(&samples, 50.0), Some(42.0));
        assert_eq!(percentile(&samples, 0.0), Some(42.0));
        assert_eq!(percentile(&samples, 100.0), Some(42.0));
    }

    #[test]
    fn test_percentile_invalid_p() {
        let samples = vec![1.0, 2.0, 3.0];
        assert_eq!(percentile(&samples, -1.0), None);
        assert_eq!(percentile(&samples, 101.0), None);
    }

    #[test]
    fn test_percentile_simple_case() {
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        // Test exact percentiles
        assert_eq!(percentile(&samples, 0.0), Some(1.0));
        assert_eq!(percentile(&samples, 50.0), Some(3.0));
        assert_eq!(percentile(&samples, 100.0), Some(5.0));
    }

    #[test]
    fn test_percentile_with_interpolation() {
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];

        // p25 should be 3.25 (interpolation between 3 and 4)
        let p25 = percentile(&samples, 25.0).unwrap();
        assert!((p25 - 3.25).abs() < 1e-10);

        // p75 should be 7.75 (interpolation between 7 and 8)
        let p75 = percentile(&samples, 75.0).unwrap();
        assert!((p75 - 7.75).abs() < 1e-10);
    }

    #[test]
    fn test_percentile_unsorted_samples() {
        let samples = vec![5.0, 1.0, 3.0, 2.0, 4.0];
        assert_eq!(percentile(&samples, 50.0), Some(3.0));
    }

    #[test]
    fn test_percentile_summary_empty() {
        assert!(PercentileSummary::from_samples(&[]).is_none());
    }

    #[test]
    fn test_percentile_summary_single_sample() {
        let samples = vec![42.0];
        let summary = PercentileSummary::from_samples(&samples).unwrap();

        assert_eq!(summary.count, 1);
        assert_eq!(summary.min, 42.0);
        assert_eq!(summary.max, 42.0);
        assert_eq!(summary.mean, 42.0);
        assert_eq!(summary.std_dev, 0.0);
        assert_eq!(summary.p25, 42.0);
        assert_eq!(summary.p50, 42.0);
        assert_eq!(summary.p75, 42.0);
        assert_eq!(summary.p95, 42.0);
        assert_eq!(summary.p99, 42.0);
    }

    #[test]
    fn test_percentile_summary_basic() {
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let summary = PercentileSummary::from_samples(&samples).unwrap();

        assert_eq!(summary.count, 10);
        assert_eq!(summary.min, 1.0);
        assert_eq!(summary.max, 10.0);
        assert_eq!(summary.mean, 5.5);
        assert_eq!(summary.p50, 5.5);

        // Check that std_dev is reasonable
        assert!(summary.std_dev > 2.8 && summary.std_dev < 3.1);
    }

    #[test]
    fn test_percentile_summary_mean_and_std_dev() {
        // Test with known values
        let samples = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let summary = PercentileSummary::from_samples(&samples).unwrap();

        // Mean should be 5.0
        assert_eq!(summary.mean, 5.0);

        // Sample std dev for this dataset
        // Variance = sum((x - mean)^2) / (n-1) = (9+1+1+1+0+0+4+16)/7 = 32/7 = 4.571...
        // std_dev = sqrt(32/7) ≈ 2.138
        let expected_std_dev = (32.0_f64 / 7.0).sqrt();
        assert!((summary.std_dev - expected_std_dev).abs() < 1e-10);
    }

    #[test]
    fn test_percentile_summary_unsorted() {
        let samples = vec![10.0, 1.0, 5.0, 3.0, 8.0, 2.0, 9.0, 4.0, 7.0, 6.0];
        let summary = PercentileSummary::from_samples(&samples).unwrap();

        assert_eq!(summary.min, 1.0);
        assert_eq!(summary.max, 10.0);
        assert_eq!(summary.mean, 5.5);
    }

    #[test]
    fn test_coefficient_of_variation() {
        let samples = vec![10.0, 12.0, 14.0, 16.0, 18.0];
        let summary = PercentileSummary::from_samples(&samples).unwrap();

        let cv = summary.coefficient_of_variation();
        assert!(cv > 0.0 && cv < 1.0);
    }

    #[test]
    fn test_coefficient_of_variation_zero_mean() {
        // Create a summary with zero mean (this would be artificial)
        let summary = PercentileSummary {
            min: -5.0,
            p25: -2.5,
            p50: 0.0,
            p75: 2.5,
            p95: 4.5,
            p99: 4.9,
            max: 5.0,
            mean: 0.0,
            std_dev: 3.0,
            count: 10,
        };

        assert_eq!(summary.coefficient_of_variation(), f64::INFINITY);
    }

    #[test]
    fn test_coefficient_of_variation_low_variance() {
        let samples = vec![100.0, 100.0, 100.0, 100.0, 100.0];
        let summary = PercentileSummary::from_samples(&samples).unwrap();

        let cv = summary.coefficient_of_variation();
        assert_eq!(cv, 0.0);
    }

    #[test]
    fn test_percentile_with_duplicates() {
        let samples = vec![1.0, 2.0, 2.0, 2.0, 3.0, 4.0, 5.0];
        let p50 = percentile(&samples, 50.0).unwrap();

        // Median should be the middle value (index 3 in sorted array)
        assert_eq!(p50, 2.0);
    }

    #[test]
    fn test_percentile_extreme_values() {
        let samples = vec![1.0, 1000.0, 2.0, 3.0, 4.0];
        let summary = PercentileSummary::from_samples(&samples).unwrap();

        assert_eq!(summary.min, 1.0);
        assert_eq!(summary.max, 1000.0);
        assert_eq!(summary.p50, 3.0);

        // Mean should be affected by the outlier
        assert!(summary.mean > 100.0);
    }
}
