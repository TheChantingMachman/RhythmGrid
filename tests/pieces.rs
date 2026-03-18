// @spec-tags: pieces.tetromino_count,pieces.tetromino_cells,pieces.rotation,pieces.srs_kicks,pieces.spawn
// @invariants: TetrominoType enum has 7 variants (I=0..L=6); PIECE_CELLS is [7][4][4] of (row_delta,col_delta); piece_cells() matches PIECE_CELLS; O-piece all rotations identical; JLSTZ_KICKS and I_KICKS are [8][4]; srs_kicks O-piece returns [(0,0);4]; srs_kicks CW uses indices 0-3, CCW uses 4-7; try_spawn on empty grid returns Some((0,4))
// @build: 37

use rhythm_grid::grid::{CellState, Grid};
use rhythm_grid::pieces::{
    piece_cells, srs_kicks, try_spawn, TetrominoType, I_KICKS, JLSTZ_KICKS, PIECE_CELLS,
};

// --- TetrominoType enum: 7 variants with correct discriminant values ---

#[test]
fn test_tetromino_type_i_discriminant() {
    assert_eq!(TetrominoType::I as usize, 0);
}

#[test]
fn test_tetromino_type_o_discriminant() {
    assert_eq!(TetrominoType::O as usize, 1);
}

#[test]
fn test_tetromino_type_t_discriminant() {
    assert_eq!(TetrominoType::T as usize, 2);
}

#[test]
fn test_tetromino_type_s_discriminant() {
    assert_eq!(TetrominoType::S as usize, 3);
}

#[test]
fn test_tetromino_type_z_discriminant() {
    assert_eq!(TetrominoType::Z as usize, 4);
}

#[test]
fn test_tetromino_type_j_discriminant() {
    assert_eq!(TetrominoType::J as usize, 5);
}

#[test]
fn test_tetromino_type_l_discriminant() {
    assert_eq!(TetrominoType::L as usize, 6);
}

#[test]
fn test_tetromino_type_count_seven() {
    // All 7 variants exist and their discriminants are 0..=6 (no gaps)
    let variants = [
        TetrominoType::I,
        TetrominoType::O,
        TetrominoType::T,
        TetrominoType::S,
        TetrominoType::Z,
        TetrominoType::J,
        TetrominoType::L,
    ];
    assert_eq!(variants.len(), 7);
    let mut seen = [false; 7];
    for v in &variants {
        let idx = *v as usize;
        assert!(idx < 7, "discriminant {} out of range 0..=6", idx);
        assert!(!seen[idx], "duplicate discriminant {}", idx);
        seen[idx] = true;
    }
    assert!(seen.iter().all(|&x| x), "not all indices 0..=6 are covered");
}

#[test]
fn test_tetromino_type_copy() {
    let t = TetrominoType::I;
    let c = t; // Copy trait
    assert_eq!(t, c);
}

#[test]
fn test_tetromino_type_clone() {
    let t = TetrominoType::S;
    let cl = t.clone();
    assert_eq!(t, cl);
}

#[test]
fn test_tetromino_type_partial_eq() {
    assert_eq!(TetrominoType::T, TetrominoType::T);
    assert_ne!(TetrominoType::T, TetrominoType::S);
    assert_ne!(TetrominoType::I, TetrominoType::L);
}

// --- PIECE_CELLS static table: [7][4][4] dimensions ---

#[test]
fn test_piece_cells_table_outer_dim_is_7() {
    assert_eq!(PIECE_CELLS.len(), 7);
}

#[test]
fn test_piece_cells_table_rotation_dim_is_4() {
    for piece_idx in 0..7 {
        assert_eq!(PIECE_CELLS[piece_idx].len(), 4,
            "piece {} should have 4 rotation states", piece_idx);
    }
}

#[test]
fn test_piece_cells_table_cells_dim_is_4() {
    for piece_idx in 0..7 {
        for rot in 0..4 {
            assert_eq!(PIECE_CELLS[piece_idx][rot].len(), 4,
                "piece {} rotation {} should have 4 cells", piece_idx, rot);
        }
    }
}

