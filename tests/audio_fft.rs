// @spec-tags: core,audio,analysis
// @invariants: fft_bands returns normalized (bass, mids, highs) energy per band; edge cases return (0,0,0); pure tones route to correct band; values in [0.0,1.0]
// @build: 56

use rhythm_grid::audio::fft_bands;
use std::f32::consts::PI;

/// Generate a mono sine wave at `freq_hz` for `num_samples` samples.
fn sine_wave(freq_hz: f32, sample_rate: u32, num_samples: usize) -> Vec<f32> {
    (0..num_samples)
        .map(|i| (2.0 * PI * freq_hz * i as f32 / sample_rate as f32).sin())
        .collect()
}

// --- Edge cases ---

#[test]
fn fft_bands_empty_samples_returns_zero() {
    assert_eq!(fft_bands(&[], 44100), (0.0f32, 0.0f32, 0.0f32));
}

#[test]
fn fft_bands_zero_sample_rate_returns_zero() {
    let samples = vec![1.0f32; 1024];
    assert_eq!(fft_bands(&samples, 0), (0.0f32, 0.0f32, 0.0f32));
}

#[test]
fn fft_bands_all_zero_samples_returns_zero() {
    // total_power is zero → (0.0, 0.0, 0.0)
    let samples = vec![0.0f32; 1024];
    assert_eq!(fft_bands(&samples, 44100), (0.0f32, 0.0f32, 0.0f32));
}

#[test]
fn fft_bands_does_not_panic_on_single_sample() {
    // Hann window edge case with N=1 — must not panic
    let (bass, mids, highs) = fft_bands(&[1.0f32], 44100);
    assert!(bass >= 0.0 && bass <= 1.0, "bass={bass}");
    assert!(mids >= 0.0 && mids <= 1.0, "mids={mids}");
    assert!(highs >= 0.0 && highs <= 1.0, "highs={highs}");
}

// --- Return type and signature ---

#[test]
fn fft_bands_return_type_is_f32_triple() {
    let samples = sine_wave(440.0, 44100, 4096);
    let (_bass, _mids, _highs): (f32, f32, f32) = fft_bands(&samples, 44100);
}

// --- Normalization invariants ---

#[test]
fn fft_bands_values_in_0_to_1() {
    // 440 Hz (mids) — general normalization check
    let samples = sine_wave(440.0, 44100, 4096);
    let (bass, mids, highs) = fft_bands(&samples, 44100);
    assert!(bass >= 0.0 && bass <= 1.0, "bass={bass} out of [0,1]");
    assert!(mids >= 0.0 && mids <= 1.0, "mids={mids} out of [0,1]");
    assert!(highs >= 0.0 && highs <= 1.0, "highs={highs} out of [0,1]");
}

#[test]
fn fft_bands_sum_does_not_exceed_1() {
    // total_power includes all bins; band powers are a subset → sum ≤ 1.0
    let samples = sine_wave(1000.0, 44100, 4096);
    let (bass, mids, highs) = fft_bands(&samples, 44100);
    let sum = bass + mids + highs;
    assert!(sum <= 1.0 + 1e-5, "band sum={sum} exceeds 1.0");
}

#[test]
fn fft_bands_all_zero_non_negative() {
    let samples = vec![0.0f32; 512];
    let (bass, mids, highs) = fft_bands(&samples, 44100);
    assert!(bass >= 0.0);
    assert!(mids >= 0.0);
    assert!(highs >= 0.0);
}

// --- Band routing: pure tones ---

#[test]
fn fft_bands_100hz_tone_routes_to_bass() {
    // 100 Hz is within bass range (20–250 Hz)
    let samples = sine_wave(100.0, 44100, 4096);
    let (bass, mids, highs) = fft_bands(&samples, 44100);
    assert!(bass > 0.9, "bass={bass} should dominate for 100 Hz tone");
    assert!(mids < 0.1, "mids={mids} should be near-zero for 100 Hz tone");
    assert!(highs < 0.1, "highs={highs} should be near-zero for 100 Hz tone");
}

