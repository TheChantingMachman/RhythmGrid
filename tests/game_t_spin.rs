// @spec-tags: core,game,scoring
// @invariants: detect_t_spin returns false for non-T pieces and when last_move_was_rotate=false; returns true when 3+ of the 4 bounding-box corners are filled (OOB or Occupied) for a T-piece with last_move_was_rotate=true; t_spin_score applies T_SPIN_* constants * level; GameSession::last_move_was_rotate is set true by rotate() and false by move_horizontal()
// @build: 79

use rhythm_grid::game::{
    detect_t_spin, t_spin_score, GameSession, ActivePiece,
    T_SPIN_ZERO, T_SPIN_SINGLE, T_SPIN_DOUBLE, T_SPIN_TRIPLE,
};
use rhythm_grid::grid::{Grid, CellState};
use rhythm_grid::pieces::TetrominoType;

// ── T-spin constants ──────────────────────────────────────────────────────────

#[test]
fn constant_t_spin_zero() {
    assert_eq!(T_SPIN_ZERO, 100u32);
}

#[test]
fn constant_t_spin_single() {
    assert_eq!(T_SPIN_SINGLE, 200u32);
}

#[test]
fn constant_t_spin_double() {
    assert_eq!(T_SPIN_DOUBLE, 600u32);
}

#[test]
fn constant_t_spin_triple() {
    assert_eq!(T_SPIN_TRIPLE, 800u32);
}

// ── detect_t_spin ─────────────────────────────────────────────────────────────

#[test]
fn t_spin_false_when_not_t_piece() {
    // I-piece with 3+ corners filled and last_move_was_rotate=true → false
    // Corners of bounding box for piece at (row=5, col=5): (4,4), (4,6), (6,4), (6,6)
    let mut grid = Grid::new();
    grid.cells[4][4] = CellState::Occupied(0);
    grid.cells[4][6] = CellState::Occupied(0);
    grid.cells[6][4] = CellState::Occupied(0);
    let piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 5,
        col: 5,
    };
    assert!(!detect_t_spin(&grid, &piece, true));
}

#[test]
fn t_spin_false_when_last_move_was_not_rotate() {
    // T-piece with 3+ corners filled but last_move_was_rotate=false → false
    let mut grid = Grid::new();
    grid.cells[4][4] = CellState::Occupied(0);
    grid.cells[4][6] = CellState::Occupied(0);
    grid.cells[6][4] = CellState::Occupied(0);
    let piece = ActivePiece {
        piece_type: TetrominoType::T,
        rotation: 0,
        row: 5,
        col: 5,
    };
    assert!(!detect_t_spin(&grid, &piece, false));
}

#[test]
fn t_spin_true_when_3_corners_occupied() {
    // T-piece at (18, 4): corners at (17,3), (17,5), (19,3), (19,5)
    // Fill 3 corners, leave (19,5) empty → 3 corners filled → true
    let mut grid = Grid::new();
    grid.cells[17][3] = CellState::Occupied(0);
    grid.cells[17][5] = CellState::Occupied(0);
    grid.cells[19][3] = CellState::Occupied(0);
    let piece = ActivePiece {
        piece_type: TetrominoType::T,
        rotation: 0,
        row: 18,
        col: 4,
    };
    assert!(detect_t_spin(&grid, &piece, true));
}

#[test]
fn t_spin_true_when_4_corners_occupied() {
    // All 4 corners of bounding box filled → true
    let mut grid = Grid::new();
    grid.cells[17][3] = CellState::Occupied(0);
    grid.cells[17][5] = CellState::Occupied(0);
    grid.cells[19][3] = CellState::Occupied(0);
    grid.cells[19][5] = CellState::Occupied(0);
    let piece = ActivePiece {
        piece_type: TetrominoType::T,
        rotation: 0,
        row: 18,
        col: 4,
    };
    assert!(detect_t_spin(&grid, &piece, true));
}

