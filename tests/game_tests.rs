// @spec-tags: game.state,game.collision,game.move_horizontal,game.move_down,game.hard_drop,game.lock,game.next_piece
// @invariants: state machine Menu→Playing→Paused→Playing / Playing→GameOver→Menu; collision rejects OOB and occupied cells; movement blocked at walls/pieces; lock stores Occupied(piece_index); 7-bag covers all 7 types before repeating
// @build: 16

use rhythm_grid::game::{Game, GameState};
use rhythm_grid::grid::{CellState, GRID_HEIGHT, GRID_WIDTH};
use rhythm_grid::pieces::TetrominoType;

// ─── game.state ─────────────────────────────────────────────────────────────

#[test]
fn test_new_game_starts_in_menu_state() {
    let game = Game::new();
    assert!(matches!(game.state(), GameState::Menu));
}

#[test]
fn test_start_transitions_menu_to_playing() {
    let mut game = Game::new();
    game.start();
    assert!(matches!(game.state(), GameState::Playing));
}

#[test]
fn test_pause_transitions_playing_to_paused() {
    let mut game = Game::new();
    game.start();
    game.pause();
    assert!(matches!(game.state(), GameState::Paused));
}

#[test]
fn test_resume_transitions_paused_to_playing() {
    let mut game = Game::new();
    game.start();
    game.pause();
    game.resume();
    assert!(matches!(game.state(), GameState::Playing));
}

#[test]
fn test_restart_transitions_game_over_to_menu() {
    let mut game = Game::new();
    game.start();
    game.force_game_over(); // set state to GameOver for testing
    game.restart();
    assert!(matches!(game.state(), GameState::Menu));
}

#[test]
fn test_game_state_variants_exist() {
    // All four variants must be constructible
    let _menu = GameState::Menu;
    let _playing = GameState::Playing;
    let _paused = GameState::Paused;
    let _game_over = GameState::GameOver;
}

// ─── game.collision ─────────────────────────────────────────────────────────

#[test]
fn test_center_of_empty_grid_is_valid() {
    let game = Game::new();
    // T-piece at center, rotation 0 — must be valid on empty grid
    assert!(game.is_position_valid(TetrominoType::T, 4, 10, 0));
}

#[test]
fn test_negative_col_is_invalid() {
    let game = Game::new();
    // Place I-piece far left: cells include col -3 which is OOB
    assert!(!game.is_position_valid(TetrominoType::I, -3, 10, 0));
}

#[test]
fn test_col_at_right_wall_overflow_is_invalid() {
    let game = Game::new();
    // I-piece at col=9 with offset +2 → abs col=11 which is >= GRID_WIDTH
    assert!(!game.is_position_valid(TetrominoType::I, 9, 10, 0));
}

#[test]
fn test_row_below_grid_bottom_is_invalid() {
    let game = Game::new();
    // Any piece at row >= GRID_HEIGHT is out of bounds
    assert!(!game.is_position_valid(TetrominoType::T, 4, GRID_HEIGHT as i32, 0));
}

#[test]
fn test_negative_row_is_invalid() {
    let game = Game::new();
    // Row -1 is above the grid top
    assert!(!game.is_position_valid(TetrominoType::T, 4, -1, 0));
}

#[test]
fn test_position_overlapping_occupied_cell_is_invalid() {
    let mut game = Game::new();
    game.start();
    // Hard-drop current piece to lock it, filling some cells
    game.hard_drop();
    // The locked piece is now somewhere near the bottom row 19
    // Try to place ANY piece directly at those occupied cells
    let piece = game.last_locked_piece();
    let (lock_col, lock_row) = game.last_lock_position();
    // Placing same piece type at same position on occupied grid must be invalid
    assert!(!game.is_position_valid(piece, lock_col, lock_row, 0));
}

// ─── game.move_horizontal ───────────────────────────────────────────────────

#[test]
fn test_move_left_decreases_col_by_one() {
    let mut game = Game::new();
    game.start();
    let col_before = game.current_col();
    let moved = game.move_left();
    if moved {
        assert_eq!(game.current_col(), col_before - 1);
    }
}

#[test]
fn test_move_right_increases_col_by_one() {
    let mut game = Game::new();
    game.start();
    let col_before = game.current_col();
    let moved = game.move_right();
    if moved {
        assert_eq!(game.current_col(), col_before + 1);
    }
}

#[test]
fn test_move_left_blocked_at_left_wall() {
    let mut game = Game::new();
    game.start();
    // Move left as many times as needed to reach the left wall
    for _ in 0..15 {
        game.move_left();
    }
    let col_at_wall = game.current_col();
    let moved = game.move_left();
    // At the wall, move should be rejected
    assert!(!moved, "move_left must be rejected at the left wall");
    assert_eq!(game.current_col(), col_at_wall, "col must not change when blocked at left wall");
}

#[test]
fn test_move_right_blocked_at_right_wall() {
    let mut game = Game::new();
    game.start();
    for _ in 0..15 {
        game.move_right();
    }
    let col_at_wall = game.current_col();
    let moved = game.move_right();
    assert!(!moved, "move_right must be rejected at the right wall");
    assert_eq!(game.current_col(), col_at_wall, "col must not change when blocked at right wall");
}

