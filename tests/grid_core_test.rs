// @spec-tags: core,grid
// @invariants: Grid dimensions are 10 wide × 20 tall; cells are Empty or Occupied(Color)
// @build: 11

use rhythm_grid::grid::{Grid, CellState, Color, WIDTH, HEIGHT};

// --- grid.dimensions ---

#[test]
fn test_width_constant_is_10() {
    assert_eq!(WIDTH, 10);
}

#[test]
fn test_height_constant_is_20() {
    assert_eq!(HEIGHT, 20);
}

#[test]
fn test_grid_width_method_returns_10() {
    let g = Grid::new();
    assert_eq!(g.width(), 10);
}

#[test]
fn test_grid_height_method_returns_20() {
    let g = Grid::new();
    assert_eq!(g.height(), 20);
}

#[test]
fn test_new_grid_all_cells_empty() {
    let g = Grid::new();
    for row in 0..HEIGHT {
        for col in 0..WIDTH {
            assert_eq!(
                g.get(col, row),
                Some(&CellState::Empty),
                "cell ({col},{row}) should be Empty after new()"
            );
        }
    }
}

#[test]
fn test_get_out_of_bounds_returns_none() {
    let g = Grid::new();
    assert_eq!(g.get(WIDTH, 0), None);
    assert_eq!(g.get(0, HEIGHT), None);
    assert_eq!(g.get(WIDTH, HEIGHT), None);
}

// --- grid.cell_state ---

#[test]
fn test_cell_state_empty_variant() {
    let state = CellState::Empty;
    assert_eq!(state, CellState::Empty);
}

#[test]
fn test_cell_state_occupied_with_color() {
    let color = Color::new(255, 128, 0);
    let state = CellState::Occupied(color);
    assert_eq!(state, CellState::Occupied(Color { r: 255, g: 128, b: 0 }));
}

#[test]
fn test_color_rgb_fields() {
    let c = Color::new(10, 20, 30);
    assert_eq!(c.r, 10);
    assert_eq!(c.g, 20);
    assert_eq!(c.b, 30);
}

#[test]
fn test_set_cell_to_occupied() {
    let mut g = Grid::new();
    let color = Color::new(1, 2, 3);
    let result = g.set(0, 0, CellState::Occupied(color));
    assert!(result, "set() should return true for in-bounds cell");
    assert_eq!(g.get(0, 0), Some(&CellState::Occupied(Color::new(1, 2, 3))));
}

#[test]
fn test_set_cell_back_to_empty() {
    let mut g = Grid::new();
    g.set(3, 5, CellState::Occupied(Color::new(255, 0, 0)));
    g.set(3, 5, CellState::Empty);
    assert_eq!(g.get(3, 5), Some(&CellState::Empty));
}

#[test]
fn test_set_out_of_bounds_returns_false() {
    let mut g = Grid::new();
    assert!(!g.set(WIDTH, 0, CellState::Empty));
    assert!(!g.set(0, HEIGHT, CellState::Empty));
}

#[test]
fn test_occupied_color_equality() {
    let c1 = Color::new(100, 150, 200);
    let c2 = Color::new(100, 150, 200);
    assert_eq!(CellState::Occupied(c1), CellState::Occupied(c2));
}

#[test]
fn test_empty_ne_occupied() {
    let state = CellState::Occupied(Color::new(0, 0, 0));
    assert_ne!(CellState::Empty, state);
}
