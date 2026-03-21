// @spec-tags: core,game,movement,rotation,scoring
// @invariants: move_horizontal/move_down return true and mutate piece on success, false and leave piece unchanged on rejection; hard_drop locks piece at lowest valid row and returns lines cleared; rotate tries unshifted then SRS kicks updating rotation+col+row on first valid position, returns false only when all attempts fail; lock_piece writes CellState::Occupied(type_index) for each cell and triggers line clearing; clear_lines removes fully-occupied rows, shifts rows above down by one, inserts empty row at index 0, returns count
// @build: 38

use rhythm_grid::game::{
    clear_lines, hard_drop, is_valid_position, lock_piece, move_down, move_horizontal, rotate,
    ActivePiece,
};
use rhythm_grid::grid::{CellState, Grid, HEIGHT, WIDTH};
use rhythm_grid::pieces::{piece_cells, TetrominoType};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn t_piece(row: i32, col: i32) -> ActivePiece {
    ActivePiece {
        piece_type: TetrominoType::T,
        rotation: 0,
        row,
        col,
    }
}

fn i_piece(row: i32, col: i32) -> ActivePiece {
    ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row,
        col,
    }
}

fn fill_row(grid: &mut Grid, row: usize) {
    for col in 0..WIDTH {
        grid.cells[row][col] = CellState::Occupied(0);
    }
}

// ── ActivePiece struct ────────────────────────────────────────────────────────

#[test]
fn active_piece_fields_accessible() {
    let p = ActivePiece {
        piece_type: TetrominoType::T,
        rotation: 2,
        row: 5,
        col: 4,
    };
    assert_eq!(p.piece_type, TetrominoType::T);
    assert_eq!(p.rotation, 2);
    assert_eq!(p.row, 5);
    assert_eq!(p.col, 4);
}

#[test]
fn active_piece_derives_clone_copy_partialeq_debug() {
    let p = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 3,
        col: 5,
    };
    let q = p; // Copy
    assert_eq!(p, q); // PartialEq
    let _s = format!("{:?}", p); // Debug
    let r = p.clone(); // Clone
    assert_eq!(p, r);
}

// ── move_horizontal ───────────────────────────────────────────────────────────
//
// T rotation 0 cells: [(-1,0),(0,-1),(0,0),(0,1)]
// At row=5, col=5: absolute (4,5),(5,4),(5,5),(5,6)

#[test]
fn move_horizontal_right_succeeds_in_open_space() {
    let grid = Grid::new();
    let mut piece = t_piece(5, 5);
    let moved = move_horizontal(&grid, &mut piece, 1);
    assert!(moved, "move right in open space must return true");
    assert_eq!(piece.col, 6, "col must increase by 1");
    assert_eq!(piece.row, 5, "row must not change");
}

#[test]
fn move_horizontal_left_succeeds_in_open_space() {
    let grid = Grid::new();
    let mut piece = t_piece(5, 5);
    let moved = move_horizontal(&grid, &mut piece, -1);
    assert!(moved, "move left in open space must return true");
    assert_eq!(piece.col, 4, "col must decrease by 1");
    assert_eq!(piece.row, 5, "row must not change");
}

#[test]
fn move_horizontal_right_rejected_at_right_wall() {
    let grid = Grid::new();
    // T rot-0 cells include offset (0,+1). At col=8: (0,+1) → abs col=9 (valid).
    // At col=9: (0,+1) → abs col=10 ≥ WIDTH → invalid.
    let mut piece = t_piece(5, 8);
    // Verify piece is currently valid at col=8
    let cells = piece_cells(piece.piece_type, piece.rotation);
    assert!(
        is_valid_position(&grid, &cells, piece.row, piece.col),
        "piece must start in valid position for this test"
    );
    let moved = move_horizontal(&grid, &mut piece, 1);
    assert!(!moved, "move right into wall must return false");
    assert_eq!(piece.col, 8, "col must be unchanged on rejection");
}

#[test]
fn move_horizontal_left_rejected_at_left_wall() {
    let grid = Grid::new();
    // T rot-0 cells include offset (0,-1). At col=1: (0,-1) → abs col=0 (valid).
    // At col=0: (0,-1) → abs col=-1 < 0 → invalid.
    let mut piece = t_piece(5, 1);
    let cells = piece_cells(piece.piece_type, piece.rotation);
    assert!(
        is_valid_position(&grid, &cells, piece.row, piece.col),
        "piece must start in valid position for this test"
    );
    let moved = move_horizontal(&grid, &mut piece, -1);
    assert!(!moved, "move left into wall must return false");
    assert_eq!(piece.col, 1, "col must be unchanged on rejection");
}

