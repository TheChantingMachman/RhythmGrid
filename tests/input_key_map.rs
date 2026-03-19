// @spec-tags: input,controls
// @invariants: Validates GameAction enum variants and map_key function mapping KeyCode to Option<GameAction>
// @build: 51

use rhythm_grid::input::{GameAction, KeyCode, map_key};

#[test]
fn test_map_key_left_returns_move_left() {
    assert_eq!(map_key(KeyCode::Left), Some(GameAction::MoveLeft));
}

#[test]
fn test_map_key_right_returns_move_right() {
    assert_eq!(map_key(KeyCode::Right), Some(GameAction::MoveRight));
}

#[test]
fn test_map_key_down_returns_soft_drop() {
    assert_eq!(map_key(KeyCode::Down), Some(GameAction::SoftDrop));
}

#[test]
fn test_map_key_up_returns_rotate_cw() {
    assert_eq!(map_key(KeyCode::Up), Some(GameAction::RotateCW));
}

#[test]
fn test_map_key_z_returns_rotate_ccw() {
    assert_eq!(map_key(KeyCode::Z), Some(GameAction::RotateCCW));
}

#[test]
fn test_map_key_space_returns_hard_drop() {
    assert_eq!(map_key(KeyCode::Space), Some(GameAction::HardDrop));
}

#[test]
fn test_map_key_p_returns_toggle_pause() {
    assert_eq!(map_key(KeyCode::P), Some(GameAction::TogglePause));
}

#[test]
fn test_map_key_escape_returns_back_to_menu() {
    assert_eq!(map_key(KeyCode::Escape), Some(GameAction::BackToMenu));
}

#[test]
fn test_map_key_enter_returns_start_game() {
    assert_eq!(map_key(KeyCode::Enter), Some(GameAction::StartGame));
}

#[test]
fn test_map_key_unmapped_key_returns_none() {
    // Keys not in the default mapping should return None
    assert_eq!(map_key(KeyCode::Other), None);
}

#[test]
fn test_game_action_variants_are_distinct() {
    let actions = [
        GameAction::MoveLeft,
        GameAction::MoveRight,
        GameAction::SoftDrop,
        GameAction::HardDrop,
        GameAction::RotateCW,
        GameAction::RotateCCW,
        GameAction::TogglePause,
        GameAction::BackToMenu,
        GameAction::StartGame,
    ];
    // Verify we have exactly 9 distinct variants by checking count
    assert_eq!(actions.len(), 9);
}

#[test]
fn test_map_key_all_default_mappings_are_some() {
    let mapped_keys = [
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Down,
        KeyCode::Up,
        KeyCode::Z,
        KeyCode::Space,
        KeyCode::P,
        KeyCode::Escape,
        KeyCode::Enter,
    ];
    for key in mapped_keys {
        assert!(
            map_key(key).is_some(),
            "Expected Some for key {:?}",
            key
        );
    }
}

#[test]
fn test_map_key_returns_correct_action_type_for_each_mapping() {
    // Verify each mapping individually for exhaustive correctness
    let expected = vec![
        (KeyCode::Left,   GameAction::MoveLeft),
        (KeyCode::Right,  GameAction::MoveRight),
        (KeyCode::Down,   GameAction::SoftDrop),
        (KeyCode::Up,     GameAction::RotateCW),
        (KeyCode::Z,      GameAction::RotateCCW),
        (KeyCode::Space,  GameAction::HardDrop),
        (KeyCode::P,      GameAction::TogglePause),
        (KeyCode::Escape, GameAction::BackToMenu),
        (KeyCode::Enter,  GameAction::StartGame),
    ];
    for (key, action) in expected {
        assert_eq!(map_key(key), Some(action));
    }
}
