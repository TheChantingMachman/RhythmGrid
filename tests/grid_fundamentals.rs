// @spec-tags: core,grid
// @invariants: Grid dimensions are exactly 10 wide x 20 tall; CellState is Empty or Occupied with a color
// @build: 22

use rhythm_grid::grid::{CellState, GRID_HEIGHT, GRID_WIDTH};

#[test]
fn test_grid_width_constant() {
    assert_eq!(GRID_WIDTH, 10);
}

#[test]
fn test_grid_height_constant() {
    assert_eq!(GRID_HEIGHT, 20);
}

#[test]
fn test_grid_dimensions_correct_ratio() {
    // Height must be exactly double the width
    assert_eq!(GRID_HEIGHT, GRID_WIDTH * 2);
}

#[test]
fn test_cell_state_empty_variant_exists() {
    let cell = CellState::Empty;
    assert!(matches!(cell, CellState::Empty));
}

#[test]
fn test_cell_state_occupied_variant_carries_color() {
    // Occupied wraps a color value; we use a concrete color to verify round-trip
    let color = [255u8, 128u8, 0u8];
    let cell = CellState::Occupied(color);
    match cell {
        CellState::Occupied(c) => assert_eq!(c, [255u8, 128u8, 0u8]),
        CellState::Empty => panic!("expected Occupied, got Empty"),
    }
}

#[test]
fn test_cell_state_empty_is_not_occupied() {
    let cell = CellState::Empty;
    assert!(!matches!(cell, CellState::Occupied(_)));
}

#[test]
fn test_cell_state_occupied_is_not_empty() {
    let cell = CellState::Occupied([0u8, 0u8, 0u8]);
    assert!(!matches!(cell, CellState::Empty));
}

#[test]
fn test_cell_state_different_colors_are_distinct() {
    let red = CellState::Occupied([255u8, 0u8, 0u8]);
    let blue = CellState::Occupied([0u8, 0u8, 255u8]);
    match (red, blue) {
        (CellState::Occupied(r), CellState::Occupied(b)) => assert_ne!(r, b),
        _ => panic!("both cells should be Occupied"),
    }
}
