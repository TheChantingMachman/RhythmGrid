// @spec-tags: audio.procedural
// @invariants: DEFAULT_BPM=120; generate_procedural(bpm) returns AudioData at 44100 Hz mono covering exactly 4 beats; beat onsets are 440 Hz sine pulses of 50 ms; inter-beat samples are 0.0; output is deterministic
// @build: 30

use rhythm_grid::audio::{generate_procedural, AudioData, DEFAULT_BPM};

// Resolved spec constants (from scope-check assumptions):
//   sample_rate = 44100 Hz, channels = 1 (mono), duration = 4 beats = one bar
//   beat onset = 440 Hz sine pulse, 50 ms = 2205 samples at 44100 Hz
//   all non-pulse samples = 0.0

const SAMPLE_RATE: u32 = 44100;
const PULSE_SAMPLES: usize = 2205; // 50 ms * 44100 Hz

/// Expected total sample count for a given BPM over 4 beats.
fn expected_samples(bpm: u32) -> usize {
    let beat_interval = SAMPLE_RATE as f64 * 60.0 / bpm as f64;
    (beat_interval * 4.0).round() as usize
}

/// Expected sample index at which beat `n` (0-based) begins.
fn beat_onset(bpm: u32, beat: usize) -> usize {
    let beat_interval = SAMPLE_RATE as f64 * 60.0 / bpm as f64;
    (beat_interval * beat as f64).round() as usize
}

#[test]
fn default_bpm_is_120() {
    assert_eq!(DEFAULT_BPM, 120u32, "DEFAULT_BPM must equal 120");
}

#[test]
fn procedural_audio_returns_valid_pcm() {
    let audio: AudioData = generate_procedural(DEFAULT_BPM);
    assert!(audio.sample_rate > 0,    "sample_rate must be non-zero");
    assert!(audio.channels >= 1,      "channels must be >= 1");
    assert!(!audio.samples.is_empty(), "samples must be non-empty");
}

#[test]
fn procedural_audio_uses_requested_bpm() {
    // Use BPM=60 so beat_interval = 44100 samples — easy to verify.
    let bpm: u32 = 60;
    let audio: AudioData = generate_procedural(bpm);

    let total = expected_samples(bpm);
    assert_eq!(
        audio.samples.len(), total,
        "for {} BPM, expected {} samples (4 beats), got {}",
        bpm, total, audio.samples.len()
    );

    // Beat 1 onset at sample 44100; the pulse starts there → should be non-zero.
    let onset1 = beat_onset(bpm, 1);
    assert!(
        audio.samples[onset1] != 0.0,
        "sample at beat-1 onset (index {}) should be non-zero (sine pulse)", onset1
    );

    // Midpoint between beat 0 and beat 1 must be silent (outside any pulse).
    let mid = PULSE_SAMPLES + 100; // well past the 50 ms pulse of beat 0
    assert_eq!(
        audio.samples[mid], 0.0,
        "sample at index {} (inter-beat silence) should be 0.0", mid
    );
}

#[test]
fn procedural_audio_deterministic() {
    let bpm: u32 = DEFAULT_BPM;
    let a: AudioData = generate_procedural(bpm);
    let b: AudioData = generate_procedural(bpm);
    assert_eq!(a.sample_rate, b.sample_rate, "sample_rate must be identical across calls");
    assert_eq!(a.channels, b.channels, "channels must be identical across calls");
    assert_eq!(a.samples.len(), b.samples.len(), "sample count must be identical across calls");
    for (i, (sa, sb)) in a.samples.iter().zip(b.samples.iter()).enumerate() {
        assert_eq!(
            sa.to_bits(), sb.to_bits(),
            "sample[{}] differs between two identical calls: {} vs {}", i, sa, sb
        );
    }
}

#[test]
fn procedural_audio_default_bpm_works() {
    let audio: AudioData = generate_procedural(DEFAULT_BPM);
    assert!(
        !audio.samples.is_empty(),
        "generate_procedural(DEFAULT_BPM={}) must return non-empty samples", DEFAULT_BPM
    );
    let total = expected_samples(DEFAULT_BPM);
    assert_eq!(
        audio.samples.len(), total,
        "for DEFAULT_BPM={}, expected {} samples, got {}", DEFAULT_BPM, total, audio.samples.len()
    );
}
