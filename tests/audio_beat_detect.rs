// @spec-tags: core,audio,analysis
// @invariants: BeatDetector: >= 0.3s gap; MultiBeatDetector: configurable, default 0.15s gap; 43-sample rolling window; 1.5x spike threshold; BeatEvent and BandBeatEvent carry correct timestamps; MultiBeatDetector holds 7 independent BandDetectorState structs
// @build: 92

use rhythm_grid::audio::{BeatDetector, BeatEvent, MultiBeatDetector, BandBeatEvent};

// --- Struct derives ---

#[test]
fn beat_event_derives_partialeq() {
    let a = BeatEvent { timestamp_secs: 1.0 };
    let b = BeatEvent { timestamp_secs: 1.0 };
    assert_eq!(a, b);
}

#[test]
fn beat_event_derives_clone() {
    let a = BeatEvent { timestamp_secs: 2.5 };
    let b = a.clone();
    assert_eq!(a.timestamp_secs, b.timestamp_secs);
}

#[test]
fn beat_event_derives_debug() {
    let a = BeatEvent { timestamp_secs: 0.5 };
    let s = format!("{:?}", a);
    assert!(s.contains("BeatEvent") || s.contains("timestamp_secs"));
}

#[test]
fn beat_event_timestamp_field_is_accessible() {
    let e = BeatEvent { timestamp_secs: 3.14 };
    assert!((e.timestamp_secs - 3.14).abs() < 1e-9);
}

// --- BeatDetector construction ---

#[test]
fn beat_detector_new_creates_instance() {
    let _detector = BeatDetector::new();
    // Construction must not panic.
}

// --- No beat when amplitude is below threshold ---

/// Warm up the detector with `n` samples of the given amplitude, feeding incrementing timestamps.
fn warm_up(detector: &mut BeatDetector, amplitude: f32, n: usize, start_ts: f64, step: f64) -> f64 {
    let mut ts = start_ts;
    for _ in 0..n {
        detector.detect(amplitude, ts);
        ts += step;
    }
    ts
}

#[test]
fn detect_no_beat_when_below_threshold() {
    let mut detector = BeatDetector::new();
    // Warm up window: 43 samples at 0.4 amplitude. Mean = 0.4, threshold = 0.6.
    let ts = warm_up(&mut detector, 0.4, 43, 0.0, 0.01);
    // Feed 0.5 → 0.5 < 0.4*1.5 = 0.6 → no beat.
    let result = detector.detect(0.5, ts);
    assert_eq!(result, None);
}

#[test]
fn detect_no_beat_when_exactly_at_threshold() {
    let mut detector = BeatDetector::new();
    // Mean = 0.4, threshold = 0.6. Feed exactly 0.6 → 0.6 is NOT > 0.6 → no beat.
    let ts = warm_up(&mut detector, 0.4, 43, 0.0, 0.01);
    let result = detector.detect(0.6, ts);
    assert_eq!(result, None);
}

#[test]
fn detect_no_beat_when_amplitude_zero() {
    let mut detector = BeatDetector::new();
    // Any mean * 1.5: 0.0 is not > threshold.
    let result = detector.detect(0.0, 0.0);
    assert_eq!(result, None);
}

// --- Beat detected when amplitude exceeds threshold ---

#[test]
fn detect_returns_beat_above_threshold() {
    let mut detector = BeatDetector::new();
    // Mean = 0.4, threshold = 0.6. Feed 0.7 → 0.7 > 0.6 → beat.
    let ts = warm_up(&mut detector, 0.4, 43, 0.0, 0.01);
    let result = detector.detect(0.7, ts);
    assert!(result.is_some(), "Expected a beat event, got None");
}

#[test]
fn detect_beat_event_has_correct_timestamp() {
    let mut detector = BeatDetector::new();
    let ts = warm_up(&mut detector, 0.4, 43, 0.0, 0.01);
    let beat_ts = ts + 1.0; // use a distinct timestamp
    let result = detector.detect(0.7, beat_ts);
    let event = result.expect("Expected beat event");
    assert!((event.timestamp_secs - beat_ts).abs() < 1e-9);
}

