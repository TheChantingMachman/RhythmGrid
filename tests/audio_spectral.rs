// @spec-tags: core,audio,analysis
// @invariants: spectral_centroid returns energy-weighted center frequency normalized to [0.0,1.0]; SpectralFluxDetector computes half-wave-rectified frame-to-frame band energy change
// @build: 83

use rhythm_grid::audio::{spectral_centroid, SpectralFluxDetector};
use std::f32::consts::PI;

/// Generate a mono sine wave at `freq_hz` for `num_samples` samples.
fn sine_wave(freq_hz: f32, sample_rate: u32, num_samples: usize) -> Vec<f32> {
    (0..num_samples)
        .map(|i| (2.0 * PI * freq_hz * i as f32 / sample_rate as f32).sin())
        .collect()
}

// ─── spectral_centroid: edge cases ──────────────────────────────────────────

#[test]
fn spectral_centroid_empty_returns_zero() {
    assert_eq!(spectral_centroid(&[], 44100), 0.0);
}

#[test]
fn spectral_centroid_all_zero_samples_returns_zero() {
    let samples = vec![0.0f32; 4096];
    assert_eq!(spectral_centroid(&samples, 44100), 0.0);
}

#[test]
fn spectral_centroid_zero_sample_rate_returns_zero() {
    let samples = vec![1.0f32; 1024];
    assert_eq!(spectral_centroid(&samples, 0), 0.0);
}

// ─── spectral_centroid: return value invariants ──────────────────────────────

#[test]
fn spectral_centroid_return_value_in_0_to_1() {
    // Any non-trivial input must produce a result in [0.0, 1.0]
    let samples = sine_wave(1000.0, 44100, 4096);
    let result = spectral_centroid(&samples, 44100);
    assert!(
        result >= 0.0 && result <= 1.0,
        "spectral_centroid={result} is out of [0.0, 1.0]"
    );
}

#[test]
fn spectral_centroid_return_type_is_f32() {
    let samples = sine_wave(440.0, 44100, 4096);
    let _: f32 = spectral_centroid(&samples, 44100);
}

// ─── spectral_centroid: normalization formula ────────────────────────────────

#[test]
fn spectral_centroid_low_frequency_tone_near_zero() {
    // 100 Hz tone: centroid_hz ≈ 100, normalized = (100 - 20) / 19980 ≈ 0.004
    // Must be well below 0.1 (low end of the 0–1 range)
    let samples = sine_wave(100.0, 44100, 4096);
    let result = spectral_centroid(&samples, 44100);
    assert!(
        result < 0.1,
        "spectral_centroid={result} for 100 Hz tone should be near 0.0 (low frequency)"
    );
}

#[test]
fn spectral_centroid_high_frequency_tone_near_one() {
    // 15000 Hz tone: centroid_hz ≈ 15000, normalized = (15000 - 20) / 19980 ≈ 0.75
    // Must be well above 0.5 (high end of the 0–1 range)
    let samples = sine_wave(15000.0, 44100, 4096);
    let result = spectral_centroid(&samples, 44100);
    assert!(
        result > 0.5,
        "spectral_centroid={result} for 15000 Hz tone should be near 1.0 (high frequency)"
    );
}

#[test]
fn spectral_centroid_low_to_high_frequency_ordering() {
    // A pure low-frequency tone must yield a lower centroid than a pure high-frequency tone
    let low_samples = sine_wave(200.0, 44100, 4096);
    let high_samples = sine_wave(10000.0, 44100, 4096);
    let low_centroid = spectral_centroid(&low_samples, 44100);
    let high_centroid = spectral_centroid(&high_samples, 44100);
    assert!(
        low_centroid < high_centroid,
        "low_centroid={low_centroid} should be less than high_centroid={high_centroid}"
    );
}