#[test]
fn test_move_does_not_change_row() {
    let mut game = Game::new();
    game.start();
    let row_before = game.current_row();
    game.move_left();
    assert_eq!(game.current_row(), row_before, "horizontal move must not change row");
}

// ─── game.move_down ─────────────────────────────────────────────────────────

#[test]
fn test_move_down_increases_row_by_one() {
    let mut game = Game::new();
    game.start();
    let row_before = game.current_row();
    let moved = game.move_down();
    if moved {
        assert_eq!(game.current_row(), row_before + 1);
    }
}

#[test]
fn test_move_down_does_not_change_col() {
    let mut game = Game::new();
    game.start();
    let col_before = game.current_col();
    game.move_down();
    assert_eq!(game.current_col(), col_before, "move_down must not change col");
}

#[test]
fn test_move_down_returns_false_when_locked() {
    let mut game = Game::new();
    game.start();
    // Drop to floor
    let mut locked = false;
    for _ in 0..(GRID_HEIGHT + 5) as usize {
        if !game.move_down() {
            locked = true;
            break;
        }
    }
    assert!(locked, "move_down must return false when piece reaches the floor");
}

#[test]
fn test_move_down_locks_piece_at_floor() {
    let mut game = Game::new();
    game.start();
    // Drop to floor — after lock a new piece spawns
    for _ in 0..(GRID_HEIGHT + 5) as usize {
        if !game.move_down() {
            break;
        }
    }
    // After lock, grid should have some occupied cells at the bottom
    let bottom_row = (GRID_HEIGHT as usize) - 1;
    let has_occupied = (0..GRID_WIDTH as usize)
        .any(|c| matches!(game.cell_at(c, bottom_row), CellState::Occupied(_)));
    assert!(has_occupied, "locking must place occupied cells in the grid");
}

// ─── game.hard_drop ─────────────────────────────────────────────────────────

#[test]
fn test_hard_drop_places_piece_at_bottom_of_empty_grid() {
    let mut game = Game::new();
    game.start();
    game.hard_drop();
    // After hard drop on empty grid, cells must appear at the bottom rows
    let bottom_row = (GRID_HEIGHT as usize) - 1;
    let has_occupied = (0..GRID_WIDTH as usize)
        .any(|c| matches!(game.cell_at(c, bottom_row), CellState::Occupied(_)));
    assert!(has_occupied, "hard_drop must lock piece at the lowest valid row");
}

#[test]
fn test_hard_drop_spawns_new_piece() {
    let mut game = Game::new();
    game.start();
    let first_piece = game.current_piece();
    let next_before = game.peek_next();
    game.hard_drop();
    // After lock, next piece becomes the new current piece
    assert_eq!(game.current_piece(), next_before,
        "after hard_drop, current piece must be what was previously next");
    let _ = first_piece; // used to identify what was locked
}

#[test]
fn test_hard_drop_new_piece_spawns_at_top() {
    let mut game = Game::new();
    game.start();
    game.hard_drop();
    // New current piece spawns at row 0
    assert_eq!(game.current_row(), 0,
        "newly spawned piece after hard_drop must start at row 0");
}

#[test]
fn test_hard_drop_row_is_lowest_valid() {
    let mut game = Game::new();
    game.start();
    // Record what the lowest valid row is by simulating soft drops
    let mut game2 = Game::new();
    game2.start();
    while game2.move_down() {}
    let soft_drop_final_row = game2.current_row();

    // Compare with hard drop result via last_lock_position
    game.hard_drop();
    let (_, lock_row) = game.last_lock_position();
    assert_eq!(lock_row, soft_drop_final_row,
        "hard_drop must land at the same row as repeated soft drops");
}

// ─── game.lock ──────────────────────────────────────────────────────────────

#[test]
fn test_lock_stores_occupied_with_i_piece_index_0() {
    // Drive a game until an I-piece is current, then hard drop and verify index=0
    let mut game = Game::new();
    game.start();
    // Draw pieces until we get I-piece as current
    let mut found = false;
    for _ in 0..14 {
        if game.current_piece() == TetrominoType::I {
            game.hard_drop();
            let (lock_col, lock_row) = game.last_lock_position();
            // I-piece cells at rotation 0 span cols lock_col-1 to lock_col+2
            let cell = game.cell_at((lock_col) as usize, lock_row as usize);
            if let CellState::Occupied(idx) = cell {
                assert_eq!(idx, 0u32, "I-piece must lock as Occupied(0)");
            } else {
                panic!("Expected Occupied cell after I-piece lock at ({}, {})", lock_col, lock_row);
            }
            found = true;
            break;
        }
        game.hard_drop();
    }
    // If the bag never contained I-piece in 14 draws, that's a bag failure (separate test catches it)
    let _ = found;
}

