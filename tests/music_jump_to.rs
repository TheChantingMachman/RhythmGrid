// @spec-tags: core,music,playback
// @invariants: Playlist::jump_to sets current_index clamped to valid bounds, returns current() after setting, returns None for empty playlist
// @build: 93

use rhythm_grid::music::Playlist;
use std::path::PathBuf;

fn make_paths(n: usize) -> Vec<PathBuf> {
    (0..n)
        .map(|i| PathBuf::from(format!("/music/track_{:02}.mp3", i)))
        .collect()
}

// ── jump_to: empty playlist ─────────────────────────────────────────────────

#[test]
fn jump_to_empty_playlist_returns_none() {
    let mut playlist = Playlist::new(vec![]);
    assert_eq!(
        playlist.jump_to(0),
        None,
        "jump_to on empty playlist must return None"
    );
}

#[test]
fn jump_to_empty_playlist_large_index_returns_none() {
    let mut playlist = Playlist::new(vec![]);
    assert_eq!(
        playlist.jump_to(999),
        None,
        "jump_to with large index on empty playlist must return None"
    );
}

#[test]
fn jump_to_empty_playlist_current_still_none() {
    let mut playlist = Playlist::new(vec![]);
    playlist.jump_to(0);
    assert_eq!(
        playlist.current(),
        None,
        "current() after jump_to on empty playlist must still be None"
    );
}

// ── jump_to: jump to index 0 ────────────────────────────────────────────────

#[test]
fn jump_to_index_zero_returns_first_track() {
    let mut playlist = Playlist::new(make_paths(3));
    let result = playlist.jump_to(0);
    assert_eq!(
        result,
        Some(&PathBuf::from("/music/track_00.mp3")),
        "jump_to(0) must return the first track"
    );
}

#[test]
fn jump_to_index_zero_current_is_first_track() {
    let mut playlist = Playlist::new(make_paths(3));
    playlist.jump_to(0);
    assert_eq!(
        playlist.current(),
        Some(&PathBuf::from("/music/track_00.mp3")),
        "current() after jump_to(0) must be the first track"
    );
}

// ── jump_to: jump to middle index ───────────────────────────────────────────

#[test]
fn jump_to_middle_index_returns_correct_track() {
    let mut playlist = Playlist::new(make_paths(5));
    let result = playlist.jump_to(2);
    assert_eq!(
        result,
        Some(&PathBuf::from("/music/track_02.mp3")),
        "jump_to(2) must return track at index 2"
    );
}

#[test]
fn jump_to_middle_index_current_updated() {
    let mut playlist = Playlist::new(make_paths(5));
    playlist.jump_to(3);
    assert_eq!(
        playlist.current(),
        Some(&PathBuf::from("/music/track_03.mp3")),
        "current() after jump_to(3) must be track at index 3"
    );
}

// ── jump_to: jump to last valid index ───────────────────────────────────────

#[test]
fn jump_to_last_index_returns_last_track() {
    let mut playlist = Playlist::new(make_paths(4));
    let result = playlist.jump_to(3);
    assert_eq!(
        result,
        Some(&PathBuf::from("/music/track_03.mp3")),
        "jump_to(last) must return the last track"
    );
}

#[test]
fn jump_to_last_index_current_is_last_track() {
    let mut playlist = Playlist::new(make_paths(4));
    playlist.jump_to(3);
    assert_eq!(
        playlist.current(),
        Some(&PathBuf::from("/music/track_03.mp3")),
        "current() after jump_to(last) must be the last track"
    );
}

// ── jump_to: clamping — index >= files.len() ────────────────────────────────

#[test]
fn jump_to_index_equal_len_clamps_to_last() {
    let mut playlist = Playlist::new(make_paths(3));
    // files.len() = 3, valid indices 0..=2; index 3 >= len, clamp to 2
    let result = playlist.jump_to(3);
    assert_eq!(
        result,
        Some(&PathBuf::from("/music/track_02.mp3")),
        "jump_to(files.len()) must clamp to last track"
    );
}

