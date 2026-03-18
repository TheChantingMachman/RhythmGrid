pub const GRID_WIDTH: usize = 10;
pub const GRID_HEIGHT: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Empty,
    Occupied([u8; 3]),
}