#[test]
fn detect_beat_just_above_threshold_triggers() {
    let mut detector = BeatDetector::new();
    // Mean = 0.4, threshold = 0.6. 0.61 > 0.6 → beat.
    let ts = warm_up(&mut detector, 0.4, 43, 0.0, 0.01);
    let result = detector.detect(0.61, ts);
    assert!(result.is_some());
}

// --- Inter-beat gap enforcement (minimum 0.3s) ---

#[test]
fn inter_beat_gap_suppresses_beat_before_0_3s() {
    let mut detector = BeatDetector::new();
    let ts = warm_up(&mut detector, 0.4, 43, 0.0, 0.01);
    // First beat at ts.
    let first = detector.detect(0.7, ts);
    assert!(first.is_some(), "First beat should fire");

    // Re-warm window for a clean mean (the spike entered the window).
    // Feed more 0.4 samples to restore the mean.
    let ts2 = warm_up(&mut detector, 0.4, 43, ts + 0.001, 0.001);

    // Second spike at ts + 0.2s — less than 0.3s gap.
    let second_ts = ts + 0.2;
    let second = detector.detect(0.7, second_ts);
    assert_eq!(second, None, "Beat within 0.3s gap should be suppressed");

    // Feed a large amplitude again ensuring ts2 > ts + 0.3 to confirm gap logic.
    // We need to ensure we're beyond 0.3s from the first beat.
    // ts2 = ts + 43*0.001 = ts + 0.043; still < 0.3s from ts.
    let _ = ts2;
}

#[test]
fn inter_beat_gap_allows_beat_after_0_3s() {
    let mut detector = BeatDetector::new();
    let ts = warm_up(&mut detector, 0.4, 43, 0.0, 0.01);

    // First beat.
    let first = detector.detect(0.7, ts);
    assert!(first.is_some());

    // Warm up window again before second spike (need mean to be back at ~0.4).
    let ts2 = warm_up(&mut detector, 0.4, 43, ts + 0.001, 0.001);
    let _ = ts2;

    // Second spike at ts + 0.31s — just beyond the 0.3s gap.
    let second_ts = ts + 0.31;
    let second = detector.detect(0.7, second_ts);
    assert!(second.is_some(), "Beat after 0.3s gap should fire");
}

#[test]
fn inter_beat_gap_beat_exactly_at_0_3s_fires() {
    let mut detector = BeatDetector::new();
    let ts = warm_up(&mut detector, 0.4, 43, 0.0, 0.01);

    let first = detector.detect(0.7, ts);
    assert!(first.is_some());

    // Warm up window so mean returns to ~0.4.
    warm_up(&mut detector, 0.4, 43, ts + 0.001, 0.001);

    // At exactly 0.3s, the gap condition is "at least 0.3s", so it should fire.
    let second_ts = ts + 0.3;
    let second = detector.detect(0.7, second_ts);
    assert!(second.is_some(), "Beat at exactly 0.3s gap should fire");
}

// --- Rolling window behavior ---

#[test]
fn detect_rolling_window_is_43_samples() {
    // After 43 samples of 0.4 and 43 more samples of 0.0, the mean should be ~0.0.
    // A high amplitude should then trigger against the near-zero mean.
    let mut detector = BeatDetector::new();
    // Fill with high values then replace with zeros.
    warm_up(&mut detector, 1.0, 43, 0.0, 0.01);
    // Now feed 43 zeros — these replace the old window completely.
    let ts = warm_up(&mut detector, 0.0, 43, 0.43, 0.31); // step > 0.3 to not suppress beats
    // Mean is now ~0.0, any amplitude > 0.0 should trigger.
    let result = detector.detect(0.01, ts);
    assert!(result.is_some(), "With near-zero mean, any spike > 0 should trigger");
}

// ============================================================
// MultiBeatDetector / BandBeatEvent tests
// ============================================================

