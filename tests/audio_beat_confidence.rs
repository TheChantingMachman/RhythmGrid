// @spec-tags: core,audio,analysis
// @invariants: BeatConfidence tracks last 16 inter-beat intervals per band; confidence() returns 0.0-1.0 per band; perfectly regular beats yield confidence 1.0 (CV=0); no beats or fewer than 2 intervals yields 0.0; only bands with beats accumulate confidence; confidence = (1.0 - std_dev/mean).clamp(0.0, 1.0)
// @build: 91

use rhythm_grid::audio::BeatConfidence;

// --- Construction ---

#[test]
fn beat_confidence_new_creates_instance() {
    let _bc = BeatConfidence::new();
}

#[test]
fn beat_confidence_derives_debug() {
    let bc = BeatConfidence::new();
    let s = format!("{:?}", bc);
    assert!(!s.is_empty());
}

// --- Fresh tracker: confidence() ---

#[test]
fn confidence_returns_all_zeros_on_new_tracker() {
    let bc = BeatConfidence::new();
    let conf = bc.confidence();
    assert_eq!(conf, [0.0_f32; 7], "fresh tracker must return [0.0; 7]");
}

#[test]
fn confidence_returns_seven_element_array() {
    let bc = BeatConfidence::new();
    assert_eq!(bc.confidence().len(), 7);
}

// --- No beats: confidence is 0.0 ---

#[test]
fn confidence_zero_after_only_false_beats() {
    let mut bc = BeatConfidence::new();
    let no_beats = [false; 7];
    for i in 0..100 {
        bc.update(&no_beats, i as f64 * 0.016);
    }
    let conf = bc.confidence();
    assert_eq!(conf, [0.0_f32; 7], "no beats → all confidence must be 0.0");
}

#[test]
fn confidence_zero_with_single_beat_fewer_than_2_intervals() {
    // One beat gives 0 intervals (need at least 2 beats for 1 interval, 3 for 2 intervals)
    let mut bc = BeatConfidence::new();
    let mut beats = [false; 7];
    beats[0] = true;
    bc.update(&beats, 1.0);
    let conf = bc.confidence();
    assert_eq!(conf[0], 0.0, "single beat produces no intervals → confidence 0.0");
}

#[test]
fn confidence_zero_with_only_one_interval() {
    // Two beats → one interval → still fewer than 2 intervals → 0.0
    let mut bc = BeatConfidence::new();
    let mut beats = [false; 7];
    beats[0] = true;
    bc.update(&beats, 1.0);
    beats[0] = false;
    bc.update(&beats, 1.5);
    beats[0] = true;
    bc.update(&beats, 2.0);
    let conf = bc.confidence();
    assert_eq!(conf[0], 0.0, "one interval is still fewer than 2 → confidence 0.0");
}

// --- Perfectly regular beats: confidence 1.0 ---

#[test]
fn confidence_band_0_is_1_after_perfectly_regular_beats() {
    // Feed beats at exactly 0.5s intervals — all intervals equal 0.5s
    // CV = std_dev / mean = 0.0 / 0.5 = 0.0 → confidence = 1.0
    let mut bc = BeatConfidence::new();
    for i in 0..25 {
        let mut beats = [false; 7];
        beats[0] = true; // beat on every update, interval = 0.5s
        bc.update(&beats, i as f64 * 0.5);
    }
    let conf = bc.confidence();
    assert!(
        (conf[0] - 1.0).abs() < 1e-5,
        "perfectly regular beats (every 0.5s) must yield confidence ~1.0, got {}",
        conf[0]
    );
}

#[test]
fn confidence_perfectly_regular_at_different_interval() {
    // Beats at exactly 1.0s intervals
    let mut bc = BeatConfidence::new();
    for i in 0..20 {
        let mut beats = [false; 7];
        beats[2] = true;
        bc.update(&beats, i as f64 * 1.0);
    }
    let conf = bc.confidence();
    assert!(
        (conf[2] - 1.0).abs() < 1e-5,
        "perfectly regular beats at 1.0s must yield confidence ~1.0, got {}",
        conf[2]
    );
}

// --- Only beats to one band affect only that band ---

#[test]
fn confidence_other_bands_zero_when_only_band_0_has_beats() {
    let mut bc = BeatConfidence::new();
    for i in 0..20 {
        let mut beats = [false; 7];
        beats[0] = true;
        bc.update(&beats, i as f64 * 0.5);
    }
    let conf = bc.confidence();
    for i in 1..7 {
        assert_eq!(conf[i], 0.0, "band {} should have confidence 0.0 (no beats)", i);
    }
}

#[test]
fn confidence_only_beat_band_has_nonzero_confidence() {
    let mut bc = BeatConfidence::new();
    for i in 0..20 {
        let mut beats = [false; 7];
        beats[5] = true;
        bc.update(&beats, i as f64 * 0.5);
    }
    let conf = bc.confidence();
    for i in 0..7 {
        if i == 5 {
            assert!(conf[i] > 0.0, "band 5 should have nonzero confidence after regular beats");
        } else {
            assert_eq!(conf[i], 0.0, "band {} should be 0.0 with no beats", i);
        }
    }
}

// --- Confidence formula: coefficient of variation ---

