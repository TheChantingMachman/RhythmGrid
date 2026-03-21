// @spec-tags: core,game,timing
// @invariants: LOCK_DELAY_MS=400 and MAX_LOCK_RESETS=15 constants; GameSession lock delay fields initialize to false/0/0; tick() starts lock delay when move_down fails; lock delay accumulates across ticks and expires after 400ms; move_horizontal/rotate reset accumulator and increment resets when lock_delay_active; hard_drop bypasses lock delay; resets exhausted causes immediate lock; GameSession::move_horizontal and rotate return bool matching free fn result
// @build: 72

use rhythm_grid::game::{
    tick, GameSession, GameState, TickResult, ActivePiece,
    LOCK_DELAY_MS, MAX_LOCK_RESETS,
};
use rhythm_grid::pieces::{TetrominoType, TETROMINO_TYPES};

// --- Constants ---

#[test]
fn lock_delay_ms_is_400() {
    assert_eq!(LOCK_DELAY_MS, 400u64);
}

#[test]
fn max_lock_resets_is_15() {
    assert_eq!(MAX_LOCK_RESETS, 15u32);
}

// --- GameSession field initialization ---

#[test]
fn game_session_new_lock_delay_active_is_false() {
    let session = GameSession::new();
    assert!(!session.lock_delay_active);
}

#[test]
fn game_session_new_lock_delay_accumulator_is_zero() {
    let session = GameSession::new();
    assert_eq!(session.lock_delay_accumulator_ms, 0u64);
}

#[test]
fn game_session_new_lock_delay_resets_is_zero() {
    let session = GameSession::new();
    assert_eq!(session.lock_delay_resets, 0u32);
}

// --- Lock delay activation via tick() ---

#[test]
fn lock_delay_activates_when_move_down_fails() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0);
    assert!(session.lock_delay_active);
}

#[test]
fn lock_delay_accumulator_initialized_to_zero_on_activation() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0);
    assert_eq!(session.lock_delay_accumulator_ms, 0);
}

#[test]
fn lock_delay_resets_initialized_to_zero_on_activation() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0);
    assert_eq!(session.lock_delay_resets, 0);
}

#[test]
fn lock_delay_tick_returns_nothing_on_activation() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    let result = tick(&mut session, 1.0);
    assert_eq!(result, TickResult::Nothing);
}

// --- Lock delay accumulation ---

#[test]
fn lock_delay_accumulates_dt_during_active_delay() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0); // activates lock delay
    tick(&mut session, 0.2); // 200ms into lock delay
    assert_eq!(session.lock_delay_accumulator_ms, 200);
}

#[test]
fn lock_delay_does_not_accumulate_gravity_while_active() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0); // activates lock delay, gravity_accumulator_ms resets to 0
    tick(&mut session, 0.2); // lock delay tick — should NOT add to gravity
    assert_eq!(session.gravity_accumulator_ms, 0);
}

#[test]
fn lock_delay_returns_nothing_before_expiry() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0); // lock delay active
    let result = tick(&mut session, 0.3); // 300ms < 400ms
    assert_eq!(result, TickResult::Nothing);
    assert!(session.lock_delay_active);
}

// --- Lock delay expiry ---

#[test]
fn lock_delay_expires_at_exactly_400ms() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0); // lock delay active
    let result = tick(&mut session, 0.4); // 400ms = LOCK_DELAY_MS
    assert!(
        matches!(result, TickResult::PieceLocked { .. }),
        "Expected PieceLocked at 400ms, got {:?}",
        result
    );
}

#[test]
fn lock_delay_expires_after_400ms_accumulation_across_ticks() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0); // lock delay active
    tick(&mut session, 0.2); // 200ms — still active
    let result = tick(&mut session, 0.2); // total 400ms — should expire
    assert!(
        matches!(result, TickResult::PieceLocked { .. }),
        "Expected PieceLocked after cumulative 400ms, got {:?}",
        result
    );
}

#[test]
fn lock_delay_cleared_after_expiry() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0); // lock delay active
    tick(&mut session, 0.4); // expires and locks
    assert!(!session.lock_delay_active);
    assert_eq!(session.lock_delay_accumulator_ms, 0);
    assert_eq!(session.lock_delay_resets, 0);
}

#[test]
fn lock_delay_expiry_spawns_new_piece() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0);
    tick(&mut session, 0.4);
    assert!(session.active_piece.row <= 1);
}

// --- GameSession::move_horizontal ---

#[test]
fn game_session_move_horizontal_returns_true_when_space_available() {
    let mut session = GameSession::new();
    // Spawn piece is at col 4 (middle), moving left by 1 should succeed.
    let result = session.move_horizontal(-1);
    assert!(result);
}

#[test]
fn game_session_move_horizontal_returns_false_at_boundary() {
    let mut session = GameSession::new();
    // Move far left until we hit the wall.
    for _ in 0..10 {
        session.move_horizontal(-1);
    }
    let result = session.move_horizontal(-1);
    assert!(!result);
}

#[test]
fn game_session_move_horizontal_resets_lock_delay_accumulator_on_success() {
    let mut session = GameSession::new();
    // Activate lock delay.
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0); // lock_delay_active=true, accumulator=0
    tick(&mut session, 0.2); // accumulator=200ms
    assert_eq!(session.lock_delay_accumulator_ms, 200);
    // Successful move resets accumulator.
    let moved = session.move_horizontal(-1);
    assert!(moved);
    assert_eq!(session.lock_delay_accumulator_ms, 0);
}