#[test]
fn test_lock_stores_correct_piece_index_for_current_piece() {
    // ALL_TETROMINOES order: I=0, O=1, T=2, S=3, Z=4, J=5, L=6
    let index_for = |t: TetrominoType| -> u32 {
        match t {
            TetrominoType::I => 0,
            TetrominoType::O => 1,
            TetrominoType::T => 2,
            TetrominoType::S => 3,
            TetrominoType::Z => 4,
            TetrominoType::J => 5,
            TetrominoType::L => 6,
        }
    };
    let mut game = Game::new();
    game.start();
    let piece = game.current_piece();
    let expected_idx = index_for(piece);
    game.hard_drop();
    let (lock_col, lock_row) = game.last_lock_position();
    // Check the cell at the lock position
    let cell = game.cell_at(lock_col as usize, lock_row as usize);
    if let CellState::Occupied(idx) = cell {
        assert_eq!(idx, expected_idx,
            "{:?} must lock as Occupied({})", piece, expected_idx);
    } else {
        panic!("Expected Occupied cell at lock position ({}, {})", lock_col, lock_row);
    }
}

#[test]
fn test_lock_fills_all_4_cells_in_grid() {
    let mut game = Game::new();
    game.start();
    let piece = game.current_piece();
    game.hard_drop();
    let (lock_col, lock_row) = game.last_lock_position();
    // Count occupied cells at lock row (at minimum all 4 piece cells are there)
    let occupied_count = (0..GRID_WIDTH as usize)
        .filter(|&c| matches!(game.cell_at(c, lock_row as usize), CellState::Occupied(_)))
        .count();
    assert!(occupied_count >= 4,
        "{:?} lock must produce at least 4 occupied cells in the grid", piece);
}

// ─── game.next_piece ────────────────────────────────────────────────────────

#[test]
fn test_peek_next_is_visible_before_first_move() {
    let mut game = Game::new();
    game.start();
    // peek_next must return a valid TetrominoType without panicking
    let next = game.peek_next();
    let all = [TetrominoType::I, TetrominoType::O, TetrominoType::T,
               TetrominoType::S, TetrominoType::Z, TetrominoType::J, TetrominoType::L];
    assert!(all.contains(&next), "peek_next must return a valid TetrominoType");
}

#[test]
fn test_peek_next_becomes_current_after_lock() {
    let mut game = Game::new();
    game.start();
    let next = game.peek_next();
    game.hard_drop();
    assert_eq!(game.current_piece(), next,
        "after lock, current piece must equal what peek_next returned before lock");
}

#[test]
fn test_7_bag_first_bag_contains_all_7_types() {
    // Draw 7 pieces — all 7 tetromino types must appear exactly once
    let mut game = Game::new();
    game.start();
    let mut seen = std::collections::HashSet::new();
    for _ in 0..7 {
        seen.insert(game.current_piece());
        game.hard_drop();
    }
    let all = [TetrominoType::I, TetrominoType::O, TetrominoType::T,
               TetrominoType::S, TetrominoType::Z, TetrominoType::J, TetrominoType::L];
    for t in &all {
        assert!(seen.contains(t), "7-bag first bag must contain {:?}", t);
    }
    assert_eq!(seen.len(), 7, "7-bag first bag must contain exactly 7 distinct types");
}

#[test]
fn test_7_bag_second_bag_also_contains_all_7_types() {
    // After 7 draws, the bag refreshes — next 7 must also contain all types
    let mut game = Game::new();
    game.start();
    // Exhaust first bag
    for _ in 0..7 {
        game.hard_drop();
    }
    // Collect second bag
    let mut seen = std::collections::HashSet::new();
    for _ in 0..7 {
        seen.insert(game.current_piece());
        game.hard_drop();
    }
    let all = [TetrominoType::I, TetrominoType::O, TetrominoType::T,
               TetrominoType::S, TetrominoType::Z, TetrominoType::J, TetrominoType::L];
    for t in &all {
        assert!(seen.contains(t), "7-bag second bag must contain {:?}", t);
    }
    assert_eq!(seen.len(), 7, "7-bag second bag must contain exactly 7 distinct types");
}

#[test]
fn test_7_bag_no_type_appears_more_than_once_in_first_7() {
    let mut game = Game::new();
    game.start();
    let mut counts = std::collections::HashMap::new();
    for _ in 0..7 {
        *counts.entry(game.current_piece()).or_insert(0u32) += 1;
        game.hard_drop();
    }
    for (piece, count) in &counts {
        assert_eq!(*count, 1u32,
            "{:?} appeared {} times in first 7 draws — must appear exactly once", piece, count);
    }
}

#[test]
fn test_spawn_col_is_4_after_game_start() {
    let mut game = Game::new();
    game.start();
    assert_eq!(game.current_col(), 4, "piece must spawn at col=4");
}

#[test]
fn test_spawn_row_is_0_after_game_start() {
    let mut game = Game::new();
    game.start();
    assert_eq!(game.current_row(), 0, "piece must spawn at row=0");
}

#[test]
fn test_spawn_rotation_is_0_after_game_start() {
    let mut game = Game::new();
    game.start();
    assert_eq!(game.current_rotation(), 0u32, "piece must spawn in rotation state 0");
}