#[test]
fn spectral_centroid_mid_frequency_in_middle_range() {
    // 1000 Hz: centroid_hz ≈ 1000, normalized = (1000 - 20) / 19980 ≈ 0.049
    // Should be in 0.0–0.2 range (1000 Hz is still relatively low in log-perceptual terms,
    // but linearly it's close to the lower end of 20–20000 Hz)
    let samples = sine_wave(1000.0, 44100, 4096);
    let result = spectral_centroid(&samples, 44100);
    assert!(
        result >= 0.0 && result <= 0.2,
        "spectral_centroid={result} for 1000 Hz tone should be in [0.0, 0.2]"
    );
}

#[test]
fn spectral_centroid_normalized_formula_at_20hz_boundary() {
    // A very-low-frequency tone should produce a value very close to 0.0
    // (20 Hz maps to exactly 0.0 in the normalization formula)
    let samples = sine_wave(30.0, 44100, 8192);
    let result = spectral_centroid(&samples, 44100);
    assert!(
        result < 0.05,
        "spectral_centroid={result} for 30 Hz tone should be very close to 0.0"
    );
}

#[test]
fn spectral_centroid_monotonically_increases_with_frequency() {
    // Centroid values for ascending pure tones must be strictly ascending
    let freqs = [100.0f32, 500.0, 2000.0, 8000.0, 15000.0];
    let sample_rate = 44100u32;
    let num_samples = 4096;

    let centroids: Vec<f32> = freqs
        .iter()
        .map(|&f| spectral_centroid(&sine_wave(f, sample_rate, num_samples), sample_rate))
        .collect();

    for i in 1..centroids.len() {
        assert!(
            centroids[i] > centroids[i - 1],
            "centroid[{i}]={} ({}Hz) should exceed centroid[{}]={} ({}Hz)",
            centroids[i], freqs[i], i - 1, centroids[i - 1], freqs[i - 1]
        );
    }
}

// ─── SpectralFluxDetector: construction ──────────────────────────────────────

#[test]
fn spectral_flux_detector_new_initializes() {
    // Must not panic; Debug must be derivable (implicit through format)
    let detector = SpectralFluxDetector::new();
    let _ = format!("{detector:?}");
}

// ─── SpectralFluxDetector: first-call guarantee ──────────────────────────────

#[test]
fn spectral_flux_first_call_all_zero_bands_returns_zero() {
    // Initial state is zeroed; feeding all-zero bands → no increases → 0.0
    let mut detector = SpectralFluxDetector::new();
    let result = detector.detect(&[0.0f32; 7]);
    assert_eq!(result, 0.0, "first call with all-zero bands should return 0.0");
}

#[test]
fn spectral_flux_result_is_non_negative() {
    let mut detector = SpectralFluxDetector::new();
    let bands = [0.2, 0.5, 0.1, 0.8, 0.3, 0.6, 0.4];
    let result = detector.detect(&bands);
    assert!(result >= 0.0, "detect result={result} must be >= 0.0");
}

// ─── SpectralFluxDetector: identical frames ───────────────────────────────────

#[test]
fn spectral_flux_identical_consecutive_frames_returns_zero() {
    let mut detector = SpectralFluxDetector::new();
    let bands = [0.3, 0.5, 0.2, 0.7, 0.1, 0.4, 0.6];
    detector.detect(&bands); // prime with first frame
    let result = detector.detect(&bands); // identical second frame
    assert_eq!(
        result, 0.0,
        "identical consecutive frames should return 0.0, got {result}"
    );
}

// ─── SpectralFluxDetector: half-wave rectification ───────────────────────────

#[test]
fn spectral_flux_decreasing_band_does_not_contribute() {
    // Bands go from high to low — no positive differences → 0.0
    let mut detector = SpectralFluxDetector::new();
    let high_bands = [0.8, 0.8, 0.8, 0.8, 0.8, 0.8, 0.8];
    detector.detect(&high_bands); // prime
    let low_bands = [0.2, 0.2, 0.2, 0.2, 0.2, 0.2, 0.2];
    let result = detector.detect(&low_bands);
    assert_eq!(
        result, 0.0,
        "only decreasing bands → flux should be 0.0, got {result}"
    );
}