fn warm_up_multi(detector: &mut MultiBeatDetector, bands: &[f32; 7], n: usize, start_ts: f64, step: f64) -> f64 {
    let mut ts = start_ts;
    for _ in 0..n {
        detector.detect_bands(bands, ts);
        ts += step;
    }
    ts
}

// --- BandBeatEvent struct derives ---

#[test]
fn band_beat_event_derives_partialeq() {
    let a = BandBeatEvent { band: 0, timestamp_secs: 1.0 };
    let b = BandBeatEvent { band: 0, timestamp_secs: 1.0 };
    assert_eq!(a, b);
}

#[test]
fn band_beat_event_derives_clone() {
    let a = BandBeatEvent { band: 3, timestamp_secs: 2.5 };
    let b = a.clone();
    assert_eq!(b.band, 3);
    assert!((b.timestamp_secs - 2.5).abs() < 1e-9);
}

#[test]
fn band_beat_event_derives_debug() {
    let a = BandBeatEvent { band: 1, timestamp_secs: 0.5 };
    let s = format!("{:?}", a);
    assert!(s.contains("BandBeatEvent") || s.contains("band"));
}

#[test]
fn band_beat_event_fields_accessible() {
    let e = BandBeatEvent { band: 6, timestamp_secs: 3.14 };
    assert_eq!(e.band, 6);
    assert!((e.timestamp_secs - 3.14).abs() < 1e-9);
}

// --- MultiBeatDetector construction ---

#[test]
fn multi_beat_detector_new_creates_instance() {
    let _d = MultiBeatDetector::new();
}

// --- detect_bands basic behavior ---

#[test]
fn detect_bands_no_beats_on_flat_input() {
    let mut detector = MultiBeatDetector::new();
    // Warm up 50 frames of 0.4 — mean = 0.4, threshold = 0.6.
    let mut ts = 0.0_f64;
    for _ in 0..50 {
        detector.detect_bands(&[0.4; 7], ts);
        ts += 0.01;
    }
    // 0.5 < 0.4 * 1.5 = 0.6 → no beat.
    let result = detector.detect_bands(&[0.5; 7], ts);
    assert!(result.is_empty(), "Expected no beats, got {:?}", result);
}

#[test]
fn detect_bands_returns_empty_vec_on_zero_input() {
    let mut detector = MultiBeatDetector::new();
    let result = detector.detect_bands(&[0.0; 7], 0.0);
    assert!(result.is_empty());
}

// --- Spike detection per band ---

#[test]
fn detect_bands_spike_in_single_band() {
    let mut detector = MultiBeatDetector::new();
    // Warm up 43 frames of 0.4 for all bands. Mean = 0.4, threshold = 0.6.
    let ts = warm_up_multi(&mut detector, &[0.4; 7], 43, 0.0, 0.01);
    // Band 3 spikes to 0.7 (> 0.6), others stay at 0.4 (< 0.6).
    let input = [0.4, 0.4, 0.4, 0.7, 0.4, 0.4, 0.4];
    let events = detector.detect_bands(&input, ts);
    assert_eq!(events.len(), 1, "Expected exactly 1 event, got {:?}", events);
    assert_eq!(events[0].band, 3);
}

#[test]
fn detect_bands_spike_in_multiple_bands() {
    let mut detector = MultiBeatDetector::new();
    let ts = warm_up_multi(&mut detector, &[0.4; 7], 43, 0.0, 0.01);
    // Bands 0, 2, 6 spike.
    let input = [0.7, 0.4, 0.7, 0.4, 0.4, 0.4, 0.7];
    let events = detector.detect_bands(&input, ts);
    assert_eq!(events.len(), 3, "Expected 3 events, got {:?}", events);
    let bands: std::collections::HashSet<usize> = events.iter().map(|e| e.band).collect();
    assert!(bands.contains(&0));
    assert!(bands.contains(&2));
    assert!(bands.contains(&6));
}

