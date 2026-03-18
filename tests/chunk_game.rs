// @spec-tags: game.line_clear
// @invariants: clear_lines returns usize count of cleared rows; multiple simultaneous clears supported
// @build: 18

use rhythm_grid::game::Game;
use rhythm_grid::grid::{CellState, GRID_WIDTH, GRID_HEIGHT};

/// Fill a complete row at the given row index with Occupied cells (type_index 0 = I-piece).
fn fill_row(game: &mut Game, row: usize) {
    for col in 0..GRID_WIDTH as usize {
        game.grid[row][col] = CellState::Occupied(0);
    }
}

#[test]
fn test_clear_single_full_row_returns_1() {
    let mut game = Game::new();
    game.start();
    // Fill bottom row completely
    fill_row(&mut game, GRID_HEIGHT as usize - 1);
    // Trigger a lock event (hard_drop on whatever piece is active)
    game.hard_drop();
    // The clear count from this lock should be at least 1
    assert_eq!(game.last_clear_count(), 1);
}

#[test]
fn test_clear_two_full_rows_returns_2() {
    let mut game = Game::new();
    game.start();
    let h = GRID_HEIGHT as usize;
    fill_row(&mut game, h - 1);
    fill_row(&mut game, h - 2);
    game.hard_drop();
    assert_eq!(game.last_clear_count(), 2);
}

#[test]
fn test_clear_four_full_rows_returns_4() {
    let mut game = Game::new();
    game.start();
    let h = GRID_HEIGHT as usize;
    fill_row(&mut game, h - 1);
    fill_row(&mut game, h - 2);
    fill_row(&mut game, h - 3);
    fill_row(&mut game, h - 4);
    game.hard_drop();
    assert_eq!(game.last_clear_count(), 4);
}

#[test]
fn test_no_full_rows_returns_0() {
    let mut game = Game::new();
    game.start();
    // Fill only 9 of 10 columns in the bottom row — row is not complete
    for col in 0..9 {
        game.grid[GRID_HEIGHT as usize - 1][col] = CellState::Occupied(0);
    }
    game.hard_drop();
    assert_eq!(game.last_clear_count(), 0);
}

#[test]
fn test_rows_above_shift_down_after_clear() {
    let mut game = Game::new();
    game.start();
    let h = GRID_HEIGHT as usize;
    // Fill bottom row completely
    fill_row(&mut game, h - 1);
    // Place a partial marker in the row above (column 0 only)
    game.grid[h - 2][0] = CellState::Occupied(1); // O-piece index
    // Trigger lock/clear
    game.hard_drop();
    // Bottom row (h-1) should now be empty (the full row was cleared and content shifted down)
    assert!(matches!(game.grid[h - 1][0], CellState::Occupied(1)),
        "partial row should have shifted down to row h-1");
    // The row that was h-2 should now be at h-1; row above (h-2) should be empty
    assert!(matches!(game.grid[h - 2][0], CellState::Empty),
        "row h-2 should be empty after shift");
}

#[test]
fn test_lines_cleared_accumulates_across_locks() {
    let mut game = Game::new();
    game.start();
    let h = GRID_HEIGHT as usize;

    // First lock: clear 1 row
    fill_row(&mut game, h - 1);
    game.hard_drop();
    let after_first = game.lines_cleared();

    // Second lock: clear 1 more row
    fill_row(&mut game, h - 1);
    game.hard_drop();
    let after_second = game.lines_cleared();

    assert_eq!(after_second, after_first + 1,
        "lines_cleared should accumulate: got {} then {}", after_first, after_second);
}

#[test]
fn test_last_clear_count_resets_each_lock() {
    let mut game = Game::new();
    game.start();
    let h = GRID_HEIGHT as usize;

    // First lock clears 2 rows
    fill_row(&mut game, h - 1);
    fill_row(&mut game, h - 2);
    game.hard_drop();
    assert_eq!(game.last_clear_count(), 2);

    // Second lock clears 0 rows
    game.hard_drop();
    assert_eq!(game.last_clear_count(), 0,
        "last_clear_count must reset to 0 when no rows are cleared");
}
