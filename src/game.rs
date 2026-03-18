use crate::grid::{CellState, GRID_HEIGHT, GRID_WIDTH};
use crate::pieces::{TetrominoType, ALL_TETROMINOES, SPAWN_COL, SPAWN_ROW};

const W: usize = GRID_WIDTH as usize;
const H: usize = GRID_HEIGHT as usize;

/// Game state machine. Valid transitions:
///   Menu -> Playing (start)
///   Playing -> Paused (pause)
///   Paused -> Playing (resume)
///   Playing -> GameOver (spawn collision)
///   GameOver -> Menu (restart)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    Menu,
    Playing,
    Paused,
    GameOver,
}

/// The active falling piece.
#[derive(Debug, Clone)]
pub struct Piece {
    pub piece_type: TetrominoType,
    pub col: i32,
    pub row: i32,
    pub rotation: u32,
}

pub struct Game {
    pub game_state: GameState,
    pub grid: [[CellState; W]; H],
    pub current_piece: Option<Piece>,
    /// The next piece that will spawn after the current one locks.
    pub next_piece: TetrominoType,
    /// Total lines cleared since game start.
    pub total_lines_cleared: usize,
    /// Lines cleared in the most recent lock.
    last_clear_count: usize,
    /// 7-bag: remaining pieces in the current bag, drawn from the end.
    bag: Vec<TetrominoType>,
    rng: u64,
    /// Last locked piece type and position, for test introspection.
    last_locked: Option<(TetrominoType, i32, i32)>,
}

// xorshift64 PRNG for bag shuffling.
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn shuffle_bag(rng: &mut u64, items: &mut Vec<TetrominoType>) {
    let n = items.len();
    for i in (1..n).rev() {
        let j = (xorshift64(rng) as usize) % (i + 1);
        items.swap(i, j);
    }
}

impl Game {
    pub fn new() -> Self {
        Self::with_seed(7)
    }

    pub fn with_seed(seed: u64) -> Self {
        let mut rng = if seed == 0 { 1 } else { seed };
        let mut bag: Vec<TetrominoType> = ALL_TETROMINOES.to_vec();
        shuffle_bag(&mut rng, &mut bag);
        let next_piece = bag.pop().unwrap();
        Game {
            game_state: GameState::Menu,
            grid: [[CellState::Empty; W]; H],
            current_piece: None,
            next_piece,
            total_lines_cleared: 0,
            last_clear_count: 0,
            bag,
            rng,
            last_locked: None,
        }
    }

    // --- State transitions ---

    /// Transition: Menu -> Playing. Spawns the first piece.
    pub fn start(&mut self) {
        if self.game_state == GameState::Menu {
            self.game_state = GameState::Playing;
            self.spawn_next_piece();
        }
    }

    /// Transition: Playing -> Paused.
    pub fn pause(&mut self) {
        if self.game_state == GameState::Playing {
            self.game_state = GameState::Paused;
        }
    }

    /// Transition: Paused -> Playing.
    pub fn resume(&mut self) {
        if self.game_state == GameState::Paused {
            self.game_state = GameState::Playing;
        }
    }

    /// Transition: GameOver -> Menu. Resets the grid and piece state.
    pub fn restart(&mut self) {
        if self.game_state == GameState::GameOver {
            self.game_state = GameState::Menu;
            self.grid = [[CellState::Empty; W]; H];
            self.current_piece = None;
            self.last_locked = None;
            self.total_lines_cleared = 0;
            self.last_clear_count = 0;
            self.bag = ALL_TETROMINOES.to_vec();
            shuffle_bag(&mut self.rng, &mut self.bag);
            self.next_piece = self.bag.pop().unwrap();
        }
    }

    // --- Collision ---

    /// Returns true if placing `pt` at (`col`, `row`) with `rotation` is within bounds
    /// and does not overlap any occupied cell.
    pub fn is_valid_position(&self, pt: TetrominoType, col: i32, row: i32, rotation: u32) -> bool {
        for (dc, dr) in pt.cells_rotated(rotation) {
            let c = col + dc;
            let r = row + dr;
            if c < 0 || c >= GRID_WIDTH as i32 || r < 0 || r >= GRID_HEIGHT as i32 {
                return false;
            }
            if self.grid[r as usize][c as usize] != CellState::Empty {
                return false;
            }
        }
        true
    }

    // --- Movement ---

