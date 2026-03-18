// @spec-tags: game.level
// @invariants: level starts at 1, increases by 1 every 10 lines cleared, computed as (lines_cleared / 10) + 1
// @build: 19

use rhythm_grid::game::Game;
use rhythm_grid::grid::{CellState, GRID_WIDTH, GRID_HEIGHT};

/// Fill a complete row at the given row index with Occupied cells.
fn fill_row(game: &mut Game, row: usize) {
    for col in 0..GRID_WIDTH as usize {
        game.grid[row][col] = CellState::Occupied(0);
    }
}

// ─── game.level ─────────────────────────────────────────────────────────────

#[test]
fn test_level_starts_at_1_on_new_game() {
    let mut game = Game::new();
    game.start();
    assert_eq!(game.level(), 1, "level must be 1 at game start (0 lines cleared)");
}

#[test]
fn test_level_is_1_before_any_lines_cleared() {
    let mut game = Game::new();
    game.start();
    // No rows filled; hard_drop clears nothing
    game.hard_drop();
    assert_eq!(game.level(), 1, "level must remain 1 with 0 lines cleared");
}

#[test]
fn test_level_is_1_with_9_lines_cleared() {
    // Clear 9 lines total (9 < 10) — level must stay at 1
    let mut game = Game::new();
    game.start();
    let h = GRID_HEIGHT as usize;
    for _ in 0..9 {
        fill_row(&mut game, h - 1);
        game.hard_drop();
    }
    assert_eq!(game.lines_cleared(), 9, "setup: expected 9 lines cleared");
    assert_eq!(game.level(), 1, "level must be 1 with 9 lines cleared");
}

#[test]
fn test_level_increases_to_2_at_10_lines_cleared() {
    // Clearing exactly 10 lines triggers level 2
    let mut game = Game::new();
    game.start();
    let h = GRID_HEIGHT as usize;
    for _ in 0..10 {
        fill_row(&mut game, h - 1);
        game.hard_drop();
    }
    assert_eq!(game.lines_cleared(), 10, "setup: expected 10 lines cleared");
    assert_eq!(game.level(), 2, "level must be 2 at exactly 10 lines cleared");
}

#[test]
fn test_level_is_2_with_19_lines_cleared() {
    // 19 lines cleared: (19 / 10) + 1 = 2
    let mut game = Game::new();
    game.start();
    let h = GRID_HEIGHT as usize;
    for _ in 0..19 {
        fill_row(&mut game, h - 1);
        game.hard_drop();
    }
    assert_eq!(game.lines_cleared(), 19, "setup: expected 19 lines cleared");
    assert_eq!(game.level(), 2, "level must be 2 with 19 lines cleared");
}

#[test]
fn test_level_increases_to_3_at_20_lines_cleared() {
    // 20 lines cleared: (20 / 10) + 1 = 3
    let mut game = Game::new();
    game.start();
    let h = GRID_HEIGHT as usize;
    for _ in 0..20 {
        fill_row(&mut game, h - 1);
        game.hard_drop();
    }
    assert_eq!(game.lines_cleared(), 20, "setup: expected 20 lines cleared");
    assert_eq!(game.level(), 3, "level must be 3 at exactly 20 lines cleared");
}

#[test]
fn test_level_formula_lines_per_level_is_10() {
    // Verify the boundary is exactly 10: level changes at multiples of 10
    let mut game = Game::new();
    game.start();
    let h = GRID_HEIGHT as usize;

    // Clear 9 lines
    for _ in 0..9 {
        fill_row(&mut game, h - 1);
        game.hard_drop();
    }
    let level_at_9 = game.level();

    // Clear 1 more to hit 10
    fill_row(&mut game, h - 1);
    game.hard_drop();
    let level_at_10 = game.level();

    assert_eq!(level_at_9, 1, "level must be 1 at 9 lines");
    assert_eq!(level_at_10, 2, "level must be 2 at 10 lines — lines_per_level boundary is 10");
    assert_eq!(level_at_10, level_at_9 + 1, "level must increment by exactly 1 at the 10-line boundary");
}

#[test]
fn test_level_is_computed_from_lines_cleared() {
    // level() must equal (lines_cleared() / 10) + 1 at any point
    let mut game = Game::new();
    game.start();
    let h = GRID_HEIGHT as usize;

    for expected_clears in [0usize, 1, 5, 9, 10, 11, 19, 20] {
        // Reset game state for each sub-case would require a fresh game; instead we step through
        let _ = expected_clears; // structural placeholder
    }

    // Fresh game for deterministic check at 15 lines
    let mut game2 = Game::new();
    game2.start();
    for _ in 0..15 {
        fill_row(&mut game2, h - 1);
        game2.hard_drop();
    }
    let lc = game2.lines_cleared();
    let expected_level = (lc / 10) + 1;
    assert_eq!(game2.level(), expected_level,
        "level() must equal (lines_cleared() / 10) + 1; lines_cleared={}, expected level={}",
        lc, expected_level);
}

#[test]
fn test_level_is_public_and_queryable_without_state_mutation() {
    // level() must be callable on a shared reference (externally queryable)
    let mut game = Game::new();
    game.start();
    let game_ref: &Game = &game;
    let _level = game_ref.level();
    // If this compiles and returns a value, the method is accessible to external systems
    assert!(_level >= 1, "level() must return a value >= 1 on a shared reference");
}

#[test]
fn test_starting_level_constant_is_1() {
    // The spec mandates starting_level = 1; a fresh game must reflect this
    let mut game = Game::new();
    game.start();
    // With zero lines cleared the formula (0 / 10) + 1 = 1 matches starting_level
    assert_eq!(game.level(), 1,
        "starting_level constant is 1; level() must return 1 before any clears");
}