#[test]
fn game_session_move_horizontal_increments_lock_delay_resets_on_success() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0); // lock_delay_active=true, resets=0
    let moved = session.move_horizontal(-1);
    assert!(moved);
    assert_eq!(session.lock_delay_resets, 1);
}

#[test]
fn game_session_move_horizontal_no_reset_when_lock_delay_inactive() {
    let mut session = GameSession::new();
    // lock_delay_active is false by default
    session.lock_delay_accumulator_ms = 0;
    session.lock_delay_resets = 0;
    session.move_horizontal(-1);
    // Fields should be unchanged (not incremented when inactive)
    assert_eq!(session.lock_delay_resets, 0);
    assert_eq!(session.lock_delay_accumulator_ms, 0);
}

#[test]
fn game_session_move_horizontal_no_reset_when_resets_exhausted() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0); // lock_delay_active=true
    session.lock_delay_resets = MAX_LOCK_RESETS; // exhaust resets
    session.lock_delay_accumulator_ms = 200;
    // Move still succeeds (piece can still move) but no reset/increment.
    let moved = session.move_horizontal(-1);
    assert!(moved);
    assert_eq!(session.lock_delay_resets, MAX_LOCK_RESETS); // not incremented
    assert_eq!(session.lock_delay_accumulator_ms, 200); // not reset
}

// --- GameSession::rotate ---

#[test]
fn game_session_rotate_resets_lock_delay_accumulator_on_success() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::T,
        rotation: 0,
        row: 18,
        col: 4,
    };
    tick(&mut session, 1.0); // T at row 18 can fall to row 19, so this returns PieceMoved
    // Now place at row 19 and re-activate lock delay
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::T,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0); // lock_delay_active=true
    tick(&mut session, 0.2); // accumulator=200ms
    let rotated = session.rotate(true);
    if rotated {
        assert_eq!(session.lock_delay_accumulator_ms, 0);
    }
}

#[test]
fn game_session_rotate_increments_lock_delay_resets_on_success() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::T,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0); // lock_delay_active=true, resets=0
    let rotated = session.rotate(true);
    if rotated {
        assert_eq!(session.lock_delay_resets, 1);
    }
}

#[test]
fn game_session_rotate_no_reset_when_lock_delay_inactive() {
    let mut session = GameSession::new();
    // lock_delay_active is false
    session.lock_delay_resets = 0;
    session.lock_delay_accumulator_ms = 0;
    session.rotate(true);
    assert_eq!(session.lock_delay_resets, 0);
}

#[test]
fn game_session_rotate_no_reset_when_resets_exhausted() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::T,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0); // lock_delay_active=true
    session.lock_delay_resets = MAX_LOCK_RESETS;
    session.lock_delay_accumulator_ms = 200;
    session.rotate(true); // may or may not succeed, but no reset/increment
    assert_eq!(session.lock_delay_resets, MAX_LOCK_RESETS);
    assert_eq!(session.lock_delay_accumulator_ms, 200);
}

// --- Lock delay exhaustion via resets ---

#[test]
fn lock_delay_locks_when_resets_exhausted_on_next_tick() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0); // lock_delay_active=true
    // Exhaust resets manually.
    session.lock_delay_resets = MAX_LOCK_RESETS;
    // Even with < 500ms accumulated, the next tick should lock immediately.
    let result = tick(&mut session, 0.1); // only 100ms but resets exhausted
    assert!(
        matches!(result, TickResult::PieceLocked { .. }),
        "Expected PieceLocked when resets exhausted, got {:?}",
        result
    );
}

// --- GameSession::hard_drop ---

#[test]
fn game_session_hard_drop_returns_piece_locked() {
    let mut session = GameSession::new();
    let result = session.hard_drop();
    assert!(
        matches!(result, TickResult::PieceLocked { .. }),
        "Expected PieceLocked from hard_drop, got {:?}",
        result
    );
}

#[test]
fn game_session_hard_drop_clears_lock_delay_fields() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0); // lock_delay_active=true
    tick(&mut session, 0.2); // accumulator=200ms
    session.hard_drop();
    assert!(!session.lock_delay_active);
    assert_eq!(session.lock_delay_accumulator_ms, 0);
    assert_eq!(session.lock_delay_resets, 0);
}

#[test]
fn game_session_hard_drop_resets_gravity_accumulator() {
    let mut session = GameSession::new();
    session.gravity_accumulator_ms = 500;
    session.hard_drop();
    assert_eq!(session.gravity_accumulator_ms, 0);
}

#[test]
fn game_session_hard_drop_spawns_new_piece() {
    let mut session = GameSession::new();
    session.hard_drop();
    assert!(session.active_piece.row <= 1);
}

#[test]
fn game_session_hard_drop_bypasses_lock_delay_when_active() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0); // lock_delay_active=true, accumulator=0
    // hard_drop should lock immediately regardless of lock delay state.
    let result = session.hard_drop();
    assert!(
        matches!(result, TickResult::PieceLocked { .. }),
        "Expected PieceLocked from hard_drop even when lock delay active, got {:?}",
        result
    );
}

#[test]
fn game_session_hard_drop_state_remains_playing_when_grid_not_full() {
    let mut session = GameSession::new();
    session.hard_drop();
    assert_eq!(session.state, GameState::Playing);
}

// --- hard_drop resets can_hold ---

#[test]
fn game_session_hard_drop_resets_can_hold() {
    let mut session = GameSession::new();
    session.can_hold = false;
    session.hard_drop();
    assert!(session.can_hold, "can_hold must reset to true after hard_drop");
}
