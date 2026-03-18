// @spec-tags: grid.dimensions,grid.cell_state
// @invariants: Grid is 10 columns wide, 20 rows tall; cells initialise to Empty; CellState is Empty or Occupied(u32)
// @build: 26

use rhythm_grid::{
    grid::{CellState, Grid, HEIGHT, WIDTH},
};

// ── Constants ────────────────────────────────────────────────────────────────

#[test]
fn width_constant_is_10() {
    assert_eq!(WIDTH, 10usize);
}

#[test]
fn height_constant_is_20() {
    assert_eq!(HEIGHT, 20usize);
}

// ── Grid::new ────────────────────────────────────────────────────────────────

#[test]
fn grid_new_cells_array_has_20_rows() {
    let g = Grid::new();
    assert_eq!(g.cells.len(), 20);
}

#[test]
fn grid_new_every_row_has_10_columns() {
    let g = Grid::new();
    for row in &g.cells {
        assert_eq!(row.len(), 10);
    }
}

#[test]
fn grid_new_all_cells_are_empty() {
    let g = Grid::new();
    for row in &g.cells {
        for cell in row {
            assert_eq!(*cell, CellState::Empty);
        }
    }
}

// ── CellState variants ───────────────────────────────────────────────────────

#[test]
fn cell_state_empty_equals_empty() {
    assert_eq!(CellState::Empty, CellState::Empty);
}

#[test]
fn cell_state_occupied_carries_u32_value() {
    let c = CellState::Occupied(7u32);
    assert_eq!(c, CellState::Occupied(7));
}

#[test]
fn cell_state_occupied_different_values_are_not_equal() {
    assert_ne!(CellState::Occupied(0), CellState::Occupied(1));
}

#[test]
fn cell_state_empty_not_equal_to_occupied() {
    assert_ne!(CellState::Empty, CellState::Occupied(0));
}

#[test]
fn cell_state_occupied_max_u32() {
    let c = CellState::Occupied(u32::MAX);
    assert_eq!(c, CellState::Occupied(u32::MAX));
}

// ── CellState derived traits ─────────────────────────────────────────────────

#[test]
fn cell_state_is_copy() {
    let a = CellState::Occupied(42);
    let b = a; // copy
    assert_eq!(a, b);
}

#[test]
fn cell_state_is_clone() {
    let a = CellState::Occupied(3);
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn cell_state_debug_is_implemented() {
    // Just ensure it compiles and produces a non-empty string
    let s = format!("{:?}", CellState::Empty);
    assert!(!s.is_empty());
    let s2 = format!("{:?}", CellState::Occupied(5));
    assert!(!s2.is_empty());
}

// ── Direct cell read/write via pub field ─────────────────────────────────────

#[test]
fn cells_field_is_publicly_writable() {
    let mut g = Grid::new();
    g.cells[0][0] = CellState::Occupied(1);
    assert_eq!(g.cells[0][0], CellState::Occupied(1));
}

#[test]
fn write_last_row_last_col_and_read_back() {
    let mut g = Grid::new();
    g.cells[HEIGHT - 1][WIDTH - 1] = CellState::Occupied(99);
    assert_eq!(g.cells[HEIGHT - 1][WIDTH - 1], CellState::Occupied(99));
}

#[test]
fn writing_one_cell_does_not_affect_neighbours() {
    let mut g = Grid::new();
    g.cells[5][5] = CellState::Occupied(10);
    assert_eq!(g.cells[5][4], CellState::Empty);
    assert_eq!(g.cells[5][6], CellState::Empty);
    assert_eq!(g.cells[4][5], CellState::Empty);
    assert_eq!(g.cells[6][5], CellState::Empty);
}

// ── Type-level dimension check ───────────────────────────────────────────────

#[test]
fn cells_type_matches_fixed_size_array() {
    // Compile-time proof: Grid::new().cells must be [[CellState; 10]; 20].
    // Assigning to a typed local would fail to compile if the type doesn't match.
    let g = Grid::new();
    let _: [[CellState; 10]; 20] = g.cells;
}