#[test]
fn detect_bands_all_bands_spike() {
    let mut detector = MultiBeatDetector::new();
    let ts = warm_up_multi(&mut detector, &[0.4; 7], 43, 0.0, 0.01);
    let events = detector.detect_bands(&[0.7; 7], ts);
    assert_eq!(events.len(), 7, "Expected 7 events, got {:?}", events);
    for i in 0..7 {
        assert_eq!(events[i].band, i);
    }
}

#[test]
fn detect_bands_beat_event_has_correct_timestamp() {
    let mut detector = MultiBeatDetector::new();
    let _ = warm_up_multi(&mut detector, &[0.4; 7], 43, 0.0, 0.01);
    let events = detector.detect_bands(&[0.7; 7], 5.0);
    assert_eq!(events.len(), 7);
    for e in &events {
        assert!((e.timestamp_secs - 5.0).abs() < 1e-9, "Expected ts=5.0, got {}", e.timestamp_secs);
    }
}

// --- Per-band inter-beat gap (0.15s default minimum) ---

#[test]
fn detect_bands_gap_suppresses_within_0_15s() {
    let mut detector = MultiBeatDetector::new();
    // Warm up 43 frames at timestamps 0.0..0.42 (step 0.01).
    let ts = warm_up_multi(&mut detector, &[0.4; 7], 43, 0.0, 0.01); // ts = 0.43
    // Spike band 0 at t=0.43.
    let first = detector.detect_bands(&[0.7, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4], ts);
    assert!(!first.is_empty() && first.iter().any(|e| e.band == 0), "Band 0 should fire first");
    // Re-warm 43 frames to restore the mean.
    warm_up_multi(&mut detector, &[0.4; 7], 43, ts + 0.001, 0.001);
    // Second spike at t=0.43 + 0.1 = 0.53 — only 0.1s elapsed for band 0 → suppressed (< 0.15s).
    let second = detector.detect_bands(&[0.7, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4], ts + 0.1);
    assert!(!second.iter().any(|e| e.band == 0), "Band 0 should be suppressed at 0.1s gap (< 0.15s)");
}

#[test]
fn detect_bands_gap_allows_after_0_15s() {
    let mut detector = MultiBeatDetector::new();
    let ts = warm_up_multi(&mut detector, &[0.4; 7], 43, 0.0, 0.01); // ts = 0.43
    let first = detector.detect_bands(&[0.7, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4], ts);
    assert!(first.iter().any(|e| e.band == 0));
    warm_up_multi(&mut detector, &[0.4; 7], 43, ts + 0.001, 0.001);
    // 0.16s > 0.15s → should fire.
    let second = detector.detect_bands(&[0.7, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4], ts + 0.16);
    assert!(second.iter().any(|e| e.band == 0), "Band 0 should fire after 0.16s gap (> 0.15s)");
}

#[test]
fn detect_bands_gap_fires_at_exactly_0_15s() {
    let mut detector = MultiBeatDetector::new();
    let ts = warm_up_multi(&mut detector, &[0.4; 7], 43, 0.0, 0.01); // ts = 0.43
    let first = detector.detect_bands(&[0.7, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4], ts);
    assert!(first.iter().any(|e| e.band == 0));
    warm_up_multi(&mut detector, &[0.4; 7], 43, ts + 0.001, 0.001);
    // Exactly 0.15s → should fire (>= 0.15s condition).
    let second = detector.detect_bands(&[0.7, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4], ts + 0.15);
    assert!(second.iter().any(|e| e.band == 0), "Band 0 should fire at exactly 0.15s gap");
}

// --- Band independence ---

#[test]
fn detect_bands_independent_rolling_means() {
    let mut detector = MultiBeatDetector::new();
    // Band 0 warmed with 0.8, bands 1-6 with 0.2.
    let warmup = [0.8_f32, 0.2, 0.2, 0.2, 0.2, 0.2, 0.2];
    let ts = warm_up_multi(&mut detector, &warmup, 43, 0.0, 0.01);
    // Band 0: threshold = 0.8 * 1.5 = 1.2; feed 0.5 → no beat.
    // Band 1: threshold = 0.2 * 1.5 = 0.3; feed 0.5 → beat.
    let input = [0.5_f32, 0.5, 0.2, 0.2, 0.2, 0.2, 0.2];
    let events = detector.detect_bands(&input, ts);
    assert!(!events.iter().any(|e| e.band == 0), "Band 0 should NOT fire");
    assert!(events.iter().any(|e| e.band == 1), "Band 1 should fire");
}

