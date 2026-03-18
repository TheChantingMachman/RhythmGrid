// @spec-tags: config.load_save
// @invariants: Settings struct defaults, TOML serialization round-trip, load/save behavior, missing file fallback
// @build: 36

use rhythm_grid::config::{Settings, load_settings, save_settings};
use std::path::Path;

#[test]
fn settings_has_default_impl() {
    let _s = Settings::default();
}

#[test]
fn default_volume_is_0_8() {
    assert_eq!(Settings::default().volume, 0.8f32);
}

#[test]
fn default_speed_is_1_0() {
    assert_eq!(Settings::default().speed, 1.0f32);
}

#[test]
fn load_missing_file_returns_defaults() {
    let s = load_settings(Path::new("/tmp/rhythmgrid_test_nonexistent_b36.toml"));
    assert_eq!(s, Settings::default());
}

#[test]
fn save_then_load_round_trips() {
    let path = std::env::temp_dir().join("rhythmgrid_b36_round_trip.toml");
    let settings = Settings { volume: 0.3, speed: 1.7 };
    save_settings(&settings, &path);
    let loaded = load_settings(&path);
    assert_eq!(loaded, settings);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn save_creates_file() {
    let path = std::env::temp_dir().join("rhythmgrid_b36_save_creates.toml");
    let settings = Settings::default();
    save_settings(&settings, &path);
    assert!(path.exists());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn save_writes_valid_toml() {
    let path = std::env::temp_dir().join("rhythmgrid_b36_valid_toml.toml");
    let settings = Settings::default();
    save_settings(&settings, &path);
    let contents = std::fs::read_to_string(&path).expect("file should exist after save");
    assert!(contents.contains("volume"));
    assert!(contents.contains("speed"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn load_reads_toml_fields() {
    let path = std::env::temp_dir().join("rhythmgrid_b36_load_reads.toml");
    std::fs::write(&path, "volume = 0.4\nspeed = 1.2\n").expect("write temp toml");
    let s = load_settings(&path);
    assert_eq!(s.volume, 0.4f32);
    assert_eq!(s.speed, 1.2f32);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn settings_derives_debug() {
    let s = format!("{:?}", Settings::default());
    assert!(!s.is_empty());
}

#[test]
fn settings_derives_partial_eq() {
    assert_eq!(Settings::default(), Settings::default());
}
