// @spec-tags: render,state,ui
// @invariants: board_state returns occupied/active/ghost lists with row>=0 cells only; held_piece_state returns None when no piece held and Some with correct type_index and rotation-0 cells when held; game_status maps all GameSession fields correctly with level computed via level_for_lines
// @build: 86

use rhythm_grid::game::{GameSession, GameState as GameStateFromGame};
use rhythm_grid::grid::CellState;
use rhythm_grid::pieces::{TetrominoType, PIECE_CELLS};
use rhythm_grid::render::{
    board_state, game_status, held_piece_state, BoardRenderState, GameState, GameStatusRender,
    HeldPieceRender, RenderCell,
};

// --- RenderCell derives ---

#[test]
fn render_cell_derives_debug() {
    let cell = RenderCell { row: 1, col: 2, type_index: 3 };
    let s = format!("{:?}", cell);
    assert!(!s.is_empty());
}

#[test]
fn render_cell_derives_clone() {
    let cell = RenderCell { row: 5, col: 3, type_index: 0 };
    let cloned = cell.clone();
    assert_eq!(cloned.row, 5);
    assert_eq!(cloned.col, 3);
    assert_eq!(cloned.type_index, 0);
}

#[test]
fn render_cell_derives_partial_eq() {
    let a = RenderCell { row: 1, col: 2, type_index: 4 };
    let b = RenderCell { row: 1, col: 2, type_index: 4 };
    let c = RenderCell { row: 1, col: 2, type_index: 5 };
    assert_eq!(a, b);
    assert_ne!(a, c);
}

// --- BoardRenderState derives ---

#[test]
fn board_render_state_derives_debug() {
    let brs = BoardRenderState { occupied: vec![], active: vec![], ghost: vec![] };
    let s = format!("{:?}", brs);
    assert!(!s.is_empty());
}

#[test]
fn board_render_state_derives_partial_eq() {
    let a = BoardRenderState { occupied: vec![], active: vec![], ghost: vec![] };
    let b = BoardRenderState { occupied: vec![], active: vec![], ghost: vec![] };
    assert_eq!(a, b);
}

// --- board_state: empty grid ---

#[test]
fn board_state_empty_grid_occupied_is_empty() {
    let mut session = GameSession::new();
    // Force known piece with no negative row offsets: O piece at row=5
    session.active_piece.piece_type = TetrominoType::O;
    session.active_piece.rotation = 0;
    session.active_piece.row = 5;
    session.active_piece.col = 4;
    let state = board_state(&session);
    assert!(state.occupied.is_empty(), "fresh grid should yield no occupied cells");
}

#[test]
fn board_state_o_piece_at_mid_board_yields_four_active_cells() {
    // O piece rotation-0 offsets: (0,0),(0,1),(1,0),(1,1) — all dr>=0, so row=5 gives rows 5,5,6,6
    let mut session = GameSession::new();
    session.active_piece.piece_type = TetrominoType::O;
    session.active_piece.rotation = 0;
    session.active_piece.row = 5;
    session.active_piece.col = 4;
    let state = board_state(&session);
    assert_eq!(state.active.len(), 4, "O piece mid-board should produce 4 active cells");
}

#[test]
fn board_state_active_cells_have_correct_type_index_for_o_piece() {
    let mut session = GameSession::new();
    session.active_piece.piece_type = TetrominoType::O; // type_index = 1
    session.active_piece.rotation = 0;
    session.active_piece.row = 5;
    session.active_piece.col = 4;
    let state = board_state(&session);
    for cell in &state.active {
        assert_eq!(cell.type_index, 1, "O piece cells should have type_index=1");
    }
}

