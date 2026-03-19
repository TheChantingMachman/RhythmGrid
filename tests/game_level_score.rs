// @spec-tags: game.level,game.score
// @invariants: level_for_lines and score_for_lines are pure free functions in game; six constants exported; level=1+lines/10 (no cap); score maps 1->100,2->300,3->500,4->800 times level, 0 otherwise
// @build: 42

use rhythm_grid::game::{
    level_for_lines, score_for_lines,
    LINES_PER_LEVEL, STARTING_LEVEL,
    SCORE_SINGLE, SCORE_DOUBLE, SCORE_TRIPLE, SCORE_TETRIS,
};

// ── Constants ─────────────────────────────────────────────────────────────────

#[test]
fn constant_lines_per_level() {
    assert_eq!(LINES_PER_LEVEL, 10u32);
}

#[test]
fn constant_starting_level() {
    assert_eq!(STARTING_LEVEL, 1u32);
}

#[test]
fn constant_score_single() {
    assert_eq!(SCORE_SINGLE, 100u32);
}

#[test]
fn constant_score_double() {
    assert_eq!(SCORE_DOUBLE, 300u32);
}

#[test]
fn constant_score_triple() {
    assert_eq!(SCORE_TRIPLE, 500u32);
}

#[test]
fn constant_score_tetris() {
    assert_eq!(SCORE_TETRIS, 800u32);
}

// ── level_for_lines ───────────────────────────────────────────────────────────

#[test]
fn level_for_zero_lines_is_one() {
    assert_eq!(level_for_lines(0), 1);
}

#[test]
fn level_for_nine_lines_is_one() {
    assert_eq!(level_for_lines(9), 1);
}

#[test]
fn level_for_ten_lines_is_two() {
    assert_eq!(level_for_lines(10), 2);
}

#[test]
fn level_for_nineteen_lines_is_two() {
    assert_eq!(level_for_lines(19), 2);
}

#[test]
fn level_for_twenty_lines_is_three() {
    assert_eq!(level_for_lines(20), 3);
}

#[test]
fn level_for_ninety_nine_lines_is_ten() {
    assert_eq!(level_for_lines(99), 10);
}

#[test]
fn level_for_one_hundred_lines_is_eleven() {
    assert_eq!(level_for_lines(100), 11);
}

#[test]
fn level_has_no_upper_cap() {
    // 1000 lines cleared → level 101
    assert_eq!(level_for_lines(1000), 101);
}

// ── score_for_lines ───────────────────────────────────────────────────────────

#[test]
fn score_single_at_level_one() {
    assert_eq!(score_for_lines(1, 1), 100);
}

#[test]
fn score_double_at_level_one() {
    assert_eq!(score_for_lines(2, 1), 300);
}

#[test]
fn score_triple_at_level_one() {
    assert_eq!(score_for_lines(3, 1), 500);
}

#[test]
fn score_tetris_at_level_one() {
    assert_eq!(score_for_lines(4, 1), 800);
}

#[test]
fn score_single_at_level_two() {
    assert_eq!(score_for_lines(1, 2), 200);
}

#[test]
fn score_double_at_level_two() {
    assert_eq!(score_for_lines(2, 2), 600);
}

#[test]
fn score_triple_at_level_two() {
    assert_eq!(score_for_lines(3, 2), 1000);
}

#[test]
fn score_tetris_at_level_two() {
    assert_eq!(score_for_lines(4, 2), 1600);
}

#[test]
fn score_tetris_at_level_five() {
    assert_eq!(score_for_lines(4, 5), 4000);
}

#[test]
fn score_zero_lines_returns_zero() {
    assert_eq!(score_for_lines(0, 1), 0);
}

#[test]
fn score_five_lines_returns_zero() {
    assert_eq!(score_for_lines(5, 1), 0);
}

#[test]
fn score_zero_lines_at_high_level_returns_zero() {
    assert_eq!(score_for_lines(0, 10), 0);
}

// ── Caller convention: level computed before clear ────────────────────────────

#[test]
fn score_uses_level_active_at_moment_of_clear() {
    // lines_before_clear=9 → level=1; clearing 1 line → score=100
    let lines_before = 9u32;
    let level = level_for_lines(lines_before);
    assert_eq!(level, 1);
    assert_eq!(score_for_lines(1, level), 100);
}

#[test]
fn score_at_level_boundary_uses_pre_clear_level() {
    // lines_before_clear=10 → level=2; clearing 1 line → score=200 (not 100)
    let lines_before = 10u32;
    let level = level_for_lines(lines_before);
    assert_eq!(level, 2);
    assert_eq!(score_for_lines(1, level), 200);
}