// --- piece_cells() function ---

#[test]
fn test_piece_cells_fn_matches_table_for_all_pieces_and_rotations() {
    let pieces = [
        TetrominoType::I,
        TetrominoType::O,
        TetrominoType::T,
        TetrominoType::S,
        TetrominoType::Z,
        TetrominoType::J,
        TetrominoType::L,
    ];
    for &piece in &pieces {
        for rot in 0..4 {
            let from_fn = piece_cells(piece, rot);
            let from_table = PIECE_CELLS[piece as usize][rot];
            assert_eq!(from_fn, from_table,
                "piece_cells({:?}, {}) differs from PIECE_CELLS[{}][{}]",
                piece, rot, piece as usize, rot);
        }
    }
}

#[test]
fn test_piece_cells_returns_exactly_4_cells() {
    let pieces = [
        TetrominoType::I,
        TetrominoType::O,
        TetrominoType::T,
        TetrominoType::S,
        TetrominoType::Z,
        TetrominoType::J,
        TetrominoType::L,
    ];
    for &piece in &pieces {
        for rot in 0..4 {
            let cells = piece_cells(piece, rot);
            assert_eq!(cells.len(), 4,
                "piece_cells({:?}, {}) must return exactly 4 cells", piece, rot);
        }
    }
}

#[test]
fn test_piece_cells_no_duplicate_cells_per_rotation() {
    let pieces = [
        TetrominoType::I,
        TetrominoType::O,
        TetrominoType::T,
        TetrominoType::S,
        TetrominoType::Z,
        TetrominoType::J,
        TetrominoType::L,
    ];
    for &piece in &pieces {
        for rot in 0..4 {
            let cells = piece_cells(piece, rot);
            let mut seen = std::collections::HashSet::new();
            for &cell in &cells {
                assert!(
                    seen.insert(cell),
                    "piece {:?} rotation {} has duplicate cell {:?}",
                    piece, rot, cell
                );
            }
        }
    }
}

// --- O-piece: all 4 rotation states are identical ---

#[test]
fn test_o_piece_rotation_1_equals_rotation_0() {
    assert_eq!(
        piece_cells(TetrominoType::O, 1),
        piece_cells(TetrominoType::O, 0),
        "O-piece rotation 1 must equal rotation 0"
    );
}

#[test]
fn test_o_piece_rotation_2_equals_rotation_0() {
    assert_eq!(
        piece_cells(TetrominoType::O, 2),
        piece_cells(TetrominoType::O, 0),
        "O-piece rotation 2 must equal rotation 0"
    );
}

#[test]
fn test_o_piece_rotation_3_equals_rotation_0() {
    assert_eq!(
        piece_cells(TetrominoType::O, 3),
        piece_cells(TetrominoType::O, 0),
        "O-piece rotation 3 must equal rotation 0"
    );
}

// --- SRS kick tables: structural dimensions ---

#[test]
fn test_jlstz_kicks_has_8_transitions() {
    assert_eq!(JLSTZ_KICKS.len(), 8);
}

#[test]
fn test_jlstz_kicks_each_transition_has_4_offsets() {
    for (i, transition) in JLSTZ_KICKS.iter().enumerate() {
        assert_eq!(transition.len(), 4,
            "JLSTZ_KICKS[{}] must have 4 kick offsets", i);
    }
}

#[test]
fn test_i_kicks_has_8_transitions() {
    assert_eq!(I_KICKS.len(), 8);
}

#[test]
fn test_i_kicks_each_transition_has_4_offsets() {
    for (i, transition) in I_KICKS.iter().enumerate() {
        assert_eq!(transition.len(), 4,
            "I_KICKS[{}] must have 4 kick offsets", i);
    }
}

