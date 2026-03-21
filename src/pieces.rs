// Pieces subsystem: tetromino types, rotation states, SRS wall kicks, spawn logic.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TetrominoType {
    I = 0,
    O = 1,
    T = 2,
    S = 3,
    Z = 4,
    J = 5,
    L = 6,
}

/// Rotation table: PIECE_CELLS[piece as usize][rotation][cell_idx] = (row_delta, col_delta).
/// Rotation states: 0=spawn, 1=90°CW, 2=180°, 3=270°CW.
/// CW transform: (r, c) -> (c, -r).
pub static PIECE_CELLS: [[[(i32, i32); 4]; 4]; 7] = [
    // I (idx 0)
    [
        [(0, -1), (0,  0), (0,  1), (0,  2)], // rot 0
        [(-1, 0), (0,  0), (1,  0), (2,  0)], // rot 1
        [(0, -2), (0, -1), (0,  0), (0,  1)], // rot 2
        [(-2, 0), (-1, 0), (0,  0), (1,  0)], // rot 3
    ],
    // O (idx 1) — all rotations identical
    [
        [(0, 0), (0, 1), (1, 0), (1, 1)],
        [(0, 0), (0, 1), (1, 0), (1, 1)],
        [(0, 0), (0, 1), (1, 0), (1, 1)],
        [(0, 0), (0, 1), (1, 0), (1, 1)],
    ],
    // T (idx 2)
    [
        [(-1,  0), (0, -1), (0,  0), (0,  1)], // rot 0
        [(-1,  0), (0,  0), (0,  1), (1,  0)], // rot 1
        [(0, -1), (0,  0), (0,  1), (1,  0)],  // rot 2
        [(-1,  0), (0, -1), (0,  0), (1,  0)], // rot 3
    ],
    // S (idx 3)
    [
        [(-1,  0), (-1,  1), (0, -1), (0,  0)], // rot 0
        [(-1,  0), (0,  0), (0,  1), (1,  1)],  // rot 1
        [(0,  0), (0,  1), (1, -1), (1,  0)],   // rot 2
        [(-1, -1), (0, -1), (0,  0), (1,  0)],  // rot 3
    ],
    // Z (idx 4)
    [
        [(-1, -1), (-1,  0), (0,  0), (0,  1)], // rot 0
        [(-1,  1), (0,  0), (0,  1), (1,  0)],  // rot 1
        [(0, -1), (0,  0), (1,  0), (1,  1)],   // rot 2
        [(-1,  0), (0, -1), (0,  0), (1, -1)],  // rot 3
    ],
    // J (idx 5)
    [
        [(-1, -1), (0, -1), (0,  0), (0,  1)],  // rot 0
        [(-1,  0), (-1,  1), (0,  0), (1,  0)], // rot 1
        [(0, -1), (0,  0), (0,  1), (1,  1)],   // rot 2
        [(-1,  0), (0,  0), (1, -1), (1,  0)],  // rot 3
    ],
    // L (idx 6)
    [
        [(-1,  1), (0, -1), (0,  0), (0,  1)],  // rot 0
        [(-1,  0), (0,  0), (1,  0), (1,  1)],  // rot 1
        [(0, -1), (0,  0), (0,  1), (1, -1)],   // rot 2
        [(-1, -1), (-1,  0), (0,  0), (1,  0)], // rot 3
    ],
];

/// Returns the 4 (row_delta, col_delta) offsets for the given piece and rotation state (0–3).
pub fn piece_cells(piece: TetrominoType, rotation: usize) -> [(i32, i32); 4] {
    PIECE_CELLS[piece as usize][rotation]
}

