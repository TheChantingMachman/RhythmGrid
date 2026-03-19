// Render state derivation — pipeline-owned testable logic.
// Actual draw calls live in main.rs (co-authored).

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
        6 => [255, 165, 0, 255],   // L orange
        _ => unreachable!("type_index must be 0..=6"),
    }
}

pub fn cell_rect(row: u32, col: u32, board_x: i32, board_y: i32, cell_size: u32) -> (i32, i32, u32, u32) {
    let px_x = board_x + (col * cell_size) as i32;
    let px_y = board_y + (row * cell_size) as i32;
    (px_x, px_y, cell_size, cell_size)
}
