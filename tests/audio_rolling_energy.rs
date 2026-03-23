// @spec-tags: core,audio,analysis
// @invariants: RollingEnergy tracks a circular buffer of window_secs * frames_per_sec frames; averages() returns rolling mean per band; dominant_bands() returns top-N band indices sorted descending by average; newly created tracker returns [0.0;7] averages and [0,1,...] dominant_bands; feeding constant data produces correct averages
// @build: 91

use rhythm_grid::audio::RollingEnergy;

// --- Construction ---

#[test]
fn rolling_energy_new_creates_instance() {
    let _re = RollingEnergy::new(5.0, 60.0);
}

#[test]
fn rolling_energy_derives_debug() {
    let re = RollingEnergy::new(5.0, 60.0);
    let s = format!("{:?}", re);
    assert!(!s.is_empty());
}

// --- Fresh tracker: averages() ---

#[test]
fn averages_returns_seven_zeros_on_new_tracker() {
    let re = RollingEnergy::new(5.0, 60.0);
    let avgs = re.averages();
    assert_eq!(avgs, [0.0_f32; 7], "fresh tracker must return [0.0; 7] averages");
}

#[test]
fn averages_returns_array_of_seven() {
    let re = RollingEnergy::new(5.0, 60.0);
    let avgs = re.averages();
    assert_eq!(avgs.len(), 7);
}

// --- Fresh tracker: dominant_bands() ---

#[test]
fn dominant_bands_returns_first_n_indices_when_all_zero() {
    let re = RollingEnergy::new(5.0, 60.0);
    let dominant = re.dominant_bands(3);
    assert_eq!(dominant, vec![0usize, 1, 2],
        "when all averages are 0.0, dominant_bands(3) must return [0,1,2]");
}

#[test]
fn dominant_bands_count_1_returns_index_0_when_all_zero() {
    let re = RollingEnergy::new(5.0, 60.0);
    let dominant = re.dominant_bands(1);
    assert_eq!(dominant, vec![0usize]);
}

#[test]
fn dominant_bands_count_7_returns_all_indices_when_all_zero() {
    let re = RollingEnergy::new(5.0, 60.0);
    let dominant = re.dominant_bands(7);
    assert_eq!(dominant, vec![0usize, 1, 2, 3, 4, 5, 6]);
}

#[test]
fn dominant_bands_count_exceeds_7_returns_all_7_indices() {
    let re = RollingEnergy::new(5.0, 60.0);
    let dominant = re.dominant_bands(10);
    assert_eq!(dominant.len(), 7);
}

#[test]
fn dominant_bands_count_0_returns_empty() {
    let re = RollingEnergy::new(5.0, 60.0);
    let dominant = re.dominant_bands(0);
    assert!(dominant.is_empty());
}

// --- update() and averages() ---

