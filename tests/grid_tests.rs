// @spec-tags: core,grid
// @invariants: GRID_WIDTH=10, GRID_HEIGHT=20; CellState has Empty and Occupied(type_index) variants where type_index is 0-6 matching ALL_TETROMINOES order
// @build: 12

use rhythm_grid::grid::{CellState, GRID_HEIGHT, GRID_WIDTH};

#[test]
fn test_grid_width_constant_is_10() {
    assert_eq!(GRID_WIDTH, 10);
}

#[test]
fn test_grid_height_constant_is_20() {
    assert_eq!(GRID_HEIGHT, 20);
}

#[test]
fn test_grid_total_cells_derived_from_constants() {
    // 10 columns × 20 rows = 200 cells
    assert_eq!(GRID_WIDTH * GRID_HEIGHT, 200);
}

#[test]
fn test_cell_state_empty_variant_exists() {
    let cell = CellState::Empty;
    assert!(matches!(cell, CellState::Empty));
}

#[test]
fn test_cell_state_empty_does_not_match_occupied() {
    let cell = CellState::Empty;
    assert!(!matches!(cell, CellState::Occupied(_)));
}

#[test]
fn test_cell_state_occupied_variant_exists() {
    let cell = CellState::Occupied(0); // type_index 0 = I-piece
    assert!(matches!(cell, CellState::Occupied(_)));
}

#[test]
fn test_cell_state_occupied_does_not_match_empty() {
    let cell = CellState::Occupied(2); // type_index 2 = T-piece
    assert!(!matches!(cell, CellState::Empty));
}

#[test]
fn test_cell_state_occupied_preserves_type_index() {
    let cell = CellState::Occupied(3); // type_index 3 = S-piece
    if let CellState::Occupied(stored) = cell {
        assert_eq!(stored, 3u32);
    } else {
        panic!("Expected Occupied variant");
    }
}

#[test]
fn test_cell_state_occupied_distinct_type_indices_are_distinct() {
    let cell_i = CellState::Occupied(0); // I-piece index
    let cell_l = CellState::Occupied(6); // L-piece index
    if let (CellState::Occupied(i), CellState::Occupied(l)) = (cell_i, cell_l) {
        assert_ne!(i, l);
    } else {
        panic!("Expected two Occupied variants");
    }
}
