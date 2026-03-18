use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub volume: f32,
    pub speed: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { volume: 0.8, speed: 1.0 }
    }
}

pub fn load_settings(path: &Path) -> Settings {
    match std::fs::read_to_string(path) {
        Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Settings::default(),
        Err(_) => Settings::default(),
    }
}

pub fn save_settings(settings: &Settings, path: &Path) -> Result<(), std::io::Error> {
    let contents = toml::to_string(settings)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)
}

pub fn config_dir() -> PathBuf {
    if let Ok(base) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(base).join("rhythmgrid")
    } else {
        home_dir().join(".config").join("rhythmgrid")
    }
}

pub fn data_dir() -> PathBuf {
    if let Ok(base) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(base).join("rhythmgrid")
    } else {
        home_dir().join(".local").join("share").join("rhythmgrid")
    }
}

fn home_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
    } else {
        PathBuf::from("/")
    }
}
