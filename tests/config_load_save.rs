// @spec-tags: core,config
// @invariants: Settings struct defaults (volume, speed, music_folder, theme, shuffle, window_width, window_height, window_x, window_y), TOML serialization round-trip, load/save behavior, missing file fallback, music_folder optional field handling, theme and shuffle serde defaults, window size and position fields, window field serde defaults
// @build: 90

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
fn default_music_folder_is_none() {
    assert_eq!(Settings::default().music_folder, None);
}

#[test]
fn load_missing_file_returns_defaults() {
    let s = load_settings(Path::new("/tmp/rhythmgrid_test_nonexistent_b36.toml"));
    assert_eq!(s, Settings::default());
}

#[test]
fn save_then_load_round_trips() {
    let path = std::env::temp_dir().join("rhythmgrid_b45_round_trip.toml");
    let settings = Settings { volume: 0.3, speed: 1.7, ..Settings::default() };
    save_settings(&settings, &path);
    let loaded = load_settings(&path);
    assert_eq!(loaded, settings);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn save_then_load_with_music_folder() {
    let path = std::env::temp_dir().join("rhythmgrid_b45_round_trip_music_folder.toml");
    let settings = Settings { volume: 0.5, speed: 1.0, music_folder: Some("/music".to_string()), ..Settings::default() };
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
fn save_writes_music_folder_to_toml() {
    let path = std::env::temp_dir().join("rhythmgrid_b45_save_music_folder.toml");
    let settings = Settings { volume: 0.8, speed: 1.0, music_folder: Some("/path/to/music".to_string()), ..Settings::default() };
    save_settings(&settings, &path);
    let contents = std::fs::read_to_string(&path).expect("file should exist after save");
    assert!(contents.contains("music_folder"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn load_reads_toml_fields() {
    let path = std::env::temp_dir().join("rhythmgrid_b45_load_reads.toml");
    std::fs::write(&path, "volume = 0.4\nspeed = 1.2\n").expect("write temp toml");
    let s = load_settings(&path);
    assert_eq!(s.volume, 0.4f32);
    assert_eq!(s.speed, 1.2f32);
    assert_eq!(s.music_folder, None);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn load_toml_with_music_folder() {
    let path = std::env::temp_dir().join("rhythmgrid_b45_load_music_folder.toml");
    std::fs::write(&path, "volume = 0.8\nspeed = 1.0\nmusic_folder = \"/home/user/music\"\n").expect("write temp toml");
    let s = load_settings(&path);
    assert_eq!(s.music_folder, Some("/home/user/music".to_string()));
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

#[test]
fn default_theme_is_default() {
    assert_eq!(Settings::default().theme, "Default".to_string());
}

#[test]
fn default_shuffle_is_false() {
    assert_eq!(Settings::default().shuffle, false);
}

#[test]
fn save_then_load_round_trips_with_theme_and_shuffle() {
    let path = std::env::temp_dir().join("rhythmgrid_b87_round_trip_theme_shuffle.toml");
    let settings = Settings {
        volume: 0.6,
        speed: 1.5,
        theme: "Water".to_string(),
        shuffle: true,
        ..Settings::default()
    };
    save_settings(&settings, &path);
    let loaded = load_settings(&path);
    assert_eq!(loaded, settings);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn load_old_toml_without_theme_shuffle_uses_defaults() {
    let path = std::env::temp_dir().join("rhythmgrid_b87_old_toml.toml");
    std::fs::write(&path, "volume = 0.5\nspeed = 1.2\n").expect("write temp toml");
    let s = load_settings(&path);
    assert_eq!(s.theme, "Default".to_string());
    assert_eq!(s.shuffle, false);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn save_writes_theme_and_shuffle_to_toml() {
    let path = std::env::temp_dir().join("rhythmgrid_b87_save_theme_shuffle.toml");
    let settings = Settings {
        volume: 0.8,
        speed: 1.0,
        theme: "Debug".to_string(),
        shuffle: true,
        ..Settings::default()
    };
    save_settings(&settings, &path);
    let contents = std::fs::read_to_string(&path).expect("file should exist after save");
    assert!(contents.contains("theme"));
    assert!(contents.contains("shuffle"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn load_toml_with_explicit_theme_and_shuffle() {
    let path = std::env::temp_dir().join("rhythmgrid_b87_explicit_theme_shuffle.toml");
    std::fs::write(&path, "volume = 0.8\nspeed = 1.0\ntheme = \"Water\"\nshuffle = true\n")
        .expect("write temp toml");
    let s = load_settings(&path);
    assert_eq!(s.theme, "Water".to_string());
    assert_eq!(s.shuffle, true);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn default_window_width_is_1000() {
    assert_eq!(Settings::default().window_width, 1000u32);
}

#[test]
fn default_window_height_is_800() {
    assert_eq!(Settings::default().window_height, 800u32);
}

#[test]
fn default_window_x_is_none() {
    assert_eq!(Settings::default().window_x, None);
}

#[test]
fn default_window_y_is_none() {
    assert_eq!(Settings::default().window_y, None);
}

#[test]
fn load_old_toml_without_window_fields_uses_defaults() {
    let path = std::env::temp_dir().join("rhythmgrid_b90_old_toml_window.toml");
    std::fs::write(&path, "volume = 0.5\nspeed = 1.2\n").expect("write temp toml");
    let s = load_settings(&path);
    assert_eq!(s.window_width, 1000u32);
    assert_eq!(s.window_height, 800u32);
    assert_eq!(s.window_x, None);
    assert_eq!(s.window_y, None);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn save_then_load_round_trips_with_window_fields() {
    let path = std::env::temp_dir().join("rhythmgrid_b90_round_trip_window.toml");
    let settings = Settings {
        window_width: 1920,
        window_height: 1080,
        window_x: Some(100),
        window_y: Some(200),
        ..Settings::default()
    };
    save_settings(&settings, &path);
    let loaded = load_settings(&path);
    assert_eq!(loaded, settings);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn save_writes_window_fields_to_toml() {
    let path = std::env::temp_dir().join("rhythmgrid_b90_save_window_fields.toml");
    let settings = Settings {
        window_width: 1920,
        window_height: 1080,
        window_x: Some(50),
        window_y: Some(75),
        ..Settings::default()
    };
    save_settings(&settings, &path);
    let contents = std::fs::read_to_string(&path).expect("file should exist after save");
    assert!(contents.contains("window_width"));
    assert!(contents.contains("window_height"));
    assert!(contents.contains("window_x"));
    assert!(contents.contains("window_y"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn load_toml_with_explicit_window_fields() {
    let path = std::env::temp_dir().join("rhythmgrid_b90_explicit_window.toml");
    std::fs::write(
        &path,
        "volume = 0.8\nspeed = 1.0\nwindow_width = 1920\nwindow_height = 1080\nwindow_x = 50\nwindow_y = 75\n",
    )
    .expect("write temp toml");
    let s = load_settings(&path);
    assert_eq!(s.window_width, 1920u32);
    assert_eq!(s.window_height, 1080u32);
    assert_eq!(s.window_x, Some(50));
    assert_eq!(s.window_y, Some(75));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn window_position_none_omitted_from_toml() {
    let path = std::env::temp_dir().join("rhythmgrid_b90_window_none_omit.toml");
    let settings = Settings {
        window_x: None,
        window_y: None,
        ..Settings::default()
    };
    save_settings(&settings, &path);
    let contents = std::fs::read_to_string(&path).expect("file should exist after save");
    assert!(!contents.contains("window_x"));
    assert!(!contents.contains("window_y"));
    let _ = std::fs::remove_file(&path);
}
