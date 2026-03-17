// @spec-tags: core,grid
// @invariants: Grid WIDTH=10 HEIGHT=20 constants are correct; Cell is Empty|Occupied(Color); new grid is all-Empty; traits and Default impl are correct
// @build: 7

use rhythmgrid::grid::{Cell, Color, Grid, HEIGHT, WIDTH};

// ── grid.dimensions ──────────────────────────────────────────────────────────

#[test]
fn width_constant_is_ten() {
    assert_eq!(WIDTH, 10);
}

#[test]
fn height_constant_is_twenty() {
    assert_eq!(HEIGHT, 20);
}

#[test]
fn grid_width_method_matches_constant() {
    let g = Grid::new();
    assert_eq!(g.width(), WIDTH);
    assert_eq!(g.width(), 10);
}

#[test]
fn grid_height_method_matches_constant() {
    let g = Grid::new();
    assert_eq!(g.height(), HEIGHT);
    assert_eq!(g.height(), 20);
}

#[test]
fn grid_cells_array_has_height_rows() {
    let g = Grid::new();
    assert_eq!(g.cells.len(), HEIGHT);
}

#[test]
fn grid_cells_array_has_width_columns() {
    let g = Grid::new();
    assert_eq!(g.cells[0].len(), WIDTH);
}

#[test]
fn grid_default_equals_new() {
    let g1 = Grid::new();
    let g2 = Grid::default();
    // Both should have all cells empty
    for row in 0..HEIGHT {
        for col in 0..WIDTH {
            assert_eq!(g1.get(row, col), g2.get(row, col));
        }
    }
}

// ── grid.cell_state ───────────────────────────────────────────────────────────

#[test]
fn new_grid_all_cells_are_empty() {
    let g = Grid::new();
    for row in 0..HEIGHT {
        for col in 0..WIDTH {
            assert_eq!(g.get(row, col), &Cell::Empty);
        }
    }
}

#[test]
fn cell_default_is_empty() {
    let c: Cell = Default::default();
    assert_eq!(c, Cell::Empty);
}

#[test]
fn cell_empty_is_not_occupied() {
    let c = Cell::Empty;
    assert!(matches!(c, Cell::Empty));
    assert!(!matches!(c, Cell::Occupied(_)));
}

#[test]
fn cell_occupied_holds_color() {
    let color = Color(255, 128, 0);
    let c = Cell::Occupied(color);
    assert!(matches!(c, Cell::Occupied(_)));
    if let Cell::Occupied(Color(r, g, b)) = c {
        assert_eq!(r, 255);
        assert_eq!(g, 128);
        assert_eq!(b, 0);
    } else {
        panic!("expected Cell::Occupied");
    }
}

#[test]
fn cell_equality_empty() {
    assert_eq!(Cell::Empty, Cell::Empty);
    assert_ne!(Cell::Empty, Cell::Occupied(Color(0, 0, 0)));
}

#[test]
fn cell_equality_occupied_same_color() {
    let a = Cell::Occupied(Color(1, 2, 3));
    let b = Cell::Occupied(Color(1, 2, 3));
    assert_eq!(a, b);
}

#[test]
fn cell_equality_occupied_different_color() {
    let a = Cell::Occupied(Color(1, 2, 3));
    let b = Cell::Occupied(Color(9, 8, 7));
    assert_ne!(a, b);
}

#[test]
fn cell_clone_produces_equal_value() {
    let original = Cell::Occupied(Color(10, 20, 30));
    let cloned = original.clone();
    assert_eq!(original, cloned);
}

#[test]
fn color_fields_are_accessible() {
    let c = Color(11, 22, 33);
    assert_eq!(c.0, 11);
    assert_eq!(c.1, 22);
    assert_eq!(c.2, 33);
}

#[test]
fn color_derives_partialeq() {
    assert_eq!(Color(1, 2, 3), Color(1, 2, 3));
    assert_ne!(Color(1, 2, 3), Color(1, 2, 4));
}

#[test]
fn color_derives_clone() {
    let a = Color(5, 6, 7);
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn grid_set_and_get_round_trip() {
    let mut g = Grid::new();
    let color = Color(200, 100, 50);
    g.set(3, 7, Cell::Occupied(color));
    assert_eq!(g.get(3, 7), &Cell::Occupied(Color(200, 100, 50)));
    // Other cells remain empty
    assert_eq!(g.get(0, 0), &Cell::Empty);
    assert_eq!(g.get(19, 9), &Cell::Empty);
}

#[test]
fn grid_set_then_clear_cell() {
    let mut g = Grid::new();
    g.set(0, 0, Cell::Occupied(Color(1, 1, 1)));
    g.set(0, 0, Cell::Empty);
    assert_eq!(g.get(0, 0), &Cell::Empty);
}