#[test]
fn move_horizontal_right_rejected_by_occupied_cell() {
    let mut grid = Grid::new();
    // T rot-0 at row=5, col=5: cells (4,5),(5,4),(5,5),(5,6).
    // Moving right to col=6 would place a cell at (5,7). Block it.
    grid.cells[5][7] = CellState::Occupied(3);
    let mut piece = t_piece(5, 5);
    let moved = move_horizontal(&grid, &mut piece, 1);
    assert!(!moved, "move right into occupied cell must return false");
    assert_eq!(piece.col, 5, "col must be unchanged on rejection");
}

#[test]
fn move_horizontal_left_rejected_by_occupied_cell() {
    let mut grid = Grid::new();
    // T rot-0 at row=5, col=5: moving left to col=4 places cells at (4,4),(5,3),(5,4),(5,5).
    // Block (5,3).
    grid.cells[5][3] = CellState::Occupied(3);
    let mut piece = t_piece(5, 5);
    let moved = move_horizontal(&grid, &mut piece, -1);
    assert!(!moved, "move left into occupied cell must return false");
    assert_eq!(piece.col, 5, "col must be unchanged on rejection");
}

#[test]
fn move_horizontal_rotation_unchanged() {
    let grid = Grid::new();
    let mut piece = ActivePiece {
        piece_type: TetrominoType::T,
        rotation: 2,
        row: 5,
        col: 5,
    };
    move_horizontal(&grid, &mut piece, 1);
    assert_eq!(piece.rotation, 2, "rotation must not change on horizontal move");
}

// ── move_down ─────────────────────────────────────────────────────────────────
//
// T rotation 0 cells: [(-1,0),(0,-1),(0,0),(0,1)]
// At row=5, col=5: absolute (4,5),(5,4),(5,5),(5,6)

#[test]
fn move_down_succeeds_in_open_space() {
    let grid = Grid::new();
    let mut piece = t_piece(5, 5);
    let moved = move_down(&grid, &mut piece);
    assert!(moved, "move down in open space must return true");
    assert_eq!(piece.row, 6, "row must increase by 1");
    assert_eq!(piece.col, 5, "col must not change");
}

#[test]
fn move_down_rejected_at_floor() {
    let grid = Grid::new();
    // T rot-0 cells include offsets (0,-1),(0,0),(0,1) — all at `row`.
    // At row=19: absolute row=19 (valid). Moving to row=20: absolute row=20 ≥ HEIGHT → invalid.
    let mut piece = t_piece(19, 5);
    let cells = piece_cells(piece.piece_type, piece.rotation);
    assert!(
        is_valid_position(&grid, &cells, piece.row, piece.col),
        "piece must start in valid position for this test"
    );
    let moved = move_down(&grid, &mut piece);
    assert!(!moved, "move down at floor must return false");
    assert_eq!(piece.row, 19, "row must be unchanged on rejection");
}

#[test]
fn move_down_rejected_by_occupied_cell_below() {
    let mut grid = Grid::new();
    // T rot-0 at row=5, col=5: cells (4,5),(5,4),(5,5),(5,6).
    // Moving down to row=6 would place cells at (5,5),(6,4),(6,5),(6,6). Block (6,5).
    grid.cells[6][5] = CellState::Occupied(3);
    let mut piece = t_piece(5, 5);
    let moved = move_down(&grid, &mut piece);
    assert!(!moved, "move down into occupied cell must return false");
    assert_eq!(piece.row, 5, "row must be unchanged on rejection");
}

#[test]
fn move_down_does_not_change_col_on_rejection() {
    let grid = Grid::new();
    let mut piece = t_piece(19, 5);
    move_down(&grid, &mut piece);
    assert_eq!(piece.col, 5, "col must be unchanged when move down is rejected");
}

// ── hard_drop ─────────────────────────────────────────────────────────────────

