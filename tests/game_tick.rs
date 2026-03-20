// @spec-tags: core,game,timing
// @invariants: TickResult enum variants are correct; GameSession::new initializes correctly; tick() accumulates dt, fires gravity, returns PieceMoved/PieceLocked/GameOver/Nothing; TETROMINO_TYPES constant maps indices to TetrominoType
// @build: 52

use rhythm_grid::game::{tick, GameSession, GameState, TickResult, ActivePiece};
use rhythm_grid::pieces::{TetrominoType, TETROMINO_TYPES};

// --- TickResult derives ---

#[test]
fn tick_result_derives_partialeq() {
    assert_eq!(TickResult::Nothing, TickResult::Nothing);
    assert_eq!(TickResult::PieceMoved, TickResult::PieceMoved);
    assert_eq!(
        TickResult::PieceLocked { lines_cleared: 2 },
        TickResult::PieceLocked { lines_cleared: 2 }
    );
    assert_ne!(
        TickResult::PieceLocked { lines_cleared: 0 },
        TickResult::PieceLocked { lines_cleared: 1 }
    );
    assert_eq!(TickResult::GameOver, TickResult::GameOver);
}

#[test]
fn tick_result_derives_clone() {
    let r = TickResult::PieceLocked { lines_cleared: 4 };
    let r2 = r.clone();
    assert_eq!(r, r2);
}

#[test]
fn tick_result_derives_debug() {
    let s = format!("{:?}", TickResult::Nothing);
    assert!(s.contains("Nothing"));
    let s2 = format!("{:?}", TickResult::PieceLocked { lines_cleared: 1 });
    assert!(s2.contains("PieceLocked"));
    assert!(s2.contains("lines_cleared"));
}

#[test]
fn tick_result_piece_locked_lines_cleared_field() {
    let r = TickResult::PieceLocked { lines_cleared: 3 };
    if let TickResult::PieceLocked { lines_cleared } = r {
        assert_eq!(lines_cleared, 3);
    } else {
        panic!("Expected PieceLocked variant");
    }
}

// --- TETROMINO_TYPES constant ---

#[test]
fn tetromino_types_has_7_elements() {
    assert_eq!(TETROMINO_TYPES.len(), 7);
}

#[test]
fn tetromino_types_index_0_is_i() {
    assert_eq!(TETROMINO_TYPES[0], TetrominoType::I);
}

#[test]
fn tetromino_types_index_1_is_o() {
    assert_eq!(TETROMINO_TYPES[1], TetrominoType::O);
}

#[test]
fn tetromino_types_index_2_is_t() {
    assert_eq!(TETROMINO_TYPES[2], TetrominoType::T);
}

#[test]
fn tetromino_types_index_3_is_s() {
    assert_eq!(TETROMINO_TYPES[3], TetrominoType::S);
}

#[test]
fn tetromino_types_index_4_is_z() {
    assert_eq!(TETROMINO_TYPES[4], TetrominoType::Z);
}

#[test]
fn tetromino_types_index_5_is_j() {
    assert_eq!(TETROMINO_TYPES[5], TetrominoType::J);
}

#[test]
fn tetromino_types_index_6_is_l() {
    assert_eq!(TETROMINO_TYPES[6], TetrominoType::L);
}

#[test]
fn tetromino_types_all_distinct() {
    let types = TETROMINO_TYPES;
    for i in 0..7 {
        for j in (i + 1)..7 {
            assert_ne!(types[i], types[j], "Duplicate at indices {} and {}", i, j);
        }
    }
}

// --- GameSession::new ---

#[test]
fn game_session_new_starts_playing() {
    let session = GameSession::new();
    assert_eq!(session.state, GameState::Playing);
}

#[test]
fn game_session_new_score_is_zero() {
    let session = GameSession::new();
    assert_eq!(session.score, 0);
}

#[test]
fn game_session_new_total_lines_is_zero() {
    let session = GameSession::new();
    assert_eq!(session.total_lines, 0);
}

#[test]
fn game_session_new_gravity_accumulator_is_zero() {
    let session = GameSession::new();
    assert_eq!(session.gravity_accumulator_ms, 0);
}

#[test]
fn game_session_new_active_piece_is_valid_type() {
    let session = GameSession::new();
    assert!(TETROMINO_TYPES.contains(&session.active_piece.piece_type));
}

#[test]
fn game_session_new_active_piece_rotation_is_zero() {
    let session = GameSession::new();
    assert_eq!(session.active_piece.rotation, 0);
}

