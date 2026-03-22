// @spec-tags: core,game,scoring
// @invariants: combo_count tracks consecutive line clears and resets on no-clear lock; combo bonus = 50 * combo_count * level (combo_count incremented BEFORE bonus); both tick() and hard_drop() apply combo logic
// @build: 79

use rhythm_grid::game::{
    tick, GameSession, TickResult, ActivePiece, score_for_lines, level_for_lines,
};
use rhythm_grid::grid::{CellState, WIDTH};
use rhythm_grid::pieces::TetrominoType;

fn fill_row_except(session: &mut GameSession, row: usize, gap_col: usize) {
    for col in 0..WIDTH {
        if col != gap_col {
            session.grid.cells[row][col] = CellState::Occupied(0);
        }
    }
}

// ── combo_count initialization ─────────────────────────────────────────────────

#[test]
fn game_session_new_combo_count_is_zero() {
    assert_eq!(GameSession::new().combo_count, 0);
}

// ── combo increments on line clear ────────────────────────────────────────────

#[test]
fn combo_increments_on_line_clear() {
    // Fill row 19 except col 4; I-piece (rotation=1, vertical) at (row=19, col=4)
    // hard_drop locks piece, completes row 19 → lines_cleared=1, combo_count=1
    let mut session = GameSession::new();
    fill_row_except(&mut session, 19, 4);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 1,
        row: 19,
        col: 4,
    };
    let result = session.hard_drop();
    assert!(
        matches!(result, TickResult::PieceLocked { lines_cleared: 1 }),
        "Expected PieceLocked {{ lines_cleared: 1 }}, got {:?}",
        result
    );
    assert_eq!(session.combo_count, 1);
}

#[test]
fn combo_resets_on_lock_without_clear() {
    let mut session = GameSession::new();
    // First get combo_count to 1 via a line clear
    fill_row_except(&mut session, 19, 4);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 1,
        row: 19,
        col: 4,
    };
    session.hard_drop();
    assert_eq!(session.combo_count, 1);

    // Lock a piece without clearing any lines
    // After the clear, row 19 is empty; I-piece (rot=0) at row=19 occupies 4 cells (< 10)
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    let result = session.hard_drop();
    assert_eq!(
        result,
        TickResult::PieceLocked { lines_cleared: 0 },
        "Expected PieceLocked {{ lines_cleared: 0 }}, got {:?}",
        result
    );
    assert_eq!(session.combo_count, 0);
}

#[test]
fn combo_accumulates_across_consecutive_clears() {
    let mut session = GameSession::new();
    // First consecutive clear
    fill_row_except(&mut session, 19, 4);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 1,
        row: 19,
        col: 4,
    };
    session.hard_drop();
    assert_eq!(session.combo_count, 1);

    // Second consecutive clear
    fill_row_except(&mut session, 19, 4);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 1,
        row: 19,
        col: 4,
    };
    session.hard_drop();
    assert_eq!(session.combo_count, 2);
}

// ── combo scoring ─────────────────────────────────────────────────────────────

#[test]
fn combo_bonus_first_clear_at_level_1() {
    // First consecutive clear at level 1:
    //   score_for_lines(1, 1) = 100
    //   combo_count increments to 1, bonus = 50 * 1 * 1 = 50
    //   total score = 150
    let mut session = GameSession::new();
    fill_row_except(&mut session, 19, 4);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 1,
        row: 19,
        col: 4,
    };
    session.hard_drop();
    assert_eq!(session.score, score_for_lines(1, 1) + 50 * 1 * 1);
}

#[test]
fn combo_bonus_second_clear_at_level_1() {
    // After 1st clear: score = 100 + 50 = 150
    // After 2nd clear: score += score_for_lines(1,1) + 50*2*1 = 100 + 100 = 200 → total 350
    let mut session = GameSession::new();

    fill_row_except(&mut session, 19, 4);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 1,
        row: 19,
        col: 4,
    };
    session.hard_drop();
    assert_eq!(session.score, 150);

    fill_row_except(&mut session, 19, 4);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 1,
        row: 19,
        col: 4,
    };
    session.hard_drop();
    // 150 + score_for_lines(1,1) + 50*2*1 = 150 + 100 + 100 = 350
    assert_eq!(session.score, 350);
}

#[test]
fn combo_bonus_scales_with_level() {
    // At level 2 (total_lines=10 before clear), first combo clear:
    //   score_for_lines(1, 2) = 200
    //   combo_count=1, bonus = 50 * 1 * 2 = 100
    //   total = 300
    let mut session = GameSession::new();
    session.total_lines = 10; // level = level_for_lines(10) = 2
    fill_row_except(&mut session, 19, 4);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 1,
        row: 19,
        col: 4,
    };
    session.hard_drop();
    let level = level_for_lines(10);
    assert_eq!(session.combo_count, 1);
    assert_eq!(session.score, score_for_lines(1, level) + 50 * 1 * level);
}

#[test]
fn combo_resets_then_restarts() {
    // Scenario: clear → no-clear (combo resets) → clear again → combo_count == 1
    let mut session = GameSession::new();

    // Step 1: clear via col 4 gap → combo_count = 1
    fill_row_except(&mut session, 19, 4);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 1,
        row: 19,
        col: 4,
    };
    session.hard_drop();
    assert_eq!(session.combo_count, 1);

    // Step 2: lock without clear (I-piece at row=19 col=3 rot=0 occupies 4 cells < full row)
    // After the clear, grid row 19 is empty; piece falls to row 19 and locks without clearing
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 3,
    };
    let r2 = session.hard_drop();
    assert_eq!(
        r2,
        TickResult::PieceLocked { lines_cleared: 0 },
        "Expected no-clear lock, got {:?}",
        r2
    );
    assert_eq!(session.combo_count, 0);

    // Step 3: clear again using col 9 gap
    // Row 19 now has some cells from step 2 (at cols ~1-4 or similar).
    // fill_row_except overwrites all cols except 9 to Occupied, leaving col 9 empty.
    fill_row_except(&mut session, 19, 9);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 1,
        row: 19,
        col: 9,
    };
    session.hard_drop();
    assert_eq!(session.combo_count, 1, "combo_count should restart at 1 after reset");
}

// ── combo via tick() ──────────────────────────────────────────────────────────

#[test]
fn combo_works_with_tick() {
    // tick() lock path (lock delay) also applies combo logic
    let mut session = GameSession::new();
    fill_row_except(&mut session, 19, 4);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 1,
        row: 19,
        col: 4,
    };
    // First tick: piece can't fall from row=19 → lock delay starts → Nothing
    let first = tick(&mut session, 1.0);
    assert_eq!(
        first,
        TickResult::Nothing,
        "Expected Nothing while lock delay starts, got {:?}",
        first
    );
    // Second tick: lock delay expires → piece locks → row 19 clears
    let result = tick(&mut session, 0.4);
    assert!(
        matches!(result, TickResult::PieceLocked { lines_cleared: 1 }),
        "Expected PieceLocked {{ lines_cleared: 1 }}, got {:?}",
        result
    );
    assert_eq!(session.combo_count, 1);
}

#[test]
fn combo_works_with_hard_drop() {
    // hard_drop() completing a line increments combo_count
    let mut session = GameSession::new();
    fill_row_except(&mut session, 19, 4);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 1,
        row: 19,
        col: 4,
    };
    let result = session.hard_drop();
    assert!(
        matches!(result, TickResult::PieceLocked { lines_cleared: 1 }),
        "Expected PieceLocked {{ lines_cleared: 1 }}, got {:?}",
        result
    );
    assert_eq!(session.combo_count, 1);
}