#[test]
fn fft_bands_1000hz_tone_routes_to_mids() {
    // 1000 Hz is within mids range (250–4000 Hz)
    let samples = sine_wave(1000.0, 44100, 4096);
    let (bass, mids, highs) = fft_bands(&samples, 44100);
    assert!(mids > 0.9, "mids={mids} should dominate for 1000 Hz tone");
    assert!(bass < 0.1, "bass={bass} should be near-zero for 1000 Hz tone");
    assert!(highs < 0.1, "highs={highs} should be near-zero for 1000 Hz tone");
}

#[test]
fn fft_bands_8000hz_tone_routes_to_highs() {
    // 8000 Hz is within highs range (4000–20000 Hz)
    let samples = sine_wave(8000.0, 44100, 4096);
    let (bass, mids, highs) = fft_bands(&samples, 44100);
    assert!(highs > 0.9, "highs={highs} should dominate for 8000 Hz tone");
    assert!(bass < 0.1, "bass={bass} should be near-zero for 8000 Hz tone");
    assert!(mids < 0.1, "mids={mids} should be near-zero for 8000 Hz tone");
}

// --- Return-value ordering ---

#[test]
fn fft_bands_first_element_is_bass() {
    // Drive bass band with a 100 Hz tone; first tuple element should be highest
    let samples = sine_wave(100.0, 44100, 4096);
    let (bass, mids, highs) = fft_bands(&samples, 44100);
    assert!(
        bass > mids,
        "first element (bass={bass}) should exceed second (mids={mids}) for 100 Hz tone"
    );
    assert!(
        bass > highs,
        "first element (bass={bass}) should exceed third (highs={highs}) for 100 Hz tone"
    );
}

#[test]
fn fft_bands_second_element_is_mids() {
    // Drive mids band with a 1000 Hz tone; second tuple element should be highest
    let samples = sine_wave(1000.0, 44100, 4096);
    let (bass, mids, highs) = fft_bands(&samples, 44100);
    assert!(
        mids > bass,
        "second element (mids={mids}) should exceed first (bass={bass}) for 1000 Hz tone"
    );
    assert!(
        mids > highs,
        "second element (mids={mids}) should exceed third (highs={highs}) for 1000 Hz tone"
    );
}

#[test]
fn fft_bands_third_element_is_highs() {
    // Drive highs band with an 8000 Hz tone; third tuple element should be highest
    let samples = sine_wave(8000.0, 44100, 4096);
    let (bass, mids, highs) = fft_bands(&samples, 44100);
    assert!(
        highs > bass,
        "third element (highs={highs}) should exceed first (bass={bass}) for 8000 Hz tone"
    );
    assert!(
        highs > mids,
        "third element (highs={highs}) should exceed second (mids={mids}) for 8000 Hz tone"
    );
}

// --- Sample-rate sensitivity ---

#[test]
fn fft_bands_respects_sample_rate_for_band_mapping() {
    // A 100 Hz tone at 8000 Hz sample rate is still in the bass band.
    // At 8000 Hz sample_rate with 1024 samples: bin ≈ 100*1024/8000 = 12.8 → bass range.
    let samples = sine_wave(100.0, 8000, 1024);
    let (bass, _mids, _highs) = fft_bands(&samples, 8000);
    assert!(bass > 0.9, "bass={bass} should dominate for 100 Hz at 8000 Hz sample rate");
}

// --- Larger and smaller window sizes ---

#[test]
fn fft_bands_works_with_small_window() {
    // 128-sample window — must produce valid results without panic
    let samples = sine_wave(440.0, 44100, 128);
    let (bass, mids, highs) = fft_bands(&samples, 44100);
    assert!(bass >= 0.0 && bass <= 1.0);
    assert!(mids >= 0.0 && mids <= 1.0);
    assert!(highs >= 0.0 && highs <= 1.0);
}

#[test]
fn fft_bands_works_with_large_window() {
    // 8192-sample window — must produce valid results without panic
    let samples = sine_wave(1000.0, 44100, 8192);
    let (bass, mids, highs) = fft_bands(&samples, 44100);
    assert!(bass >= 0.0 && bass <= 1.0);
    assert!(mids >= 0.0 && mids <= 1.0);
    assert!(highs >= 0.0 && highs <= 1.0);
    // 1000 Hz should still land in mids
    assert!(mids > 0.9, "mids={mids} should dominate for 1000 Hz at 8192-sample window");
}
