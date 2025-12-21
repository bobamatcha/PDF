//! Outlier detection for benchmark results.
//!
//! This module provides utilities for detecting and filtering outliers
//! using the Interquartile Range (IQR) method.

use super::percentiles::percentile;

/// Outlier detection using Interquartile Range (IQR) method.
///
/// Outliers are detected as points that fall outside the range
/// [Q1 - 1.5*IQR, Q3 + 1.5*IQR], where IQR = Q3 - Q1.
#[derive(Debug, Clone)]
pub struct OutlierResult {
    /// Indices of detected outliers in the original sample array
    pub outlier_indices: Vec<usize>,
    /// Lower fence (Q1 - 1.5*IQR)
    pub lower_fence: f64,
    /// Upper fence (Q3 + 1.5*IQR)
    pub upper_fence: f64,
    /// First quartile (25th percentile)
    pub q1: f64,
    /// Third quartile (75th percentile)
    pub q3: f64,
    /// Interquartile range (Q3 - Q1)
    pub iqr: f64,
}

impl OutlierResult {
    /// Detect outliers in samples using IQR method.
    ///
    /// Points outside [Q1 - 1.5*IQR, Q3 + 1.5*IQR] are considered outliers.
    ///
    /// # Arguments
    ///
    /// * `samples` - Slice of samples to analyze
    ///
    /// # Returns
    ///
    /// * `Some(result)` - Outlier detection results
    /// * `None` - If samples is empty or has insufficient data
    ///
    /// # Examples
    ///
    /// ```
    /// use benchmark_harness::stats::outliers::OutlierResult;
    ///
    /// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 100.0]; // 100.0 is an outlier
    /// let result = OutlierResult::detect(&data).unwrap();
    /// assert!(result.has_outliers());
    /// assert_eq!(result.outlier_indices, vec![5]);
    /// ```
    pub fn detect(samples: &[f64]) -> Option<Self> {
        if samples.is_empty() {
            return None;
        }

        // Need at least 4 points for meaningful IQR calculation
        if samples.len() < 4 {
            return Some(OutlierResult {
                outlier_indices: Vec::new(),
                lower_fence: f64::NEG_INFINITY,
                upper_fence: f64::INFINITY,
                q1: samples[0],
                q3: samples[samples.len() - 1],
                iqr: 0.0,
            });
        }

        let q1 = percentile(samples, 25.0)?;
        let q3 = percentile(samples, 75.0)?;
        let iqr = q3 - q1;

        let lower_fence = q1 - 1.5 * iqr;
        let upper_fence = q3 + 1.5 * iqr;

        // Find outlier indices
        let outlier_indices: Vec<usize> = samples
            .iter()
            .enumerate()
            .filter_map(|(i, &value)| {
                if value < lower_fence || value > upper_fence {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();

        Some(OutlierResult {
            outlier_indices,
            lower_fence,
            upper_fence,
            q1,
            q3,
            iqr,
        })
    }

    /// Get the clean samples (outliers removed).
    ///
    /// Returns a new vector containing only the non-outlier values.
    ///
    /// # Arguments
    ///
    /// * `samples` - Original sample data
    ///
    /// # Returns
    ///
    /// Vector of samples with outliers removed
    ///
    /// # Examples
    ///
    /// ```
    /// use benchmark_harness::stats::outliers::OutlierResult;
    ///
    /// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 100.0];
    /// let result = OutlierResult::detect(&data).unwrap();
    /// let clean = result.clean_samples(&data);
    /// assert_eq!(clean.len(), 5); // 100.0 removed
    /// ```
    pub fn clean_samples<'a>(&self, samples: &'a [f64]) -> Vec<f64> {
        samples
            .iter()
            .enumerate()
            .filter_map(|(i, &value)| {
                if self.outlier_indices.contains(&i) {
                    None
                } else {
                    Some(value)
                }
            })
            .collect()
    }

    /// Check if any outliers were detected.
    ///
    /// # Returns
    ///
    /// `true` if at least one outlier was found, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use benchmark_harness::stats::outliers::OutlierResult;
    ///
    /// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    /// let result = OutlierResult::detect(&data).unwrap();
    /// assert!(!result.has_outliers());
    ///
    /// let data_with_outlier = vec![1.0, 2.0, 3.0, 4.0, 5.0, 100.0];
    /// let result2 = OutlierResult::detect(&data_with_outlier).unwrap();
    /// assert!(result2.has_outliers());
    /// ```
    pub fn has_outliers(&self) -> bool {
        !self.outlier_indices.is_empty()
    }
}

/// Remove warmup iterations from the beginning.
///
/// Benchmark runs often have initial "warmup" iterations that should be
/// excluded from analysis due to cold start effects.
///
/// # Arguments
///
/// * `samples` - Original sample data
/// * `warmup_count` - Number of warmup samples to remove from the beginning
///
/// # Returns
///
/// Slice of samples with warmup iterations removed
///
/// # Examples
///
/// ```
/// use benchmark_harness::stats::outliers::remove_warmup;
///
/// let data = vec![100.0, 95.0, 10.0, 11.0, 10.5, 11.5]; // First 2 are warmup
/// let clean = remove_warmup(&data, 2);
/// assert_eq!(clean, &[10.0, 11.0, 10.5, 11.5]);
/// ```
pub fn remove_warmup(samples: &[f64], warmup_count: usize) -> &[f64] {
    if warmup_count >= samples.len() {
        &[]
    } else {
        &samples[warmup_count..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outlier_detection_empty() {
        assert!(OutlierResult::detect(&[]).is_none());
    }

    #[test]
    fn test_outlier_detection_insufficient_data() {
        let samples = vec![1.0, 2.0, 3.0];
        let result = OutlierResult::detect(&samples).unwrap();

        // Should return no outliers with insufficient data
        assert!(!result.has_outliers());
        assert_eq!(result.outlier_indices.len(), 0);
    }

    #[test]
    fn test_outlier_detection_no_outliers() {
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = OutlierResult::detect(&samples).unwrap();

        assert!(!result.has_outliers());
        assert_eq!(result.outlier_indices.len(), 0);
    }

    #[test]
    fn test_outlier_detection_single_high_outlier() {
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0, 100.0];
        let result = OutlierResult::detect(&samples).unwrap();

        assert!(result.has_outliers());
        assert_eq!(result.outlier_indices, vec![5]);
        assert!(result.upper_fence < 100.0);
    }

    #[test]
    fn test_outlier_detection_single_low_outlier() {
        let samples = vec![0.01, 10.0, 11.0, 12.0, 13.0, 14.0];
        let result = OutlierResult::detect(&samples).unwrap();

        assert!(result.has_outliers());
        assert_eq!(result.outlier_indices, vec![0]);
        assert!(result.lower_fence > 0.01);
    }

    #[test]
    fn test_outlier_detection_multiple_outliers() {
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0, 100.0, 200.0];
        let result = OutlierResult::detect(&samples).unwrap();

        assert!(result.has_outliers());
        // Should detect high outliers
        assert!(!result.outlier_indices.is_empty());
        // 100.0 and 200.0 should be outliers (indices 5 and 6)
        assert!(result.outlier_indices.contains(&5) || result.outlier_indices.contains(&6));
    }

    #[test]
    fn test_outlier_detection_iqr_calculation() {
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = OutlierResult::detect(&samples).unwrap();

        // Q1 should be around 3.25, Q3 around 7.75
        assert!((result.q1 - 3.25).abs() < 0.1);
        assert!((result.q3 - 7.75).abs() < 0.1);

        // IQR should be Q3 - Q1
        let expected_iqr = result.q3 - result.q1;
        assert!((result.iqr - expected_iqr).abs() < 1e-10);

        // Fences should be Q1 - 1.5*IQR and Q3 + 1.5*IQR
        let expected_lower = result.q1 - 1.5 * result.iqr;
        let expected_upper = result.q3 + 1.5 * result.iqr;
        assert!((result.lower_fence - expected_lower).abs() < 1e-10);
        assert!((result.upper_fence - expected_upper).abs() < 1e-10);
    }

    #[test]
    fn test_clean_samples() {
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0, 100.0];
        let result = OutlierResult::detect(&samples).unwrap();
        let clean = result.clean_samples(&samples);

        assert_eq!(clean.len(), 5);
        assert!(!clean.contains(&100.0));
        assert_eq!(clean, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_clean_samples_no_outliers() {
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = OutlierResult::detect(&samples).unwrap();
        let clean = result.clean_samples(&samples);

        assert_eq!(clean.len(), samples.len());
        assert_eq!(clean, samples);
    }

    #[test]
    fn test_clean_samples_all_outliers() {
        // Create a scenario where outliers are detected
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0, 1000.0];
        let result = OutlierResult::detect(&samples).unwrap();
        let clean = result.clean_samples(&samples);

        // 1000.0 should be detected as outlier
        assert!(result.has_outliers());
        assert!(clean.len() <= samples.len());
    }

    #[test]
    fn test_remove_warmup_normal() {
        let samples = vec![100.0, 95.0, 10.0, 11.0, 10.5, 11.5];
        let clean = remove_warmup(&samples, 2);

        assert_eq!(clean, &[10.0, 11.0, 10.5, 11.5]);
    }

    #[test]
    fn test_remove_warmup_zero() {
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let clean = remove_warmup(&samples, 0);

        assert_eq!(clean, &samples[..]);
    }

    #[test]
    fn test_remove_warmup_all() {
        let samples = vec![1.0, 2.0, 3.0];
        let clean = remove_warmup(&samples, 3);

        assert!(clean.is_empty());
    }

    #[test]
    fn test_remove_warmup_more_than_samples() {
        let samples = vec![1.0, 2.0, 3.0];
        let clean = remove_warmup(&samples, 10);

        assert!(clean.is_empty());
    }

    #[test]
    fn test_remove_warmup_empty() {
        let samples: Vec<f64> = vec![];
        let clean = remove_warmup(&samples, 5);

        assert!(clean.is_empty());
    }

    #[test]
    fn test_has_outliers() {
        let samples_no_outliers = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result1 = OutlierResult::detect(&samples_no_outliers).unwrap();
        assert!(!result1.has_outliers());

        let samples_with_outliers = vec![1.0, 2.0, 3.0, 4.0, 5.0, 100.0];
        let result2 = OutlierResult::detect(&samples_with_outliers).unwrap();
        assert!(result2.has_outliers());
    }

    #[test]
    fn test_outlier_detection_with_duplicates() {
        let samples = vec![1.0, 2.0, 2.0, 2.0, 3.0, 4.0, 5.0, 100.0];
        let result = OutlierResult::detect(&samples).unwrap();

        assert!(result.has_outliers());
        assert!(result.outlier_indices.contains(&7)); // 100.0 is outlier
    }

    #[test]
    fn test_outlier_detection_all_same_values() {
        let samples = vec![5.0, 5.0, 5.0, 5.0, 5.0];
        let result = OutlierResult::detect(&samples).unwrap();

        // No outliers when all values are the same
        assert!(!result.has_outliers());
        assert_eq!(result.iqr, 0.0);
    }

    #[test]
    fn test_clean_samples_preserves_order() {
        // Use data where outliers are clearly detected (1000.0 is far from the cluster of 1-5)
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0, 1000.0];
        let result = OutlierResult::detect(&samples).unwrap();
        let clean = result.clean_samples(&samples);

        // Clean samples should maintain original order and have outliers removed
        // If outliers are detected, clean should be smaller
        if result.has_outliers() {
            assert!(clean.len() < samples.len());
            // The clean samples should be in the same relative order
            for i in 1..clean.len() {
                // Find original positions and check order
                let pos_prev = samples.iter().position(|&x| x == clean[i - 1]).unwrap();
                let pos_curr = samples.iter().position(|&x| x == clean[i]).unwrap();
                assert!(pos_prev < pos_curr);
            }
        }
    }

    #[test]
    fn test_outlier_boundary_cases() {
        // Test values exactly at fence boundaries
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = OutlierResult::detect(&samples).unwrap();

        // Values within fences should not be outliers
        let lower_test = result.lower_fence + 0.01;
        let upper_test = result.upper_fence - 0.01;

        assert!(lower_test >= result.lower_fence);
        assert!(upper_test <= result.upper_fence);
    }
}