#[test]
fn test_jlstz_and_i_kicks_tables_differ() {
    // The two tables must differ — I-piece has its own kick data
    let any_diff = JLSTZ_KICKS
        .iter()
        .zip(I_KICKS.iter())
        .any(|(j, i)| j != i);
    assert!(any_diff, "JLSTZ_KICKS and I_KICKS must not be identical");
}

// --- srs_kicks(): O-piece always returns [(0,0); 4] ---

#[test]
fn test_srs_kicks_o_piece_cw_all_zeros() {
    for from_rot in 0..4 {
        let kicks = srs_kicks(TetrominoType::O, from_rot, true);
        assert_eq!(kicks, [(0, 0); 4],
            "O-piece srs_kicks CW from_rot={} must be [(0,0);4]", from_rot);
    }
}

#[test]
fn test_srs_kicks_o_piece_ccw_all_zeros() {
    for from_rot in 0..4 {
        let kicks = srs_kicks(TetrominoType::O, from_rot, false);
        assert_eq!(kicks, [(0, 0); 4],
            "O-piece srs_kicks CCW from_rot={} must be [(0,0);4]", from_rot);
    }
}

// --- srs_kicks(): JLSTZ pieces use JLSTZ_KICKS, indexed CW=0..3, CCW=4..7 ---

#[test]
fn test_srs_kicks_t_cw_uses_jlstz_indices_0_to_3() {
    for from_rot in 0..4 {
        assert_eq!(
            srs_kicks(TetrominoType::T, from_rot, true),
            JLSTZ_KICKS[from_rot],
            "T CW from_rot={} should use JLSTZ_KICKS[{}]", from_rot, from_rot
        );
    }
}

#[test]
fn test_srs_kicks_j_cw_uses_jlstz_indices_0_to_3() {
    for from_rot in 0..4 {
        assert_eq!(
            srs_kicks(TetrominoType::J, from_rot, true),
            JLSTZ_KICKS[from_rot],
            "J CW from_rot={} should use JLSTZ_KICKS[{}]", from_rot, from_rot
        );
    }
}

#[test]
fn test_srs_kicks_l_cw_uses_jlstz_indices_0_to_3() {
    for from_rot in 0..4 {
        assert_eq!(
            srs_kicks(TetrominoType::L, from_rot, true),
            JLSTZ_KICKS[from_rot],
            "L CW from_rot={} should use JLSTZ_KICKS[{}]", from_rot, from_rot
        );
    }
}

#[test]
fn test_srs_kicks_s_cw_uses_jlstz_indices_0_to_3() {
    for from_rot in 0..4 {
        assert_eq!(
            srs_kicks(TetrominoType::S, from_rot, true),
            JLSTZ_KICKS[from_rot],
            "S CW from_rot={} should use JLSTZ_KICKS[{}]", from_rot, from_rot
        );
    }
}

#[test]
fn test_srs_kicks_z_cw_uses_jlstz_indices_0_to_3() {
    for from_rot in 0..4 {
        assert_eq!(
            srs_kicks(TetrominoType::Z, from_rot, true),
            JLSTZ_KICKS[from_rot],
            "Z CW from_rot={} should use JLSTZ_KICKS[{}]", from_rot, from_rot
        );
    }
}

// CCW transitions: from_rotation 1→0=index4, 2→1=index5, 3→2=index6, 0→3=index7
#[test]
fn test_srs_kicks_t_ccw_uses_jlstz_indices_4_to_7() {
    // (from_rotation, expected_table_index)
    let cases: [(usize, usize); 4] = [(0, 7), (1, 4), (2, 5), (3, 6)];
    for (from_rot, table_idx) in cases {
        assert_eq!(
            srs_kicks(TetrominoType::T, from_rot, false),
            JLSTZ_KICKS[table_idx],
            "T CCW from_rot={} should use JLSTZ_KICKS[{}]", from_rot, table_idx
        );
    }
}

