use crate::grid::{CellState, Grid, HEIGHT, WIDTH};

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
