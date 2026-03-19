// Input mapping — pipeline-owned testable logic.
// Wiring to winit events lives in main.rs (co-authored).

/// Project-local KeyCode enum — decoupled from winit so tests run without a windowing dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Left,
    Right,
    Down,
    Up,
    Z,
    Space,
    P,
    Escape,
    Enter,
    /// Catch-all for keys with no game-action binding.
    Other,
}

/// Actions the game logic can respond to, independent of input source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameAction {
    MoveLeft,
    MoveRight,
    SoftDrop,
    HardDrop,
    RotateCW,
    RotateCCW,
    TogglePause,
    BackToMenu,
    StartGame,
}

/// Maps a key press to a game action, if one is bound.
pub fn map_key(key: KeyCode) -> Option<GameAction> {
    match key {
        KeyCode::Left    => Some(GameAction::MoveLeft),
        KeyCode::Right   => Some(GameAction::MoveRight),
        KeyCode::Down    => Some(GameAction::SoftDrop),
        KeyCode::Up      => Some(GameAction::RotateCW),
        KeyCode::Z       => Some(GameAction::RotateCCW),
        KeyCode::Space   => Some(GameAction::HardDrop),
        KeyCode::P       => Some(GameAction::TogglePause),
        KeyCode::Escape  => Some(GameAction::BackToMenu),
        KeyCode::Enter   => Some(GameAction::StartGame),
        _                => None,
    }
}
