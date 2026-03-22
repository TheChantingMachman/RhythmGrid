// @spec-tags: core,game
// @invariants: GameSession tracks pieces_placed, lines_cleared_total, max_combo, time_played_secs; all initialize to 0; pieces_placed increments on every lock; max_combo tracks the highest combo_count reached; time_played_secs accumulates dt only while state == Playing
// @build: 79

use rhythm_grid::game::{tick, GameSession, GameState, TickResult, ActivePiece};
use rhythm_grid::grid::{CellState, WIDTH};
use rhythm_grid::pieces::TetrominoType;

fn fill_row_except(session: &mut GameSession, row: usize, gap_col: usize) {
    for col in 0..WIDTH {
        if col != gap_col {
            session.grid.cells[row][col] = CellState::Occupied(0);
        }
    }
}

// ── Initialization ────────────────────────────────────────────────────────────

#[test]
fn game_session_new_pieces_placed_is_zero() {
    assert_eq!(GameSession::new().pieces_placed, 0);
}

#[test]
fn game_session_new_max_combo_is_zero() {
    assert_eq!(GameSession::new().max_combo, 0);
}

#[test]
fn game_session_new_time_played_secs_is_zero() {
    assert!((GameSession::new().time_played_secs - 0.0).abs() < 1e-9);
}

#[test]
fn game_session_new_lines_cleared_total_is_zero() {
    assert_eq!(GameSession::new().lines_cleared_total, 0);
}

// ── pieces_placed ─────────────────────────────────────────────────────────────

#[test]
fn pieces_placed_increments_on_hard_drop() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    session.hard_drop();
    assert_eq!(session.pieces_placed, 1);
}

#[test]
fn pieces_placed_increments_on_tick_lock() {
    let mut session = GameSession::new();
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    tick(&mut session, 1.0); // lock delay starts
    tick(&mut session, 0.4); // lock delay expires → piece locks
    assert_eq!(session.pieces_placed, 1);
}

#[test]
fn pieces_placed_increments_twice_after_two_locks() {
    let mut session = GameSession::new();
    // First lock
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    session.hard_drop();
    assert_eq!(session.pieces_placed, 1);

    // Second lock — place piece at row=18 so it settles on top of the first
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 18,
        col: 4,
    };
    session.hard_drop();
    assert_eq!(session.pieces_placed, 2);
}

// ── max_combo ─────────────────────────────────────────────────────────────────

#[test]
fn max_combo_tracks_highest() {
    let mut session = GameSession::new();

    // First consecutive clear → combo_count=1, max_combo=1
    fill_row_except(&mut session, 19, 4);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 1,
        row: 19,
        col: 4,
    };
    session.hard_drop();
    assert_eq!(session.combo_count, 1);
    assert_eq!(session.max_combo, 1);

    // Second consecutive clear → combo_count=2, max_combo=2
    fill_row_except(&mut session, 19, 4);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 1,
        row: 19,
        col: 4,
    };
    session.hard_drop();
    assert_eq!(session.combo_count, 2);
    assert_eq!(session.max_combo, 2);

    // Lock without clear → combo_count resets to 0, max_combo stays 2
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 3,
    };
    session.hard_drop();
    assert_eq!(session.combo_count, 0);
    assert_eq!(session.max_combo, 2);
}

#[test]
fn max_combo_does_not_decrease() {
    let mut session = GameSession::new();

    // Build up max_combo = 2 via two consecutive clears
    fill_row_except(&mut session, 19, 4);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 1,
        row: 19,
        col: 4,
    };
    session.hard_drop(); // combo=1, max_combo=1

    fill_row_except(&mut session, 19, 4);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 1,
        row: 19,
        col: 4,
    };
    session.hard_drop(); // combo=2, max_combo=2

    // Reset combo via no-clear lock
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 3,
    };
    session.hard_drop(); // combo=0, max_combo=2

    // One more clear → combo=1 (< 2), max_combo must NOT decrease to 1
    fill_row_except(&mut session, 19, 9);
    session.active_piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 1,
        row: 19,
        col: 9,
    };
    session.hard_drop(); // combo=1
    assert_eq!(session.max_combo, 2, "max_combo must not decrease from 2 to 1");
}

// ── time_played_secs ──────────────────────────────────────────────────────────

#[test]
fn time_played_secs_accumulates_dt() {
    let mut session = GameSession::new();
    assert_eq!(session.state, GameState::Playing);
    tick(&mut session, 0.5);
    tick(&mut session, 0.3);
    assert!(
        session.time_played_secs >= 0.8 - 1e-9,
        "time_played_secs should be >= 0.8, got {}",
        session.time_played_secs
    );
}

#[test]
fn time_played_secs_does_not_accumulate_when_paused() {
    let mut session = GameSession::new();
    session.state = GameState::Paused;
    tick(&mut session, 1.0);
    assert!(
        (session.time_played_secs - 0.0).abs() < 1e-9,
        "time_played_secs must not increase when Paused, got {}",
        session.time_played_secs
    );
}

#[test]
fn time_played_secs_does_not_accumulate_when_game_over() {
    let mut session = GameSession::new();
    session.state = GameState::GameOver;
    tick(&mut session, 1.0);
    assert!(
        (session.time_played_secs - 0.0).abs() < 1e-9,
        "time_played_secs must not increase when GameOver, got {}",
        session.time_played_secs
    );
}

#[test]
fn time_played_secs_does_not_accumulate_when_menu() {
    let mut session = GameSession::new();
    session.state = GameState::Menu;
    tick(&mut session, 1.0);
    assert!(
        (session.time_played_secs - 0.0).abs() < 1e-9,
        "time_played_secs must not increase when Menu, got {}",
        session.time_played_secs
    );
}
