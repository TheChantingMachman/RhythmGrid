// @spec-tags: game.next_piece
// @invariants: PieceBag is a 7-bag randomizer; next() returns values in 0..=6; all 7 types appear before any repeat; peek() returns the same value next() will return without advancing the bag
// @build: 33

use rhythm_grid::game::PieceBag;
use std::collections::HashSet;

// ── Range of next() ───────────────────────────────────────────────────────────

#[test]
fn next_always_returns_value_in_0_to_6() {
    let mut bag = PieceBag::new();
    for _ in 0..49 {
        let piece = bag.next();
        assert!(
            piece <= 6,
            "next() returned {} which is outside 0..=6",
            piece
        );
    }
}

// ── 7-bag permutation property ────────────────────────────────────────────────

#[test]
fn first_seven_calls_are_a_permutation_of_0_to_6() {
    let mut bag = PieceBag::new();
    let mut seen = HashSet::new();
    for _ in 0..7 {
        let piece = bag.next();
        assert!(
            seen.insert(piece),
            "piece {} appeared more than once in the first 7 draws",
            piece
        );
    }
    assert_eq!(seen, (0..=6usize).collect::<HashSet<_>>());
}

#[test]
fn second_seven_calls_are_also_a_permutation_of_0_to_6() {
    let mut bag = PieceBag::new();
    // exhaust the first bag
    for _ in 0..7 {
        bag.next();
    }
    // second bag
    let mut seen = HashSet::new();
    for _ in 0..7 {
        let piece = bag.next();
        assert!(
            seen.insert(piece),
            "piece {} appeared more than once in the second 7 draws",
            piece
        );
    }
    assert_eq!(seen, (0..=6usize).collect::<HashSet<_>>());
}

#[test]
fn multiple_bags_all_are_permutations() {
    let mut bag = PieceBag::new();
    for bag_index in 0..5 {
        let mut seen = HashSet::new();
        for _ in 0..7 {
            let piece = bag.next();
            assert!(
                seen.insert(piece),
                "bag {}: piece {} appeared more than once",
                bag_index,
                piece
            );
        }
        assert_eq!(seen, (0..=6usize).collect::<HashSet<_>>());
    }
}

// ── peek() semantics ──────────────────────────────────────────────────────────

#[test]
fn peek_returns_same_value_as_next() {
    let mut bag = PieceBag::new();
    let peeked = bag.peek();
    let drawn = bag.next();
    assert_eq!(
        peeked, drawn,
        "peek() must return the same value as the subsequent next()"
    );
}

#[test]
fn peek_does_not_advance_bag() {
    let mut bag = PieceBag::new();
    let first_peek = bag.peek();
    let second_peek = bag.peek();
    assert_eq!(
        first_peek, second_peek,
        "repeated peek() calls must return the same value"
    );
    let drawn = bag.next();
    assert_eq!(
        first_peek, drawn,
        "next() after two peeks must return the same value as peek()"
    );
}

#[test]
fn peek_after_next_reflects_new_front() {
    let mut bag = PieceBag::new();
    let _first = bag.next();
    let peeked = bag.peek();
    let drawn = bag.next();
    assert_eq!(
        peeked, drawn,
        "peek() after next() must match the following next() call"
    );
}

#[test]
fn peek_value_is_in_0_to_6() {
    let bag = PieceBag::new();
    let p = bag.peek();
    assert!(p <= 6, "peek() returned {} which is outside 0..=6", p);
}

// ── Eager initialization ──────────────────────────────────────────────────────

#[test]
fn peek_valid_on_fresh_bag_without_any_next_call() {
    // PieceBag::new() must eagerly initialize so peek() is valid immediately.
    let bag = PieceBag::new();
    let p = bag.peek();
    assert!(p <= 6, "peek() on fresh bag returned {} outside 0..=6", p);
}