#[test]
fn board_state_active_cells_have_correct_positions_for_o_piece() {
    // O at row=5, col=4: cells at (5,4),(5,5),(6,4),(6,5)
    let mut session = GameSession::new();
    session.active_piece.piece_type = TetrominoType::O;
    session.active_piece.rotation = 0;
    session.active_piece.row = 5;
    session.active_piece.col = 4;
    let state = board_state(&session);
    let mut positions: Vec<(i32, i32)> = state.active.iter().map(|c| (c.row, c.col)).collect();
    positions.sort();
    assert_eq!(positions, vec![(5, 4), (5, 5), (6, 4), (6, 5)]);
}

#[test]
fn board_state_o_piece_mid_board_yields_four_ghost_cells() {
    let mut session = GameSession::new();
    session.active_piece.piece_type = TetrominoType::O;
    session.active_piece.rotation = 0;
    session.active_piece.row = 5;
    session.active_piece.col = 4;
    let state = board_state(&session);
    assert_eq!(state.ghost.len(), 4, "O piece on empty board should produce 4 ghost cells");
}

#[test]
fn board_state_ghost_type_index_matches_active() {
    let mut session = GameSession::new();
    session.active_piece.piece_type = TetrominoType::O;
    session.active_piece.rotation = 0;
    session.active_piece.row = 5;
    session.active_piece.col = 4;
    let state = board_state(&session);
    for cell in &state.ghost {
        assert_eq!(cell.type_index, 1, "ghost cells should have same type_index as active piece");
    }
}

#[test]
fn board_state_ghost_rows_are_below_or_equal_to_active_rows_on_empty_board() {
    // O piece at row=5 on empty board: ghost drops to row=18
    let mut session = GameSession::new();
    session.active_piece.piece_type = TetrominoType::O;
    session.active_piece.rotation = 0;
    session.active_piece.row = 5;
    session.active_piece.col = 4;
    let state = board_state(&session);
    let max_active_row = state.active.iter().map(|c| c.row).max().unwrap();
    let min_ghost_row = state.ghost.iter().map(|c| c.row).min().unwrap();
    assert!(
        min_ghost_row >= max_active_row,
        "ghost piece should be at or below active piece: min_ghost_row={} max_active_row={}",
        min_ghost_row,
        max_active_row
    );
}

#[test]
fn board_state_ghost_drops_to_bottom_on_empty_grid() {
    // O piece at row=5, col=4 on empty grid: last valid row=18 (cells at 18,19 in bounds)
    let mut session = GameSession::new();
    session.active_piece.piece_type = TetrominoType::O;
    session.active_piece.rotation = 0;
    session.active_piece.row = 5;
    session.active_piece.col = 4;
    let state = board_state(&session);
    // Ghost positions: (18,4),(18,5),(19,4),(19,5)
    let mut ghost_positions: Vec<(i32, i32)> = state.ghost.iter().map(|c| (c.row, c.col)).collect();
    ghost_positions.sort();
    assert_eq!(ghost_positions, vec![(18, 4), (18, 5), (19, 4), (19, 5)]);
}

#[test]
fn board_state_all_active_cells_have_non_negative_rows() {
    let mut session = GameSession::new();
    session.active_piece.piece_type = TetrominoType::O;
    session.active_piece.rotation = 0;
    session.active_piece.row = 5;
    session.active_piece.col = 4;
    let state = board_state(&session);
    for cell in &state.active {
        assert!(cell.row >= 0, "active cell row must be >= 0, got {}", cell.row);
    }
}

#[test]
fn board_state_all_ghost_cells_have_non_negative_rows() {
    let mut session = GameSession::new();
    session.active_piece.piece_type = TetrominoType::O;
    session.active_piece.rotation = 0;
    session.active_piece.row = 5;
    session.active_piece.col = 4;
    let state = board_state(&session);
    for cell in &state.ghost {
        assert!(cell.row >= 0, "ghost cell row must be >= 0, got {}", cell.row);
    }
}

