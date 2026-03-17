// @spec-tags: core,grid
// @invariants: GRID_WIDTH=10, GRID_HEIGHT=20; CellState has Empty and Occupied(color) variants
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
    let color: u32 = 0xFF0000FF; // red, fully opaque RGBA
    let cell = CellState::Occupied(color);
    assert!(matches!(cell, CellState::Occupied(_)));
}

#[test]
fn test_cell_state_occupied_does_not_match_empty() {
    let cell = CellState::Occupied(0x00FF00FF_u32);
    assert!(!matches!(cell, CellState::Empty));
}

#[test]
fn test_cell_state_occupied_preserves_color_value() {
    let color: u32 = 0x1A2B3C4D;
    let cell = CellState::Occupied(color);
    if let CellState::Occupied(stored) = cell {
        assert_eq!(stored, color);
    } else {
        panic!("Expected Occupied variant");
    }
}

#[test]
fn test_cell_state_occupied_distinct_colors_are_distinct() {
    let red: u32 = 0xFF0000FF;
    let blue: u32 = 0x0000FFFF;
    let cell_red = CellState::Occupied(red);
    let cell_blue = CellState::Occupied(blue);
    if let (CellState::Occupied(r), CellState::Occupied(b)) = (cell_red, cell_blue) {
        assert_ne!(r, b);
    } else {
        panic!("Expected two Occupied variants");
    }
}
