// @spec-tags: core,audio,playback
// @invariants: PlaybackState enum has Playing/Paused/Stopped variants, derives Debug/Clone/PartialEq; AudioPlayer::new(DecodedAudio) starts Stopped at position 0; play() -> Playing (no-op if already Playing); pause() -> Paused only when Playing (no-op when Stopped/Paused); stop() -> Stopped, resets position (no-op if already Stopped)
// @build: 35

use rhythm_grid::audio::{AudioPlayer, DecodedAudio, PlaybackState};

fn make_audio() -> DecodedAudio {
    DecodedAudio {
        sample_rate: 44100,
        channels: 2,
        samples: vec![0.0f32; 100],
    }
}

// ── PlaybackState derives ─────────────────────────────────────────────────────

#[test]
fn playback_state_debug_derive() {
    let s = format!("{:?}", PlaybackState::Stopped);
    assert!(s.contains("Stopped"));
}

#[test]
fn playback_state_partial_eq_same() {
    assert_eq!(PlaybackState::Playing, PlaybackState::Playing);
}

#[test]
fn playback_state_partial_eq_different() {
    assert_ne!(PlaybackState::Playing, PlaybackState::Paused);
}

// ── AudioPlayer::new() initial state ─────────────────────────────────────────

#[test]
fn new_initial_state_is_stopped() {
    let player = AudioPlayer::new(make_audio());
    assert_eq!(player.state(), PlaybackState::Stopped);
}

#[test]
fn new_initial_position_is_zero() {
    let player = AudioPlayer::new(make_audio());
    assert_eq!(player.position(), 0);
}

// ── play() ────────────────────────────────────────────────────────────────────

#[test]
fn play_sets_state_to_playing() {
    let mut player = AudioPlayer::new(make_audio());
    player.play();
    assert_eq!(player.state(), PlaybackState::Playing);
}

#[test]
fn play_is_noop_when_already_playing() {
    let mut player = AudioPlayer::new(make_audio());
    player.play();
    player.play();
    assert_eq!(player.state(), PlaybackState::Playing);
}

// ── pause() ───────────────────────────────────────────────────────────────────

#[test]
fn pause_sets_paused_when_playing() {
    let mut player = AudioPlayer::new(make_audio());
    player.play();
    player.pause();
    assert_eq!(player.state(), PlaybackState::Paused);
}

#[test]
fn pause_is_noop_when_stopped() {
    let mut player = AudioPlayer::new(make_audio());
    player.pause();
    assert_eq!(player.state(), PlaybackState::Stopped);
}

#[test]
fn pause_is_noop_when_already_paused() {
    let mut player = AudioPlayer::new(make_audio());
    player.play();
    player.pause();
    player.pause();
    assert_eq!(player.state(), PlaybackState::Paused);
}

// ── stop() ────────────────────────────────────────────────────────────────────

#[test]
fn stop_sets_state_to_stopped() {
    let mut player = AudioPlayer::new(make_audio());
    player.play();
    player.stop();
    assert_eq!(player.state(), PlaybackState::Stopped);
}

#[test]
fn stop_resets_position_to_zero() {
    let mut player = AudioPlayer::new(make_audio());
    player.play();
    player.stop();
    assert_eq!(player.position(), 0);
}

#[test]
fn stop_is_noop_when_already_stopped() {
    let mut player = AudioPlayer::new(make_audio());
    // Should not panic
    player.stop();
    assert_eq!(player.state(), PlaybackState::Stopped);
    assert_eq!(player.position(), 0);
}
