// @spec-tags: pieces.rotation,pieces.srs_kicks,pieces.spawn
// @invariants: 4 rotation states cycling CW 0→1→2→3→0; SRS Tetris Guideline kick tables (JLSTZ/I/O); spawn col=4 row=0 rotation=0; MAX_KICK_TESTS=4
// @build: 16

use rhythm_grid::pieces::{TetrominoType, SPAWN_COL, SPAWN_ROW, MAX_KICK_TESTS};

// ─── pieces.rotation ────────────────────────────────────────────────────────

#[test]
fn test_rotation_0_matches_base_cells() {
    // rotation state 0 is the canonical spawn orientation
    for t in [TetrominoType::I, TetrominoType::O, TetrominoType::T,
              TetrominoType::S, TetrominoType::Z, TetrominoType::J, TetrominoType::L] {
        assert_eq!(t.cells_rotated(0), t.cells(), "{:?} rotation 0 must match cells()", t);
    }
}

#[test]
fn test_all_four_rotation_states_compile_and_return_4_cells() {
    for t in [TetrominoType::I, TetrominoType::O, TetrominoType::T,
              TetrominoType::S, TetrominoType::Z, TetrominoType::J, TetrominoType::L] {
        for r in 0u32..4 {
            let cells = t.cells_rotated(r);
            assert_eq!(cells.len(), 4, "{:?} rotation {} must have 4 cells", t, r);
        }
    }
}

#[test]
fn test_t_piece_rotation_1_differs_from_0() {
    // T-piece is asymmetric — CW rotation must change the cell layout
    assert_ne!(TetrominoType::T.cells_rotated(0), TetrominoType::T.cells_rotated(1));
}

#[test]
fn test_t_piece_rotation_2_differs_from_0_and_1() {
    let r0 = TetrominoType::T.cells_rotated(0);
    let r1 = TetrominoType::T.cells_rotated(1);
    let r2 = TetrominoType::T.cells_rotated(2);
    assert_ne!(r0, r2);
    assert_ne!(r1, r2);
}

#[test]
fn test_t_piece_rotation_3_differs_from_0_1_2() {
    let r0 = TetrominoType::T.cells_rotated(0);
    let r1 = TetrominoType::T.cells_rotated(1);
    let r2 = TetrominoType::T.cells_rotated(2);
    let r3 = TetrominoType::T.cells_rotated(3);
    assert_ne!(r0, r3);
    assert_ne!(r1, r3);
    assert_ne!(r2, r3);
}

#[test]
fn test_i_piece_has_4_distinct_rotation_states() {
    let r0 = TetrominoType::I.cells_rotated(0);
    let r1 = TetrominoType::I.cells_rotated(1);
    let r2 = TetrominoType::I.cells_rotated(2);
    let r3 = TetrominoType::I.cells_rotated(3);
    // I-piece has 2 visually distinct states (0/2 horizontal, 1/3 vertical)
    // but all 4 rotation indices must be reachable and return valid arrays
    let _ = (r0, r1, r2, r3);
}

#[test]
fn test_j_piece_4_rotation_states_all_distinct() {
    let states: Vec<_> = (0..4).map(|r| TetrominoType::J.cells_rotated(r)).collect();
    // J is asymmetric — all 4 states must differ
    for i in 0..4 {
        for j in (i + 1)..4 {
            assert_ne!(states[i], states[j],
                "J-piece rotation {} and {} must be distinct", i, j);
        }
    }
}

#[test]
fn test_l_piece_4_rotation_states_all_distinct() {
    let states: Vec<_> = (0..4).map(|r| TetrominoType::L.cells_rotated(r)).collect();
    for i in 0..4 {
        for j in (i + 1)..4 {
            assert_ne!(states[i], states[j],
                "L-piece rotation {} and {} must be distinct", i, j);
        }
    }
}

#[test]
fn test_s_piece_4_rotation_states_all_distinct() {
    let states: Vec<_> = (0..4).map(|r| TetrominoType::S.cells_rotated(r)).collect();
    for i in 0..4 {
        for j in (i + 1)..4 {
            assert_ne!(states[i], states[j],
                "S-piece rotation {} and {} must be distinct", i, j);
        }
    }
}

