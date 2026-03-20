// @spec-tags: core,config,filesystem
// @invariants: config_dir() returns XDG_CONFIG_HOME/rhythmgrid or ~/.config/rhythmgrid; data_dir() returns XDG_DATA_HOME/rhythmgrid or ~/.local/share/rhythmgrid; no directory is created on disk; both paths always end with "rhythmgrid"
// @build: 30

use rhythm_grid::config::{config_dir, data_dir};
use std::path::PathBuf;
use std::sync::Mutex;

// Serialize all env-var-mutating tests to avoid data races when tests run in parallel.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Temporarily sets an env var, runs `f`, then restores the original value.
fn with_env<F: FnOnce()>(key: &str, value: &str, f: F) {
    let _guard = ENV_LOCK.lock().unwrap();
    let old = std::env::var(key).ok();
    unsafe { std::env::set_var(key, value) };
    f();
    match old {
        Some(v) => unsafe { std::env::set_var(key, v) },
        None    => unsafe { std::env::remove_var(key) },
    }
}

/// Temporarily removes an env var, runs `f`, then restores the original value.
fn without_env<F: FnOnce()>(key: &str, f: F) {
    let _guard = ENV_LOCK.lock().unwrap();
    let old = std::env::var(key).ok();
    unsafe { std::env::remove_var(key) };
    f();
    if let Some(v) = old {
        unsafe { std::env::set_var(key, v) };
    }
}

// ── config_dir ────────────────────────────────────────────────────────────────

#[test]
fn config_dir_uses_xdg_config_home() {
    with_env("XDG_CONFIG_HOME", "/tmp/test_xdg_cfg_b30", || {
        let result = config_dir();
        assert_eq!(
            result,
            PathBuf::from("/tmp/test_xdg_cfg_b30/rhythmgrid"),
            "config_dir() must return XDG_CONFIG_HOME/rhythmgrid"
        );
    });
}

#[test]
fn config_dir_falls_back_to_dot_config() {
    without_env("XDG_CONFIG_HOME", || {
        let result = config_dir();
        let result_str = result.to_string_lossy();
        assert!(
            result_str.contains(".config/rhythmgrid"),
            "config_dir() without XDG_CONFIG_HOME must end with .config/rhythmgrid, got: {}",
            result_str
        );
    });
}

#[test]
fn config_dir_appends_rhythmgrid_suffix() {
    with_env("XDG_CONFIG_HOME", "/tmp/any_base_b30", || {
        let result = config_dir();
        assert_eq!(
            result.file_name().and_then(|n| n.to_str()),
            Some("rhythmgrid"),
            "config_dir() must always end with 'rhythmgrid', got: {:?}", result
        );
    });
}

// ── data_dir ──────────────────────────────────────────────────────────────────

#[test]
fn data_dir_uses_xdg_data_home() {
    with_env("XDG_DATA_HOME", "/tmp/test_xdg_data_b30", || {
        let result = data_dir();
        assert_eq!(
            result,
            PathBuf::from("/tmp/test_xdg_data_b30/rhythmgrid"),
            "data_dir() must return XDG_DATA_HOME/rhythmgrid"
        );
    });
}

#[test]
fn data_dir_falls_back_to_dot_local_share() {
    without_env("XDG_DATA_HOME", || {
        let result = data_dir();
        let result_str = result.to_string_lossy();
        assert!(
            result_str.contains(".local/share/rhythmgrid"),
            "data_dir() without XDG_DATA_HOME must end with .local/share/rhythmgrid, got: {}",
            result_str
        );
    });
}

#[test]
fn data_dir_appends_rhythmgrid_suffix() {
    with_env("XDG_DATA_HOME", "/tmp/any_data_base_b30", || {
        let result = data_dir();
        assert_eq!(
            result.file_name().and_then(|n| n.to_str()),
            Some("rhythmgrid"),
            "data_dir() must always end with 'rhythmgrid', got: {:?}", result
        );
    });
}
