pub const WIDTH: usize = 10;
pub const HEIGHT: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color(pub u8, pub u8, pub u8);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cell {
    Empty,
    Occupied(Color),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    pub cells: [[Cell; WIDTH]; HEIGHT],
}

impl Grid {
    pub fn new() -> Self {
        Grid {
            cells: [[Cell::Empty; WIDTH]; HEIGHT],
        }
    }

    pub fn width(&self) -> usize {
        WIDTH
    }

    pub fn height(&self) -> usize {
        HEIGHT
    }

    pub fn get(&self, row: usize, col: usize) -> &Cell {
        &self.cells[row][col]
    }

    pub fn set(&mut self, row: usize, col: usize, cell: Cell) {
        self.cells[row][col] = cell;
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Cell {
    fn default() -> Self {
        Cell::Empty
    }
}