#[test]
fn hard_drop_lands_at_floor_on_empty_grid() {
    let mut grid = Grid::new();
    // T rot-0 cells: [(-1,0),(0,-1),(0,0),(0,1)]. At row=r: top cell at r-1.
    // Max valid row: row=19, top cell at row 18, bottom cells at row 19 — valid.
    // Row 20 would be out of bounds.
    let piece = t_piece(1, 5);
    hard_drop(&mut grid, &piece);
    // Piece must be locked at row=19.
    assert_eq!(
        grid.cells[18][5],
        CellState::Occupied(TetrominoType::T as u32),
        "T cell (-1,0) must be locked at row 18"
    );
    assert_eq!(
        grid.cells[19][4],
        CellState::Occupied(TetrominoType::T as u32),
        "T cell (0,-1) must be locked at row 19 col 4"
    );
    assert_eq!(
        grid.cells[19][5],
        CellState::Occupied(TetrominoType::T as u32),
        "T cell (0,0) must be locked at row 19 col 5"
    );
    assert_eq!(
        grid.cells[19][6],
        CellState::Occupied(TetrominoType::T as u32),
        "T cell (0,1) must be locked at row 19 col 6"
    );
}

#[test]
fn hard_drop_original_piece_position_unchanged() {
    let mut grid = Grid::new();
    let piece = t_piece(1, 5);
    hard_drop(&mut grid, &piece);
    // Original piece is not mutated — hard_drop takes &ActivePiece
    assert_eq!(piece.row, 1, "hard_drop must not mutate the piece row");
    assert_eq!(piece.col, 5, "hard_drop must not mutate the piece col");
}

#[test]
fn hard_drop_returns_zero_when_no_lines_cleared() {
    let mut grid = Grid::new();
    let piece = t_piece(1, 5);
    let cleared = hard_drop(&mut grid, &piece);
    assert_eq!(cleared, 0, "hard_drop on empty grid clears no lines");
}

#[test]
fn hard_drop_stops_above_occupied_row() {
    let mut grid = Grid::new();
    // Fill row 17 completely — T piece cannot land below it.
    fill_row(&mut grid, 17);
    // T rot-0 at row=r: bottom cells at row r. T lands at row=16 (cells at 15,16).
    // At row=16: cells (15,5),(16,4),(16,5),(16,6) — all above occupied row 17.
    // At row=17: cells (16,5),(17,4),(17,5),(17,6) — row 17 occupied → invalid.
    let piece = t_piece(1, 5);
    hard_drop(&mut grid, &piece);
    assert_eq!(
        grid.cells[15][5],
        CellState::Occupied(TetrominoType::T as u32),
        "T top cell locked at row 15 when row 17 is occupied"
    );
    assert_eq!(
        grid.cells[16][4],
        CellState::Occupied(TetrominoType::T as u32),
        "T bottom cells locked at row 16"
    );
}

#[test]
fn hard_drop_clears_completed_line() {
    let mut grid = Grid::new();
    // Fill row 19 cols 0..4 and 7..10 (7 cells), leaving 4,5,6 for T-piece bottom row.
    for col in 0..4usize {
        grid.cells[19][col] = CellState::Occupied(0);
    }
    for col in 7..10usize {
        grid.cells[19][col] = CellState::Occupied(0);
    }
    // T rot-0 at col=5 drops to row=19; adds cells at (18,5),(19,4),(19,5),(19,6).
    // Row 19: 7 pre-filled + 3 from T = 10 → full line cleared.
    let piece = t_piece(1, 5);
    let cleared = hard_drop(&mut grid, &piece);
    assert_eq!(cleared, 1, "hard_drop must return 1 when a line is completed");
}

// ── rotate ────────────────────────────────────────────────────────────────────

#[test]
fn rotate_cw_succeeds_in_open_space() {
    let grid = Grid::new();
    let mut piece = t_piece(10, 5);
    let rotated = rotate(&grid, &mut piece, true);
    assert!(rotated, "CW rotate in open space must return true");
    assert_eq!(piece.rotation, 1, "rotation must become 1 after CW from 0");
}

#[test]
fn rotate_ccw_succeeds_in_open_space() {
    let grid = Grid::new();
    let mut piece = t_piece(10, 5);
    let rotated = rotate(&grid, &mut piece, false);
    assert!(rotated, "CCW rotate in open space must return true");
    assert_eq!(piece.rotation, 3, "rotation must become 3 after CCW from 0");
}

