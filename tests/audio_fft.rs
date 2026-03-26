// @spec-tags: core,audio,analysis
// @invariants: fft_bands returns [f32; 7] with normalized energy in 7 bands (sub-bass, bass, low-mids, mids, upper-mids, presence, brilliance); edge cases return [0.0; 7]; pure tones route to correct band index; values in [0.0,1.0]
// @build: 97

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
    assert_eq!(fft_bands(&[], 44100), [0.0f32; 7]);
}

#[test]
fn fft_bands_zero_sample_rate_returns_zero() {
    let samples = vec![1.0f32; 1024];
    assert_eq!(fft_bands(&samples, 0), [0.0f32; 7]);
}

#[test]
fn fft_bands_all_zero_samples_returns_zero() {
    // total_power is zero → [0.0; 7]
    let samples = vec![0.0f32; 1024];
    assert_eq!(fft_bands(&samples, 44100), [0.0f32; 7]);
}

#[test]
fn fft_bands_does_not_panic_on_single_sample() {
    // Hann window edge case with N=1 — must not panic
    let bands = fft_bands(&[1.0f32], 44100);
    assert!(bands.iter().all(|&v| v >= 0.0 && v <= 1.0));
}

// --- Return type and signature ---

#[test]
fn fft_bands_return_type_is_7_element_array() {
    let samples = sine_wave(440.0, 44100, 4096);
    let _bands: [f32; 7] = fft_bands(&samples, 44100);
}

// --- Normalization invariants ---

#[test]
fn fft_bands_values_in_0_to_1() {
    // 440 Hz (mids) — general normalization check
    let samples = sine_wave(440.0, 44100, 4096);
    let bands = fft_bands(&samples, 44100);
    for (i, &v) in bands.iter().enumerate() {
        assert!(v >= 0.0 && v <= 1.0, "bands[{i}]={v} out of [0,1]");
    }
}

#[test]
fn fft_bands_sum_does_not_exceed_1() {
    // total_power includes all bins in [20Hz,20000Hz]; band powers are a subset → sum ≤ 1.0
    let samples = sine_wave(1000.0, 44100, 4096);
    let bands = fft_bands(&samples, 44100);
    let sum: f32 = bands.iter().sum();
    assert!(sum <= 1.0 + 1e-5, "band sum={sum} exceeds 1.0");
}

#[test]
fn fft_bands_all_zero_non_negative() {
    let samples = vec![0.0f32; 512];
    let bands = fft_bands(&samples, 44100);
    for (i, &v) in bands.iter().enumerate() {
        assert!(v >= 0.0, "bands[{i}]={v} is negative");
    }
}

// --- Band routing: pure tones ---

#[test]
fn fft_bands_100hz_tone_routes_to_bass() {
    // 100 Hz is within bass range (61–250 Hz) → index [1]
    let samples = sine_wave(100.0, 44100, 4096);
    let bands = fft_bands(&samples, 44100);
    assert!(bands[1] > 0.9, "bands[1]={} should dominate for 100 Hz tone", bands[1]);
    assert!(bands[0] < 0.1, "bands[0]={} should be near-zero for 100 Hz tone", bands[0]);
    assert!(bands[2] < 0.1, "bands[2]={} should be near-zero for 100 Hz tone", bands[2]);
    assert!(bands[3] < 0.1, "bands[3]={} should be near-zero for 100 Hz tone", bands[3]);
    assert!(bands[4] < 0.1, "bands[4]={} should be near-zero for 100 Hz tone", bands[4]);
    assert!(bands[5] < 0.1, "bands[5]={} should be near-zero for 100 Hz tone", bands[5]);
    assert!(bands[6] < 0.1, "bands[6]={} should be near-zero for 100 Hz tone", bands[6]);
}

#[test]
fn fft_bands_1000hz_tone_routes_to_mids() {
    // 1000 Hz is within mids range (501–2000 Hz) → index [3]
    let samples = sine_wave(1000.0, 44100, 4096);
    let bands = fft_bands(&samples, 44100);
    assert!(bands[3] > 0.9, "bands[3]={} should dominate for 1000 Hz tone", bands[3]);
    assert!(bands[0] < 0.1, "bands[0]={} should be near-zero for 1000 Hz tone", bands[0]);
    assert!(bands[1] < 0.1, "bands[1]={} should be near-zero for 1000 Hz tone", bands[1]);
    assert!(bands[2] < 0.1, "bands[2]={} should be near-zero for 1000 Hz tone", bands[2]);
    assert!(bands[4] < 0.1, "bands[4]={} should be near-zero for 1000 Hz tone", bands[4]);
    assert!(bands[5] < 0.1, "bands[5]={} should be near-zero for 1000 Hz tone", bands[5]);
    assert!(bands[6] < 0.1, "bands[6]={} should be near-zero for 1000 Hz tone", bands[6]);
}