#[test]
fn jump_to_large_index_clamps_to_last() {
    let mut playlist = Playlist::new(make_paths(3));
    let result = playlist.jump_to(999);
    assert_eq!(
        result,
        Some(&PathBuf::from("/music/track_02.mp3")),
        "jump_to with very large index must clamp to last track"
    );
}

#[test]
fn jump_to_out_of_bounds_current_is_last_track() {
    let mut playlist = Playlist::new(make_paths(5));
    playlist.jump_to(100);
    assert_eq!(
        playlist.current(),
        Some(&PathBuf::from("/music/track_04.mp3")),
        "current() after out-of-bounds jump_to must be last track"
    );
}

// ── jump_to: single-item playlist ───────────────────────────────────────────

#[test]
fn jump_to_single_item_index_zero_returns_only_track() {
    let mut playlist = Playlist::new(make_paths(1));
    let result = playlist.jump_to(0);
    assert_eq!(
        result,
        Some(&PathBuf::from("/music/track_00.mp3")),
        "jump_to(0) on single-item playlist must return the only track"
    );
}

#[test]
fn jump_to_single_item_large_index_clamps_to_only_track() {
    let mut playlist = Playlist::new(make_paths(1));
    let result = playlist.jump_to(50);
    assert_eq!(
        result,
        Some(&PathBuf::from("/music/track_00.mp3")),
        "jump_to with large index on single-item playlist must clamp to the only track"
    );
}

// ── jump_to: return value matches current() ─────────────────────────────────

#[test]
fn jump_to_return_value_matches_current_after_call() {
    let mut playlist = Playlist::new(make_paths(5));
    let returned = playlist.jump_to(3).cloned();
    assert_eq!(
        returned.as_ref(),
        playlist.current(),
        "jump_to return value must equal current() after the call"
    );
}

#[test]
fn jump_to_clamped_return_value_matches_current_after_call() {
    let mut playlist = Playlist::new(make_paths(4));
    let returned = playlist.jump_to(999).cloned();
    assert_eq!(
        returned.as_ref(),
        playlist.current(),
        "jump_to (clamped) return value must equal current() after the call"
    );
}

// ── jump_to: overwrite previously advanced position ─────────────────────────

#[test]
fn jump_to_overrides_advance_position() {
    let mut playlist = Playlist::new(make_paths(5));
    playlist.advance(); // index 1
    playlist.advance(); // index 2
    let result = playlist.jump_to(4);
    assert_eq!(
        result,
        Some(&PathBuf::from("/music/track_04.mp3")),
        "jump_to must override current position set by advance()"
    );
}

#[test]
fn jump_to_then_advance_continues_from_jumped_position() {
    let mut playlist = Playlist::new(make_paths(5));
    playlist.jump_to(2); // jump to index 2
    playlist.advance();  // should go to index 3
    assert_eq!(
        playlist.current(),
        Some(&PathBuf::from("/music/track_03.mp3")),
        "advance() after jump_to must continue from jumped position"
    );
}

#[test]
fn jump_to_clamped_then_advance_wraps_correctly() {
    let mut playlist = Playlist::new(make_paths(3));
    playlist.jump_to(999); // clamps to index 2 (last)
    playlist.advance();    // should wrap to index 0
    assert_eq!(
        playlist.current(),
        Some(&PathBuf::from("/music/track_00.mp3")),
        "advance() after clamped jump_to must wrap to first track"
    );
}

// ── jump_to: consecutive jumps ──────────────────────────────────────────────

#[test]
fn jump_to_called_twice_uses_second_index() {
    let mut playlist = Playlist::new(make_paths(5));
    playlist.jump_to(4);
    let result = playlist.jump_to(1);
    assert_eq!(
        result,
        Some(&PathBuf::from("/music/track_01.mp3")),
        "second jump_to must set current to the second index"
    );
}

#[test]
fn jump_to_back_to_zero_after_jumping_forward() {
    let mut playlist = Playlist::new(make_paths(5));
    playlist.jump_to(4);
    playlist.jump_to(0);
    assert_eq!(
        playlist.current(),
        Some(&PathBuf::from("/music/track_00.mp3")),
        "jump_to(0) after jumping to last must return to first track"
    );
}