#[test]
fn detect_bands_independent_gap_timers() {
    let mut detector = MultiBeatDetector::new();
    // Warm up 43 frames of 0.4 at 0.01s intervals; last frame at t=0.42, next ts=0.43.
    let _ = warm_up_multi(&mut detector, &[0.4; 7], 43, 0.0, 0.01);

    // Helper to feed a single band as spike, others at 0.4.
    let spike_band = |det: &mut MultiBeatDetector, idx: usize, ts: f64| -> Vec<BandBeatEvent> {
        let mut arr = [0.4_f32; 7];
        arr[idx] = 0.7;
        det.detect_bands(&arr, ts)
    };
    let flat = |det: &mut MultiBeatDetector, ts: f64| {
        det.detect_bands(&[0.4; 7], ts);
    };

    // t=1.0: spike band 0 → fires.
    let e1 = spike_band(&mut detector, 0, 1.0);
    assert!(e1.iter().any(|e| e.band == 0), "Band 0 should fire at t=1.0");

    flat(&mut detector, 1.05);

    // t=1.1: spike band 0 → suppressed (0.1s < 0.15s default gap).
    let e2 = spike_band(&mut detector, 0, 1.1);
    assert!(!e2.iter().any(|e| e.band == 0), "Band 0 should be suppressed at t=1.1 (0.1s < 0.15s gap)");

    flat(&mut detector, 1.12);

    // t=1.16: spike band 0 → fires (0.16s > 0.15s default gap).
    let e3 = spike_band(&mut detector, 0, 1.16);
    assert!(e3.iter().any(|e| e.band == 0), "Band 0 should fire at t=1.16 (0.16s > 0.15s gap)");

    flat(&mut detector, 1.2);

    // t=1.3: spike band 1 → fires (no prior beat for band 1).
    let e4 = spike_band(&mut detector, 1, 1.3);
    assert!(e4.iter().any(|e| e.band == 1), "Band 1 should fire at t=1.3");

    flat(&mut detector, 1.5);
    flat(&mut detector, 1.6);

    // t=1.8: spike band 1 → fires (0.5s since t=1.3).
    let e5 = spike_band(&mut detector, 1, 1.8);
    assert!(e5.iter().any(|e| e.band == 1), "Band 1 should fire at t=1.8 (0.5s since last beat)");
}

#[test]
fn detect_bands_two_bands_different_times() {
    let mut detector = MultiBeatDetector::new();
    let ts = warm_up_multi(&mut detector, &[0.4; 7], 43, 0.0, 0.01);

    // t=1.0: spike band 2 only.
    let e1 = detector.detect_bands(&[0.4, 0.4, 0.7, 0.4, 0.4, 0.4, 0.4], 1.0);
    assert_eq!(e1.len(), 1);
    assert_eq!(e1[0].band, 2);
    assert!((e1[0].timestamp_secs - 1.0).abs() < 1e-9);

    // feed some flat frames to keep mean stable.
    warm_up_multi(&mut detector, &[0.4; 7], 5, ts + 0.01, 0.01);

    // t=1.5: spike band 5 only.
    let e2 = detector.detect_bands(&[0.4, 0.4, 0.4, 0.4, 0.4, 0.7, 0.4], 1.5);
    assert_eq!(e2.len(), 1);
    assert_eq!(e2[0].band, 5);
    assert!((e2[0].timestamp_secs - 1.5).abs() < 1e-9);
}

// --- Rolling window per band ---

