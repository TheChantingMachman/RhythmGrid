// @spec-tags: render,state,ui
// @invariants: next_piece_state returns NextPieceRender with type_index equal to bag.peek() as u32 and cells equal to PIECE_CELLS[type_index as usize][0]; does not advance the bag; NextPieceRender derives Debug and PartialEq and has fields type_index: u32 and cells: [(i32, i32); 4]
// @build: 91

use rhythm_grid::game::GameSession;
use rhythm_grid::pieces::PIECE_CELLS;
use rhythm_grid::render::{next_piece_state, NextPieceRender};

// --- NextPieceRender derives ---

#[test]
fn next_piece_render_derives_debug() {
    let r = NextPieceRender { type_index: 0, cells: PIECE_CELLS[0][0] };
    let s = format!("{:?}", r);
    assert!(!s.is_empty());
}

#[test]
fn next_piece_render_derives_partial_eq_equal() {
    let a = NextPieceRender { type_index: 1, cells: PIECE_CELLS[1][0] };
    let b = NextPieceRender { type_index: 1, cells: PIECE_CELLS[1][0] };
    assert_eq!(a, b);
}

#[test]
fn next_piece_render_derives_partial_eq_not_equal() {
    let a = NextPieceRender { type_index: 1, cells: PIECE_CELLS[1][0] };
    let b = NextPieceRender { type_index: 2, cells: PIECE_CELLS[2][0] };
    assert_ne!(a, b);
}

#[test]
fn next_piece_render_type_index_field_is_u32() {
    let r = NextPieceRender { type_index: 3, cells: PIECE_CELLS[3][0] };
    let _: u32 = r.type_index;
}

#[test]
fn next_piece_render_cells_is_four_element_array() {
    let r = NextPieceRender { type_index: 0, cells: PIECE_CELLS[0][0] };
    assert_eq!(r.cells.len(), 4);
}

// --- next_piece_state: type_index matches bag.peek() ---

#[test]
fn next_piece_state_type_index_matches_bag_peek() {
    let session = GameSession::new();
    let expected = session.bag.peek() as u32;
    let result = next_piece_state(&session);
    assert_eq!(
        result.type_index, expected,
        "type_index must equal bag.peek() as u32"
    );
}

#[test]
fn next_piece_state_type_index_in_valid_range() {
    let session = GameSession::new();
    let result = next_piece_state(&session);
    assert!(result.type_index <= 6, "type_index must be 0..=6, got {}", result.type_index);
}

// --- next_piece_state: cells match rotation-0 from PIECE_CELLS ---

#[test]
fn next_piece_state_cells_match_piece_cells_rotation_0() {
    let session = GameSession::new();
    let peek = session.bag.peek();
    let result = next_piece_state(&session);
    assert_eq!(
        result.cells,
        PIECE_CELLS[peek][0],
        "cells must equal PIECE_CELLS[bag.peek()][0] (rotation-0 offsets)"
    );
}

#[test]
fn next_piece_state_cells_length_is_four() {
    let session = GameSession::new();
    let result = next_piece_state(&session);
    assert_eq!(result.cells.len(), 4);
}

#[test]
fn next_piece_state_cells_are_i32_pairs() {
    let session = GameSession::new();
    let result = next_piece_state(&session);
    // Each cell is (i32, i32); verify they're accessible
    for &(row_offset, col_offset) in &result.cells {
        let _: i32 = row_offset;
        let _: i32 = col_offset;
    }
}

// --- next_piece_state: does not advance the bag ---

#[test]
fn next_piece_state_does_not_advance_bag() {
    let session = GameSession::new();
    let peek_before = session.bag.peek();
    let _ = next_piece_state(&session);
    let peek_after = session.bag.peek();
    assert_eq!(peek_before, peek_after, "next_piece_state must not advance the bag");
}

#[test]
fn next_piece_state_idempotent_for_same_session() {
    let session = GameSession::new();
    let a = next_piece_state(&session);
    let b = next_piece_state(&session);
    assert_eq!(a, b, "next_piece_state must return same result for same session state");
}

// --- next_piece_state: type_index and cells agree with each other ---

#[test]
fn next_piece_state_cells_agree_with_type_index() {
    let session = GameSession::new();
    let result = next_piece_state(&session);
    let expected_cells = PIECE_CELLS[result.type_index as usize][0];
    assert_eq!(
        result.cells, expected_cells,
        "result.cells must match PIECE_CELLS[result.type_index][0]"
    );
}

// --- next_piece_state: covers multiple bag positions ---

#[test]
fn next_piece_state_consistent_after_advancing_bag() {
    // Advance the bag and verify next_piece_state always matches the new peek
    let mut session = GameSession::new();
    for _ in 0..7 {
        let peek = session.bag.peek();
        let result = next_piece_state(&session);
        assert_eq!(
            result.type_index,
            peek as u32,
            "type_index must match bag.peek() at every bag position"
        );
        assert_eq!(
            result.cells,
            PIECE_CELLS[peek][0],
            "cells must match PIECE_CELLS[peek][0] at every bag position"
        );
        session.bag.next(); // advance to next piece
    }
}
