// Vanish-zone-aware movement wrappers.
// Pipeline's is_valid_position rejects r < 0, but try_spawn allows it.
// These wrappers permit cells above the grid (vanish zone).

use rhythm_grid::grid::*;
use rhythm_grid::game::ActivePiece;
use rhythm_grid::pieces::*;

pub fn is_valid_position_vz(grid: &Grid, cells: &[(i32, i32)], row: i32, col: i32) -> bool {
    for &(dr, dc) in cells {
        let r = row + dr;
        let c = col + dc;
        if c < 0 || c as usize >= WIDTH { return false; }
        if r < 0 { continue; }
        if r as usize >= HEIGHT { return false; }
        if grid.cells[r as usize][c as usize] != CellState::Empty { return false; }
    }
    true
}

pub fn move_horizontal_vz(grid: &Grid, piece: &mut ActivePiece, delta: i32) -> bool {
    let cells = piece_cells(piece.piece_type, piece.rotation);
    let new_col = piece.col + delta;
    if is_valid_position_vz(grid, &cells, piece.row, new_col) { piece.col = new_col; true } else { false }
}

pub fn move_down_vz(grid: &Grid, piece: &mut ActivePiece) -> bool {
    let cells = piece_cells(piece.piece_type, piece.rotation);
    if is_valid_position_vz(grid, &cells, piece.row + 1, piece.col) { piece.row += 1; true } else { false }
}

pub fn rotate_vz(grid: &Grid, piece: &mut ActivePiece, clockwise: bool) -> bool {
    let new_rot = if clockwise { (piece.rotation + 1) % 4 } else { (piece.rotation + 3) % 4 };
    let cells = piece_cells(piece.piece_type, new_rot);
    if is_valid_position_vz(grid, &cells, piece.row, piece.col) {
        piece.rotation = new_rot; return true;
    }
    let kicks = srs_kicks(piece.piece_type, piece.rotation, clockwise);
    for k in &kicks {
        if is_valid_position_vz(grid, &cells, piece.row + k.1, piece.col + k.0) {
            piece.rotation = new_rot; piece.col += k.0; piece.row += k.1; return true;
        }
    }
    false
}