#[test]
fn detect_bands_43_sample_window_per_band() {
    let mut detector = MultiBeatDetector::new();
    // Fill window with 1.0, then replace with 0.0 — mean should approach 0.0.
    warm_up_multi(&mut detector, &[1.0; 7], 43, 0.0, 0.01);
    // Feed 43 zeros, spacing > 0.3s to avoid inter-beat suppression.
    warm_up_multi(&mut detector, &[0.0; 7], 43, 0.43, 0.31);
    // Mean ~0.0; any value > 0 should be spike.
    let ts = 0.43 + 43.0 * 0.31;
    let events = detector.detect_bands(&[0.01; 7], ts);
    assert_eq!(events.len(), 7, "All 7 bands should spike against near-zero mean, got {:?}", events);
}

// --- MultiBeatDetector::with_min_gap constructor ---

#[test]
fn multi_beat_detector_with_min_gap_creates_instance() {
    let _d = MultiBeatDetector::with_min_gap(0.1);
    // Must not panic.
}

#[test]
fn multi_beat_detector_with_min_gap_allows_beats_at_custom_gap() {
    let mut detector = MultiBeatDetector::with_min_gap(0.1);
    // Warm up 43 frames at 0.4; mean = 0.4, threshold = 0.6.
    let ts = warm_up_multi(&mut detector, &[0.4; 7], 43, 0.0, 0.01);
    // Spike band 0 at ts.
    let first = detector.detect_bands(&[0.7, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4], ts);
    assert!(first.iter().any(|e| e.band == 0), "Band 0 should fire on first spike");
    // Re-warm to restore mean.
    warm_up_multi(&mut detector, &[0.4; 7], 43, ts + 0.001, 0.001);
    // Spike at ts + 0.11 → 0.11 > 0.1s custom gap → should fire.
    let second = detector.detect_bands(&[0.7, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4], ts + 0.11);
    assert!(second.iter().any(|e| e.band == 0), "Band 0 should fire at 0.11s gap (> 0.1s custom gap)");
}

#[test]
fn multi_beat_detector_with_min_gap_suppresses_within_custom_gap() {
    let mut detector = MultiBeatDetector::with_min_gap(0.1);
    let ts = warm_up_multi(&mut detector, &[0.4; 7], 43, 0.0, 0.01);
    let first = detector.detect_bands(&[0.7, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4], ts);
    assert!(first.iter().any(|e| e.band == 0), "Band 0 should fire on first spike");
    warm_up_multi(&mut detector, &[0.4; 7], 43, ts + 0.001, 0.001);
    // Spike at ts + 0.05 → 0.05 < 0.1s custom gap → suppressed.
    let second = detector.detect_bands(&[0.7, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4], ts + 0.05);
    assert!(!second.iter().any(|e| e.band == 0), "Band 0 should be suppressed at 0.05s gap (< 0.1s custom gap)");
}

#[test]
fn multi_beat_detector_with_min_gap_large_gap() {
    let mut detector = MultiBeatDetector::with_min_gap(1.0);
    let ts = warm_up_multi(&mut detector, &[0.4; 7], 43, 0.0, 0.01);
    let first = detector.detect_bands(&[0.7, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4], ts);
    assert!(first.iter().any(|e| e.band == 0), "Band 0 should fire on first spike");
    warm_up_multi(&mut detector, &[0.4; 7], 43, ts + 0.001, 0.001);
    // Spike at ts + 0.5 → 0.5 < 1.0s custom gap → suppressed.
    let second = detector.detect_bands(&[0.7, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4], ts + 0.5);
    assert!(!second.iter().any(|e| e.band == 0), "Band 0 should be suppressed at 0.5s gap (< 1.0s custom gap)");
    // Re-warm before final spike.
    warm_up_multi(&mut detector, &[0.4; 7], 43, ts + 0.51, 0.001);
    // Spike at ts + 1.0 → 1.0 >= 1.0s custom gap → fires.
    let third = detector.detect_bands(&[0.7, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4], ts + 1.0);
    assert!(third.iter().any(|e| e.band == 0), "Band 0 should fire at ts + 1.0 (exactly 1.0s gap)");
}

