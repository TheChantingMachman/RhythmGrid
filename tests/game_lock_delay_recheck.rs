// @spec-tags: core,game,bugfix
// @invariants: After move_horizontal/rotate succeeds during active lock delay, recheck is_valid_position at row+1; if space below exists set lock_delay_active=false without touching resets or accumulator; recheck is guarded by lock_delay_active; recheck runs even when resets are exhausted; gravity success path (PieceMoved) zeroes lock_delay_resets; gravity failure path (lock delay activation) does NOT zero lock_delay_resets; resets accumulate across recheck deactivation/reactivation cycles
// @build: 95

use rhythm_grid::game::{
    tick, GameSession, GameState, TickResult, ActivePiece,
    LOCK_DELAY_MS, MAX_LOCK_RESETS,
    is_valid_position,
};
use rhythm_grid::grid::{Grid, CellState, WIDTH, HEIGHT};
use rhythm_grid::pieces::{TetrominoType, TETROMINO_TYPES, PIECE_CELLS};

// Helper: create a GameSession with an I-piece (rot 0) resting on a partial ledge.
// I-piece rot 0 cells: (0,-1),(0,0),(0,1),(0,2) relative to pivot.
// Piece placed at row 18, col 6 → occupies row 18, cols 5,6,7,8.
// Row 19 cols 5..WIDTH are filled. Gravity fires on tick(1.0) and move_down fails
// because row 19 cols 5,6,7,8 are occupied → lock_delay_active = true.
// After move_horizontal(-5) → piece at col 1, occupies row 18 cols 0,1,2,3.
// Row 19 cols 0..4 are empty → is_valid_position at row+1 returns true.
fn setup_i_piece_on_partial_ledge() -> GameSession {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 18,
        col: 6,
    };
    // Fill right portion of row 19 (cols 5..WIDTH) to create the ledge.
    for col in 5..WIDTH {
        session.grid.cells[19][col] = CellState::Occupied(0);
    }
    session
}

// --- Test 1 ---

#[test]
fn move_horizontal_deactivates_lock_delay_when_piece_leaves_surface() {
    let mut session = setup_i_piece_on_partial_ledge();

    // Gravity fires (1000ms), move_down tries row 19 cols 5,6,7,8 (occupied) → fails.
    // Lock delay activates.
    tick(&mut session, 1.0);
    assert!(session.lock_delay_active, "setup: lock delay must be active after tick");

    // Verify setup: is_valid_position at row+1 is currently false (resting on ledge).
    let cells_before = PIECE_CELLS[TetrominoType::I as usize][0];
    assert!(
        !is_valid_position(&session.grid, &cells_before, session.active_piece.row + 1, session.active_piece.col),
        "setup: piece must be resting on surface before the move"
    );

    // Move left so piece hangs over empty cols 0..4 of row 19.
    let moved = session.move_horizontal(-5);
    assert!(moved, "move_horizontal must succeed");
    assert_eq!(session.active_piece.col, 1);

    // Recheck: space below (row 19 cols 0,1,2,3 are empty) → lock_delay_active deactivated.
    assert!(
        !session.lock_delay_active,
        "lock_delay_active must be false after piece moves off surface"
    );
}

// --- Test 2 ---

#[test]
fn move_horizontal_keeps_lock_delay_when_piece_still_on_surface() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };

    // I-piece at row 19 (bottom). Gravity fires, move_down tries row 20 (out of bounds) → fails.
    tick(&mut session, 1.0);
    assert!(session.lock_delay_active, "setup: lock delay must be active");

    // Move left by 1. Piece is still at row 19 (grid bottom). Row 20 is out of bounds.
    let moved = session.move_horizontal(-1);
    assert!(moved, "move_horizontal must succeed");

    // Recheck: row+1=20 is out of bounds → is_valid_position false → lock delay stays active.
    assert!(
        session.lock_delay_active,
        "lock_delay_active must remain true when piece is still on surface"
    );
}

// --- Test 3 ---

#[test]
fn rotate_deactivates_lock_delay_when_piece_leaves_surface() {
    let mut session = GameSession::new();
    // I-piece at rot 1 (vertical): cells (-1,0),(0,0),(1,0),(2,0) relative to pivot.
    // At row 17, col 5 → occupies rows 16,17,18,19 at col 5.
    // Bottom cell at row 19 → move_down tries row 20 (out of bounds) → fails.
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 1,
        row: 17,
        col: 5,
    };

    tick(&mut session, 1.0);
    assert!(session.lock_delay_active, "setup: lock delay must be active");

    // Rotate CW: I rot 1 → rot 2 (horizontal). Cells (0,-2),(0,-1),(0,0),(0,1).
    // At row 17, col 5 → occupies row 17, cols 3,4,5,6 — all valid on empty grid.
    // After rotation: piece at row 17 (horizontal). Row 18 cols 3,4,5,6 are all empty.
    // is_valid_position at row+1=18 → true → lock_delay_active deactivated.
    let rotated = session.rotate(true);
    assert!(rotated, "rotation must succeed on empty grid");

    assert!(
        !session.lock_delay_active,
        "lock_delay_active must be false after rotation moves piece off surface"
    );
}