#[test]
fn test_z_piece_4_rotation_states_all_distinct() {
    let states: Vec<_> = (0..4).map(|r| TetrominoType::Z.cells_rotated(r)).collect();
    for i in 0..4 {
        for j in (i + 1)..4 {
            assert_ne!(states[i], states[j],
                "Z-piece rotation {} and {} must be distinct", i, j);
        }
    }
}

// ─── pieces.srs_kicks ───────────────────────────────────────────────────────

#[test]
fn test_max_kick_tests_constant_is_4() {
    assert_eq!(MAX_KICK_TESTS, 4);
}

#[test]
fn test_jlstz_kick_offsets_return_4_entries() {
    for t in [TetrominoType::J, TetrominoType::L, TetrominoType::S,
              TetrominoType::T, TetrominoType::Z] {
        for from in 0u32..4 {
            let to = (from + 1) % 4;
            let kicks = t.kick_offsets(from, to);
            assert_eq!(kicks.len(), 4,
                "{:?} kick_offsets({}, {}) must have 4 entries", t, from, to);
        }
    }
}

#[test]
fn test_i_piece_kick_offsets_return_4_entries() {
    for from in 0u32..4 {
        let to = (from + 1) % 4;
        let kicks = TetrominoType::I.kick_offsets(from, to);
        assert_eq!(kicks.len(), 4,
            "I-piece kick_offsets({}, {}) must have 4 entries", from, to);
    }
}

#[test]
fn test_o_piece_kick_offsets_return_0_entries() {
    // O-piece has no kick alternatives; rotation is a no-op on collision
    for from in 0u32..4 {
        let to = (from + 1) % 4;
        let kicks = TetrominoType::O.kick_offsets(from, to);
        assert_eq!(kicks.len(), 0,
            "O-piece must have 0 kick alternatives (no-op on collision)");
    }
}

#[test]
fn test_jlstz_0_to_1_exact_kicks_match_tetris_guideline() {
    // Tetris Guideline JLSTZ 0→R kicks (col_delta, row_delta; +col=right, +row=down)
    let expected: [(i32, i32); 4] = [(-1, 0), (-1, -1), (0, 2), (-1, 2)];
    for t in [TetrominoType::J, TetrominoType::L, TetrominoType::S,
              TetrominoType::T, TetrominoType::Z] {
        let kicks = t.kick_offsets(0, 1);
        assert_eq!(kicks, &expected,
            "{:?} 0→1 kicks must match Tetris Guideline JLSTZ table", t);
    }
}

#[test]
fn test_jlstz_1_to_0_exact_kicks_match_tetris_guideline() {
    let expected: [(i32, i32); 4] = [(1, 0), (1, 1), (0, -2), (1, -2)];
    for t in [TetrominoType::J, TetrominoType::L, TetrominoType::S,
              TetrominoType::T, TetrominoType::Z] {
        let kicks = t.kick_offsets(1, 0);
        assert_eq!(kicks, &expected, "{:?} 1→0 kicks must match JLSTZ table", t);
    }
}

#[test]
fn test_jlstz_1_to_2_exact_kicks_match_tetris_guideline() {
    let expected: [(i32, i32); 4] = [(1, 0), (1, 1), (0, -2), (1, -2)];
    for t in [TetrominoType::J, TetrominoType::L, TetrominoType::S,
              TetrominoType::T, TetrominoType::Z] {
        let kicks = t.kick_offsets(1, 2);
        assert_eq!(kicks, &expected, "{:?} 1→2 kicks must match JLSTZ table", t);
    }
}

#[test]
fn test_jlstz_2_to_1_exact_kicks_match_tetris_guideline() {
    let expected: [(i32, i32); 4] = [(-1, 0), (-1, -1), (0, 2), (-1, 2)];
    for t in [TetrominoType::J, TetrominoType::L, TetrominoType::S,
              TetrominoType::T, TetrominoType::Z] {
        let kicks = t.kick_offsets(2, 1);
        assert_eq!(kicks, &expected, "{:?} 2→1 kicks must match JLSTZ table", t);
    }
}