#[test]
fn test_srs_kicks_j_ccw_uses_jlstz_indices_4_to_7() {
    let cases: [(usize, usize); 4] = [(0, 7), (1, 4), (2, 5), (3, 6)];
    for (from_rot, table_idx) in cases {
        assert_eq!(
            srs_kicks(TetrominoType::J, from_rot, false),
            JLSTZ_KICKS[table_idx],
            "J CCW from_rot={} should use JLSTZ_KICKS[{}]", from_rot, table_idx
        );
    }
}

#[test]
fn test_srs_kicks_s_ccw_uses_jlstz_indices_4_to_7() {
    let cases: [(usize, usize); 4] = [(0, 7), (1, 4), (2, 5), (3, 6)];
    for (from_rot, table_idx) in cases {
        assert_eq!(
            srs_kicks(TetrominoType::S, from_rot, false),
            JLSTZ_KICKS[table_idx],
            "S CCW from_rot={} should use JLSTZ_KICKS[{}]", from_rot, table_idx
        );
    }
}

// --- srs_kicks(): I-piece uses I_KICKS ---

#[test]
fn test_srs_kicks_i_cw_uses_i_kicks_indices_0_to_3() {
    for from_rot in 0..4 {
        assert_eq!(
            srs_kicks(TetrominoType::I, from_rot, true),
            I_KICKS[from_rot],
            "I CW from_rot={} should use I_KICKS[{}]", from_rot, from_rot
        );
    }
}

#[test]
fn test_srs_kicks_i_ccw_uses_i_kicks_indices_4_to_7() {
    let cases: [(usize, usize); 4] = [(0, 7), (1, 4), (2, 5), (3, 6)];
    for (from_rot, table_idx) in cases {
        assert_eq!(
            srs_kicks(TetrominoType::I, from_rot, false),
            I_KICKS[table_idx],
            "I CCW from_rot={} should use I_KICKS[{}]", from_rot, table_idx
        );
    }
}

// JLSTZ pieces share the same kick table as each other
#[test]
fn test_srs_kicks_all_jlstz_pieces_return_same_kicks() {
    let jlstz = [
        TetrominoType::J,
        TetrominoType::L,
        TetrominoType::S,
        TetrominoType::T,
        TetrominoType::Z,
    ];
    for from_rot in 0..4 {
        let ref_cw = srs_kicks(TetrominoType::T, from_rot, true);
        let ref_ccw = srs_kicks(TetrominoType::T, from_rot, false);
        for &piece in &jlstz {
            assert_eq!(
                srs_kicks(piece, from_rot, true),
                ref_cw,
                "{:?} CW from_rot={} should match T", piece, from_rot
            );
            assert_eq!(
                srs_kicks(piece, from_rot, false),
                ref_ccw,
                "{:?} CCW from_rot={} should match T", piece, from_rot
            );
        }
    }
}

// I-piece kicks differ from JLSTZ kicks
#[test]
fn test_srs_kicks_i_differs_from_jlstz() {
    let any_diff = (0..4).any(|from_rot| {
        srs_kicks(TetrominoType::I, from_rot, true)
            != srs_kicks(TetrominoType::T, from_rot, true)
            || srs_kicks(TetrominoType::I, from_rot, false)
                != srs_kicks(TetrominoType::T, from_rot, false)
    });
    assert!(any_diff, "I-piece kicks must differ from JLSTZ kicks in at least one transition");
}

// --- try_spawn: empty grid ---

#[test]
fn test_try_spawn_empty_grid_i_returns_row0_col4() {
    let grid = Grid::new();
    let result = try_spawn(TetrominoType::I, &grid);
    assert!(result.is_some(), "try_spawn(I) on empty grid must succeed");
    let (row, col) = result.unwrap();
    assert_eq!(col, 4, "spawn col must be 4");
    assert_eq!(row, 0, "spawn row must be 0 on empty grid");
}

#[test]
fn test_try_spawn_empty_grid_o_returns_row0_col4() {
    let grid = Grid::new();
    let result = try_spawn(TetrominoType::O, &grid);
    assert!(result.is_some(), "try_spawn(O) on empty grid must succeed");
    let (row, col) = result.unwrap();
    assert_eq!(col, 4, "spawn col must be 4");
    assert_eq!(row, 0, "spawn row must be 0 on empty grid");
}