    /// Move the current piece left one column. Returns true if successful.
    pub fn move_left(&mut self) -> bool {
        if self.game_state != GameState::Playing {
            return false;
        }
        let (pt, col, row, rotation) = match &self.current_piece {
            Some(p) => (p.piece_type, p.col, p.row, p.rotation),
            None => return false,
        };
        if self.is_valid_position(pt, col - 1, row, rotation) {
            self.current_piece.as_mut().unwrap().col -= 1;
            true
        } else {
            false
        }
    }

    /// Move the current piece right one column. Returns true if successful.
    pub fn move_right(&mut self) -> bool {
        if self.game_state != GameState::Playing {
            return false;
        }
        let (pt, col, row, rotation) = match &self.current_piece {
            Some(p) => (p.piece_type, p.col, p.row, p.rotation),
            None => return false,
        };
        if self.is_valid_position(pt, col + 1, row, rotation) {
            self.current_piece.as_mut().unwrap().col += 1;
            true
        } else {
            false
        }
    }

    /// Soft drop: move the current piece down one row.
    /// Returns true if the piece moved, false if the piece locked in place.
    /// After locking, current_piece remains at the lock position (no auto-spawn).
    pub fn move_down(&mut self) -> bool {
        if self.game_state != GameState::Playing {
            return false;
        }
        let (pt, col, row, rotation) = match &self.current_piece {
            Some(p) => (p.piece_type, p.col, p.row, p.rotation),
            None => return false,
        };
        if self.is_valid_position(pt, col, row + 1, rotation) {
            self.current_piece.as_mut().unwrap().row += 1;
            true
        } else {
            self.lock_cells(pt, col, row, rotation);
            false
        }
    }

    /// Hard drop: instantly move the piece to the lowest valid row, lock it, and spawn next piece.
    pub fn hard_drop(&mut self) {
        if self.game_state != GameState::Playing {
            return;
        }
        let (pt, col, mut row, rotation) = match &self.current_piece {
            Some(p) => (p.piece_type, p.col, p.row, p.rotation),
            None => return,
        };
        while self.is_valid_position(pt, col, row + 1, rotation) {
            row += 1;
        }
        self.lock_cells(pt, col, row, rotation);
        self.current_piece = None;
        if self.game_state == GameState::Playing {
            self.spawn_next_piece();
        }
    }

    // --- Rotation ---

    /// Rotate the current piece clockwise using SRS wall kicks.
    /// Returns true if the rotation succeeded.
    pub fn rotate_cw(&mut self) -> bool {
        self.try_rotate(true)
    }

    /// Rotate the current piece counter-clockwise using SRS wall kicks.
    /// Returns true if the rotation succeeded.
    pub fn rotate_ccw(&mut self) -> bool {
        self.try_rotate(false)
    }

    fn try_rotate(&mut self, clockwise: bool) -> bool {
        if self.game_state != GameState::Playing {
            return false;
        }
        let (pt, col, row, from) = match &self.current_piece {
            Some(p) => (p.piece_type, p.col, p.row, p.rotation % 4),
            None => return false,
        };
        let to = if clockwise { (from + 1) % 4 } else { (from + 3) % 4 };
        // First try the unshifted rotation position
        if self.is_valid_position(pt, col, row, to) {
            let p = self.current_piece.as_mut().unwrap();
            p.rotation = to;
            return true;
        }
        // Then try each wall kick offset
        for &(dc, dr) in pt.srs_kicks(from, to) {
            if self.is_valid_position(pt, col + dc, row + dr, to) {
                let p = self.current_piece.as_mut().unwrap();
                p.col += dc;
                p.row += dr;
                p.rotation = to;
                return true;
            }
        }
        false
    }

    // --- Accessors ---

    /// Returns the current game state.
    pub fn state(&self) -> GameState {
        self.game_state
    }

    /// Force transition to GameOver (for testing).
    pub fn force_game_over(&mut self) {
        self.game_state = GameState::GameOver;
    }

    /// Alias for `is_valid_position`.
    pub fn is_position_valid(&self, pt: TetrominoType, col: i32, row: i32, rotation: u32) -> bool {
        self.is_valid_position(pt, col, row, rotation)
    }

    /// Returns the cell state at (col, row).
    pub fn cell_at(&self, col: usize, row: usize) -> CellState {
        self.grid[row][col]
    }

    /// Returns the current piece's TetrominoType, or panics if none.
    pub fn current_piece(&self) -> TetrominoType {
        self.current_piece.as_ref().expect("no current piece").piece_type
    }

    /// Returns the current piece's column.
    pub fn current_col(&self) -> i32 {
        self.current_piece.as_ref().expect("no current piece").col
    }

    /// Returns the current piece's row.
    pub fn current_row(&self) -> i32 {
        self.current_piece.as_ref().expect("no current piece").row
    }