#[test]
fn rotate_cw_wraps_from_3_to_0() {
    let grid = Grid::new();
    // T rot-3 cells: [(-1,0),(0,-1),(0,0),(1,0)] — valid at row=10, col=5
    let mut piece = ActivePiece {
        piece_type: TetrominoType::T,
        rotation: 3,
        row: 10,
        col: 5,
    };
    let rotated = rotate(&grid, &mut piece, true);
    assert!(rotated, "CW rotate from rotation 3 must succeed in open space");
    assert_eq!(piece.rotation, 0, "rotation must wrap from 3 to 0");
}

#[test]
fn rotate_ccw_wraps_from_0_to_3() {
    let grid = Grid::new();
    let mut piece = t_piece(10, 5);
    rotate(&grid, &mut piece, false);
    assert_eq!(piece.rotation, 3, "rotation must wrap from 0 to 3 on CCW");
}

#[test]
fn rotate_uses_srs_kick_when_unshifted_position_blocked_by_wall() {
    let grid = Grid::new();
    // T rot-1 at col=0, row=10: cells [(-1,0),(0,0),(0,1),(1,0)] → (9,0),(10,0),(10,1),(11,0) — valid.
    // CW rotate to rot-2 at same anchor col=0: cells [(0,-1),(0,0),(0,1),(1,0)] → (10,-1) invalid.
    // JLSTZ kick CW 1→2 offset[0] = (col_delta=1, row_delta=0):
    //   test_col=1, test_row=10 → cells (10,0),(10,1),(10,2),(11,1) — valid (empty grid).
    // First kick resolves: piece.col becomes 1, piece.rotation becomes 2.
    let mut piece = ActivePiece {
        piece_type: TetrominoType::T,
        rotation: 1,
        row: 10,
        col: 0,
    };
    let rotated = rotate(&grid, &mut piece, true);
    assert!(rotated, "rotate must succeed via SRS kick when unshifted is against wall");
    assert_eq!(piece.rotation, 2, "rotation must be updated to 2");
    assert_eq!(piece.col, 1, "kick col_delta=1 must be applied");
    assert_eq!(piece.row, 10, "kick row_delta=0 means row unchanged");
}

#[test]
fn rotate_returns_false_when_all_positions_blocked() {
    let mut grid = Grid::new();
    // T rot-1 at col=0, row=5: cells (4,0),(5,0),(5,1),(6,0) — valid (these are NOT in grid).
    // CW rotate to rot-2, unshifted col=0: cell (5,-1) → out of bounds (fails).
    // JLSTZ kick CW 1→2: offsets [(1,0),(1,1),(0,-2),(1,-2)]
    //   Kick 0: col=1, row=5 → cells (5,0),(5,1),(5,2),(6,1). Block (6,1).
    //   Kick 1: col=1, row=6 → cells (6,0),(6,1),(6,2),(7,1). (6,1) is occupied → fails.
    //   Kick 2: col=0, row=3 → cells (3,-1)... col -1 out of bounds → fails.
    //   Kick 3: col=1, row=3 → cells (3,0),(3,1),(3,2),(4,1). Block (4,1).
    // All attempts fail → returns false.
    grid.cells[6][1] = CellState::Occupied(0); // blocks kick 0 and kick 1
    grid.cells[4][1] = CellState::Occupied(0); // blocks kick 3
    let mut piece = ActivePiece {
        piece_type: TetrominoType::T,
        rotation: 1,
        row: 5,
        col: 0,
    };
    let rotated = rotate(&grid, &mut piece, true);
    assert!(!rotated, "rotate must return false when all SRS positions are blocked");
    assert_eq!(piece.rotation, 1, "rotation must be unchanged on failure");
    assert_eq!(piece.col, 0, "col must be unchanged on failure");
    assert_eq!(piece.row, 5, "row must be unchanged on failure");
}

#[test]
fn rotate_o_piece_succeeds_rotation_index_changes() {
    let grid = Grid::new();
    // O piece: all rotations identical, CW always succeeds (rotation index still increments mod 4).
    let mut piece = ActivePiece {
        piece_type: TetrominoType::O,
        rotation: 0,
        row: 10,
        col: 4,
    };
    let rotated = rotate(&grid, &mut piece, true);
    assert!(rotated, "O-piece rotation must always succeed in open space");
    assert_eq!(piece.rotation, 1);
}