#[test]
fn averages_band_0_equals_1_after_feeding_constant_ones_to_band_0() {
    // Feed 100 frames with band 0 = 1.0, others = 0.0
    let mut re = RollingEnergy::new(5.0, 60.0);
    let bands = [1.0_f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    for _ in 0..100 {
        re.update(&bands);
    }
    let avgs = re.averages();
    assert!(
        (avgs[0] - 1.0).abs() < 1e-4,
        "after 100 frames of [1,0,0,0,0,0,0], band 0 average should be ~1.0, got {}",
        avgs[0]
    );
}

#[test]
fn averages_other_bands_zero_after_feeding_ones_only_to_band_0() {
    let mut re = RollingEnergy::new(5.0, 60.0);
    let bands = [1.0_f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    for _ in 0..100 {
        re.update(&bands);
    }
    let avgs = re.averages();
    for i in 1..7 {
        assert!(
            avgs[i].abs() < 1e-6,
            "band {} should be 0.0 after feeding zeros, got {}",
            i, avgs[i]
        );
    }
}

#[test]
fn averages_all_bands_equal_0_5_after_constant_half_energy() {
    let mut re = RollingEnergy::new(2.0, 60.0);
    let bands = [0.5_f32; 7];
    for _ in 0..200 {
        re.update(&bands);
    }
    let avgs = re.averages();
    for i in 0..7 {
        assert!(
            (avgs[i] - 0.5).abs() < 1e-4,
            "band {} should converge to 0.5, got {}",
            i, avgs[i]
        );
    }
}

// --- Circular buffer: window_secs * frames_per_sec ---

#[test]
fn average_reflects_window_duration() {
    // Window = 2s at 60fps = 120 slots. Feed 120 frames of 1.0 then 120 frames of 0.0.
    // After the second 120 frames, old 1.0 values are pushed out — average should be ~0.0.
    let window_secs = 2.0_f32;
    let fps = 60.0_f32;
    let window_frames = (window_secs * fps) as usize; // 120

    let mut re = RollingEnergy::new(window_secs, fps);
    let ones = [1.0_f32; 7];
    let zeros = [0.0_f32; 7];

    for _ in 0..window_frames {
        re.update(&ones);
    }
    let avgs_after_ones = re.averages();
    for i in 0..7 {
        assert!(
            (avgs_after_ones[i] - 1.0).abs() < 1e-4,
            "average should be ~1.0 after filling window, band {}: got {}",
            i, avgs_after_ones[i]
        );
    }

    for _ in 0..window_frames {
        re.update(&zeros);
    }
    let avgs_after_zeros = re.averages();
    for i in 0..7 {
        assert!(
            avgs_after_zeros[i].abs() < 1e-4,
            "average should be ~0.0 after replacing window with zeros, band {}: got {}",
            i, avgs_after_zeros[i]
        );
    }
}

#[test]
fn average_decays_toward_zero_after_spike() {
    // Feed spike then zeros — average should drop after window fills with zeros
    let mut re = RollingEnergy::new(1.0, 60.0); // 60-frame window
    let spike = [1.0_f32; 7];
    let zeros = [0.0_f32; 7];

    // Fill with spike
    for _ in 0..60 {
        re.update(&spike);
    }
    let avg_after_spike = re.averages()[0];
    assert!((avg_after_spike - 1.0).abs() < 1e-4, "should be ~1.0 after spike fill");

    // Feed zeros for full window
    for _ in 0..60 {
        re.update(&zeros);
    }
    let avg_after_decay = re.averages()[0];
    assert!(
        avg_after_decay < 0.01,
        "average should decay toward 0 after feeding zeros for full window, got {}",
        avg_after_decay
    );
}

// --- dominant_bands() with non-zero data ---

#[test]
fn dominant_bands_returns_band_0_first_when_band_0_has_highest_energy() {
    let mut re = RollingEnergy::new(5.0, 60.0);
    let bands = [1.0_f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    for _ in 0..100 {
        re.update(&bands);
    }
    let dominant = re.dominant_bands(1);
    assert_eq!(dominant, vec![0usize], "band 0 should dominate when it has highest energy");
}

#[test]
fn dominant_bands_sorted_descending_by_energy() {
    // Band 3 has highest energy, band 1 second, band 5 third
    let mut re = RollingEnergy::new(5.0, 60.0);
    for _ in 0..100 {
        re.update(&[0.2_f32, 0.5, 0.1, 1.0, 0.0, 0.3, 0.0]);
    }
    let dominant = re.dominant_bands(3);
    assert_eq!(dominant.len(), 3);
    // Band 3 (1.0) > band 1 (0.5) > band 5 (0.3)
    assert_eq!(dominant[0], 3, "highest energy band should be first");
    assert_eq!(dominant[1], 1, "second highest energy band should be second");
    assert_eq!(dominant[2], 5, "third highest energy band should be third");
}

#[test]
fn dominant_bands_count_2_returns_top_2() {
    let mut re = RollingEnergy::new(5.0, 60.0);
    for _ in 0..100 {
        re.update(&[0.0_f32, 0.0, 0.0, 0.0, 0.8, 0.0, 0.9]);
    }
    let dominant = re.dominant_bands(2);
    assert_eq!(dominant.len(), 2);
    assert_eq!(dominant[0], 6, "band 6 (0.9) should be first");
    assert_eq!(dominant[1], 4, "band 4 (0.8) should be second");
}

#[test]
fn dominant_bands_all_7_contains_all_indices() {
    let mut re = RollingEnergy::new(5.0, 60.0);
    for _ in 0..100 {
        re.update(&[0.1_f32, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7]);
    }
    let dominant = re.dominant_bands(7);
    assert_eq!(dominant.len(), 7);
    let mut sorted = dominant.clone();
    sorted.sort();
    assert_eq!(sorted, vec![0usize, 1, 2, 3, 4, 5, 6], "all 7 indices must be present");
}

// --- Window size: minimum 1 slot ---

#[test]
fn rolling_energy_with_tiny_window_does_not_panic() {
    // window_secs * frames_per_sec = 0.001 * 1.0 = 0.001, truncated to 0, clamped to minimum 1
    let mut re = RollingEnergy::new(0.001, 1.0);
    re.update(&[0.5_f32; 7]);
    let avgs = re.averages();
    assert_eq!(avgs.len(), 7);
}

#[test]
fn rolling_energy_update_many_times_does_not_panic() {
    let mut re = RollingEnergy::new(5.0, 60.0);
    for i in 0..1000 {
        let v = (i % 7) as f32 / 7.0;
        re.update(&[v; 7]);
    }
    let avgs = re.averages();
    for avg in &avgs {
        assert!(*avg >= 0.0 && *avg <= 1.0, "averages must be in [0.0, 1.0]");
    }
}
