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

// --- Playlist ---

fn shuffle_vec(files: &mut Vec<PathBuf>) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x123456789abcdef0);
    let mut state = seed;
    let n = files.len();
    for i in (1..n).rev() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let j = (state >> 33) as usize % (i + 1);
        files.swap(i, j);
    }
}

#[derive(Debug, Clone)]
pub struct Playlist {
    files: Vec<PathBuf>,
    original_order: Vec<PathBuf>,
    current_index: usize,
    shuffle: bool,
}

impl Playlist {
    pub fn new(files: Vec<PathBuf>) -> Self {
        let original_order = files.clone();
        Playlist {
            files,
            original_order,
            current_index: 0,
            shuffle: false,
        }
    }

    pub fn current(&self) -> Option<&PathBuf> {
        self.files.get(self.current_index)
    }

    pub fn advance(&mut self) {
        if self.files.is_empty() {
            return;
        }
        self.current_index = (self.current_index + 1) % self.files.len();
    }

    pub fn prev_track(&mut self) -> Option<&PathBuf> {
        if self.files.is_empty() {
            return None;
        }
        self.current_index = (self.current_index + self.files.len() - 1) % self.files.len();
        self.files.get(self.current_index)
    }

    pub fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        if self.shuffle {
            shuffle_vec(&mut self.files);
            self.current_index = 0;
        } else {
            self.files = self.original_order.clone();
            self.current_index = 0;
        }
    }
}