#[test]
fn multi_beat_detector_with_min_gap_fires_at_exactly_custom_gap() {
    let mut detector = MultiBeatDetector::with_min_gap(0.2);
    let ts = warm_up_multi(&mut detector, &[0.4; 7], 43, 0.0, 0.01);
    let first = detector.detect_bands(&[0.7, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4], ts);
    assert!(first.iter().any(|e| e.band == 0));
    warm_up_multi(&mut detector, &[0.4; 7], 43, ts + 0.001, 0.001);
    // Exactly ts + 0.2 → >= 0.2 condition → fires.
    let second = detector.detect_bands(&[0.7, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4], ts + 0.2);
    assert!(second.iter().any(|e| e.band == 0), "Band 0 should fire at exactly 0.2s gap (custom gap)");
}

// --- MultiBeatDetector::new() default is 0.15s ---

#[test]
fn multi_beat_detector_new_allows_beats_0_2s_apart() {
    // Proves default min_gap is 0.15s: a 0.2s gap (> 0.15s) must fire.
    let mut detector = MultiBeatDetector::new();
    let ts = warm_up_multi(&mut detector, &[0.4; 7], 43, 0.0, 0.01);
    let first = detector.detect_bands(&[0.7, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4], ts);
    assert!(first.iter().any(|e| e.band == 0), "Band 0 should fire on first spike");
    warm_up_multi(&mut detector, &[0.4; 7], 43, ts + 0.001, 0.001);
    // 0.2s > 0.15s default gap → must fire (would be suppressed if default were 0.3s).
    let second = detector.detect_bands(&[0.7, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4], ts + 0.2);
    assert!(second.iter().any(|e| e.band == 0), "Band 0 should fire at 0.2s gap (default is 0.15s, not 0.3s)");
}

#[test]
fn multi_beat_detector_new_suppresses_within_0_15s() {
    let mut detector = MultiBeatDetector::new();
    let ts = warm_up_multi(&mut detector, &[0.4; 7], 43, 0.0, 0.01);
    let first = detector.detect_bands(&[0.7, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4], ts);
    assert!(first.iter().any(|e| e.band == 0), "Band 0 should fire on first spike");
    warm_up_multi(&mut detector, &[0.4; 7], 43, ts + 0.001, 0.001);
    // 0.1s < 0.15s default gap → suppressed.
    let second = detector.detect_bands(&[0.7, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4], ts + 0.1);
    assert!(!second.iter().any(|e| e.band == 0), "Band 0 should be suppressed at 0.1s gap (< 0.15s default)");
}

// --- Backward compatibility: BeatDetector still uses 0.3s ---

#[test]
fn beat_detector_still_uses_0_3s_gap_after_refactor() {
    // Confirms single-band BeatDetector was not changed by the MultiBeatDetector refactor.
    let mut detector = BeatDetector::new();
    let ts = warm_up(&mut detector, 0.4, 43, 0.0, 0.01);
    let first = detector.detect(0.7, ts);
    assert!(first.is_some(), "First beat should fire");
    warm_up(&mut detector, 0.4, 43, ts + 0.001, 0.001);
    // 0.2s < 0.3s BeatDetector gap → suppressed.
    let second = detector.detect(0.7, ts + 0.2);
    assert_eq!(second, None, "BeatDetector should suppress beat at 0.2s gap (BeatDetector still uses 0.3s)");
}

// --- Backward compatibility (API coexistence) ---

#[test]
fn old_api_coexists_with_new_api() {
    let mut old = BeatDetector::new();
    let mut new = MultiBeatDetector::new();

    // Warm up both.
    let ts = warm_up(&mut old, 0.4, 43, 0.0, 0.01);
    warm_up_multi(&mut new, &[0.4; 7], 43, 0.0, 0.01);

    // Old API fires.
    let beat = old.detect(0.7, ts);
    assert!(beat.is_some(), "BeatDetector should return Some(BeatEvent)");

    // New API fires.
    let events = new.detect_bands(&[0.7; 7], ts);
    assert_eq!(events.len(), 7, "MultiBeatDetector should return 7 BandBeatEvents");
}
