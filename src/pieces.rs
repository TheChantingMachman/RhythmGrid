/// Number of distinct tetromino types.
pub const TETROMINO_COUNT: usize = 7;

/// Number of cells that make up each tetromino.
pub const CELLS_PER_PIECE: usize = 4;

/// All tetromino variants in canonical order.
pub const ALL: [Tetromino; TETROMINO_COUNT] = [
    Tetromino::I,
    Tetromino::O,
    Tetromino::T,
    Tetromino::S,
    Tetromino::Z,
    Tetromino::J,
    Tetromino::L,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tetromino {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl Tetromino {
    /// Returns the 4 cell offsets (col, row) that define this tetromino's shape.
    pub fn cells(self) -> [(i32, i32); CELLS_PER_PIECE] {
        match self {
            Tetromino::I => [(0, 0), (1, 0), (2, 0), (3, 0)],
            Tetromino::O => [(0, 0), (1, 0), (0, 1), (1, 1)],
            Tetromino::T => [(0, 0), (1, 0), (2, 0), (1, 1)],
            Tetromino::S => [(1, 0), (2, 0), (0, 1), (1, 1)],
            Tetromino::Z => [(0, 0), (1, 0), (1, 1), (2, 1)],
            Tetromino::J => [(0, 0), (0, 1), (1, 1), (2, 1)],
            Tetromino::L => [(2, 0), (0, 1), (1, 1), (2, 1)],
        }
    }

    /// Returns all 7 tetromino variants.
    pub fn all() -> [Tetromino; TETROMINO_COUNT] {
        ALL
    }
}