// --- Test 4 ---

#[test]
fn rotate_keeps_lock_delay_when_piece_still_on_surface() {
    let mut session = GameSession::new();
    // T-piece rot 0 at bottom. Gravity fires → move_down fails → lock delay active.
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::T,
        rotation: 0,
        row: 19,
        col: 4,
    };

    tick(&mut session, 1.0);
    assert!(session.lock_delay_active, "setup: lock delay must be active");

    // Rotate CW. SRS kicks may shift the piece but it remains near the bottom.
    // After rotation, the new piece shape still has a cell that reaches row 19 or beyond,
    // so is_valid_position at row+1 returns false → lock delay stays active.
    let rotated = session.rotate(true);
    if rotated {
        assert!(
            session.lock_delay_active,
            "lock_delay_active must remain true when piece is still on surface after rotation"
        );
    }
    // If rotation fails entirely, lock delay is trivially still active (no recheck runs).
}

// --- Test 5 ---

#[test]
fn recheck_preserves_lock_delay_resets_counter() {
    let mut session = setup_i_piece_on_partial_ledge();

    tick(&mut session, 1.0);
    assert!(session.lock_delay_active, "setup: lock delay must be active");
    assert_eq!(session.lock_delay_resets, 0, "setup: resets must be 0");

    // move_horizontal off the ledge:
    // 1. Timer reset block fires (resets 0 < 15): accumulator→0, resets→1.
    // 2. Recheck fires: space below → lock_delay_active = false.
    // The recheck must NOT undo the resets increment.
    let moved = session.move_horizontal(-5);
    assert!(moved, "move_horizontal must succeed");

    assert!(
        !session.lock_delay_active,
        "lock_delay_active must be false after recheck"
    );
    assert_eq!(
        session.lock_delay_resets, 1,
        "lock_delay_resets must be 1 (timer reset ran before recheck, recheck must not undo it)"
    );
}

// --- Test 6 ---

#[test]
fn recheck_preserves_lock_delay_accumulator() {
    let mut session = setup_i_piece_on_partial_ledge();

    tick(&mut session, 1.0); // lock_delay_active=true, accumulator=0
    tick(&mut session, 0.2); // accumulator=200ms
    assert_eq!(session.lock_delay_accumulator_ms, 200, "setup: accumulator must be 200ms");

    // move_horizontal off the ledge:
    // 1. Timer reset block fires: accumulator→0, resets→1.
    // 2. Recheck fires: space below → lock_delay_active = false.
    // The 0 value comes from the timer reset, not from the recheck.
    let moved = session.move_horizontal(-5);
    assert!(moved, "move_horizontal must succeed");

    assert!(!session.lock_delay_active, "lock_delay_active must be false");
    assert_eq!(
        session.lock_delay_accumulator_ms, 0,
        "accumulator must be 0 (reset by timer block; recheck must not modify it)"
    );
}

// --- Test 7 ---

#[test]
fn gravity_resumes_after_recheck_deactivates_lock_delay() {
    let mut session = setup_i_piece_on_partial_ledge();

    tick(&mut session, 1.0); // gravity fires, lock delay activates
    assert!(session.lock_delay_active);

    // Move off surface → recheck deactivates lock delay.
    let moved = session.move_horizontal(-5);
    assert!(moved);
    assert!(!session.lock_delay_active, "lock delay must be deactivated");

    // Piece is now at row 18, col 1. Row 19 cols 0..4 are empty.
    // tick(1.0): lock_delay_active=false → gravity tick → piece falls to row 19 → PieceMoved.
    let result = tick(&mut session, 1.0);
    assert_eq!(
        result,
        TickResult::PieceMoved,
        "gravity must resume normally after recheck deactivates lock delay; got {:?}",
        result
    );
}

// --- Test 8 ---

