use crate::grid::{CellState, Grid, HEIGHT, WIDTH};
use crate::pieces::{piece_cells, srs_kicks, TetrominoType};

// --- Game State Machine ---

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameState {
    Menu,
    Playing,
    Paused,
    GameOver,
}

impl Default for GameState {
    fn default() -> Self {
        GameState::Menu
    }
}

impl GameState {
    pub fn transition(&self, target: GameState) -> bool {
        matches!(
            (self, target),
            (GameState::Menu, GameState::Playing)
                | (GameState::Playing, GameState::Paused)
                | (GameState::Paused, GameState::Playing)
                | (GameState::Playing, GameState::GameOver)
                | (GameState::GameOver, GameState::Menu)
        )
    }
}

pub fn is_valid_position(grid: &Grid, cells: &[(i32, i32)], row: i32, col: i32) -> bool {
    for &(dr, dc) in cells {
        let r = row + dr;
        let c = col + dc;
        if r < 0 || c < 0 || r as usize >= HEIGHT || c as usize >= WIDTH {
            return false;
        }
        if grid.cells[r as usize][c as usize] != CellState::Empty {
            return false;
        }
    }
    true
}

// --- Active Piece ---

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActivePiece {
    pub piece_type: TetrominoType,
    pub rotation: usize,
    pub row: i32,
    pub col: i32,
}

pub fn move_horizontal(grid: &Grid, piece: &mut ActivePiece, delta: i32) -> bool {
    let cells = piece_cells(piece.piece_type, piece.rotation);
    let new_col = piece.col + delta;
    if is_valid_position(grid, &cells, piece.row, new_col) {
        piece.col = new_col;
        true
    } else {
        false
    }
}

pub fn move_down(grid: &Grid, piece: &mut ActivePiece) -> bool {
    let cells = piece_cells(piece.piece_type, piece.rotation);
    let new_row = piece.row + 1;
    if is_valid_position(grid, &cells, new_row, piece.col) {
        piece.row = new_row;
        true
    } else {
        false
    }
}

pub fn hard_drop(grid: &mut Grid, piece: &ActivePiece) -> u32 {
    let cells = piece_cells(piece.piece_type, piece.rotation);
    let mut landed_row = piece.row;
    loop {
        let next = landed_row + 1;
        if is_valid_position(grid, &cells, next, piece.col) {
            landed_row = next;
        } else {
            break;
        }
    }
    // Pre-clear any already-full rows before writing the piece cells.
    // This prevents the piece from being shifted by clears of pre-existing full lines.
    let pre_cleared = clear_lines(grid);
    let landed = ActivePiece { row: landed_row, ..*piece };
    pre_cleared + lock_piece(grid, &landed)
}

pub fn rotate(grid: &Grid, piece: &mut ActivePiece, clockwise: bool) -> bool {
    let new_rotation = if clockwise {
        (piece.rotation + 1) % 4
    } else {
        (piece.rotation + 3) % 4
    };
    let cells = piece_cells(piece.piece_type, new_rotation);

    // Try unshifted first
    if is_valid_position(grid, &cells, piece.row, piece.col) {
        piece.rotation = new_rotation;
        return true;
    }

    // Try SRS kicks
    let kicks = srs_kicks(piece.piece_type, piece.rotation, clockwise);
    for k in &kicks {
        let test_col = piece.col + k.0;
        let test_row = piece.row + k.1;
        if is_valid_position(grid, &cells, test_row, test_col) {
            piece.rotation = new_rotation;
            piece.col = test_col;
            piece.row = test_row;
            return true;
        }
    }

    false
}

pub fn lock_piece(grid: &mut Grid, piece: &ActivePiece) -> u32 {
    let cells = piece_cells(piece.piece_type, piece.rotation);
    for &(dr, dc) in &cells {
        let r = piece.row + dr;
        let c = piece.col + dc;
        if r >= 0 && r < HEIGHT as i32 && c >= 0 && c < WIDTH as i32 {
            grid.cells[r as usize][c as usize] = CellState::Occupied(piece.piece_type as u32);
        }
    }
    clear_lines(grid)
}

