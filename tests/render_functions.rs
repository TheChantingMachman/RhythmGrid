// @spec-tags: render.piece_colors,render.board_layout
// @invariants: piece_color maps type_index 0-6 to correct RGBA values and panics for >=7; cell_rect returns correct pixel rectangle; constants CELL_SIZE/BOARD_WIDTH_PX/BOARD_HEIGHT_PX have correct values
// @build: 49

use rhythm_grid::render::{cell_rect, piece_color, BOARD_HEIGHT_PX, BOARD_WIDTH_PX, CELL_SIZE};

// --- render.board_layout constants ---

#[test]
fn cell_size_constant() {
    assert_eq!(CELL_SIZE, 30);
}

#[test]
fn board_width_px_constant() {
    assert_eq!(BOARD_WIDTH_PX, 300);
}

#[test]
fn board_height_px_constant() {
    assert_eq!(BOARD_HEIGHT_PX, 600);
}

#[test]
fn board_width_equals_10_cells() {
    assert_eq!(BOARD_WIDTH_PX, CELL_SIZE * 10);
}

#[test]
fn board_height_equals_20_cells() {
    assert_eq!(BOARD_HEIGHT_PX, CELL_SIZE * 20);
}

// --- render.piece_colors ---

#[test]
fn piece_color_i_cyan() {
    assert_eq!(piece_color(0), [0, 255, 255, 255]);
}

#[test]
fn piece_color_o_yellow() {
    assert_eq!(piece_color(1), [255, 255, 0, 255]);
}

#[test]
fn piece_color_t_purple() {
    assert_eq!(piece_color(2), [128, 0, 128, 255]);
}

#[test]
fn piece_color_s_green() {
    assert_eq!(piece_color(3), [0, 255, 0, 255]);
}

#[test]
fn piece_color_z_red() {
    assert_eq!(piece_color(4), [255, 0, 0, 255]);
}

#[test]
fn piece_color_j_blue() {
    assert_eq!(piece_color(5), [0, 0, 255, 255]);
}

#[test]
fn piece_color_l_orange() {
    assert_eq!(piece_color(6), [255, 165, 0, 255]);
}

#[test]
fn piece_color_all_have_full_alpha() {
    for i in 0..7 {
        let color = piece_color(i);
        assert_eq!(color[3], 255, "type_index {} should have alpha=255", i);
    }
}

#[test]
#[should_panic]
fn piece_color_index_7_panics() {
    piece_color(7);
}

#[test]
#[should_panic]
fn piece_color_index_large_panics() {
    piece_color(100);
}

// --- render.board_layout cell_rect ---

#[test]
fn cell_rect_origin_cell() {
    // row=0, col=0 at board origin (0,0) with cell_size=30
    let (x, y, w, h) = cell_rect(0, 0, 0, 0, 30);
    assert_eq!(x, 0);
    assert_eq!(y, 0);
    assert_eq!(w, 30);
    assert_eq!(h, 30);
}

#[test]
fn cell_rect_col_offset() {
    // col=3 → px_x = board_x + 3*cell_size
    let (x, y, w, h) = cell_rect(0, 3, 0, 0, 30);
    assert_eq!(x, 90);
    assert_eq!(y, 0);
    assert_eq!(w, 30);
    assert_eq!(h, 30);
}

#[test]
fn cell_rect_row_offset() {
    // row=5 → px_y = board_y + 5*cell_size
    let (x, y, w, h) = cell_rect(5, 0, 0, 0, 30);
    assert_eq!(x, 0);
    assert_eq!(y, 150);
    assert_eq!(w, 30);
    assert_eq!(h, 30);
}

#[test]
fn cell_rect_row_and_col_offset() {
    let (x, y, w, h) = cell_rect(2, 4, 0, 0, 30);
    assert_eq!(x, 120); // 4 * 30
    assert_eq!(y, 60);  // 2 * 30
    assert_eq!(w, 30);
    assert_eq!(h, 30);
}

#[test]
fn cell_rect_with_board_offset() {
    // board at (100, 200)
    let (x, y, w, h) = cell_rect(1, 2, 100, 200, 30);
    assert_eq!(x, 160); // 100 + 2*30
    assert_eq!(y, 230); // 200 + 1*30
    assert_eq!(w, 30);
    assert_eq!(h, 30);
}

#[test]
fn cell_rect_negative_board_origin() {
    let (x, y, w, h) = cell_rect(0, 0, -10, -20, 30);
    assert_eq!(x, -10);
    assert_eq!(y, -20);
    assert_eq!(w, 30);
    assert_eq!(h, 30);
}

#[test]
fn cell_rect_width_height_equal_cell_size() {
    let cell_size = 30u32;
    let (_, _, w, h) = cell_rect(3, 5, 50, 50, cell_size);
    assert_eq!(w, cell_size);
    assert_eq!(h, cell_size);
}

#[test]
fn cell_rect_last_cell_of_board() {
    // bottom-right cell: row=19, col=9
    let (x, y, w, h) = cell_rect(19, 9, 0, 0, 30);
    assert_eq!(x, 270); // 9 * 30
    assert_eq!(y, 570); // 19 * 30
    assert_eq!(w, 30);
    assert_eq!(h, 30);
}

#[test]
fn cell_rect_custom_cell_size() {
    let (x, y, w, h) = cell_rect(1, 1, 0, 0, 16);
    assert_eq!(x, 16);
    assert_eq!(y, 16);
    assert_eq!(w, 16);
    assert_eq!(h, 16);
}
