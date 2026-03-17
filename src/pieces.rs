pub const TETROMINO_COUNT: usize = 7;
pub const CELLS_PER_PIECE: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TetrominoType {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl TetrominoType {
    /// Returns the 4 cell offsets (col, row) for this tetromino in spawn orientation.
    pub fn cells(&self) -> [(i32, i32); CELLS_PER_PIECE] {
        match self {
            TetrominoType::I => [(-1, 0), (0, 0), (1, 0), (2, 0)],
            TetrominoType::O => [(0, 0), (1, 0), (0, 1), (1, 1)],
            TetrominoType::T => [(-1, 0), (0, 0), (1, 0), (0, 1)],
            TetrominoType::S => [(-1, 0), (0, 0), (0, 1), (1, 1)],
            TetrominoType::Z => [(0, 0), (1, 0), (-1, 1), (0, 1)],
            TetrominoType::J => [(-1, 0), (0, 0), (1, 0), (-1, 1)],
            TetrominoType::L => [(-1, 0), (0, 0), (1, 0), (1, 1)],
        }
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
