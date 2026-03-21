// @spec-tags: core,game
// @invariants: hold_piece() swaps active piece with held piece; hold blocked when can_hold=false; can_hold=false after successful hold; held stored as TetrominoType; on failed spawn hold is rejected and all state restored; no GameOver on spawn failure during hold
// @build: 72

use rhythm_grid::game::{GameSession, GameState, ActivePiece};
use rhythm_grid::grid::{CellState, Grid};
use rhythm_grid::pieces::TetrominoType;

// --- hold_piece blocked when can_hold is false ---

#[test]
fn hold_piece_returns_false_when_can_hold_is_false() {
    let mut session = GameSession::new();
    session.can_hold = false;
    let result = session.hold_piece();
    assert!(!result, "hold_piece must return false when can_hold is false");
}

#[test]
fn hold_piece_no_state_change_when_can_hold_is_false() {
    let mut session = GameSession::new();
    session.can_hold = false;
    let original_held = session.held_piece;
    let original_type = session.active_piece.piece_type;
    session.hold_piece();
    assert_eq!(session.held_piece, original_held, "held_piece must not change when can_hold is false");
    assert_eq!(session.active_piece.piece_type, original_type, "active_piece must not change when can_hold is false");
}

#[test]
fn hold_piece_can_hold_stays_false_when_blocked() {
    let mut session = GameSession::new();
    session.can_hold = false;
    session.hold_piece();
    assert!(!session.can_hold, "can_hold must remain false after a blocked hold attempt");
}

// --- hold_piece with no previously held piece ---

#[test]
fn hold_piece_returns_true_when_held_is_none_and_spawn_succeeds() {
    let mut session = GameSession::new();
    assert_eq!(session.held_piece, None);
    let result = session.hold_piece();
    assert!(result, "hold_piece must return true when held is None and spawn succeeds");
}

#[test]
fn hold_piece_stores_current_type_in_held_when_held_is_none() {
    let mut session = GameSession::new();
    let original_type = session.active_piece.piece_type;
    session.held_piece = None;
    session.hold_piece();
    assert_eq!(
        session.held_piece,
        Some(original_type),
        "held_piece must store the previous active piece type"
    );
}

#[test]
fn hold_piece_sets_can_hold_false_on_success_from_none() {
    let mut session = GameSession::new();
    session.held_piece = None;
    session.hold_piece();
    assert!(!session.can_hold, "can_hold must be false after a successful hold");
}

#[test]
fn hold_piece_spawns_new_active_piece_when_held_is_none() {
    let mut session = GameSession::new();
    let original_type = session.active_piece.piece_type;
    session.held_piece = None;
    session.hold_piece();
    // New active piece spawns near top of grid
    assert!(session.active_piece.row <= 1, "new active piece must spawn near top row");
    // The new active piece is different from the held piece (was stored)
    assert_eq!(session.held_piece, Some(original_type));
}

#[test]
fn hold_piece_new_active_piece_has_rotation_zero_when_spawned_from_bag() {
    let mut session = GameSession::new();
    session.held_piece = None;
    session.hold_piece();
    assert_eq!(session.active_piece.rotation, 0, "spawned piece must start at rotation 0");
}

#[test]
fn hold_piece_resets_gravity_accumulator_on_success() {
    let mut session = GameSession::new();
    session.gravity_accumulator_ms = 500;
    session.hold_piece();
    assert_eq!(session.gravity_accumulator_ms, 0, "gravity_accumulator_ms must reset to 0 on successful hold");
}

#[test]
fn hold_piece_resets_lock_delay_on_success() {
    let mut session = GameSession::new();
    session.lock_delay_active = true;
    session.lock_delay_accumulator_ms = 200;
    session.lock_delay_resets = 5;
    session.hold_piece();
    assert!(!session.lock_delay_active, "lock_delay_active must be false after successful hold");
    assert_eq!(session.lock_delay_accumulator_ms, 0, "lock_delay_accumulator_ms must reset after successful hold");
    assert_eq!(session.lock_delay_resets, 0, "lock_delay_resets must reset after successful hold");
}

// --- hold_piece with previously held piece (swap) ---

#[test]
fn hold_piece_returns_true_when_held_is_some_and_spawn_succeeds() {
    let mut session = GameSession::new();
    session.held_piece = Some(TetrominoType::O);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 5,
        col: 4,
    };
    let result = session.hold_piece();
    assert!(result, "hold_piece must return true when swap spawn succeeds");
}

