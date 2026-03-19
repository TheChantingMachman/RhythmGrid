// @spec-tags: game.collision
// @invariants: is_valid_position returns true iff all piece cells land within rows 0..20, cols 0..10, and none overlap CellState::Occupied(_); cell coordinates are (row,col) i32 with anchor + offset; one out-of-bounds or occupied cell invalidates the whole piece
// @build: 30

use rhythm_grid::game::is_valid_position;
use rhythm_grid::grid::{CellState, Grid};

// Grid bounds from spec: rows 0..20 (HEIGHT=20), cols 0..10 (WIDTH=10).
// is_valid_position(grid, cells, row, col) where cells are (row_offset, col_offset) i32 pairs.
// Absolute position of cell i = (row + cells[i].0, col + cells[i].1).

// Helper: single-cell piece with offset (0,0) — anchor position IS the absolute position.
fn single_cell() -> Vec<(i32, i32)> {
    vec![(0, 0)]
}

// Helper: 2×2 block offsets (anchor = top-left).
fn block_2x2() -> Vec<(i32, i32)> {
    vec![(0, 0), (0, 1), (1, 0), (1, 1)]
}

// ── In-bounds / empty-grid cases ─────────────────────────────────────────────

#[test]
fn piece_in_center_of_empty_grid_is_valid() {
    let grid = Grid::new();
    // 2×2 block anchored at (9, 4): cells (9,4),(9,5),(10,4),(10,5) — all in bounds.
    assert!(
        is_valid_position(&grid, &block_2x2(), 9, 4),
        "2×2 piece in centre of empty grid must be valid"
    );
}

#[test]
fn piece_at_top_left_corner_is_valid() {
    let grid = Grid::new();
    assert!(
        is_valid_position(&grid, &single_cell(), 0, 0),
        "single cell at (0,0) on empty grid must be valid"
    );
}

#[test]
fn piece_at_bottom_right_corner_is_valid() {
    let grid = Grid::new();
    assert!(
        is_valid_position(&grid, &single_cell(), 19, 9),
        "single cell at (19,9) on empty grid must be valid"
    );
}

// ── Out-of-bounds cases ───────────────────────────────────────────────────────

#[test]
fn piece_partially_outside_left_bound_is_invalid() {
    let grid = Grid::new();
    // Anchor col = -1, offset (0,0) → absolute col = -1 < 0.
    assert!(
        !is_valid_position(&grid, &single_cell(), 5, -1),
        "cell at col -1 must be invalid (left out-of-bounds)"
    );
}

#[test]
fn piece_partially_outside_right_bound_is_invalid() {
    let grid = Grid::new();
    // Absolute col = 10 >= WIDTH(10) → invalid.
    assert!(
        !is_valid_position(&grid, &single_cell(), 5, 10),
        "cell at col 10 must be invalid (right out-of-bounds)"
    );
}

#[test]
fn piece_partially_below_grid_is_invalid() {
    let grid = Grid::new();
    // Anchor row = 20 >= HEIGHT(20) → invalid.
    assert!(
        !is_valid_position(&grid, &single_cell(), 20, 5),
        "cell at row 20 must be invalid (below grid)"
    );
}

#[test]
fn piece_one_cell_out_of_bounds_invalidates_whole_piece() {
    let grid = Grid::new();
    // 2×2 block at (0, 9): cells abs = (0,9),(0,10),(1,9),(1,10)
    // (0,10) and (1,10) are out of bounds — whole piece invalid.
    assert!(
        !is_valid_position(&grid, &block_2x2(), 0, 9),
        "one out-of-bounds cell must invalidate the entire piece"
    );
}

// ── Occupied-cell overlap cases ───────────────────────────────────────────────

#[test]
fn piece_overlapping_occupied_cell_is_invalid() {
    let mut grid = Grid::new();
    grid.cells[5][5] = CellState::Occupied(1);
    // Single cell placed directly on the occupied cell.
    assert!(
        !is_valid_position(&grid, &single_cell(), 5, 5),
        "cell overlapping CellState::Occupied must be invalid"
    );
}

#[test]
fn piece_adjacent_to_occupied_cell_is_valid() {
    let mut grid = Grid::new();
    grid.cells[5][5] = CellState::Occupied(1);
    // Place piece at (5,6) — adjacent but not overlapping.
    assert!(
        is_valid_position(&grid, &single_cell(), 5, 6),
        "cell adjacent to (but not overlapping) occupied cell must be valid"
    );
}

#[test]
fn all_cells_occupied_overlap_is_invalid() {
    let mut grid = Grid::new();
    // Fill a 2×2 block with occupied cells.
    grid.cells[3][3] = CellState::Occupied(2);
    grid.cells[3][4] = CellState::Occupied(2);
    grid.cells[4][3] = CellState::Occupied(2);
    grid.cells[4][4] = CellState::Occupied(2);
    // Piece exactly over all four occupied cells.
    assert!(
        !is_valid_position(&grid, &block_2x2(), 3, 3),
        "piece fully overlapping occupied cells must be invalid"
    );
}
