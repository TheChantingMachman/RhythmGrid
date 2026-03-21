// @spec-tags: core,audio,analysis
// @invariants: BeatDetector emits BeatEvent when amplitude > rolling_mean*1.5 and >= 0.3s since last beat; 43-sample rolling window; minimum inter-beat gap enforced; BeatEvent carries correct timestamp
// @build: 52

use rhythm_grid::audio::{BeatDetector, BeatEvent};

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
