use std::path::{Path, PathBuf};
use crate::audio::SUPPORTED_FORMATS;

#[derive(Debug, Clone, PartialEq)]
pub struct NowPlaying {
    pub filename: String,
    pub duration: f32,
    pub elapsed: f32,
}

pub fn scan_folder(path: &Path) -> Vec<PathBuf> {
    let entries = match std::fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut result = Vec::new();
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            if SUPPORTED_FORMATS.contains(&ext_lower.as_str()) {
                result.push(p);
            }
        }
    }
    result
}