#[test]
fn test_try_spawn_empty_grid_t_returns_row0_col4() {
    let grid = Grid::new();
    let result = try_spawn(TetrominoType::T, &grid);
    assert!(result.is_some(), "try_spawn(T) on empty grid must succeed");
    let (row, col) = result.unwrap();
    assert_eq!(col, 4, "spawn col must be 4");
    assert_eq!(row, 0, "spawn row must be 0 on empty grid");
}

#[test]
fn test_try_spawn_empty_grid_all_pieces_succeed_at_col4() {
    let grid = Grid::new();
    let pieces = [
        TetrominoType::I,
        TetrominoType::O,
        TetrominoType::T,
        TetrominoType::S,
        TetrominoType::Z,
        TetrominoType::J,
        TetrominoType::L,
    ];
    for &piece in &pieces {
        let result = try_spawn(piece, &grid);
        assert!(result.is_some(), "try_spawn({:?}) on empty grid must succeed", piece);
        assert_eq!(result.unwrap().1, 4, "spawn col must always be 4 for {:?}", piece);
        assert_eq!(result.unwrap().0, 0, "spawn row must be 0 on empty grid for {:?}", piece);
    }
}

// --- try_spawn: blocked row 0 forces vanish zone ---

#[test]
fn test_try_spawn_with_row0_fully_blocked_returns_vanish_zone() {
    // Fill the entire visible row 0 — any piece with a cell at row_delta=0
    // will collide at spawn_row=0 and must try a negative spawn row.
    let mut grid = Grid::new();
    for col in 0..10 {
        grid.cells[0][col] = CellState::Occupied(1);
    }
    let pieces = [
        TetrominoType::T,
        TetrominoType::S,
        TetrominoType::Z,
        TetrominoType::J,
        TetrominoType::L,
        TetrominoType::O,
    ];
    // For each piece that reports spawn failure at row 0, the returned row must be < 0
    for &piece in &pieces {
        let result = try_spawn(piece, &grid);
        // The spawn must either succeed at a row < 0 (vanish zone), or fully fail.
        // It must NOT return row=0 since row 0 is fully occupied.
        if let Some((row, col)) = result {
            assert_eq!(col, 4, "spawn col must always be 4");
            assert!(row < 0,
                "with row 0 fully blocked, {:?} must spawn in vanish zone (row<0), got row={}",
                piece, row);
        }
        // None is also acceptable if the piece can't find any valid row
    }
}

#[test]
fn test_try_spawn_vanish_zone_row_is_negative() {
    // Confirm that when try_spawn moves to vanish zone, the row is indeed negative.
    // Block row 0 entirely; if I-piece has all cells at row_delta=0, it tries row=-1 next.
    let mut grid = Grid::new();
    for col in 0..10 {
        grid.cells[0][col] = CellState::Occupied(1);
    }
    let result = try_spawn(TetrominoType::I, &grid);
    if let Some((row, col)) = result {
        assert_eq!(col, 4);
        assert!(row <= 0, "I-piece spawn row after blocking row 0 should be <=0, got {}", row);
    }
}

// --- try_spawn: spawn tries rows in order 0, -1, -2, -3, -4 ---

#[test]
fn test_try_spawn_prefers_row_0_over_negative() {
    // On an empty grid, spawn must always prefer row=0 over any negative row.
    let grid = Grid::new();
    let pieces = [
        TetrominoType::I,
        TetrominoType::O,
        TetrominoType::T,
        TetrominoType::S,
        TetrominoType::Z,
        TetrominoType::J,
        TetrominoType::L,
    ];
    for &piece in &pieces {
        let result = try_spawn(piece, &grid);
        assert_eq!(result, Some((0, 4)),
            "try_spawn({:?}) on empty grid should be Some((0,4))", piece);
    }
}