// --- tick() returns Nothing when not Playing ---

#[test]
fn tick_returns_nothing_when_paused() {
    let mut session = GameSession::new();
    session.state = GameState::Paused;
    let result = tick(&mut session, 1.0);
    assert_eq!(result, TickResult::Nothing);
}

#[test]
fn tick_returns_nothing_when_game_over() {
    let mut session = GameSession::new();
    session.state = GameState::GameOver;
    let result = tick(&mut session, 1.0);
    assert_eq!(result, TickResult::Nothing);
}

#[test]
fn tick_returns_nothing_when_menu() {
    let mut session = GameSession::new();
    session.state = GameState::Menu;
    let result = tick(&mut session, 1.0);
    assert_eq!(result, TickResult::Nothing);
}

#[test]
fn tick_does_not_advance_accumulator_when_not_playing() {
    let mut session = GameSession::new();
    session.state = GameState::Paused;
    tick(&mut session, 0.5);
    assert_eq!(session.gravity_accumulator_ms, 0);
}

// --- tick() accumulates dt_secs ---

#[test]
fn tick_accumulates_dt_below_gravity_threshold() {
    let mut session = GameSession::new();
    // Level 1 gravity interval is 1000ms. dt=0.4s → 400ms < 1000ms → no gravity.
    let result = tick(&mut session, 0.4);
    assert_eq!(result, TickResult::Nothing);
    assert_eq!(session.gravity_accumulator_ms, 400);
}

#[test]
fn tick_accumulates_incrementally() {
    let mut session = GameSession::new();
    tick(&mut session, 0.3);
    assert_eq!(session.gravity_accumulator_ms, 300);
    tick(&mut session, 0.2);
    assert_eq!(session.gravity_accumulator_ms, 500);
}

#[test]
fn tick_dt_to_accumulated_ms_is_truncated() {
    let mut session = GameSession::new();
    // (0.4999 * 1000.0) as u64 = 499
    tick(&mut session, 0.4999);
    assert_eq!(session.gravity_accumulator_ms, 499);
}

// --- tick() fires gravity and returns PieceMoved ---

#[test]
fn tick_returns_piece_moved_when_gravity_fires_and_piece_can_fall() {
    let mut session = GameSession::new();
    // Level 1 gravity interval = 1000ms. Enough dt to trigger.
    let result = tick(&mut session, 1.0);
    assert_eq!(result, TickResult::PieceMoved);
}

#[test]
fn tick_resets_accumulator_after_piece_moved() {
    let mut session = GameSession::new();
    let result = tick(&mut session, 1.0);
    assert_eq!(result, TickResult::PieceMoved);
    assert_eq!(session.gravity_accumulator_ms, 0);
}

#[test]
fn tick_piece_moved_advances_row() {
    let mut session = GameSession::new();
    let initial_row = session.active_piece.row;
    tick(&mut session, 1.0);
    assert_eq!(session.active_piece.row, initial_row + 1);
}

// --- tick() returns PieceLocked when piece cannot fall ---

#[test]
fn tick_returns_piece_locked_when_piece_at_bottom() {
    let mut session = GameSession::new();
    // Place I-piece at row 19 (bottom). move_down tries row 20 → out of bounds → lock.
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    let result = tick(&mut session, 1.0);
    assert!(
        matches!(result, TickResult::PieceLocked { lines_cleared: _ }),
        "Expected PieceLocked, got {:?}",
        result
    );
}

#[test]
fn tick_piece_locked_zero_lines_when_row_not_full() {
    let mut session = GameSession::new();
    // I-piece at row 19 locks only 4 cells in row 19 (not full row of 10).
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    let result = tick(&mut session, 1.0);
    assert_eq!(result, TickResult::PieceLocked { lines_cleared: 0 });
}

#[test]
fn tick_piece_locked_resets_accumulator() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0);
    assert_eq!(session.gravity_accumulator_ms, 0);
}

#[test]
fn tick_piece_locked_spawns_new_active_piece() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0);
    // After locking the I-piece at row 19, a new piece should be active.
    // The new piece should not be at row 19 (it was just locked there).
    // It should be at a spawn position near row 0.
    assert!(session.active_piece.row <= 1);
}

#[test]
fn tick_piece_locked_state_remains_playing() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0);
    // Grid is not full at top, so next spawn should succeed, state stays Playing.
    assert_eq!(session.state, GameState::Playing);
}
