// @spec-tags: game.gravity
// @invariants: gravity_interval_ms(level) returns max(100, 1000-(level-1)*100) clamped to 100ms at level 10+; gravity_tick returns (true,0) when move_down succeeds after interval elapses, or (false,accumulated_ms) unchanged when interval has not yet elapsed
// @build: 43

use rhythm_grid::game::{gravity_interval_ms, gravity_tick, ActivePiece};
use rhythm_grid::grid::Grid;
use rhythm_grid::pieces::TetrominoType;

fn t_piece(row: i32, col: i32) -> ActivePiece {
    ActivePiece { piece_type: TetrominoType::T, rotation: 0, row, col }
}

// ── gravity_interval_ms ───────────────────────────────────────────────────────

#[test]
fn gravity_interval_level_1_is_1000ms() {
    // max(100, 1000 - (1-1)*100) = max(100, 1000) = 1000
    assert_eq!(gravity_interval_ms(1), 1000);
}

#[test]
fn gravity_interval_level_2_is_900ms() {
    // max(100, 1000 - (2-1)*100) = max(100, 900) = 900
    assert_eq!(gravity_interval_ms(2), 900);
}

#[test]
fn gravity_interval_level_5_is_600ms() {
    // max(100, 1000 - (5-1)*100) = max(100, 600) = 600
    assert_eq!(gravity_interval_ms(5), 600);
}

#[test]
fn gravity_interval_level_9_is_200ms() {
    // max(100, 1000 - (9-1)*100) = max(100, 200) = 200
    assert_eq!(gravity_interval_ms(9), 200);
}

#[test]
fn gravity_interval_level_10_clamps_to_100ms() {
    // max(100, 1000 - (10-1)*100) = max(100, 100) = 100
    assert_eq!(gravity_interval_ms(10), 100);
}

#[test]
fn gravity_interval_level_15_clamps_to_100ms() {
    // max(100, 1000 - (15-1)*100) = max(100, -400) = 100
    assert_eq!(gravity_interval_ms(15), 100);
}

#[test]
fn gravity_interval_level_20_clamps_to_100ms() {
    // Deeply past the clamp point — must stay at 100
    assert_eq!(gravity_interval_ms(20), 100);
}

#[test]
fn gravity_interval_decreases_monotonically_until_clamp() {
    // Each level from 1..=10 must be <= the previous level's interval
    let mut prev = gravity_interval_ms(1);
    for level in 2..=10 {
        let curr = gravity_interval_ms(level);
        assert!(
            curr <= prev,
            "interval at level {} ({}) must be <= level {} ({})",
            level, curr, level - 1, prev
        );
        prev = curr;
    }
}

// ── gravity_tick: interval not yet elapsed ────────────────────────────────────

#[test]
fn gravity_tick_zero_accumulated_does_not_drop() {
    let grid = Grid::new();
    let mut piece = t_piece(5, 5);
    let (dropped, new_acc) = gravity_tick(&grid, &mut piece, 0, 1);
    assert!(!dropped, "zero accumulated ms must not trigger a drop");
    assert_eq!(new_acc, 0, "accumulator must remain 0 when interval not reached");
    assert_eq!(piece.row, 5, "piece row must not change when interval not reached");
}

#[test]
fn gravity_tick_one_below_interval_does_not_drop() {
    let grid = Grid::new();
    let mut piece = t_piece(5, 5);
    let interval = gravity_interval_ms(1); // 1000
    let accumulated = interval - 1;        // 999
    let (dropped, new_acc) = gravity_tick(&grid, &mut piece, accumulated, 1);
    assert!(!dropped, "999ms must not trigger drop when interval is 1000ms");
    assert_eq!(new_acc, accumulated, "accumulator unchanged when interval not reached");
    assert_eq!(piece.row, 5, "piece row must not change when interval not reached");
}

#[test]
fn gravity_tick_below_level5_interval_does_not_drop() {
    let grid = Grid::new();
    let mut piece = t_piece(5, 5);
    // Level 5 interval is 600ms; 599ms should not trigger
    let (dropped, new_acc) = gravity_tick(&grid, &mut piece, 599, 5);
    assert!(!dropped, "599ms should not trigger drop at level 5 (interval=600ms)");
    assert_eq!(new_acc, 599, "accumulator must be returned unchanged");
    assert_eq!(piece.row, 5, "row must not change");
}

// ── gravity_tick: interval elapsed and move_down succeeds ─────────────────────

#[test]
fn gravity_tick_exactly_at_interval_drops_and_resets() {
    let grid = Grid::new();
    let mut piece = t_piece(5, 5);
    let interval = gravity_interval_ms(1); // 1000
    let (dropped, new_acc) = gravity_tick(&grid, &mut piece, interval, 1);
    assert!(dropped, "accumulated == interval must trigger a drop");
    assert_eq!(new_acc, 0, "accumulator must reset to 0 after a drop");
}

#[test]
fn gravity_tick_above_interval_drops_and_resets() {
    let grid = Grid::new();
    let mut piece = t_piece(5, 5);
    let interval = gravity_interval_ms(1); // 1000
    let accumulated = interval + 500;      // 1500 — well past threshold
    let (dropped, new_acc) = gravity_tick(&grid, &mut piece, accumulated, 1);
    assert!(dropped, "accumulated > interval must trigger a drop");
    assert_eq!(new_acc, 0, "accumulator must reset to 0 after drop");
}

#[test]
fn gravity_tick_drops_piece_by_one_row() {
    let grid = Grid::new();
    let mut piece = t_piece(5, 5);
    let interval = gravity_interval_ms(1);
    gravity_tick(&grid, &mut piece, interval, 1);
    assert_eq!(piece.row, 6, "piece row must increase by 1 after a successful gravity drop");
}

#[test]
fn gravity_tick_level10_drops_at_exactly_100ms() {
    let grid = Grid::new();
    let mut piece = t_piece(5, 5);
    // Level 10 clamped interval is 100ms
    let (dropped, new_acc) = gravity_tick(&grid, &mut piece, 100, 10);
    assert!(dropped, "100ms must trigger drop at level 10 (interval=100ms)");
    assert_eq!(new_acc, 0);
}

#[test]
fn gravity_tick_level10_no_drop_below_100ms() {
    let grid = Grid::new();
    let mut piece = t_piece(5, 5);
    let (dropped, new_acc) = gravity_tick(&grid, &mut piece, 99, 10);
    assert!(!dropped, "99ms must not trigger drop at level 10 (interval=100ms)");
    assert_eq!(new_acc, 99, "accumulator must be returned unchanged");
}