/// SRS wall kick offsets for JLSTZ pieces.
/// Indexed by transition: CW at 0–3 (0→1, 1→2, 2→3, 3→0), CCW at 4–7 (1→0, 2→1, 3→2, 0→3).
/// Each offset is (col_delta, row_delta).
pub static JLSTZ_KICKS: [[(i32, i32); 4]; 8] = [
    // CW 0→1
    [(-1,  0), (-1, -1), (0,  2), (-1,  2)],
    // CW 1→2
    [( 1,  0), ( 1,  1), (0, -2), ( 1, -2)],
    // CW 2→3
    [(-1,  0), (-1, -1), (0,  2), (-1,  2)],
    // CW 3→0
    [( 1,  0), ( 1,  1), (0, -2), ( 1, -2)],
    // CCW 1→0
    [( 1,  0), ( 1, -1), (0,  2), ( 1,  2)],
    // CCW 2→1
    [(-1,  0), (-1,  1), (0, -2), (-1, -2)],
    // CCW 3→2
    [( 1,  0), ( 1, -1), (0,  2), ( 1,  2)],
    // CCW 0→3
    [(-1,  0), (-1,  1), (0, -2), (-1, -2)],
];

/// SRS wall kick offsets for the I piece.
/// Indexed by transition: CW at 0–3 (0→1, 1→2, 2→3, 3→0), CCW at 4–7 (1→0, 2→1, 3→2, 0→3).
/// Each offset is (col_delta, row_delta).
pub static I_KICKS: [[(i32, i32); 4]; 8] = [
    // CW 0→1
    [(-2,  0), ( 1,  0), (-2,  1), ( 1, -2)],
    // CW 1→2
    [(-1,  0), ( 2,  0), (-1, -2), ( 2,  1)],
    // CW 2→3
    [( 2,  0), (-1,  0), ( 2, -1), (-1,  2)],
    // CW 3→0
    [( 1,  0), (-2,  0), ( 1,  2), (-2, -1)],
    // CCW 1→0
    [( 2,  0), (-1,  0), ( 2, -1), (-1,  2)],
    // CCW 2→1
    [( 1,  0), (-2,  0), ( 1,  2), (-2, -1)],
    // CCW 3→2
    [(-2,  0), ( 1,  0), (-2,  1), ( 1, -2)],
    // CCW 0→3
    [(-1,  0), ( 2,  0), (-1, -2), ( 2,  1)],
];

/// Returns the 4 SRS kick offsets as (col_delta, row_delta) for the given piece/transition.
/// O-piece always returns [(0, 0); 4].
pub fn srs_kicks(piece: TetrominoType, from_rotation: usize, clockwise: bool) -> [(i32, i32); 4] {
    if piece == TetrominoType::O {
        return [(0, 0); 4];
    }
    let idx = if clockwise {
        from_rotation // 0→1=0, 1→2=1, 2→3=2, 3→0=3
    } else {
        (from_rotation + 3) % 4 + 4 // 1→0=4, 2→1=5, 3→2=6, 0→3=7
    };
    match piece {
        TetrominoType::I => I_KICKS[idx],
        _ => JLSTZ_KICKS[idx],
    }
}

pub const TETROMINO_TYPES: [TetrominoType; 7] = [
    TetrominoType::I,
    TetrominoType::O,
    TetrominoType::T,
    TetrominoType::S,
    TetrominoType::Z,
    TetrominoType::J,
    TetrominoType::L,
];

/// Tries to spawn the piece at top-center of the grid (col=4).
/// Tries spawn rows 0, -1, -2, -3, -4 in order.
/// For each candidate row, only checks cells where row_delta + spawn_row >= 0.
/// Returns Some((row, col)) for the first valid position, or None if all five fail.
pub fn try_spawn(piece: TetrominoType, grid: &crate::grid::Grid) -> Option<(i32, i32)> {
    let spawn_col: i32 = 4;
    let cells = piece_cells(piece, 0);

    for offset in 0..5i32 {
        let spawn_row = -offset;
        let mut valid = true;
        let mut has_visible = false;
        for &(dr, dc) in &cells {
            let r = spawn_row + dr;
            let c = spawn_col + dc;
            // Only check cells that are on the visible grid (r >= 0)
            if r < 0 {
                continue;
            }
            has_visible = true;
            let r_u = r as usize;
            let c_u = c as usize;
            if r_u >= crate::grid::HEIGHT || c_u >= crate::grid::WIDTH {
                valid = false;
                break;
            }
            if grid.cells[r_u][c_u] != crate::grid::CellState::Empty {
                valid = false;
                break;
            }
        }
        if valid && has_visible {
            return Some((spawn_row, spawn_col));
        }
    }
    None
}
