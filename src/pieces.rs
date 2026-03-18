pub const TETROMINO_COUNT: usize = 7;
pub const CELLS_PER_PIECE: usize = 4;
pub const ROTATION_STATES: usize = 4;
pub const MAX_KICK_TESTS: usize = 4;
pub const SPAWN_COL: i32 = 4;
pub const SPAWN_ROW: i32 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TetrominoType {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

/// All rotation cells indexed by [piece_index][rotation_state][cell_index] = (col, row).
/// Rotation states: 0=spawn, 1=90°CW, 2=180°, 3=270°CW.
/// CW rotation transform: (col, row) -> (row, -col).
/// Piece indices match ALL_TETROMINOES order: I=0, O=1, T=2, S=3, Z=4, J=5, L=6.
static PIECE_CELLS: [[[(i32, i32); 4]; 4]; 7] = [
    // I (0)
    [
        [(-1, 0), (0, 0), (1, 0), (2, 0)],   // R0 horizontal
        [(0, -2), (0, -1), (0, 0), (0, 1)],  // R1 vertical
        [(-2, 0), (-1, 0), (0, 0), (1, 0)],  // R2 horizontal (shifted)
        [(0, -1), (0, 0), (0, 1), (0, 2)],   // R3 vertical (shifted)
    ],
    // O (1) — all states identical (symmetric piece)
    [
        [(0, 0), (1, 0), (0, 1), (1, 1)],
        [(0, 0), (1, 0), (0, 1), (1, 1)],
        [(0, 0), (1, 0), (0, 1), (1, 1)],
        [(0, 0), (1, 0), (0, 1), (1, 1)],
    ],
    // T (2)
    [
        [(-1, 0), (0, 0), (1, 0), (0, 1)],   // R0
        [(0, 1), (0, 0), (0, -1), (1, 0)],   // R1
        [(1, 0), (0, 0), (-1, 0), (0, -1)],  // R2
        [(0, -1), (0, 0), (0, 1), (-1, 0)],  // R3
    ],
    // S (3)
    [
        [(-1, 0), (0, 0), (0, 1), (1, 1)],   // R0
        [(0, 1), (0, 0), (1, 0), (1, -1)],   // R1
        [(1, 0), (0, 0), (0, -1), (-1, -1)], // R2
        [(0, -1), (0, 0), (-1, 0), (-1, 1)], // R3
    ],
    // Z (4)
    [
        [(0, 0), (1, 0), (-1, 1), (0, 1)],   // R0
        [(0, 0), (0, -1), (1, 1), (1, 0)],   // R1
        [(0, 0), (-1, 0), (1, -1), (0, -1)], // R2
        [(0, 0), (0, 1), (-1, -1), (-1, 0)], // R3
    ],
    // J (5)
    [
        [(-1, 0), (0, 0), (1, 0), (-1, 1)],  // R0
        [(0, 1), (0, 0), (0, -1), (1, 1)],   // R1
        [(1, 0), (0, 0), (-1, 0), (1, -1)],  // R2
        [(0, -1), (0, 0), (0, 1), (-1, -1)], // R3
    ],
    // L (6)
    [
        [(-1, 0), (0, 0), (1, 0), (1, 1)],   // R0
        [(0, 1), (0, 0), (0, -1), (1, -1)],  // R1
        [(1, 0), (0, 0), (-1, 0), (-1, -1)], // R2
        [(0, -1), (0, 0), (0, 1), (-1, 1)],  // R3
    ],
];

/// SRS wall kick offsets for JLSTZ pieces, indexed by transition (see kick_index).
/// Each entry has 5 offsets (col_delta, row_delta): the zero-offset attempt plus 4 kicks.
/// Coordinate system: positive col = right, positive row = down.
/// Source: Tetris Guideline JLSTZ table, converted from y-up to y-down.
static JLSTZ_KICKS: [[(i32, i32); 4]; 8] = [
    [(-1, 0), (-1, -1), (0, 2), (-1, 2)],   // [0] 0→1
    [(1, 0), (1, 1), (0, -2), (1, -2)],     // [1] 1→2
    [(1, 0), (1, -1), (0, 2), (1, 2)],      // [2] 2→3
    [(-1, 0), (-1, 1), (0, -2), (-1, -2)],  // [3] 3→0
    [(1, 0), (1, 1), (0, -2), (1, -2)],     // [4] 1→0
    [(-1, 0), (-1, -1), (0, 2), (-1, 2)],   // [5] 2→1
    [(-1, 0), (-1, 1), (0, -2), (-1, -2)],  // [6] 3→2
    [(1, 0), (1, -1), (0, 2), (1, 2)],      // [7] 0→3
];

/// SRS wall kick offsets for the I-piece, indexed by transition (see kick_index).
/// Source: Tetris Guideline I-piece table, converted from y-up to y-down.
static I_KICKS: [[(i32, i32); 4]; 8] = [
    [(-2, 0), (1, 0), (-2, -1), (1, 2)],    // [0] 0→1
    [(-1, 0), (2, 0), (-1, 2), (2, -1)],    // [1] 1→2
    [(2, 0), (-1, 0), (2, 1), (-1, -2)],    // [2] 2→3
    [(1, 0), (-2, 0), (1, -2), (-2, 1)],    // [3] 3→0
    [(2, 0), (-1, 0), (2, 1), (-1, -2)],    // [4] 1→0
    [(1, 0), (-2, 0), (1, -2), (-2, 1)],    // [5] 2→1
    [(-2, 0), (1, 0), (-2, -1), (1, 2)],    // [6] 3→2
    [(-1, 0), (2, 0), (-1, 2), (2, -1)],    // [7] 0→3
];

/// O-piece has no kicks: rotation is a no-op on collision.
static O_KICKS: [(i32, i32); 0] = [];

fn kick_index(from: u32, to: u32) -> usize {
    match (from % 4, to % 4) {
        (0, 1) => 0,
        (1, 2) => 1,
        (2, 3) => 2,
        (3, 0) => 3,
        (1, 0) => 4,
        (2, 1) => 5,
        (3, 2) => 6,
        (0, 3) => 7,
        _ => 0,
    }
}

impl TetrominoType {
    /// Returns the zero-based index of this tetromino type, matching ALL_TETROMINOES order.
    pub fn index(&self) -> usize {
        match self {
            TetrominoType::I => 0,
            TetrominoType::O => 1,
            TetrominoType::T => 2,
            TetrominoType::S => 3,
            TetrominoType::Z => 4,
            TetrominoType::J => 5,
            TetrominoType::L => 6,
        }
    }

    /// Returns the 4 cell offsets (col, row) for this tetromino in its spawn orientation (rotation 0).
    pub fn cells(&self) -> [(i32, i32); CELLS_PER_PIECE] {
        self.cells_rotated(0)
    }

    /// Returns the 4 cell offsets (col, row) for this tetromino at the given rotation state (0–3).
    pub fn cells_rotated(&self, rotation: u32) -> [(i32, i32); CELLS_PER_PIECE] {
        PIECE_CELLS[self.index()][(rotation % 4) as usize]
    }

    /// Returns the SRS wall kick offsets to try when rotating from `from` to `to` state.
    /// Includes the zero-offset attempt as the first entry, followed by up to 4 kick offsets.
    /// O-piece returns only the zero-offset entry (no kicks).
    pub fn srs_kicks(&self, from: u32, to: u32) -> &'static [(i32, i32)] {
        match self {
            TetrominoType::O => &O_KICKS,
            TetrominoType::I => &I_KICKS[kick_index(from, to)],
            _ => &JLSTZ_KICKS[kick_index(from, to)],
        }
    }

    /// Alias for `srs_kicks`.
    pub fn kick_offsets(&self, from: u32, to: u32) -> &'static [(i32, i32)] {
        self.srs_kicks(from, to)
    }
}

pub const ALL_TETROMINOES: [TetrominoType; TETROMINO_COUNT] = [
    TetrominoType::I,
    TetrominoType::O,
    TetrominoType::T,
    TetrominoType::S,
    TetrominoType::Z,
    TetrominoType::J,
    TetrominoType::L,
];