#[test]
fn board_state_vanish_zone_cells_excluded_from_active() {
    // T piece rotation-0 offsets: (-1,0),(0,-1),(0,0),(0,1)
    // At row=0: cell at (-1,4) is in vanish zone — only 3 active cells visible
    let mut session = GameSession::new();
    session.active_piece.piece_type = TetrominoType::T;
    session.active_piece.rotation = 0;
    session.active_piece.row = 0;
    session.active_piece.col = 4;
    let state = board_state(&session);
    assert_eq!(state.active.len(), 3, "T piece at row=0 has one cell in vanish zone");
    for cell in &state.active {
        assert!(cell.row >= 0, "no active cell may have row < 0");
    }
}

#[test]
fn board_state_i_piece_at_row_0_all_four_active_visible() {
    // I piece rotation-0 offsets: (0,-1),(0,0),(0,1),(0,2) — all dr=0, all visible at row=0
    let mut session = GameSession::new();
    session.active_piece.piece_type = TetrominoType::I;
    session.active_piece.rotation = 0;
    session.active_piece.row = 0;
    session.active_piece.col = 4;
    let state = board_state(&session);
    assert_eq!(state.active.len(), 4, "I piece at row=0 should have all 4 cells visible");
}

// --- board_state: occupied cells from grid ---

#[test]
fn board_state_grid_occupied_cells_appear_in_occupied_list() {
    let mut session = GameSession::new();
    // Place locked pieces at bottom rows (far from active piece at top)
    session.grid.cells[18][2] = CellState::Occupied(5); // J piece (type_index=5)
    session.grid.cells[19][7] = CellState::Occupied(6); // L piece (type_index=6)
    let state = board_state(&session);
    assert!(
        state.occupied.iter().any(|c| c.row == 18 && c.col == 2 && c.type_index == 5),
        "J piece cell at (18,2) should be in occupied list"
    );
    assert!(
        state.occupied.iter().any(|c| c.row == 19 && c.col == 7 && c.type_index == 6),
        "L piece cell at (19,7) should be in occupied list"
    );
}

#[test]
fn board_state_occupied_count_matches_grid_filled_cells() {
    let mut session = GameSession::new();
    session.grid.cells[17][0] = CellState::Occupied(0);
    session.grid.cells[17][1] = CellState::Occupied(0);
    session.grid.cells[17][2] = CellState::Occupied(0);
    let state = board_state(&session);
    assert_eq!(state.occupied.len(), 3, "should have exactly 3 occupied cells");
}

#[test]
fn board_state_occupied_cells_have_correct_type_index() {
    let mut session = GameSession::new();
    session.grid.cells[19][5] = CellState::Occupied(2); // T piece
    let state = board_state(&session);
    let cell = state.occupied.iter().find(|c| c.row == 19 && c.col == 5).unwrap();
    assert_eq!(cell.type_index, 2, "occupied cell should have type_index matching grid value");
}

#[test]
fn board_state_all_occupied_cells_have_non_negative_rows() {
    let mut session = GameSession::new();
    session.grid.cells[0][0] = CellState::Occupied(3);
    session.grid.cells[10][5] = CellState::Occupied(4);
    let state = board_state(&session);
    for cell in &state.occupied {
        assert!(cell.row >= 0, "occupied cell should have row >= 0, got {}", cell.row);
    }
}

// --- held_piece_state: no piece held ---

#[test]
fn held_piece_state_returns_none_when_nothing_held() {
    let session = GameSession::new();
    // New session has held_piece = None
    assert!(session.held_piece.is_none());
    let result = held_piece_state(&session);
    assert!(result.is_none(), "held_piece_state must return None when nothing is held");
}

// --- HeldPieceRender derives ---

#[test]
fn held_piece_render_derives_debug() {
    let h = HeldPieceRender { type_index: 0, cells: PIECE_CELLS[0][0] };
    let s = format!("{:?}", h);
    assert!(!s.is_empty());
}

#[test]
fn held_piece_render_derives_partial_eq() {
    let a = HeldPieceRender { type_index: 1, cells: PIECE_CELLS[1][0] };
    let b = HeldPieceRender { type_index: 1, cells: PIECE_CELLS[1][0] };
    let c = HeldPieceRender { type_index: 2, cells: PIECE_CELLS[2][0] };
    assert_eq!(a, b);
    assert_ne!(a, c);
}

