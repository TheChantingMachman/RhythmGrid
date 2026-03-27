// @spec-tags: core,game,progression
// @invariants: gravity_interval_ms(level) uses Tetris Guideline exponential curve: (0.8 - (level_f - 1.0) * 0.007).powf(level_f - 1.0) * 1000, min 1ms; gravity_tick returns (true,0) when move_down succeeds after interval elapses, or (false,accumulated_ms) unchanged when interval has not yet elapsed
// @build: 99

use rhythm_grid::game::{gravity_interval_ms, gravity_tick, ActivePiece};
use rhythm_grid::grid::Grid;
use rhythm_grid::pieces::TetrominoType;

fn t_piece(row: i32, col: i32) -> ActivePiece {
    ActivePiece { piece_type: TetrominoType::T, rotation: 0, row, col }
}

// ── gravity_interval_ms ───────────────────────────────────────────────────────

#[test]
fn gravity_interval_level_1_is_1000ms() {
    // (0.8)^0 = 1.0 → 1000ms
    assert_eq!(gravity_interval_ms(1), 1000);
}

#[test]
fn gravity_interval_level_2() {
    // (0.8 - 0.007)^1 = 0.793 → 793ms
    assert_eq!(gravity_interval_ms(2), 793);
}

#[test]
fn gravity_interval_level_5() {
    let expected = {
        let lf = 5.0_f64;
        ((0.8 - (lf - 1.0) * 0.007).powf(lf - 1.0) * 1000.0).max(1.0) as u64
    };
    assert_eq!(gravity_interval_ms(5), expected);
}

#[test]
fn gravity_interval_level_9() {
    let expected = {
        let lf = 9.0_f64;
        ((0.8 - (lf - 1.0) * 0.007).powf(lf - 1.0) * 1000.0).max(1.0) as u64
    };
    assert_eq!(gravity_interval_ms(9), expected);
}

#[test]
fn gravity_interval_level_10() {
    let expected = {
        let lf = 10.0_f64;
        ((0.8 - (lf - 1.0) * 0.007).powf(lf - 1.0) * 1000.0).max(1.0) as u64
    };
    assert_eq!(gravity_interval_ms(10), expected);
}

#[test]
fn gravity_interval_level_15() {
    let expected = {
        let lf = 15.0_f64;
        ((0.8 - (lf - 1.0) * 0.007).powf(lf - 1.0) * 1000.0).max(1.0) as u64
    };
    assert_eq!(gravity_interval_ms(15), expected);
}

#[test]
fn gravity_interval_level_20() {
    let expected = {
        let lf = 20.0_f64;
        ((0.8 - (lf - 1.0) * 0.007).powf(lf - 1.0) * 1000.0).max(1.0) as u64
    };
    assert_eq!(gravity_interval_ms(20), expected);
}

#[test]
fn gravity_interval_decreases_monotonically() {
    // No clamp point — exponential continues decreasing through all levels
    let mut prev = gravity_interval_ms(1);
    for level in 2..=20 {
        let curr = gravity_interval_ms(level);
        assert!(
            curr <= prev,
            "interval at level {} ({}) must be <= level {} ({})",
            level, curr, level - 1, prev
        );
        prev = curr;
    }
}

#[test]
fn gravity_interval_minimum_is_1ms() {
    // At high levels the exponential formula yields sub-millisecond values; floor is 1ms
    assert_eq!(gravity_interval_ms(30), 1);
}

#[test]
fn gravity_interval_level_0_treated_as_level_1() {
    // level.max(1) clamps 0 to 1, so result equals level 1
    assert_eq!(gravity_interval_ms(0), 1000);
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
    let below = gravity_interval_ms(5) - 1;
    let (dropped, new_acc) = gravity_tick(&grid, &mut piece, below, 5);
    assert!(!dropped, "{}ms should not trigger drop at level 5 (interval={}ms)", below, gravity_interval_ms(5));
    assert_eq!(new_acc, below, "accumulator must be returned unchanged");
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
fn gravity_tick_level10_drops_at_interval() {
    let grid = Grid::new();
    let mut piece = t_piece(5, 5);
    let interval = gravity_interval_ms(10);
    let (dropped, new_acc) = gravity_tick(&grid, &mut piece, interval, 10);
    assert!(dropped, "{}ms must trigger drop at level 10 (interval={}ms)", interval, interval);
    assert_eq!(new_acc, 0);
}

#[test]
fn gravity_tick_level10_no_drop_below_interval() {
    let grid = Grid::new();
    let mut piece = t_piece(5, 5);
    let interval = gravity_interval_ms(10);
    let below = interval - 1;
    let (dropped, new_acc) = gravity_tick(&grid, &mut piece, below, 10);
    assert!(!dropped, "{}ms must not trigger drop at level 10 (interval={}ms)", below, interval);
    assert_eq!(new_acc, below, "accumulator must be returned unchanged");
}