#[test]
fn fft_bands_8000hz_tone_routes_to_presence() {
    // 8000 Hz is at the top of the presence range (4001–8000 Hz) → index [5]
    let samples = sine_wave(8000.0, 44100, 4096);
    let bands = fft_bands(&samples, 44100);
    assert!(bands[5] > 0.9, "bands[5]={} should dominate for 8000 Hz tone", bands[5]);
    assert!(bands[0] < 0.1, "bands[0]={} should be near-zero for 8000 Hz tone", bands[0]);
    assert!(bands[1] < 0.1, "bands[1]={} should be near-zero for 8000 Hz tone", bands[1]);
    assert!(bands[2] < 0.1, "bands[2]={} should be near-zero for 8000 Hz tone", bands[2]);
    assert!(bands[3] < 0.1, "bands[3]={} should be near-zero for 8000 Hz tone", bands[3]);
    assert!(bands[4] < 0.1, "bands[4]={} should be near-zero for 8000 Hz tone", bands[4]);
    assert!(bands[6] < 0.1, "bands[6]={} should be near-zero for 8000 Hz tone", bands[6]);
}

// --- New band routing tests ---

#[test]
fn fft_bands_sub_bass_band_routing() {
    // 40 Hz is within sub-bass (20–60 Hz) → index [0]
    let samples = sine_wave(40.0, 44100, 4096);
    let bands = fft_bands(&samples, 44100);
    assert!(bands[0] > 0.5, "bands[0]={} should dominate for 40 Hz tone (sub-bass is a narrow band)", bands[0]);
}

#[test]
fn fft_bands_low_mids_band_routing() {
    // 400 Hz is within low-mids (251–500 Hz) → index [2]
    let samples = sine_wave(400.0, 44100, 4096);
    let bands = fft_bands(&samples, 44100);
    assert!(bands[2] > 0.9, "bands[2]={} should dominate for 400 Hz tone", bands[2]);
}

#[test]
fn fft_bands_upper_mids_band_routing() {
    // 3000 Hz is within upper-mids (2001–4000 Hz) → index [4]
    let samples = sine_wave(3000.0, 44100, 4096);
    let bands = fft_bands(&samples, 44100);
    assert!(bands[4] > 0.9, "bands[4]={} should dominate for 3000 Hz tone", bands[4]);
}

#[test]
fn fft_bands_presence_band_routing() {
    // 6000 Hz is within presence (4001–8000 Hz) → index [5]
    let samples = sine_wave(6000.0, 44100, 4096);
    let bands = fft_bands(&samples, 44100);
    assert!(bands[5] > 0.9, "bands[5]={} should dominate for 6000 Hz tone", bands[5]);
}

#[test]
fn fft_bands_brilliance_band_routing() {
    // 12000 Hz is within brilliance (8001–20000 Hz) → index [6]
    let samples = sine_wave(12000.0, 44100, 4096);
    let bands = fft_bands(&samples, 44100);
    assert!(bands[6] > 0.9, "bands[6]={} should dominate for 12000 Hz tone", bands[6]);
}

#[test]
fn fft_bands_array_index_order_is_frequency_ascending() {
    // 40 Hz (sub-bass) → index [0] should dominate
    let low_samples = sine_wave(40.0, 44100, 4096);
    let low_bands = fft_bands(&low_samples, 44100);
    assert!(
        low_bands[0] > low_bands[1]
            && low_bands[0] > low_bands[2]
            && low_bands[0] > low_bands[3]
            && low_bands[0] > low_bands[4]
            && low_bands[0] > low_bands[5]
            && low_bands[0] > low_bands[6],
        "index [0] should be dominant for 40 Hz tone: {:?}", low_bands
    );
    // 12000 Hz (brilliance) → index [6] should dominate
    let high_samples = sine_wave(12000.0, 44100, 4096);
    let high_bands = fft_bands(&high_samples, 44100);
    assert!(
        high_bands[6] > high_bands[0]
            && high_bands[6] > high_bands[1]
            && high_bands[6] > high_bands[2]
            && high_bands[6] > high_bands[3]
            && high_bands[6] > high_bands[4]
            && high_bands[6] > high_bands[5],
        "index [6] should be dominant for 12000 Hz tone: {:?}", high_bands
    );
}

// --- Sample-rate sensitivity ---

#[test]
fn fft_bands_respects_sample_rate_for_band_mapping() {
    // A 100 Hz tone at 8000 Hz sample rate is still in the bass band (61–250 Hz) → index [1].
    // At 8000 Hz sample_rate with 1024 samples: bin ≈ 100*1024/8000 = 12.8 → bass range.
    let samples = sine_wave(100.0, 8000, 1024);
    let bands = fft_bands(&samples, 8000);
    assert!(bands[1] > 0.9, "bands[1]={} should dominate for 100 Hz at 8000 Hz sample rate", bands[1]);
}

// --- Larger and smaller window sizes ---

#[test]
fn fft_bands_works_with_small_window() {
    // 128-sample window — must produce valid results without panic
    let samples = sine_wave(440.0, 44100, 128);
    let bands = fft_bands(&samples, 44100);
    for (i, &v) in bands.iter().enumerate() {
        assert!(v >= 0.0 && v <= 1.0, "bands[{i}]={v} out of [0,1]");
    }
}

#[test]
fn fft_bands_works_with_large_window() {
    // 8192-sample window — must produce valid results without panic
    let samples = sine_wave(1000.0, 44100, 8192);
    let bands = fft_bands(&samples, 44100);
    for (i, &v) in bands.iter().enumerate() {
        assert!(v >= 0.0 && v <= 1.0, "bands[{i}]={v} out of [0,1]");
    }
    // 1000 Hz should still land in mids → index [3]
    assert!(bands[3] > 0.9, "bands[3]={} should dominate for 1000 Hz at 8192-sample window", bands[3]);
}
