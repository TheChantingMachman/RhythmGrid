// @spec-tags: audio.speed
// @invariants: Speed control constants, default value, set/get, and clamping behavior
// @build: 36

use rhythm_grid::audio::{AudioPlayer, DecodedAudio, DEFAULT_SPEED, MIN_SPEED, MAX_SPEED};

fn make_audio() -> DecodedAudio {
    DecodedAudio { sample_rate: 44100, channels: 2, samples: vec![0.0f32; 100] }
}

#[test]
fn default_speed_is_1_0() {
    assert_eq!(DEFAULT_SPEED, 1.0f32);
}

#[test]
fn min_speed_is_0_5() {
    assert_eq!(MIN_SPEED, 0.5f32);
}

#[test]
fn max_speed_is_2_0() {
    assert_eq!(MAX_SPEED, 2.0f32);
}

#[test]
fn new_player_speed_is_default() {
    let player = AudioPlayer::new(make_audio());
    assert_eq!(player.speed(), 1.0f32);
}

#[test]
fn set_speed_within_range() {
    let mut player = AudioPlayer::new(make_audio());
    player.set_speed(1.5);
    assert_eq!(player.speed(), 1.5f32);
}

#[test]
fn set_speed_to_min() {
    let mut player = AudioPlayer::new(make_audio());
    player.set_speed(0.5);
    assert_eq!(player.speed(), 0.5f32);
}

#[test]
fn set_speed_to_max() {
    let mut player = AudioPlayer::new(make_audio());
    player.set_speed(2.0);
    assert_eq!(player.speed(), 2.0f32);
}

#[test]
fn set_speed_clamps_above_max() {
    let mut player = AudioPlayer::new(make_audio());
    player.set_speed(3.0);
    assert_eq!(player.speed(), 2.0f32);
}

#[test]
fn set_speed_clamps_below_min() {
    let mut player = AudioPlayer::new(make_audio());
    player.set_speed(0.1);
    assert_eq!(player.speed(), 0.5f32);
}
