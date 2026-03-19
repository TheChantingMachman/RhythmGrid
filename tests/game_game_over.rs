// @spec-tags: game.game_over
// @invariants: is_game_over(grid, piece) returns true when is_valid_position returns false for the piece's current cells at (row,col); false when position is valid; used to detect immediate collision of a newly spawned piece
// @build: 43

use rhythm_grid::game::{is_game_over, ActivePiece};
use rhythm_grid::grid::{CellState, Grid};
use rhythm_grid::pieces::TetrominoType;

// T rot-0 cells: [(-1,0),(0,-1),(0,0),(0,1)]
// At row=1, col=5: absolute positions (0,5),(1,4),(1,5),(1,6)
fn spawn_t() -> ActivePiece {
    ActivePiece { piece_type: TetrominoType::T, rotation: 0, row: 1, col: 5 }
}

// ── Not game over (valid spawn position) ──────────────────────────────────────

#[test]
fn is_game_over_false_on_empty_grid() {
    let grid = Grid::new();
    let piece = spawn_t();
    assert!(!is_game_over(&grid, &piece), "piece on empty grid must not be game over");
}

#[test]
fn is_game_over_false_when_occupied_cells_are_not_under_piece() {
    let mut grid = Grid::new();
    // Fill a cell completely separate from T spawn cells (0,5),(1,4),(1,5),(1,6)
    grid.cells[2][5] = CellState::Occupied(0);
    let piece = spawn_t();
    assert!(
        !is_game_over(&grid, &piece),
        "occupied cells not overlapping spawn must not be game over"
    );
}

#[test]
fn is_game_over_false_for_i_piece_on_empty_grid() {
    let grid = Grid::new();
    // I rot-0 cells: [(0,-1),(0,0),(0,1),(0,2)]
    // At row=1, col=5: (1,4),(1,5),(1,6),(1,7) — all within bounds and empty
    let piece = ActivePiece { piece_type: TetrominoType::I, rotation: 0, row: 1, col: 5 };
    assert!(!is_game_over(&grid, &piece), "I piece at spawn on empty grid must not be game over");
}

// ── Game over (invalid position: occupied cell overlap) ───────────────────────

#[test]
fn is_game_over_true_when_one_spawn_cell_is_occupied() {
    let mut grid = Grid::new();
    // T rot-0 at row=1, col=5: one of the cells is (1,5) — block it
    grid.cells[1][5] = CellState::Occupied(0);
    let piece = spawn_t();
    assert!(
        is_game_over(&grid, &piece),
        "one occupied cell under spawn must result in game over"
    );
}

#[test]
fn is_game_over_true_when_top_spawn_cell_is_occupied() {
    let mut grid = Grid::new();
    // T rot-0 at row=1, col=5: top cell is at (0,5)
    grid.cells[0][5] = CellState::Occupied(0);
    let piece = spawn_t();
    assert!(
        is_game_over(&grid, &piece),
        "occupied cell at top of spawn cells must result in game over"
    );
}

#[test]
fn is_game_over_true_when_all_spawn_cells_blocked() {
    let mut grid = Grid::new();
    // T rot-0 at row=1, col=5: cells (0,5),(1,4),(1,5),(1,6)
    grid.cells[0][5] = CellState::Occupied(0);
    grid.cells[1][4] = CellState::Occupied(0);
    grid.cells[1][5] = CellState::Occupied(0);
    grid.cells[1][6] = CellState::Occupied(0);
    let piece = spawn_t();
    assert!(
        is_game_over(&grid, &piece),
        "all spawn cells blocked must be game over"
    );
}

// ── Game over (invalid position: out-of-bounds) ───────────────────────────────

#[test]
fn is_game_over_false_when_piece_entirely_in_vanish_zone() {
    let grid = Grid::new();
    // T rot-0 at row=-1, col=5: cells (-1+(-1),5+0)=(-2,5), (-1+0,5+(-1))=(-1,4), (-1+0,5+0)=(-1,5), (-1+0,5+1)=(-1,6)
    // All absolute rows < 0 → all in vanish zone → all unconditionally valid → is_game_over returns false
    let piece = ActivePiece { piece_type: TetrominoType::T, rotation: 0, row: -1, col: 5 };
    assert!(
        !is_game_over(&grid, &piece),
        "piece entirely in the vanish zone (all rows < 0) must not be game over"
    );
}

// ── Return type matches bool ──────────────────────────────────────────────────

#[test]
fn is_game_over_return_type_is_bool() {
    let grid = Grid::new();
    let piece = spawn_t();
    let result: bool = is_game_over(&grid, &piece);
    assert!(!result);
}