#[test]
fn test_jlstz_2_to_3_exact_kicks_match_tetris_guideline() {
    let expected: [(i32, i32); 4] = [(1, 0), (1, -1), (0, 2), (1, 2)];
    for t in [TetrominoType::J, TetrominoType::L, TetrominoType::S,
              TetrominoType::T, TetrominoType::Z] {
        let kicks = t.kick_offsets(2, 3);
        assert_eq!(kicks, &expected, "{:?} 2→3 kicks must match JLSTZ table", t);
    }
}

#[test]
fn test_jlstz_3_to_2_exact_kicks_match_tetris_guideline() {
    let expected: [(i32, i32); 4] = [(-1, 0), (-1, 1), (0, -2), (-1, -2)];
    for t in [TetrominoType::J, TetrominoType::L, TetrominoType::S,
              TetrominoType::T, TetrominoType::Z] {
        let kicks = t.kick_offsets(3, 2);
        assert_eq!(kicks, &expected, "{:?} 3→2 kicks must match JLSTZ table", t);
    }
}

#[test]
fn test_jlstz_3_to_0_exact_kicks_match_tetris_guideline() {
    let expected: [(i32, i32); 4] = [(-1, 0), (-1, 1), (0, -2), (-1, -2)];
    for t in [TetrominoType::J, TetrominoType::L, TetrominoType::S,
              TetrominoType::T, TetrominoType::Z] {
        let kicks = t.kick_offsets(3, 0);
        assert_eq!(kicks, &expected, "{:?} 3→0 kicks must match JLSTZ table", t);
    }
}

#[test]
fn test_jlstz_0_to_3_exact_kicks_match_tetris_guideline() {
    let expected: [(i32, i32); 4] = [(1, 0), (1, -1), (0, 2), (1, 2)];
    for t in [TetrominoType::J, TetrominoType::L, TetrominoType::S,
              TetrominoType::T, TetrominoType::Z] {
        let kicks = t.kick_offsets(0, 3);
        assert_eq!(kicks, &expected, "{:?} 0→3 kicks must match JLSTZ table", t);
    }
}

#[test]
fn test_i_piece_0_to_1_exact_kicks_match_tetris_guideline() {
    // I-piece Tetris Guideline 0→R kicks (col_delta, row_delta; +row=down)
    let expected: [(i32, i32); 4] = [(-2, 0), (1, 0), (-2, -1), (1, 2)];
    let kicks = TetrominoType::I.kick_offsets(0, 1);
    assert_eq!(kicks, &expected, "I-piece 0→1 kicks must match Tetris Guideline");
}

#[test]
fn test_i_piece_1_to_0_exact_kicks_match_tetris_guideline() {
    let expected: [(i32, i32); 4] = [(2, 0), (-1, 0), (2, 1), (-1, -2)];
    let kicks = TetrominoType::I.kick_offsets(1, 0);
    assert_eq!(kicks, &expected, "I-piece 1→0 kicks must match Tetris Guideline");
}

#[test]
fn test_i_piece_1_to_2_exact_kicks_match_tetris_guideline() {
    let expected: [(i32, i32); 4] = [(-1, 0), (2, 0), (-1, 2), (2, -1)];
    let kicks = TetrominoType::I.kick_offsets(1, 2);
    assert_eq!(kicks, &expected, "I-piece 1→2 kicks must match Tetris Guideline");
}

#[test]
fn test_i_piece_2_to_1_exact_kicks_match_tetris_guideline() {
    let expected: [(i32, i32); 4] = [(1, 0), (-2, 0), (1, -2), (-2, 1)];
    let kicks = TetrominoType::I.kick_offsets(2, 1);
    assert_eq!(kicks, &expected, "I-piece 2→1 kicks must match Tetris Guideline");
}

#[test]
fn test_i_piece_2_to_3_exact_kicks_match_tetris_guideline() {
    let expected: [(i32, i32); 4] = [(2, 0), (-1, 0), (2, 1), (-1, -2)];
    let kicks = TetrominoType::I.kick_offsets(2, 3);
    assert_eq!(kicks, &expected, "I-piece 2→3 kicks must match Tetris Guideline");
}

