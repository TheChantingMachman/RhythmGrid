// @spec-tags: music.now_playing
// @invariants: NowPlaying struct has pub fields filename:String, duration:f32, elapsed:f32; derives Debug, Clone, PartialEq; constructed via struct literal
// @build: 35

use rhythm_grid::music::NowPlaying;

// ── Field types and construction ──────────────────────────────────────────────

#[test]
fn now_playing_filename_field_is_string() {
    let np = NowPlaying {
        filename: String::from("track.mp3"),
        duration: 180.0,
        elapsed: 0.0,
    };
    assert_eq!(np.filename, "track.mp3");
}

#[test]
fn now_playing_duration_field_is_f32() {
    let np = NowPlaying {
        filename: String::from("a.flac"),
        duration: 300.5,
        elapsed: 0.0,
    };
    let _: f32 = np.duration;
    assert_eq!(np.duration, 300.5_f32);
}

#[test]
fn now_playing_elapsed_field_is_f32() {
    let np = NowPlaying {
        filename: String::from("b.wav"),
        duration: 120.0,
        elapsed: 45.25,
    };
    let _: f32 = np.elapsed;
    assert_eq!(np.elapsed, 45.25_f32);
}

// ── Debug derive ──────────────────────────────────────────────────────────────

#[test]
fn now_playing_debug_contains_filename() {
    let np = NowPlaying {
        filename: String::from("song.ogg"),
        duration: 200.0,
        elapsed: 10.0,
    };
    let s = format!("{:?}", np);
    assert!(s.contains("song.ogg"));
}

// ── Clone and PartialEq derives ───────────────────────────────────────────────

#[test]
fn now_playing_clone_produces_equal_value() {
    let np = NowPlaying {
        filename: String::from("c.mp3"),
        duration: 90.0,
        elapsed: 5.0,
    };
    let cloned = np.clone();
    assert_eq!(np, cloned);
}

#[test]
fn now_playing_partial_eq_differs_on_elapsed() {
    let a = NowPlaying {
        filename: String::from("d.mp3"),
        duration: 120.0,
        elapsed: 10.0,
    };
    let b = NowPlaying {
        filename: String::from("d.mp3"),
        duration: 120.0,
        elapsed: 20.0,
    };
    assert_ne!(a, b);
}