#[test]
fn t_spin_false_when_only_2_corners() {
    // Only 2 corners filled → false
    let mut grid = Grid::new();
    grid.cells[17][3] = CellState::Occupied(0);
    grid.cells[17][5] = CellState::Occupied(0);
    // (19,3) and (19,5) are empty
    let piece = ActivePiece {
        piece_type: TetrominoType::T,
        rotation: 0,
        row: 18,
        col: 4,
    };
    assert!(!detect_t_spin(&grid, &piece, true));
}

#[test]
fn t_spin_corner_out_of_bounds_counts_as_filled() {
    // T-piece at (row=5, col=0): corners at (4,-1) [OOB], (4,1), (6,-1) [OOB], (6,1)
    // OOB: (4,-1) and (6,-1) = 2 filled. Fill (4,1) → 3 total → true
    let mut grid = Grid::new();
    grid.cells[4][1] = CellState::Occupied(0);
    let piece = ActivePiece {
        piece_type: TetrominoType::T,
        rotation: 0,
        row: 5,
        col: 0,
    };
    assert!(detect_t_spin(&grid, &piece, true));
}

#[test]
fn t_spin_works_at_all_rotations() {
    // detect_t_spin checks corners from (piece.row, piece.col), not rotation shape.
    // T-piece at (1, 0): corners (0,-1) [OOB], (0,1), (2,-1) [OOB], (2,1)
    // 2 OOB + fill (0,1) = 3 corners → true at any rotation
    let mut grid = Grid::new();
    grid.cells[0][1] = CellState::Occupied(0);
    for rotation in [0, 1, 2, 3] {
        let piece = ActivePiece {
            piece_type: TetrominoType::T,
            rotation,
            row: 1,
            col: 0,
        };
        assert!(
            detect_t_spin(&grid, &piece, true),
            "detect_t_spin should return true for T-piece at rotation {rotation} with 3 corners filled"
        );
    }
}

// ── t_spin_score ──────────────────────────────────────────────────────────────

#[test]
fn t_spin_score_zero_lines_at_level_1() {
    assert_eq!(t_spin_score(0, 1), 100);
}

#[test]
fn t_spin_score_single_at_level_1() {
    assert_eq!(t_spin_score(1, 1), 200);
}

#[test]
fn t_spin_score_double_at_level_1() {
    assert_eq!(t_spin_score(2, 1), 600);
}

#[test]
fn t_spin_score_triple_at_level_1() {
    assert_eq!(t_spin_score(3, 1), 800);
}

#[test]
fn t_spin_score_zero_lines_at_level_3() {
    assert_eq!(t_spin_score(0, 3), 300);
}

#[test]
fn t_spin_score_single_at_level_5() {
    assert_eq!(t_spin_score(1, 5), 1000);
}

#[test]
fn t_spin_score_double_at_level_2() {
    assert_eq!(t_spin_score(2, 2), 1200);
}

#[test]
fn t_spin_score_triple_at_level_2() {
    assert_eq!(t_spin_score(3, 2), 1600);
}

// ── GameSession::last_move_was_rotate ─────────────────────────────────────────

#[test]
fn game_session_new_last_move_was_rotate_is_false() {
    let session = GameSession::new();
    assert!(!session.last_move_was_rotate);
}

#[test]
fn rotate_sets_last_move_was_rotate_true() {
    let mut session = GameSession::new();
    let rotated = session.rotate(true);
    assert!(rotated, "Expected CW rotation to succeed in fresh session");
    assert!(session.last_move_was_rotate);
}

#[test]
fn rotate_ccw_sets_last_move_was_rotate_true() {
    let mut session = GameSession::new();
    let rotated = session.rotate(false);
    assert!(rotated, "Expected CCW rotation to succeed in fresh session");
    assert!(session.last_move_was_rotate);
}

#[test]
fn move_horizontal_clears_last_move_was_rotate() {
    let mut session = GameSession::new();
    session.rotate(true);
    assert!(session.last_move_was_rotate);
    session.move_horizontal(1);
    assert!(!session.last_move_was_rotate);
}
