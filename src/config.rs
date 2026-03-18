use std::path::PathBuf;

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
