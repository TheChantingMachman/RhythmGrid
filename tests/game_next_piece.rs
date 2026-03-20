// @spec-tags: game.next_piece
// @invariants: PieceBag is a 7-bag randomizer; seed derived from system time at creation for unique sequences; new_seeded(seed) for deterministic testing; next() returns values in 0..=6; all 7 types appear before any repeat; peek() returns the same value next() will return without advancing the bag
// @build: 58

use rhythm_grid::game::PieceBag;
use std::collections::HashSet;
use std::thread;
use std::time::Duration;

// ── Range of next() ───────────────────────────────────────────────────────────

#[test]
fn next_always_returns_value_in_0_to_6() {
    let mut bag = PieceBag::new_seeded(12345);
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
    let mut bag = PieceBag::new_seeded(12345);
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
    let mut bag = PieceBag::new_seeded(12345);
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
    let mut bag = PieceBag::new_seeded(12345);
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
    let mut bag = PieceBag::new_seeded(12345);
    let peeked = bag.peek();
    let drawn = bag.next();
    assert_eq!(
        peeked, drawn,
        "peek() must return the same value as the subsequent next()"
    );
}

#[test]
fn peek_does_not_advance_bag() {
    let mut bag = PieceBag::new_seeded(12345);
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
    let mut bag = PieceBag::new_seeded(12345);
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
    let bag = PieceBag::new_seeded(12345);
    let p = bag.peek();
    assert!(p <= 6, "peek() returned {} which is outside 0..=6", p);
}

// ── Eager initialization ──────────────────────────────────────────────────────

#[test]
fn peek_valid_on_fresh_bag_without_any_next_call() {
    // PieceBag::new_seeded() must eagerly initialize so peek() is valid immediately.
    let bag = PieceBag::new_seeded(12345);
    let p = bag.peek();
    assert!(p <= 6, "peek() on fresh bag returned {} outside 0..=6", p);
}

// ── Deterministic seeding ─────────────────────────────────────────────────────

#[test]
fn new_seeded_with_same_seed_produces_same_sequence() {
    let mut bag1 = PieceBag::new_seeded(42);
    let mut bag2 = PieceBag::new_seeded(42);
    let seq1: Vec<usize> = (0..14).map(|_| bag1.next()).collect();
    let seq2: Vec<usize> = (0..14).map(|_| bag2.next()).collect();
    assert_eq!(
        seq1, seq2,
        "two PieceBag::new_seeded(42) instances must produce identical sequences"
    );
}

#[test]
fn new_seeded_with_different_seeds_produces_different_sequence() {
    let mut bag1 = PieceBag::new_seeded(1);
    let mut bag2 = PieceBag::new_seeded(2);
    let seq1: Vec<usize> = (0..7).map(|_| bag1.next()).collect();
    let seq2: Vec<usize> = (0..7).map(|_| bag2.next()).collect();
    assert_ne!(
        seq1, seq2,
        "PieceBag::new_seeded with different seeds must produce different sequences"
    );
}

#[test]
fn two_bags_created_at_different_times_produce_different_sequences() {
    let mut bag1 = PieceBag::new();
    thread::sleep(Duration::from_millis(2));
    let mut bag2 = PieceBag::new();
    let seq1: Vec<usize> = (0..7).map(|_| bag1.next()).collect();
    let seq2: Vec<usize> = (0..7).map(|_| bag2.next()).collect();
    assert_ne!(
        seq1, seq2,
        "two PieceBag::new() instances created at different times must produce different sequences"
    );
}

#[test]
fn new_seeded_permutation_property_holds() {
    let mut bag = PieceBag::new_seeded(99999);
    let mut seen = HashSet::new();
    for _ in 0..7 {
        let piece = bag.next();
        assert!(
            seen.insert(piece),
            "piece {} appeared more than once in seeded bag draw",
            piece
        );
    }
    assert_eq!(
        seen,
        (0..=6usize).collect::<HashSet<_>>(),
        "new_seeded bag must yield a permutation of 0..=6"
    );
}

#[test]
fn new_seeded_peek_at_bag_boundary() {
    let mut bag = PieceBag::new_seeded(42);
    // exhaust first bag
    for _ in 0..7 {
        bag.next();
    }
    // at bag boundary: peek and next must agree
    let peeked = bag.peek();
    let drawn = bag.next();
    assert_eq!(
        peeked, drawn,
        "peek() at bag boundary must return the same value as the following next()"
    );
}

#[test]
fn rng_state_carries_across_bag_refills() {
    let mut bag = PieceBag::new_seeded(42);
    let mut first_bag = HashSet::new();
    for _ in 0..7 {
        let piece = bag.next();
        assert!(
            first_bag.insert(piece),
            "piece {} appeared more than once in first bag",
            piece
        );
    }
    assert_eq!(first_bag, (0..=6usize).collect::<HashSet<_>>());

    let mut second_bag = HashSet::new();
    for _ in 0..7 {
        let piece = bag.next();
        assert!(
            second_bag.insert(piece),
            "piece {} appeared more than once in second bag",
            piece
        );
    }
    assert_eq!(second_bag, (0..=6usize).collect::<HashSet<_>>());
}
