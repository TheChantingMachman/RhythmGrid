// Render state derivation — pipeline-owned testable logic.
// Actual draw calls live in main.rs (co-authored).

use crate::game::{is_valid_position, level_for_lines, ActivePiece, GameSession};
use crate::grid::{CellState, Grid};
use crate::pieces::{piece_cells, PIECE_CELLS};

pub use crate::game::GameState;

// --- Render State Types ---

#[derive(Debug, Clone, PartialEq)]
pub struct RenderCell {
    pub row: i32,
    pub col: i32,
    pub type_index: u32,
}

#[derive(Debug, PartialEq)]
pub struct BoardRenderState {
    pub occupied: Vec<RenderCell>,
    pub active: Vec<RenderCell>,
    pub ghost: Vec<RenderCell>,
}

#[derive(Debug, PartialEq)]
pub struct HeldPieceRender {
    pub type_index: u32,
    pub cells: [(i32, i32); 4],
}

#[derive(Debug, PartialEq)]
pub struct GameStatusRender {
    pub score: u32,
    pub level: u32,
    pub total_lines: u32,
    pub combo_count: u32,
    pub max_combo: u32,
    pub pieces_placed: u32,
    pub time_played_secs: f64,
    pub state: GameState,
    pub can_hold: bool,
}

// --- Render State Functions ---

pub fn board_state(session: &GameSession) -> BoardRenderState {
    let mut occupied = Vec::new();
    for row in 0..crate::grid::HEIGHT {
        for col in 0..crate::grid::WIDTH {
            if let CellState::Occupied(type_index) = session.grid.cells[row][col] {
                if row as i32 >= 0 {
                    occupied.push(RenderCell { row: row as i32, col: col as i32, type_index });
                }
            }
        }
    }

    let active_cells = piece_cells(session.active_piece.piece_type, session.active_piece.rotation);

    let mut active = Vec::new();
    for &(dr, dc) in &active_cells {
        let r = session.active_piece.row + dr;
        let c = session.active_piece.col + dc;
        if r >= 0 {
            active.push(RenderCell {
                row: r,
                col: c,
                type_index: session.active_piece.piece_type as u32,
            });
        }
    }

    // Compute ghost row
    let mut ghost_row = session.active_piece.row;
    loop {
        let next_row = ghost_row + 1;
        if is_valid_position(&session.grid, &active_cells, next_row, session.active_piece.col) {
            ghost_row = next_row;
        } else {
            break;
        }
    }

    let mut ghost = Vec::new();
    for &(dr, dc) in &active_cells {
        let r = ghost_row + dr;
        let c = session.active_piece.col + dc;
        if r >= 0 {
            ghost.push(RenderCell {
                row: r,
                col: c,
                type_index: session.active_piece.piece_type as u32,
            });
        }
    }

    BoardRenderState { occupied, active, ghost }
}

pub fn held_piece_state(session: &GameSession) -> Option<HeldPieceRender> {
    let held_type = session.held_piece?;
    let type_index = held_type as u32;
    let cells = PIECE_CELLS[type_index as usize][0];
    Some(HeldPieceRender { type_index, cells })
}

pub fn game_status(session: &GameSession) -> GameStatusRender {
    GameStatusRender {
        score: session.score,
        level: level_for_lines(session.total_lines),
        total_lines: session.total_lines,
        combo_count: session.combo_count,
        max_combo: session.max_combo,
        pieces_placed: session.pieces_placed,
        time_played_secs: session.time_played_secs,
        state: session.state,
        can_hold: session.can_hold,
    }
}

pub const CELL_SIZE: u32 = 30;
pub const BOARD_WIDTH_PX: u32 = 300;
pub const BOARD_HEIGHT_PX: u32 = 600;