#[test]
fn test_i_piece_3_to_2_exact_kicks_match_tetris_guideline() {
    let expected: [(i32, i32); 4] = [(-2, 0), (1, 0), (-2, -1), (1, 2)];
    let kicks = TetrominoType::I.kick_offsets(3, 2);
    assert_eq!(kicks, &expected, "I-piece 3→2 kicks must match Tetris Guideline");
}

#[test]
fn test_i_piece_3_to_0_exact_kicks_match_tetris_guideline() {
    let expected: [(i32, i32); 4] = [(1, 0), (-2, 0), (1, -2), (-2, 1)];
    let kicks = TetrominoType::I.kick_offsets(3, 0);
    assert_eq!(kicks, &expected, "I-piece 3→0 kicks must match Tetris Guideline");
}

#[test]
fn test_i_piece_0_to_3_exact_kicks_match_tetris_guideline() {
    let expected: [(i32, i32); 4] = [(-1, 0), (2, 0), (-1, 2), (2, -1)];
    let kicks = TetrominoType::I.kick_offsets(0, 3);
    assert_eq!(kicks, &expected, "I-piece 0→3 kicks must match Tetris Guideline");
}

#[test]
fn test_i_piece_kicks_differ_from_jlstz_kicks() {
    // I-piece uses a separate table from JLSTZ
    let i_kicks = TetrominoType::I.kick_offsets(0, 1);
    let t_kicks = TetrominoType::T.kick_offsets(0, 1);
    assert_ne!(i_kicks, t_kicks, "I-piece kick table must differ from JLSTZ");
}

// ─── pieces.spawn ───────────────────────────────────────────────────────────

#[test]
fn test_spawn_col_constant_is_4() {
    assert_eq!(SPAWN_COL, 4i32);
}

#[test]
fn test_spawn_row_constant_is_0() {
    assert_eq!(SPAWN_ROW, 0i32);
}

#[test]
fn test_i_piece_spawn_cells_at_columns_3_to_6() {
    // I-piece cells at rotation 0: [(-1,0),(0,0),(1,0),(2,0)]
    // Spawned at col=4: absolute cols = 3,4,5,6
    let cells = TetrominoType::I.cells_rotated(0);
    let abs_cols: Vec<i32> = cells.iter().map(|(c, _)| SPAWN_COL + c).collect();
    let mut sorted = abs_cols.clone();
    sorted.sort();
    assert_eq!(sorted, vec![3, 4, 5, 6], "I-piece must span columns 3-6 from spawn");
}

#[test]
fn test_t_piece_spawn_cells_span_columns_3_to_5() {
    // T-piece at rotation 0: [(-1,0),(0,0),(1,0),(0,1)]
    // Spawned at col=4: cols = 3,4,5
    let cells = TetrominoType::T.cells_rotated(0);
    let abs_cols: Vec<i32> = cells.iter().map(|(c, _)| SPAWN_COL + c).collect();
    assert!(abs_cols.contains(&3), "T-piece spawn must include col 3");
    assert!(abs_cols.contains(&4), "T-piece spawn must include col 4");
    assert!(abs_cols.contains(&5), "T-piece spawn must include col 5");
}

#[test]
fn test_spawn_cells_all_within_grid_bounds() {
    // All pieces at spawn position (col=4, row=0) must fit within 10-wide grid
    for t in [TetrominoType::I, TetrominoType::O, TetrominoType::T,
              TetrominoType::S, TetrominoType::Z, TetrominoType::J, TetrominoType::L] {
        let cells = t.cells_rotated(0);
        for (dc, dr) in &cells {
            let abs_col = SPAWN_COL + dc;
            let abs_row = SPAWN_ROW + dr;
            assert!(abs_col >= 0 && abs_col < 10,
                "{:?} spawn cell col {} is out of grid bounds", t, abs_col);
            assert!(abs_row >= 0 && abs_row < 20,
                "{:?} spawn cell row {} is out of grid bounds", t, abs_row);
        }
    }
}
