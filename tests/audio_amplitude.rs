// @spec-tags: audio.amplitude
// @invariants: RMS amplitude computation over stored audio samples, normalized 0.0-1.0
// @build: 36

use rhythm_grid::audio::{AudioPlayer, DecodedAudio};

fn make_audio_with_samples(samples: Vec<f32>) -> DecodedAudio {
    DecodedAudio { sample_rate: 44100, channels: 1, samples }
}

#[test]
fn amplitude_of_silence_is_zero() {
    let player = AudioPlayer::new(make_audio_with_samples(vec![0.0f32; 100]));
    assert_eq!(player.amplitude(), 0.0f32);
}

#[test]
fn amplitude_of_max_signal_is_1_0() {
    let player = AudioPlayer::new(make_audio_with_samples(vec![1.0f32; 100]));
    assert_eq!(player.amplitude(), 1.0f32);
}

#[test]
fn amplitude_of_negative_max_is_1_0() {
    let player = AudioPlayer::new(make_audio_with_samples(vec![-1.0f32; 100]));
    assert_eq!(player.amplitude(), 1.0f32);
}

#[test]
fn amplitude_of_half_signal() {
    let player = AudioPlayer::new(make_audio_with_samples(vec![0.5f32; 100]));
    assert_eq!(player.amplitude(), 0.5f32);
}

#[test]
fn amplitude_of_mixed_signal() {
    let player = AudioPlayer::new(make_audio_with_samples(vec![1.0, -1.0, 0.0, 0.0]));
    // RMS = sqrt((1 + 1 + 0 + 0) / 4) = sqrt(0.5) ≈ 0.7071
    assert!((player.amplitude() - 0.7071f32).abs() < 0.001);
}

#[test]
fn amplitude_with_no_samples_is_zero() {
    let player = AudioPlayer::new(make_audio_with_samples(vec![]));
    assert_eq!(player.amplitude(), 0.0f32);
}

#[test]
fn amplitude_return_type_is_f32() {
    let player = AudioPlayer::new(make_audio_with_samples(vec![0.5f32; 10]));
    let _: f32 = player.amplitude();
}
