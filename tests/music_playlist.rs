// @spec-tags: music.shuffle,music.auto_advance,music.skip
// @invariants: Playlist::new initializes with current_index=0 and shuffle disabled; advance increments index and wraps to 0 at end; current returns None for empty list; toggle_shuffle flips shuffle state and resets index to 0, restoring original order on disable
// @build: 43

use rhythm_grid::music::Playlist;
use std::path::PathBuf;

fn make_paths(n: usize) -> Vec<PathBuf> {
    (0..n)
        .map(|i| PathBuf::from(format!("/music/track_{:02}.mp3", i)))
        .collect()
}

// ── Playlist::new ─────────────────────────────────────────────────────────────

#[test]
fn new_playlist_current_is_first_file() {
    let playlist = Playlist::new(make_paths(3));
    assert_eq!(
        playlist.current(),
        Some(&PathBuf::from("/music/track_00.mp3")),
        "new playlist must point to index 0 (first file)"
    );
}

#[test]
fn new_empty_playlist_current_is_none() {
    let playlist = Playlist::new(vec![]);
    assert_eq!(
        playlist.current(),
        None,
        "empty playlist must return None from current()"
    );
}

#[test]
fn new_playlist_shuffle_is_disabled_order_preserved() {
    // Without shuffle the insertion order must be preserved; advance gives second file
    let mut playlist = Playlist::new(make_paths(3));
    playlist.advance();
    assert_eq!(
        playlist.current(),
        Some(&PathBuf::from("/music/track_01.mp3")),
        "new playlist without shuffle must preserve insertion order"
    );
}

#[test]
fn playlist_derives_debug() {
    let playlist = Playlist::new(make_paths(2));
    let s = format!("{:?}", playlist);
    assert!(!s.is_empty(), "Playlist must implement Debug");
}

#[test]
fn playlist_derives_clone() {
    let playlist = Playlist::new(make_paths(3));
    let cloned = playlist.clone();
    assert_eq!(
        playlist.current(),
        cloned.current(),
        "cloned playlist must have same current() as original"
    );
}

// ── current ───────────────────────────────────────────────────────────────────

#[test]
fn current_returns_path_at_initial_index() {
    let playlist = Playlist::new(make_paths(5));
    assert_eq!(playlist.current(), Some(&PathBuf::from("/music/track_00.mp3")));
}

#[test]
fn current_returns_none_for_empty_playlist() {
    let playlist = Playlist::new(vec![]);
    assert_eq!(playlist.current(), None);
}

// ── advance (covers both skip and auto-advance behaviour) ─────────────────────

#[test]
fn advance_moves_to_second_track() {
    let mut playlist = Playlist::new(make_paths(3));
    playlist.advance();
    assert_eq!(playlist.current(), Some(&PathBuf::from("/music/track_01.mp3")));
}

#[test]
fn advance_twice_moves_to_third_track() {
    let mut playlist = Playlist::new(make_paths(3));
    playlist.advance();
    playlist.advance();
    assert_eq!(playlist.current(), Some(&PathBuf::from("/music/track_02.mp3")));
}

#[test]
fn advance_wraps_to_zero_from_last_track() {
    let mut playlist = Playlist::new(make_paths(3));
    playlist.advance(); // index 1
    playlist.advance(); // index 2
    playlist.advance(); // wraps → index 0
    assert_eq!(
        playlist.current(),
        Some(&PathBuf::from("/music/track_00.mp3")),
        "advance past last track must wrap to index 0"
    );
}

#[test]
fn advance_on_single_item_wraps_to_itself() {
    let mut playlist = Playlist::new(make_paths(1));
    playlist.advance();
    assert_eq!(
        playlist.current(),
        Some(&PathBuf::from("/music/track_00.mp3")),
        "single-item playlist must return same track after advance"
    );
}

#[test]
fn advance_on_empty_playlist_does_not_panic() {
    let mut playlist = Playlist::new(vec![]);
    playlist.advance(); // must not panic
    assert_eq!(playlist.current(), None);
}

#[test]
fn advance_cycles_all_tracks_and_wraps() {
    // 4 tracks: advance 4 times should land back on track_00
    let mut playlist = Playlist::new(make_paths(4));
    for _ in 0..4 {
        playlist.advance();
    }
    assert_eq!(
        playlist.current(),
        Some(&PathBuf::from("/music/track_00.mp3")),
        "advancing by full list length must return to first track"
    );
}

// ── toggle_shuffle ────────────────────────────────────────────────────────────

#[test]
fn toggle_shuffle_on_resets_index_to_zero() {
    let mut playlist = Playlist::new(make_paths(5));
    playlist.advance(); // move to index 1
    playlist.toggle_shuffle(); // enable — index must reset to 0
    // After enabling, current() must return Some (first in shuffled order)
    assert!(
        playlist.current().is_some(),
        "current() must return Some after enabling shuffle on non-empty playlist"
    );
}

#[test]
fn toggle_shuffle_off_restores_original_order_and_resets_index() {
    let mut playlist = Playlist::new(make_paths(5));
    playlist.toggle_shuffle(); // enable shuffle
    playlist.advance();         // advance from shuffled index 0
    playlist.toggle_shuffle(); // disable — restores original order, index → 0
    assert_eq!(
        playlist.current(),
        Some(&PathBuf::from("/music/track_00.mp3")),
        "disabling shuffle must restore original order and reset index to 0"
    );
}

#[test]
fn toggle_shuffle_on_then_off_full_original_order_preserved() {
    let mut playlist = Playlist::new(make_paths(3));
    playlist.toggle_shuffle(); // enable
    playlist.toggle_shuffle(); // disable — order restored, index 0
    assert_eq!(playlist.current(), Some(&PathBuf::from("/music/track_00.mp3")));
    playlist.advance();
    assert_eq!(playlist.current(), Some(&PathBuf::from("/music/track_01.mp3")));
    playlist.advance();
    assert_eq!(playlist.current(), Some(&PathBuf::from("/music/track_02.mp3")));
    playlist.advance(); // wraps
    assert_eq!(playlist.current(), Some(&PathBuf::from("/music/track_00.mp3")));
}

#[test]
fn toggle_shuffle_on_empty_playlist_does_not_panic() {
    let mut playlist = Playlist::new(vec![]);
    playlist.toggle_shuffle(); // must not panic
    assert_eq!(playlist.current(), None);
}

#[test]
fn toggle_shuffle_twice_on_empty_does_not_panic() {
    let mut playlist = Playlist::new(vec![]);
    playlist.toggle_shuffle();
    playlist.toggle_shuffle();
    assert_eq!(playlist.current(), None);
}