#[test]
fn rotate_i_piece_cw_succeeds_in_open_space() {
    let grid = Grid::new();
    // I rot-0 cells: [(0,-1),(0,0),(0,1),(0,2)] — at row=10, col=5: (10,4),(10,5),(10,6),(10,7)
    let mut piece = i_piece(10, 5);
    let rotated = rotate(&grid, &mut piece, true);
    assert!(rotated, "I-piece CW rotate must succeed in open space");
    assert_eq!(piece.rotation, 1);
}

#[test]
fn rotate_unshifted_position_preferred_over_kick() {
    let grid = Grid::new();
    // Rotate in fully open space: unshifted must be chosen (col/row unchanged).
    let mut piece = t_piece(10, 5);
    let original_col = piece.col;
    let original_row = piece.row;
    rotate(&grid, &mut piece, true);
    assert_eq!(
        piece.col, original_col,
        "col must not change when unshifted rotation is valid"
    );
    assert_eq!(
        piece.row, original_row,
        "row must not change when unshifted rotation is valid"
    );
}

// ── lock_piece ────────────────────────────────────────────────────────────────

#[test]
fn lock_piece_writes_occupied_cells_with_type_index() {
    let mut grid = Grid::new();
    // T piece (type_index = 2)
    // T rot-0 at row=5, col=5: cells (4,5),(5,4),(5,5),(5,6)
    let piece = t_piece(5, 5);
    lock_piece(&mut grid, &piece);
    assert_eq!(
        grid.cells[4][5],
        CellState::Occupied(2),
        "T cell (-1,0) must be Occupied(2)"
    );
    assert_eq!(
        grid.cells[5][4],
        CellState::Occupied(2),
        "T cell (0,-1) must be Occupied(2)"
    );
    assert_eq!(
        grid.cells[5][5],
        CellState::Occupied(2),
        "T cell (0,0) must be Occupied(2)"
    );
    assert_eq!(
        grid.cells[5][6],
        CellState::Occupied(2),
        "T cell (0,1) must be Occupied(2)"
    );
}

#[test]
fn lock_piece_stores_correct_type_index_for_i_piece() {
    let mut grid = Grid::new();
    // I piece (type_index = 0)
    // I rot-0 cells: [(0,-1),(0,0),(0,1),(0,2)] at row=10, col=5: (10,4),(10,5),(10,6),(10,7)
    let piece = i_piece(10, 5);
    lock_piece(&mut grid, &piece);
    for col in [4usize, 5, 6, 7] {
        assert_eq!(
            grid.cells[10][col],
            CellState::Occupied(0),
            "I cell at col {} must be Occupied(0)", col
        );
    }
}

#[test]
fn lock_piece_stores_correct_type_index_for_l_piece() {
    let mut grid = Grid::new();
    // L piece (type_index = 6)
    // L rot-0 cells: [(-1,1),(0,-1),(0,0),(0,1)] at row=5, col=5:
    //   (-1+5,1+5)=(4,6), (0+5,-1+5)=(5,4), (0+5,0+5)=(5,5), (0+5,1+5)=(5,6)
    let piece = ActivePiece {
        piece_type: TetrominoType::L,
        rotation: 0,
        row: 5,
        col: 5,
    };
    lock_piece(&mut grid, &piece);
    assert_eq!(grid.cells[4][6], CellState::Occupied(6), "L top cell must be Occupied(6)");
    assert_eq!(grid.cells[5][4], CellState::Occupied(6), "L left cell must be Occupied(6)");
    assert_eq!(grid.cells[5][5], CellState::Occupied(6), "L center cell must be Occupied(6)");
    assert_eq!(grid.cells[5][6], CellState::Occupied(6), "L right cell must be Occupied(6)");
}

#[test]
fn lock_piece_returns_zero_when_no_line_cleared() {
    let mut grid = Grid::new();
    let piece = t_piece(10, 5);
    let cleared = lock_piece(&mut grid, &piece);
    assert_eq!(cleared, 0, "lock_piece returns 0 when no lines are completed");
}

#[test]
fn lock_piece_triggers_line_clear_and_returns_count() {
    let mut grid = Grid::new();
    // Pre-fill row 19 except cols 4,5,6 (which T will fill).
    for col in 0..4usize {
        grid.cells[19][col] = CellState::Occupied(0);
    }
    for col in 7..10usize {
        grid.cells[19][col] = CellState::Occupied(0);
    }
    // T rot-0 at row=19, col=5: cells (18,5),(19,4),(19,5),(19,6) — completes row 19.
    let piece = t_piece(19, 5);
    let cleared = lock_piece(&mut grid, &piece);
    assert_eq!(cleared, 1, "lock_piece must return 1 when one line is completed");
}