// --- held_piece_state: after holding ---

#[test]
fn held_piece_state_returns_some_after_hold() {
    let mut session = GameSession::new();
    let held_ok = session.hold_piece();
    assert!(held_ok, "hold_piece must succeed on a fresh session");
    let result = held_piece_state(&session);
    assert!(result.is_some(), "held_piece_state must return Some after successful hold");
}

#[test]
fn held_piece_type_index_matches_held_piece() {
    let mut session = GameSession::new();
    let expected_type = session.active_piece.piece_type;
    let expected_type_index = expected_type as u32;
    session.hold_piece();
    let h = held_piece_state(&session).unwrap();
    assert_eq!(
        h.type_index, expected_type_index,
        "HeldPieceRender.type_index must match held piece type"
    );
}

#[test]
fn held_piece_cells_are_rotation_0_offsets() {
    let mut session = GameSession::new();
    let expected_type = session.active_piece.piece_type;
    let type_index = expected_type as usize;
    session.hold_piece();
    let h = held_piece_state(&session).unwrap();
    assert_eq!(
        h.cells,
        PIECE_CELLS[type_index][0],
        "HeldPieceRender.cells must be PIECE_CELLS[type_index][0] (rotation-0 offsets)"
    );
}

#[test]
fn held_piece_cells_is_fixed_size_array_of_four() {
    // Verify cells field is [(i32,i32);4] — length 4
    let h = HeldPieceRender { type_index: 0, cells: PIECE_CELLS[0][0] };
    assert_eq!(h.cells.len(), 4);
}

// Held piece for each piece type via direct field manipulation
#[test]
fn held_piece_state_i_piece_type_index_0() {
    let mut session = GameSession::new();
    session.held_piece = Some(TetrominoType::I);
    let h = held_piece_state(&session).unwrap();
    assert_eq!(h.type_index, 0);
    assert_eq!(h.cells, PIECE_CELLS[0][0]);
}

#[test]
fn held_piece_state_o_piece_type_index_1() {
    let mut session = GameSession::new();
    session.held_piece = Some(TetrominoType::O);
    let h = held_piece_state(&session).unwrap();
    assert_eq!(h.type_index, 1);
    assert_eq!(h.cells, PIECE_CELLS[1][0]);
}

#[test]
fn held_piece_state_l_piece_type_index_6() {
    let mut session = GameSession::new();
    session.held_piece = Some(TetrominoType::L);
    let h = held_piece_state(&session).unwrap();
    assert_eq!(h.type_index, 6);
    assert_eq!(h.cells, PIECE_CELLS[6][0]);
}

// --- GameStatusRender derives ---

#[test]
fn game_status_render_derives_debug() {
    let session = GameSession::new();
    let status = game_status(&session);
    let s = format!("{:?}", status);
    assert!(!s.is_empty());
}

#[test]
fn game_status_render_derives_partial_eq() {
    let session = GameSession::new();
    let a = game_status(&session);
    let b = game_status(&session);
    // Both come from same session state — fields match
    assert_eq!(a.score, b.score);
    assert_eq!(a.level, b.level);
    assert_eq!(a.state, b.state);
}

// --- game_status: new session defaults ---

#[test]
fn game_status_score_is_zero_on_new_session() {
    let session = GameSession::new();
    let status = game_status(&session);
    assert_eq!(status.score, 0);
}

#[test]
fn game_status_total_lines_is_zero_on_new_session() {
    let session = GameSession::new();
    let status = game_status(&session);
    assert_eq!(status.total_lines, 0);
}

#[test]
fn game_status_level_is_one_with_zero_lines() {
    // level_for_lines(0) = 1 + 0/10 = 1
    let session = GameSession::new();
    let status = game_status(&session);
    assert_eq!(status.level, 1, "level should be 1 when 0 lines cleared");
}