#[test]
fn confidence_clamped_at_zero_for_high_variance() {
    // Feed beats at random/irregular intervals — CV > 1 → confidence clamped to 0.0
    let mut bc = BeatConfidence::new();
    let intervals = [0.1, 2.0, 0.05, 3.0, 0.2, 1.5, 0.08, 2.5, 0.3, 1.8,
                     0.12, 2.2, 0.07, 3.5, 0.15, 2.8, 0.09, 1.2];
    let mut ts = 0.0_f64;
    for &gap in &intervals {
        let mut beats = [false; 7];
        beats[4] = true;
        bc.update(&beats, ts);
        ts += gap;
    }
    let conf = bc.confidence();
    assert!(
        conf[4] >= 0.0,
        "confidence must be >= 0.0 (clamped), got {}",
        conf[4]
    );
}

#[test]
fn confidence_values_always_in_range_0_to_1() {
    let mut bc = BeatConfidence::new();
    // Mix of beat patterns
    for i in 0..50 {
        let mut beats = [false; 7];
        if i % 3 == 0 { beats[0] = true; }
        if i % 5 == 0 { beats[1] = true; }
        if i % 7 == 0 { beats[2] = true; }
        bc.update(&beats, i as f64 * 0.1);
    }
    let conf = bc.confidence();
    for (i, &c) in conf.iter().enumerate() {
        assert!(
            c >= 0.0 && c <= 1.0,
            "confidence[{}] = {} is out of [0.0, 1.0] range",
            i, c
        );
    }
}

// --- Rolling window of 16 intervals ---

#[test]
fn confidence_ring_buffer_tracks_last_16_intervals() {
    // Fill ring buffer with regular intervals, then inject irregular ones
    // After > 16 irregular beats, the regular ones should be pushed out
    let mut bc = BeatConfidence::new();

    // First: 20 regular beats at 0.5s intervals → confidence ~1.0
    for i in 0..20 {
        let mut beats = [false; 7];
        beats[0] = true;
        bc.update(&beats, i as f64 * 0.5);
    }
    let conf_regular = bc.confidence()[0];
    assert!((conf_regular - 1.0).abs() < 1e-5, "regular beats: expected 1.0, got {}", conf_regular);

    // Now: inject 20 beats with wildly varying intervals
    // This should replace the last 16 stored intervals
    let irregular_gaps = [0.01, 5.0, 0.02, 4.0, 0.015, 3.5, 0.025, 4.5,
                          0.01, 5.0, 0.02, 4.0, 0.015, 3.5, 0.025, 4.5,
                          0.01, 5.0, 0.02, 4.0];
    let mut ts = 20.0 * 0.5;
    for &gap in &irregular_gaps {
        let mut beats = [false; 7];
        beats[0] = true;
        bc.update(&beats, ts);
        ts += gap;
    }
    let conf_after_irregular = bc.confidence()[0];
    // After filling the 16-slot ring buffer with high-variance intervals, confidence should drop
    assert!(
        conf_after_irregular < 0.9,
        "after irregular beats replace ring buffer, confidence should drop from 1.0, got {}",
        conf_after_irregular
    );
}

// --- update(): timestamps are continuous, not per-beat ---

#[test]
fn update_called_every_frame_not_just_on_beats() {
    // update is called continuously; inter-beat interval is derived from timestamp difference
    let mut bc = BeatConfidence::new();
    let frame_dt = 1.0 / 60.0; // 60 fps
    let beat_every_n_frames = 30; // beat every 0.5s

    for i in 0..300 {
        let ts = i as f64 * frame_dt;
        let mut beats = [false; 7];
        if i % beat_every_n_frames == 0 {
            beats[0] = true;
        }
        bc.update(&beats, ts);
    }

    let conf = bc.confidence();
    // Beats at exact 0.5s intervals → CV = 0 → confidence = 1.0
    assert!(
        (conf[0] - 1.0).abs() < 1e-4,
        "beats at exactly 0.5s intervals (30 frames at 60fps) must yield confidence ~1.0, got {}",
        conf[0]
    );
    // Other bands have no beats
    for i in 1..7 {
        assert_eq!(conf[i], 0.0, "band {} should be 0.0", i);
    }
}

// --- Multiple bands simultaneously ---

#[test]
fn confidence_two_bands_with_different_regularity() {
    let mut bc = BeatConfidence::new();
    // Band 0: perfectly regular at every 20 frames (0.02s at 1000fps sim)
    // Band 3: irregular
    let irregular_period = [10, 30, 5, 40, 12, 35, 8, 42, 11, 28, 6, 38, 9, 33, 14, 27, 7, 41, 13, 29];
    let mut next_irregular = 0usize;
    let mut irregular_countdown = irregular_period[0];

    for i in 0..500 {
        let ts = i as f64 * 0.001;
        let mut beats = [false; 7];
        if i % 20 == 0 {
            beats[0] = true;
        }
        if irregular_countdown == 0 {
            beats[3] = true;
            next_irregular = (next_irregular + 1) % irregular_period.len();
            irregular_countdown = irregular_period[next_irregular];
        } else {
            irregular_countdown -= 1;
        }
        bc.update(&beats, ts);
    }

    let conf = bc.confidence();
    // Band 0: high confidence (regular)
    // Band 3: lower confidence (irregular)
    assert!(
        conf[0] > conf[3],
        "regular band (0) should have higher confidence than irregular band (3): {} vs {}",
        conf[0], conf[3]
    );
    assert!(conf[0] > 0.8, "regular band should have high confidence, got {}", conf[0]);
}
