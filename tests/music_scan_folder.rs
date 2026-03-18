// @spec-tags: music.scan_folder
// @invariants: scan_folder(path:&Path)->Vec<PathBuf> scans non-recursively for mp3/wav/flac/ogg (case-insensitive); returns empty Vec for missing/empty dirs; returns full absolute paths; excludes unsupported and extensionless files
// @build: 35

use rhythm_grid::music::scan_folder;
use std::fs;
use std::path::Path;

fn make_temp_dir(suffix: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("rhythmgrid_scan_test_{}", suffix));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("failed to create temp dir");
    dir
}

// ── Supported formats found ───────────────────────────────────────────────────

#[test]
fn scan_folder_finds_mp3_files() {
    let dir = make_temp_dir("mp3");
    fs::write(dir.join("track.mp3"), b"").unwrap();
    let results = scan_folder(&dir);
    assert_eq!(results.len(), 1);
    assert!(results[0].file_name().unwrap().to_str().unwrap().ends_with(".mp3"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn scan_folder_finds_wav_files() {
    let dir = make_temp_dir("wav");
    fs::write(dir.join("sound.wav"), b"").unwrap();
    let results = scan_folder(&dir);
    assert_eq!(results.len(), 1);
    assert!(results[0].file_name().unwrap().to_str().unwrap().ends_with(".wav"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn scan_folder_finds_flac_files() {
    let dir = make_temp_dir("flac");
    fs::write(dir.join("lossless.flac"), b"").unwrap();
    let results = scan_folder(&dir);
    assert_eq!(results.len(), 1);
    assert!(results[0].file_name().unwrap().to_str().unwrap().ends_with(".flac"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn scan_folder_finds_ogg_files() {
    let dir = make_temp_dir("ogg");
    fs::write(dir.join("audio.ogg"), b"").unwrap();
    let results = scan_folder(&dir);
    assert_eq!(results.len(), 1);
    assert!(results[0].file_name().unwrap().to_str().unwrap().ends_with(".ogg"));
    fs::remove_dir_all(&dir).ok();
}

// ── Unsupported formats excluded ──────────────────────────────────────────────

#[test]
fn scan_folder_excludes_unsupported_extension() {
    let dir = make_temp_dir("unsupported");
    fs::write(dir.join("video.mp4"), b"").unwrap();
    fs::write(dir.join("doc.txt"), b"").unwrap();
    let results = scan_folder(&dir);
    assert!(results.is_empty());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn scan_folder_excludes_files_with_no_extension() {
    let dir = make_temp_dir("noext");
    fs::write(dir.join("audiofile_no_ext"), b"").unwrap();
    let results = scan_folder(&dir);
    assert!(results.is_empty());
    fs::remove_dir_all(&dir).ok();
}

// ── Empty and missing directories ─────────────────────────────────────────────

#[test]
fn scan_folder_empty_dir_returns_empty_vec() {
    let dir = make_temp_dir("empty");
    let results = scan_folder(&dir);
    assert!(results.is_empty());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn scan_folder_nonexistent_dir_returns_empty_vec() {
    let path = Path::new("/tmp/rhythmgrid_scan_definitely_missing_dir_xyz999");
    let results = scan_folder(path);
    assert!(results.is_empty());
}

// ── Full paths ────────────────────────────────────────────────────────────────

#[test]
fn scan_folder_returns_full_paths() {
    let dir = make_temp_dir("fullpath");
    fs::write(dir.join("song.mp3"), b"").unwrap();
    let results = scan_folder(&dir);
    assert_eq!(results.len(), 1);
    assert!(results[0].is_absolute());
    fs::remove_dir_all(&dir).ok();
}

// ── Case-insensitive extension matching ───────────────────────────────────────

#[test]
fn scan_folder_case_insensitive_extension_matching() {
    let dir = make_temp_dir("caseinsensitive");
    fs::write(dir.join("track.MP3"), b"").unwrap();
    fs::write(dir.join("other.Wav"), b"").unwrap();
    let results = scan_folder(&dir);
    assert_eq!(results.len(), 2);
    fs::remove_dir_all(&dir).ok();
}