#[test]
fn game_status_combo_count_is_zero_on_new_session() {
    let session = GameSession::new();
    let status = game_status(&session);
    assert_eq!(status.combo_count, 0);
}

#[test]
fn game_status_max_combo_is_zero_on_new_session() {
    let session = GameSession::new();
    let status = game_status(&session);
    assert_eq!(status.max_combo, 0);
}

#[test]
fn game_status_pieces_placed_is_zero_on_new_session() {
    let session = GameSession::new();
    let status = game_status(&session);
    assert_eq!(status.pieces_placed, 0);
}

#[test]
fn game_status_time_played_secs_is_zero_on_new_session() {
    let session = GameSession::new();
    let status = game_status(&session);
    assert_eq!(status.time_played_secs, 0.0);
}

#[test]
fn game_status_state_is_playing_on_new_session() {
    let session = GameSession::new();
    let status = game_status(&session);
    assert_eq!(status.state, GameState::Playing);
}

#[test]
fn game_status_can_hold_is_true_on_new_session() {
    let session = GameSession::new();
    let status = game_status(&session);
    assert!(status.can_hold, "can_hold must be true at start of session");
}

// --- game_status: field mapping with modified session ---

#[test]
fn game_status_score_maps_from_session_score() {
    let mut session = GameSession::new();
    session.score = 1500;
    let status = game_status(&session);
    assert_eq!(status.score, 1500);
}

#[test]
fn game_status_total_lines_maps_from_session_total_lines() {
    let mut session = GameSession::new();
    session.total_lines = 7;
    let status = game_status(&session);
    assert_eq!(status.total_lines, 7);
}

#[test]
fn game_status_level_computed_from_total_lines() {
    // level_for_lines(10) = 1 + 10/10 = 2
    let mut session = GameSession::new();
    session.total_lines = 10;
    let status = game_status(&session);
    assert_eq!(status.level, 2, "10 cleared lines should yield level 2");
}

#[test]
fn game_status_level_computed_for_20_lines() {
    // level_for_lines(20) = 1 + 20/10 = 3
    let mut session = GameSession::new();
    session.total_lines = 20;
    let status = game_status(&session);
    assert_eq!(status.level, 3);
}

#[test]
fn game_status_combo_count_maps_from_session() {
    let mut session = GameSession::new();
    session.combo_count = 4;
    let status = game_status(&session);
    assert_eq!(status.combo_count, 4);
}

#[test]
fn game_status_max_combo_maps_from_session() {
    let mut session = GameSession::new();
    session.max_combo = 8;
    let status = game_status(&session);
    assert_eq!(status.max_combo, 8);
}

#[test]
fn game_status_pieces_placed_maps_from_session() {
    let mut session = GameSession::new();
    session.pieces_placed = 42;
    let status = game_status(&session);
    assert_eq!(status.pieces_placed, 42);
}

#[test]
fn game_status_time_played_secs_maps_from_session() {
    let mut session = GameSession::new();
    session.time_played_secs = 123.456;
    let status = game_status(&session);
    assert!((status.time_played_secs - 123.456).abs() < 1e-9);
}

#[test]
fn game_status_can_hold_false_after_holding() {
    let mut session = GameSession::new();
    session.hold_piece();
    let status = game_status(&session);
    assert!(!status.can_hold, "can_hold should be false after using hold");
}

// --- GameState re-export from render ---

#[test]
fn game_state_reexported_from_render_module() {
    // GameState accessible from rhythm_grid::render
    let state: GameState = GameState::Playing;
    assert_eq!(state, GameStateFromGame::Playing);
}

#[test]
fn game_state_render_and_game_variants_are_equivalent() {
    assert_eq!(GameState::Playing, GameStateFromGame::Playing);
    assert_eq!(GameState::Paused, GameStateFromGame::Paused);
    assert_eq!(GameState::GameOver, GameStateFromGame::GameOver);
    assert_eq!(GameState::Menu, GameStateFromGame::Menu);
}