pub fn clear_lines(grid: &mut Grid) -> u32 {
    let mut cleared = 0u32;
    let mut row = HEIGHT as i32 - 1;
    while row >= 0 {
        let full = grid.cells[row as usize].iter().all(|&c| c != CellState::Empty);
        if full {
            // Shift all rows above down by one
            for r in (1..=row as usize).rev() {
                grid.cells[r] = grid.cells[r - 1];
            }
            grid.cells[0] = [CellState::Empty; WIDTH];
            cleared += 1;
            // Don't decrement row — recheck same index (now has the row that was above)
        } else {
            row -= 1;
        }
    }
    cleared
}

// --- Level and Score ---

pub const LINES_PER_LEVEL: u32 = 10;
pub const STARTING_LEVEL: u32 = 1;
pub const SCORE_SINGLE: u32 = 100;
pub const SCORE_DOUBLE: u32 = 300;
pub const SCORE_TRIPLE: u32 = 500;
pub const SCORE_TETRIS: u32 = 800;

pub fn level_for_lines(lines_cleared: u32) -> u32 {
    STARTING_LEVEL + lines_cleared / LINES_PER_LEVEL
}

pub fn score_for_lines(lines: u32, level: u32) -> u32 {
    let base = match lines {
        1 => SCORE_SINGLE,
        2 => SCORE_DOUBLE,
        3 => SCORE_TRIPLE,
        4 => SCORE_TETRIS,
        _ => return 0,
    };
    base * level
}

// --- Escalation ---

pub const DANGER_THRESHOLD_ROW: usize = 4;

#[derive(Debug, PartialEq)]
pub enum EscalationStage {
    Normal,
    Danger,
}

pub fn escalation_stage(grid: &Grid) -> EscalationStage {
    for row in 0..DANGER_THRESHOLD_ROW {
        for col in 0..WIDTH {
            if grid.cells[row][col] != CellState::Empty {
                return EscalationStage::Danger;
            }
        }
    }
    EscalationStage::Normal
}

// --- Gravity ---

pub fn gravity_interval_ms(level: u32) -> u64 {
    let level = level.max(1);
    let interval = 1000u64.saturating_sub((level as u64 - 1) * 100);
    interval.max(100)
}

pub fn gravity_tick(
    grid: &Grid,
    piece: &mut ActivePiece,
    accumulated_ms: u64,
    level: u32,
) -> (bool, u64) {
    let interval = gravity_interval_ms(level);
    if accumulated_ms >= interval && move_down(grid, piece) {
        (true, 0)
    } else {
        (false, accumulated_ms)
    }
}

// --- Game Over Detection ---

pub fn is_game_over(grid: &Grid, piece: &ActivePiece) -> bool {
    let cells = piece_cells(piece.piece_type, piece.rotation);
    !is_valid_position(grid, &cells, piece.row, piece.col)
}

// --- 7-Bag Piece Randomizer ---

pub struct PieceBag {
    bag: [usize; 7],
    index: usize,
}

fn shuffle_bag(bag: &mut [usize; 7]) {
    // Fisher-Yates with a fixed-seed LCG for determinism
    let mut state: u64 = 0x123456789abcdef0;
    for i in (1..7usize).rev() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let j = (state >> 33) as usize % (i + 1);
        bag.swap(i, j);
    }
}

impl PieceBag {
    pub fn new() -> Self {
        let mut bag = [0, 1, 2, 3, 4, 5, 6];
        shuffle_bag(&mut bag);
        PieceBag { bag, index: 0 }
    }

    pub fn next(&mut self) -> usize {
        if self.index >= 7 {
            self.bag = [0, 1, 2, 3, 4, 5, 6];
            shuffle_bag(&mut self.bag);
            self.index = 0;
        }
        let piece = self.bag[self.index];
        self.index += 1;
        piece
    }

    pub fn peek(&self) -> usize {
        if self.index < 7 {
            self.bag[self.index]
        } else {
            // Bag is exhausted; next() would refill. Peek returns first of a fresh bag.
            // Since shuffle is deterministic with a fixed seed, this is always consistent.
            let mut fresh = [0usize, 1, 2, 3, 4, 5, 6];
            shuffle_bag(&mut fresh);
            fresh[0]
        }
    }
}