// ── clear_lines ───────────────────────────────────────────────────────────────

#[test]
fn clear_lines_returns_zero_when_no_complete_rows() {
    let mut grid = Grid::new();
    // Fill row 19 partially (not full)
    for col in 0..9usize {
        grid.cells[19][col] = CellState::Occupied(1);
    }
    let cleared = clear_lines(&mut grid);
    assert_eq!(cleared, 0, "no complete rows means 0 lines cleared");
}

#[test]
fn clear_lines_returns_one_for_single_complete_row() {
    let mut grid = Grid::new();
    fill_row(&mut grid, 19);
    let cleared = clear_lines(&mut grid);
    assert_eq!(cleared, 1, "one complete row must return 1");
}

#[test]
fn clear_lines_returns_two_for_two_complete_rows() {
    let mut grid = Grid::new();
    fill_row(&mut grid, 18);
    fill_row(&mut grid, 19);
    let cleared = clear_lines(&mut grid);
    assert_eq!(cleared, 2, "two complete rows must return 2");
}

#[test]
fn clear_lines_cleared_row_becomes_empty() {
    let mut grid = Grid::new();
    fill_row(&mut grid, 19);
    clear_lines(&mut grid);
    // Row 19 was cleared; after shifting, row 19 should be the old row 18 (which was empty).
    for col in 0..WIDTH {
        assert_eq!(
            grid.cells[19][col],
            CellState::Empty,
            "cleared row position must be empty after line clear (col {})", col
        );
    }
}

#[test]
fn clear_lines_rows_above_shift_down() {
    let mut grid = Grid::new();
    // Put a marker cell in row 17, col 0.
    grid.cells[17][0] = CellState::Occupied(5);
    // Fill row 18 completely — will be cleared.
    fill_row(&mut grid, 18);
    clear_lines(&mut grid);
    // Old row 17 should shift down to new row 18.
    assert_eq!(
        grid.cells[18][0],
        CellState::Occupied(5),
        "row above cleared line must shift down by one"
    );
    // New row 0 must be empty (inserted at top).
    for col in 0..WIDTH {
        assert_eq!(
            grid.cells[0][col],
            CellState::Empty,
            "new row 0 inserted at top must be empty (col {})", col
        );
    }
}

#[test]
fn clear_lines_two_cleared_rows_shift_by_two() {
    let mut grid = Grid::new();
    // Put a marker in row 16.
    grid.cells[16][3] = CellState::Occupied(7);
    // Fill rows 17 and 18 completely.
    fill_row(&mut grid, 17);
    fill_row(&mut grid, 18);
    clear_lines(&mut grid);
    // Old row 16 shifts down by 2 → new row 18.
    assert_eq!(
        grid.cells[18][3],
        CellState::Occupied(7),
        "marker row must shift down by 2 when two lines are cleared below it"
    );
    // New rows 0 and 1 must be empty.
    for col in 0..WIDTH {
        assert_eq!(grid.cells[0][col], CellState::Empty, "new row 0 must be empty");
        assert_eq!(grid.cells[1][col], CellState::Empty, "new row 1 must be empty");
    }
}

#[test]
fn clear_lines_non_full_row_not_cleared() {
    let mut grid = Grid::new();
    // Partially fill row 19 (9 of 10 cells).
    for col in 0..9usize {
        grid.cells[19][col] = CellState::Occupied(2);
    }
    clear_lines(&mut grid);
    // Row 19 must still have the 9 occupied cells.
    for col in 0..9usize {
        assert_eq!(
            grid.cells[19][col],
            CellState::Occupied(2),
            "partially filled row must not be cleared (col {})", col
        );
    }
    assert_eq!(
        grid.cells[19][9],
        CellState::Empty,
        "empty cell in partial row must remain empty"
    );
}

#[test]
fn clear_lines_grid_height_preserved() {
    let mut grid = Grid::new();
    fill_row(&mut grid, 10);
    fill_row(&mut grid, 15);
    clear_lines(&mut grid);
    // The grid must still have HEIGHT rows accessible.
    // Simply check row 0 and row HEIGHT-1 don't panic.
    let _ = grid.cells[0][0];
    let _ = grid.cells[HEIGHT - 1][0];
}
