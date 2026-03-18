use crate::grid::{CellState, Grid, HEIGHT, WIDTH};

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
