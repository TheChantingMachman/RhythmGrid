// @spec-tags: core,audio,playback
// @invariants: Volume control constants, default value, set/get, and clamping behavior
// @build: 36

use rhythm_grid::audio::{AudioPlayer, DecodedAudio, DEFAULT_VOLUME, MIN_VOLUME, MAX_VOLUME};

fn make_audio() -> DecodedAudio {
    DecodedAudio { sample_rate: 44100, channels: 2, samples: vec![0.0f32; 100] }
}

#[test]
fn default_volume_is_0_8() {
    assert_eq!(DEFAULT_VOLUME, 0.8f32);
}

#[test]
fn min_volume_is_0_0() {
    assert_eq!(MIN_VOLUME, 0.0f32);
}

#[test]
fn max_volume_is_1_0() {
    assert_eq!(MAX_VOLUME, 1.0f32);
}

#[test]
fn new_player_volume_is_default() {
    let player = AudioPlayer::new(make_audio());
    assert_eq!(player.volume(), 0.8f32);
}

#[test]
fn set_volume_within_range() {
    let mut player = AudioPlayer::new(make_audio());
    player.set_volume(0.5);
    assert_eq!(player.volume(), 0.5f32);
}

#[test]
fn set_volume_to_zero_mutes() {
    let mut player = AudioPlayer::new(make_audio());
    player.set_volume(0.0);
    assert_eq!(player.volume(), 0.0f32);
}

#[test]
fn set_volume_to_max() {
    let mut player = AudioPlayer::new(make_audio());
    player.set_volume(1.0);
    assert_eq!(player.volume(), 1.0f32);
}

#[test]
fn set_volume_clamps_above_max() {
    let mut player = AudioPlayer::new(make_audio());
    player.set_volume(1.5);
    assert_eq!(player.volume(), 1.0f32);
}

#[test]
fn set_volume_clamps_below_min() {
    let mut player = AudioPlayer::new(make_audio());
    player.set_volume(-0.5);
    assert_eq!(player.volume(), 0.0f32);
}