#[test]
fn spectral_flux_only_positive_differences_count() {
    // Mix of increases and decreases — only increases should sum
    let mut detector = SpectralFluxDetector::new();
    // Prime with [0.5; 7]
    detector.detect(&[0.5f32; 7]);
    // New frame: bands 0..3 decrease to 0.3, bands 4..7 increase to 0.8
    let new_bands = [0.3, 0.3, 0.3, 0.3, 0.8, 0.8, 0.8];
    let result = detector.detect(&new_bands);
    // Expected: only bands 4,5,6 contribute → 3 * (0.8 - 0.5) = 0.9
    let expected = 3.0 * 0.3f32;
    assert!(
        (result - expected).abs() < 1e-5,
        "expected {expected} (only positive diffs), got {result}"
    );
}

// ─── SpectralFluxDetector: single-band spike ─────────────────────────────────

#[test]
fn spectral_flux_single_band_increase_equals_that_bands_delta() {
    let mut detector = SpectralFluxDetector::new();
    // Start with all zeros (initial state)
    // First detect with [0.0; 7] — flux = 0.0, previous updated to [0.0; 7]
    detector.detect(&[0.0f32; 7]);
    // Spike only band 3 by 0.6
    let mut spiked = [0.0f32; 7];
    spiked[3] = 0.6;
    let result = detector.detect(&spiked);
    assert!(
        (result - 0.6).abs() < 1e-5,
        "single spike in band 3 of 0.6 → flux should be 0.6, got {result}"
    );
}

#[test]
fn spectral_flux_all_bands_increase_returns_total_increase() {
    let mut detector = SpectralFluxDetector::new();
    // Prime with zeros (initial state already zeroed, but make explicit)
    detector.detect(&[0.0f32; 7]);
    // All bands increase by 0.1
    let result = detector.detect(&[0.1f32; 7]);
    let expected = 7.0 * 0.1f32;
    assert!(
        (result - expected).abs() < 1e-5,
        "all 7 bands increasing by 0.1 → flux should be {expected}, got {result}"
    );
}

// ─── SpectralFluxDetector: state update ──────────────────────────────────────

#[test]
fn spectral_flux_previous_frame_updates_after_each_call() {
    // After feeding bands A then bands B, the reference for the next call is B
    let mut detector = SpectralFluxDetector::new();
    let frame_a = [0.5f32; 7];
    let frame_b = [0.3f32; 7]; // decreases from A; flux between A→B = 0.0
    detector.detect(&frame_a);
    let flux_ab = detector.detect(&frame_b);
    assert_eq!(flux_ab, 0.0, "A→B (decreasing) should give 0.0, got {flux_ab}");

    // Now feed frame_c that is higher than frame_b — if state was properly updated to B, flux > 0
    let frame_c = [0.6f32; 7]; // increases from B by 0.3 each
    let flux_bc = detector.detect(&frame_c);
    let expected = 7.0 * (0.6 - 0.3);
    assert!(
        (flux_bc - expected).abs() < 1e-5,
        "B→C increase expected {expected}, got {flux_bc}"
    );
}

#[test]
fn spectral_flux_sequence_of_three_frames_tracks_correctly() {
    // Verify the detector correctly tracks across multiple frames
    let mut detector = SpectralFluxDetector::new();

    // Frame 1 from zeroed initial state: all bands = 0.4 → flux = 7 * 0.4 = 2.8
    let frame1 = [0.4f32; 7];
    let flux1 = detector.detect(&frame1);
    let expected1 = 7.0 * 0.4;
    assert!(
        (flux1 - expected1).abs() < 1e-5,
        "frame1 flux expected {expected1}, got {flux1}"
    );

    // Frame 2 identical to frame1 → flux = 0.0
    let flux2 = detector.detect(&frame1);
    assert_eq!(flux2, 0.0, "identical frame2 flux should be 0.0, got {flux2}");

    // Frame 3: all bands = 0.9 → flux = 7 * (0.9 - 0.4) = 3.5
    let frame3 = [0.9f32; 7];
    let flux3 = detector.detect(&frame3);
    let expected3 = 7.0 * (0.9 - 0.4);
    assert!(
        (flux3 - expected3).abs() < 1e-5,
        "frame3 flux expected {expected3}, got {flux3}"
    );
}