#[test]
fn hold_piece_active_becomes_held_type_on_swap() {
    let mut session = GameSession::new();
    let held_type = TetrominoType::O;
    session.held_piece = Some(held_type);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 5,
        col: 4,
    };
    session.hold_piece();
    assert_eq!(
        session.active_piece.piece_type,
        held_type,
        "active piece type must match the previously held type after swap"
    );
}

#[test]
fn hold_piece_held_becomes_active_type_on_swap() {
    let mut session = GameSession::new();
    let active_type = TetrominoType::I;
    session.held_piece = Some(TetrominoType::O);
    session.active_piece = ActivePiece {
        piece_type: active_type,
        rotation: 0,
        row: 5,
        col: 4,
    };
    session.hold_piece();
    assert_eq!(
        session.held_piece,
        Some(active_type),
        "held_piece must store the previously active piece type after swap"
    );
}

#[test]
fn hold_piece_swapped_piece_spawns_at_rotation_zero() {
    let mut session = GameSession::new();
    session.held_piece = Some(TetrominoType::T);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 2,
        row: 5,
        col: 4,
    };
    session.hold_piece();
    assert_eq!(session.active_piece.rotation, 0, "swapped-in piece must spawn at rotation 0");
}

#[test]
fn hold_piece_sets_can_hold_false_on_swap_success() {
    let mut session = GameSession::new();
    session.held_piece = Some(TetrominoType::O);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 5,
        col: 4,
    };
    session.hold_piece();
    assert!(!session.can_hold, "can_hold must be false after a successful swap");
}

// --- hold_piece rejected when try_spawn fails (CRITICAL: no GameOver) ---

fn fill_top_rows(session: &mut GameSession) {
    // Fill rows 0 and 1 with occupied cells — blocks any piece from spawning
    for row in 0..2 {
        for col in 0..10 {
            session.grid.cells[row][col] = CellState::Occupied(0);
        }
    }
}

#[test]
fn hold_piece_rejected_when_spawn_fails() {
    let mut session = GameSession::new();
    let original_held_type = TetrominoType::O;
    let original_active_type = TetrominoType::I;
    session.held_piece = Some(original_held_type);
    session.active_piece = ActivePiece {
        piece_type: original_active_type,
        rotation: 0,
        row: 19,
        col: 4,
    };
    fill_top_rows(&mut session);
    let result = session.hold_piece();
    assert!(!result, "hold_piece must return false when spawn fails");
    assert_eq!(
        session.state,
        GameState::Playing,
        "state must remain Playing when hold spawn fails"
    );
    assert_eq!(
        session.held_piece,
        Some(original_held_type),
        "held_piece must be unchanged when hold is rejected"
    );
    assert_eq!(
        session.active_piece.piece_type,
        original_active_type,
        "active_piece type must be unchanged when hold is rejected"
    );
}

#[test]
fn hold_piece_can_hold_unchanged_when_spawn_fails() {
    let mut session = GameSession::new();
    session.held_piece = Some(TetrominoType::O);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    fill_top_rows(&mut session);
    // can_hold is true — after rejection it must remain true (not set to false)
    session.hold_piece();
    assert!(session.can_hold, "can_hold must remain true after a rejected hold");
}

#[test]
fn hold_piece_no_game_over_when_spawn_fails_with_held_none() {
    let mut session = GameSession::new();
    // held is None — hold attempts to spawn next from bag, which should also fail
    assert_eq!(session.held_piece, None);
    fill_top_rows(&mut session);
    let result = session.hold_piece();
    // Spawn fails → hold rejected → state stays Playing (never GameOver from hold)
    assert!(!result, "hold_piece must return false when spawn fails with held=None");
    assert_eq!(
        session.state,
        GameState::Playing,
        "state must not become GameOver when hold spawn fails"
    );
}

#[test]
fn hold_piece_active_piece_row_unchanged_when_spawn_fails() {
    let mut session = GameSession::new();
    let original_held_type = TetrominoType::T;
    session.held_piece = Some(original_held_type);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::S,
        rotation: 0,
        row: 19,
        col: 3,
    };
    fill_top_rows(&mut session);
    session.hold_piece();
    assert_eq!(
        session.active_piece.row,
        19,
        "active_piece row must be restored when hold is rejected"
    );
    assert_eq!(
        session.active_piece.col,
        3,
        "active_piece col must be restored when hold is rejected"
    );
}