#[test]
fn move_horizontal_recheck_noop_when_lock_delay_inactive() {
    let mut session = GameSession::new();
    assert!(!session.lock_delay_active, "setup: lock delay must be inactive");

    // move_horizontal with lock_delay_active=false — recheck must not activate lock delay.
    session.move_horizontal(-1);

    assert!(
        !session.lock_delay_active,
        "lock_delay_active must remain false; recheck must not activate lock delay"
    );
}

// --- Test 9 ---

#[test]
fn rotate_recheck_noop_when_lock_delay_inactive() {
    let mut session = GameSession::new();
    assert!(!session.lock_delay_active, "setup: lock delay must be inactive");

    // rotate with lock_delay_active=false — recheck must not activate lock delay.
    session.rotate(true);

    assert!(
        !session.lock_delay_active,
        "lock_delay_active must remain false; recheck must not activate lock delay"
    );
}

// --- Test 10 ---

#[test]
fn move_horizontal_recheck_runs_even_when_resets_exhausted() {
    let mut session = setup_i_piece_on_partial_ledge();

    tick(&mut session, 1.0);
    assert!(session.lock_delay_active, "setup: lock delay must be active");

    // Exhaust all resets.
    session.lock_delay_resets = MAX_LOCK_RESETS;

    // move_horizontal off the surface:
    // Timer reset block: resets == MAX_LOCK_RESETS → SKIPPED (no timer reset, no increment).
    // Recheck block is separate: guarded by `if lock_delay_active` (not by resets < max).
    // → recheck still fires → space below → lock_delay_active = false.
    let moved = session.move_horizontal(-5);
    assert!(moved, "move_horizontal must succeed");

    assert!(
        !session.lock_delay_active,
        "lock_delay_active must be false: recheck must run even when resets are exhausted"
    );
}

// --- Test 11 ---

#[test]
fn gravity_success_zeroes_lock_delay_resets() {
    let mut session = GameSession::new();
    session.lock_delay_resets = 5; // leftover from previous cycle
    // Piece at default spawn position — can fall
    let result = tick(&mut session, 1.0); // gravity fires, move_down succeeds
    assert_eq!(result, TickResult::PieceMoved);
    assert_eq!(session.lock_delay_resets, 0, "gravity success must zero resets");
}

// --- Test 12 ---

#[test]
fn activation_preserves_nonzero_lock_delay_resets() {
    let mut session = GameSession::new();
    session.lock_delay_resets = 3;
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0); // gravity fires, move_down fails → lock delay activates
    assert!(session.lock_delay_active);
    assert_eq!(session.lock_delay_resets, 3, "activation must NOT zero resets");
}

// --- Test 13 ---

#[test]
fn resets_accumulate_across_recheck_deactivation_reactivation() {
    let mut session = setup_i_piece_on_partial_ledge();

    // Cycle 1: activate lock delay
    tick(&mut session, 1.0); // gravity fires, move_down fails → lock_delay_active=true
    assert!(session.lock_delay_active);
    assert_eq!(session.lock_delay_resets, 0);

    // Move off ledge: timer reset fires (resets→1), recheck deactivates
    let moved = session.move_horizontal(-5);
    assert!(moved);
    assert!(!session.lock_delay_active);
    assert_eq!(session.lock_delay_resets, 1, "one reset from first cycle");

    // Move back onto ledge area
    let moved_back = session.move_horizontal(5);
    assert!(moved_back);

    // Cycle 2: gravity fires again, move_down fails → reactivate
    tick(&mut session, 1.0);
    assert!(session.lock_delay_active);
    assert_eq!(session.lock_delay_resets, 1, "resets preserved across reactivation");

    // Move off ledge again: timer reset fires (resets→2), recheck deactivates
    let moved2 = session.move_horizontal(-5);
    assert!(moved2);
    assert!(!session.lock_delay_active);
    assert_eq!(session.lock_delay_resets, 2, "resets accumulated across two cycles");
}

// --- Test 14 ---

#[test]
fn gravity_success_resets_counter_prevents_circumvention() {
    let mut session = GameSession::new();
    session.lock_delay_resets = 10; // accumulated resets
    // Piece at spawn position, can fall
    let result = tick(&mut session, 1.0);
    assert_eq!(result, TickResult::PieceMoved);
    assert_eq!(session.lock_delay_resets, 0, "gravity success must zero resets to prevent circumvention");
}

// --- Test 15 ---

#[test]
fn lock_delay_expiry_still_zeroes_resets() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0); // lock delay activates
    session.lock_delay_resets = 5; // simulate accumulated resets
    let result = tick(&mut session, 0.4); // lock delay expires
    assert!(matches!(result, TickResult::PieceLocked { .. }));
    assert_eq!(session.lock_delay_resets, 0, "expiry cleanup must zero resets");
}
