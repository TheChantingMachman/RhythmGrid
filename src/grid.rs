pub const WIDTH: usize = 10;
pub const HEIGHT: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellState {
    Empty,
    Occupied(Color),
}

impl Default for CellState {
    fn default() -> Self {
        CellState::Empty
    }
}

#[derive(Debug, Clone)]
pub struct Grid {
    pub cells: [[CellState; WIDTH]; HEIGHT],
}

impl Grid {
    pub fn new() -> Self {
        Grid {
            cells: [[CellState::Empty; WIDTH]; HEIGHT],
        }
    }

    pub fn width(&self) -> usize {
        WIDTH
    }

    pub fn height(&self) -> usize {
        HEIGHT
    }

    pub fn get(&self, col: usize, row: usize) -> Option<&CellState> {
        if col < WIDTH && row < HEIGHT {
            Some(&self.cells[row][col])
        } else {
            None
        }
    }

    pub fn set(&mut self, col: usize, row: usize, state: CellState) -> bool {
        if col < WIDTH && row < HEIGHT {
            self.cells[row][col] = state;
            true
        } else {
            false
        }
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new()
    }
}