    /// Returns the current piece's rotation state.
    pub fn current_rotation(&self) -> u32 {
        self.current_piece.as_ref().expect("no current piece").rotation as u32
    }

    /// Returns the next piece type (preview).
    pub fn peek_next(&self) -> TetrominoType {
        self.next_piece
    }

    /// Returns the total lines cleared since game start.
    pub fn lines_cleared(&self) -> usize {
        self.total_lines_cleared
    }

    /// Returns the number of lines cleared in the most recent lock.
    pub fn last_clear_count(&self) -> usize {
        self.last_clear_count
    }

    /// Returns the current level (1-based). Increases every 10 lines cleared.
    pub fn level(&self) -> usize {
        (self.total_lines_cleared / 10) + 1
    }

    /// Returns the last locked piece type.
    pub fn last_locked_piece(&self) -> TetrominoType {
        self.last_locked.expect("no piece has been locked").0
    }

    /// Returns the (col, row) where the last piece was locked.
    pub fn last_lock_position(&self) -> (i32, i32) {
        let (_, col, row) = self.last_locked.expect("no piece has been locked");
        (col, row)
    }

    // --- Internal helpers ---

    /// Write the piece cells into the grid, clear completed lines, and record the lock position.
    /// Does NOT clear current_piece or spawn next piece.
    /// Cells above the visible grid (row < 0) are silently skipped.
    fn lock_cells(&mut self, pt: TetrominoType, col: i32, row: i32, rotation: u32) {
        let color = pt.index() as u32;
        for (dc, dr) in pt.cells_rotated(rotation) {
            let c = col + dc;
            let r = row + dr;
            if r >= 0 && (r as usize) < H && c >= 0 && (c as usize) < W {
                self.grid[r as usize][c as usize] = CellState::Occupied(color);
            }
        }
        self.last_locked = Some((pt, col, row));
        let cleared = self.clear_lines();
        self.last_clear_count = cleared;
        self.total_lines_cleared += cleared;
    }

    /// Remove fully occupied rows and shift everything above down.
    /// Returns the number of rows cleared.
    fn clear_lines(&mut self) -> usize {
        let mut dst: isize = (H as isize) - 1;
        let mut cleared = 0usize;
        for src in (0..H).rev() {
            let full = self.grid[src].iter().all(|c| *c != CellState::Empty);
            if full {
                cleared += 1;
            } else {
                if dst != src as isize {
                    self.grid[dst as usize] = self.grid[src];
                }
                dst -= 1;
            }
        }
        // Fill remaining top rows with empty
        while dst >= 0 {
            self.grid[dst as usize] = [CellState::Empty; W];
            dst -= 1;
        }
        cleared
    }

    /// Spawn `next_piece` as the current piece and draw a new `next_piece` from the bag.
    /// Tries spawn at SPAWN_ROW first; if blocked, tries up to 4 rows above (vanish zone).
    /// Transitions to GameOver only if all attempts fail.
    fn spawn_next_piece(&mut self) {
        let pt = self.next_piece;
        self.next_piece = self.draw_from_bag();
        // Try spawn at standard row, then progressively higher (vanish zone above grid)
        for offset in 0..5 {
            let try_row = SPAWN_ROW - offset;
            if self.is_spawn_valid(pt, SPAWN_COL, try_row, 0) {
                self.current_piece = Some(Piece {
                    piece_type: pt,
                    col: SPAWN_COL,
                    row: try_row,
                    rotation: 0,
                });
                return;
            }
        }
        self.game_state = GameState::GameOver;
        self.current_piece = None;
    }

    /// Like is_valid_position but allows cells above the visible grid (row < 0).
    /// Used only for spawn validation to support the vanish zone.
    fn is_spawn_valid(&self, pt: TetrominoType, col: i32, row: i32, rotation: u32) -> bool {
        for (dc, dr) in pt.cells_rotated(rotation) {
            let c = col + dc;
            let r = row + dr;
            if c < 0 || c >= GRID_WIDTH as i32 || r >= GRID_HEIGHT as i32 {
                return false;
            }
            // Cells above the grid are always valid (vanish zone)
            if r >= 0 && self.grid[r as usize][c as usize] != CellState::Empty {
                return false;
            }
        }
        true
    }

    /// Draw the next piece type from the bag, refilling with a fresh shuffled bag if exhausted.
    fn draw_from_bag(&mut self) -> TetrominoType {
        if self.bag.is_empty() {
            self.bag = ALL_TETROMINOES.to_vec();
            shuffle_bag(&mut self.rng, &mut self.bag);
        }
        self.bag.pop().unwrap()
    }
}
