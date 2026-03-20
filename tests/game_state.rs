// @spec-tags: core,game,state
// @invariants: GameState enum has Menu/Playing/Paused/GameOver variants; Default=Menu; transition(&self, target) -> bool with valid transitions: Menu->Playing, Playing->Paused, Paused->Playing, Playing->GameOver, GameOver->Menu; derives Debug, Clone, Copy, PartialEq
// @build: 35

use rhythm_grid::game::GameState;

// ── Default ───────────────────────────────────────────────────────────────────

#[test]
fn default_is_menu() {
    assert_eq!(GameState::default(), GameState::Menu);
}

// ── Valid transitions ─────────────────────────────────────────────────────────

#[test]
fn menu_to_playing_is_valid() {
    assert!(GameState::Menu.transition(GameState::Playing));
}

#[test]
fn playing_to_paused_is_valid() {
    assert!(GameState::Playing.transition(GameState::Paused));
}

#[test]
fn paused_to_playing_is_valid() {
    assert!(GameState::Paused.transition(GameState::Playing));
}

#[test]
fn playing_to_game_over_is_valid() {
    assert!(GameState::Playing.transition(GameState::GameOver));
}

#[test]
fn game_over_to_menu_is_valid() {
    assert!(GameState::GameOver.transition(GameState::Menu));
}

// ── Invalid transitions ───────────────────────────────────────────────────────

#[test]
fn menu_to_paused_is_invalid() {
    assert!(!GameState::Menu.transition(GameState::Paused));
}

#[test]
fn menu_to_game_over_is_invalid() {
    assert!(!GameState::Menu.transition(GameState::GameOver));
}

#[test]
fn paused_to_game_over_is_invalid() {
    assert!(!GameState::Paused.transition(GameState::GameOver));
}

#[test]
fn game_over_to_playing_is_invalid() {
    assert!(!GameState::GameOver.transition(GameState::Playing));
}

#[test]
fn playing_to_menu_is_invalid() {
    assert!(!GameState::Playing.transition(GameState::Menu));
}

#[test]
fn paused_to_menu_is_invalid() {
    assert!(!GameState::Paused.transition(GameState::Menu));
}

// ── Derives ───────────────────────────────────────────────────────────────────

#[test]
fn debug_format_contains_variant_name() {
    let s = format!("{:?}", GameState::Menu);
    assert!(s.contains("Menu"));
}

#[test]
fn partial_eq_same_variant() {
    assert_eq!(GameState::Playing, GameState::Playing);
}

#[test]
fn partial_eq_different_variants() {
    assert_ne!(GameState::Menu, GameState::Playing);
}

#[test]
fn copy_allows_use_after_pass_by_value() {
    let state = GameState::Playing;
    // Copy: state is still usable after being passed by value to transition
    let _ = state.transition(GameState::Paused);
    assert_eq!(state, GameState::Playing);
}
