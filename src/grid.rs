pub const WIDTH: usize = 10;
pub const HEIGHT: usize = 20;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CellState {
    Empty,
    Occupied(u32),
}

pub struct Grid {
    pub cells: [[CellState; WIDTH]; HEIGHT],
}

impl Grid {
    pub fn new() -> Grid {
        Grid {
            cells: [[CellState::Empty; WIDTH]; HEIGHT],
        }
    }
}
