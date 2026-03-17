pub const GRID_WIDTH: u32 = 10;
pub const GRID_HEIGHT: u32 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Empty,
    Occupied(u32),
}