pub fn piece_color(type_index: u32) -> [u8; 4] {
    match type_index {
        0 => [0, 255, 255, 255],   // I cyan
        1 => [255, 255, 0, 255],   // O yellow
        2 => [128, 0, 128, 255],   // T purple
        3 => [0, 255, 0, 255],     // S green
        4 => [255, 0, 0, 255],     // Z red
        5 => [0, 0, 255, 255],     // J blue
        6 => [255, 120, 0, 255],   // L deep orange
        _ => unreachable!("type_index must be 0..=6"),
    }
}

pub fn cell_rect(row: u32, col: u32, board_x: i32, board_y: i32, cell_size: u32) -> (i32, i32, u32, u32) {
    let px_x = board_x + (col * cell_size) as i32;
    let px_y = board_y + (row * cell_size) as i32;
    (px_x, px_y, cell_size, cell_size)
}

pub fn board_quads(
    grid: &Grid,
    active_piece: &ActivePiece,
    board_x: i32,
    board_y: i32,
    cell_size: u32,
) -> Vec<(i32, i32, u32, u32, [u8; 4])> {
    let mut quads = Vec::new();

    // Occupied grid cells
    for row in 0..crate::grid::HEIGHT {
        for col in 0..crate::grid::WIDTH {
            if let CellState::Occupied(type_index) = grid.cells[row][col] {
                let color = piece_color(type_index);
                let (px_x, px_y, pw, ph) = cell_rect(row as u32, col as u32, board_x, board_y, cell_size);
                quads.push((px_x, px_y, pw, ph, color));
            }
        }
    }

    // Find ghost piece row
    let active_cells = piece_cells(active_piece.piece_type, active_piece.rotation);
    let mut ghost_row = active_piece.row;
    loop {
        let next_row = ghost_row + 1;
        if is_valid_position(grid, &active_cells, next_row, active_piece.col) {
            ghost_row = next_row;
        } else {
            break;
        }
    }

    // Build set of active piece absolute positions to skip ghost overlaps
    let active_positions: std::collections::HashSet<(i32, i32)> = active_cells
        .iter()
        .map(|&(dr, dc)| (active_piece.row + dr, active_piece.col + dc))
        .collect();

    // Ghost piece cells
    let base_color = piece_color(active_piece.piece_type as u32);
    let ghost_color = [base_color[0], base_color[1], base_color[2], 80];
    for &(dr, dc) in &active_cells {
        let r = ghost_row + dr;
        let c = active_piece.col + dc;
        if active_positions.contains(&(r, c)) {
            continue;
        }
        if r >= 0 && c >= 0 && (r as usize) < crate::grid::HEIGHT && (c as usize) < crate::grid::WIDTH {
            let (px_x, px_y, pw, ph) = cell_rect(r as u32, c as u32, board_x, board_y, cell_size);
            quads.push((px_x, px_y, pw, ph, ghost_color));
        }
    }

    // Active piece cells
    for &(dr, dc) in &active_cells {
        let r = active_piece.row + dr;
        let c = active_piece.col + dc;
        if r >= 0 && c >= 0 && (r as usize) < crate::grid::HEIGHT && (c as usize) < crate::grid::WIDTH {
            let color = piece_color(active_piece.piece_type as u32);
            let (px_x, px_y, pw, ph) = cell_rect(r as u32, c as u32, board_x, board_y, cell_size);
            quads.push((px_x, px_y, pw, ph, color));
        }
    }

    quads
}

pub fn next_piece_quads(
    piece_type: usize,
    preview_x: i32,
    preview_y: i32,
    cell_size: u32,
) -> Vec<(i32, i32, u32, u32, [u8; 4])> {
    let cells = PIECE_CELLS[piece_type][0];
    let color = piece_color(piece_type as u32);
    let mut quads = Vec::new();
    for &(dr, dc) in &cells {
        let px_x = preview_x + dc * cell_size as i32;
        let px_y = preview_y + dr * cell_size as i32;
        quads.push((px_x, px_y, cell_size, cell_size, color));
    }
    quads
}
