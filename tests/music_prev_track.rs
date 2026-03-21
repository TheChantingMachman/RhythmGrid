// @spec-tags: core,music,playback
// @invariants: Playlist::prev_track decrements current_index with wrap-around from 0 to last track; returns new current track path; returns None for empty playlist
// @build: 74

use rhythm_grid::music::Playlist;
use std::path::PathBuf;

fn make_paths(n: usize) -> Vec<PathBuf> {
    (0..n)
        .map(|i| PathBuf::from(format!("/music/track_{:02}.mp3", i)))
        .collect()
}

// ── prev_track: empty playlist ─────────────────────────────────────────────

#[test]
fn prev_track_empty_playlist_returns_none() {
    let mut playlist = Playlist::new(vec![]);
    assert_eq!(
        playlist.prev_track(),
        None,
        "prev_track on empty playlist must return None"
    );
}

#[test]
fn prev_track_empty_playlist_does_not_panic() {
    let mut playlist = Playlist::new(vec![]);
    playlist.prev_track(); // must not panic
    assert_eq!(playlist.current(), None);
}

// ── prev_track: single-item playlist ──────────────────────────────────────

#[test]
fn prev_track_single_item_wraps_to_itself() {
    let mut playlist = Playlist::new(make_paths(1));
    let result = playlist.prev_track();
    assert_eq!(
        result,
        Some(&PathBuf::from("/music/track_00.mp3")),
        "single-item playlist: prev_track must return the only track"
    );
}

#[test]
fn prev_track_single_item_current_unchanged() {
    let mut playlist = Playlist::new(make_paths(1));
    playlist.prev_track();
    assert_eq!(
        playlist.current(),
        Some(&PathBuf::from("/music/track_00.mp3")),
        "single-item playlist: current() after prev_track must still return the only track"
    );
}

// ── prev_track: wrap from index 0 to last ─────────────────────────────────

#[test]
fn prev_track_at_index_zero_wraps_to_last() {
    let mut playlist = Playlist::new(make_paths(3));
    // current_index starts at 0
    let result = playlist.prev_track();
    assert_eq!(
        result,
        Some(&PathBuf::from("/music/track_02.mp3")),
        "prev_track at index 0 must wrap to last track (index 2)"
    );
}

#[test]
fn prev_track_at_index_zero_current_is_last() {
    let mut playlist = Playlist::new(make_paths(3));
    playlist.prev_track();
    assert_eq!(
        playlist.current(),
        Some(&PathBuf::from("/music/track_02.mp3")),
        "after prev_track at index 0, current() must return last track"
    );
}

#[test]
fn prev_track_at_index_zero_five_tracks_wraps_to_last() {
    let mut playlist = Playlist::new(make_paths(5));
    let result = playlist.prev_track();
    assert_eq!(
        result,
        Some(&PathBuf::from("/music/track_04.mp3")),
        "prev_track at index 0 must wrap to index 4 for 5-track playlist"
    );
}

// ── prev_track: normal decrement ──────────────────────────────────────────

#[test]
fn prev_track_from_middle_decrements_index() {
    let mut playlist = Playlist::new(make_paths(5));
    playlist.advance(); // index 1
    playlist.advance(); // index 2
    let result = playlist.prev_track(); // should go to index 1
    assert_eq!(
        result,
        Some(&PathBuf::from("/music/track_01.mp3")),
        "prev_track from index 2 must return track at index 1"
    );
}

#[test]
fn prev_track_from_last_track_decrements_to_second_to_last() {
    let mut playlist = Playlist::new(make_paths(3));
    playlist.advance(); // index 1
    playlist.advance(); // index 2 (last)
    let result = playlist.prev_track(); // should go to index 1
    assert_eq!(
        result,
        Some(&PathBuf::from("/music/track_01.mp3")),
        "prev_track from last track must return second-to-last track"
    );
}

#[test]
fn prev_track_return_value_matches_current_after_call() {
    let mut playlist = Playlist::new(make_paths(4));
    playlist.advance(); // index 1
    playlist.advance(); // index 2
    let returned = playlist.prev_track().cloned();
    assert_eq!(
        returned.as_ref(),
        playlist.current(),
        "prev_track return value must equal current() after the call"
    );
}

// ── prev_track: full cycle ─────────────────────────────────────────────────

#[test]
fn prev_track_cycles_all_tracks_backward() {
    // 3 tracks: start at 0, prev goes 0→2→1→0
    let mut playlist = Playlist::new(make_paths(3));
    assert_eq!(playlist.current(), Some(&PathBuf::from("/music/track_00.mp3")));

    let r1 = playlist.prev_track(); // 0 → 2
    assert_eq!(r1, Some(&PathBuf::from("/music/track_02.mp3")));

    let r2 = playlist.prev_track(); // 2 → 1
    assert_eq!(r2, Some(&PathBuf::from("/music/track_01.mp3")));

    let r3 = playlist.prev_track(); // 1 → 0
    assert_eq!(r3, Some(&PathBuf::from("/music/track_00.mp3")));
}

#[test]
fn prev_track_full_cycle_returns_to_start() {
    // Calling prev_track N times on N tracks returns to index 0
    let n = 4;
    let mut playlist = Playlist::new(make_paths(n));
    for _ in 0..n {
        playlist.prev_track();
    }
    assert_eq!(
        playlist.current(),
        Some(&PathBuf::from("/music/track_00.mp3")),
        "calling prev_track N times on N tracks must return to first track"
    );
}

// ── prev_track and advance interop ────────────────────────────────────────

#[test]
fn advance_then_prev_track_returns_to_original() {
    let mut playlist = Playlist::new(make_paths(5));
    playlist.advance(); // index 1
    playlist.prev_track(); // back to index 0
    assert_eq!(
        playlist.current(),
        Some(&PathBuf::from("/music/track_00.mp3")),
        "advance then prev_track must return to original track"
    );
}

#[test]
fn prev_track_then_advance_returns_to_original() {
    let mut playlist = Playlist::new(make_paths(5));
    playlist.prev_track(); // 0 → 4 (wrap)
    playlist.advance(); // 4 → 0 (wrap back via advance)... actually 4+1=5 % 5=0
    assert_eq!(
        playlist.current(),
        Some(&PathBuf::from("/music/track_00.mp3")),
        "prev_track (wrap to last) then advance (wrap to first) must return to track_00"
    );
}
