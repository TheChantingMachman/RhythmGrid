// @spec-tags: core,audio,procedural
// @invariants: generate_procedural returns mono (channels=1) DecodedAudio; DEFAULT_BPM=120; output is deterministic for same inputs; sample count = round(duration_secs * sample_rate)
// @build: 33

use rhythm_grid::audio::{generate_procedural, DecodedAudio, DEFAULT_BPM};

// ── DEFAULT_BPM constant ──────────────────────────────────────────────────────

#[test]
fn default_bpm_is_120() {
    assert_eq!(DEFAULT_BPM, 120u32);
}

// ── Mono output ───────────────────────────────────────────────────────────────

#[test]
fn generate_procedural_returns_mono() {
    let audio = generate_procedural(DEFAULT_BPM, 1.0, 44100);
    assert_eq!(audio.channels, 1u16, "procedural audio must be mono (channels=1)");
}

// ── Sample count ──────────────────────────────────────────────────────────────

#[test]
fn generate_procedural_sample_count_matches_duration_and_rate() {
    let bpm = 120u32;
    let duration_secs = 1.0f32;
    let sample_rate = 44100u32;
    let expected = (duration_secs * sample_rate as f32).round() as usize;
    let audio = generate_procedural(bpm, duration_secs, sample_rate);
    assert_eq!(
        audio.samples.len(),
        expected,
        "sample count must equal round(duration_secs * sample_rate)"
    );
}

#[test]
fn generate_procedural_sample_count_two_seconds() {
    let sample_rate = 22050u32;
    let duration_secs = 2.0f32;
    let expected = (duration_secs * sample_rate as f32).round() as usize;
    let audio = generate_procedural(100, duration_secs, sample_rate);
    assert_eq!(audio.samples.len(), expected);
}

#[test]
fn generate_procedural_sample_count_fractional_duration() {
    let sample_rate = 44100u32;
    let duration_secs = 0.5f32;
    let expected = (duration_secs * sample_rate as f32).round() as usize;
    let audio = generate_procedural(DEFAULT_BPM, duration_secs, sample_rate);
    assert_eq!(audio.samples.len(), expected);
}

// ── sample_rate is echoed back ────────────────────────────────────────────────

#[test]
fn generate_procedural_sample_rate_is_preserved() {
    let audio = generate_procedural(DEFAULT_BPM, 1.0, 48000);
    assert_eq!(audio.sample_rate, 48000u32);
}

// ── Determinism ───────────────────────────────────────────────────────────────

#[test]
fn generate_procedural_is_deterministic() {
    let a = generate_procedural(120, 1.0, 44100);
    let b = generate_procedural(120, 1.0, 44100);
    assert_eq!(a.samples, b.samples, "same inputs must produce identical samples");
}

#[test]
fn generate_procedural_different_bpm_produces_different_output() {
    let a = generate_procedural(120, 1.0, 44100);
    let b = generate_procedural(60, 1.0, 44100);
    assert_ne!(
        a.samples, b.samples,
        "different BPM must produce different sample content"
    );
}

// ── Samples are finite f32 values ─────────────────────────────────────────────

#[test]
fn generate_procedural_all_samples_are_finite() {
    let audio = generate_procedural(DEFAULT_BPM, 1.0, 44100);
    for (i, &s) in audio.samples.iter().enumerate() {
        assert!(
            s.is_finite(),
            "sample[{}] = {} is not finite",
            i,
            s
        );
    }
}
