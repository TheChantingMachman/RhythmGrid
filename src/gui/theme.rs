// Theme constants — all visual params grouped for future swappability.

pub struct Theme {
    pub win_w: u32,
    pub win_h: u32,
    pub bg: [u8; 4],
    pub board_margin_left: f32,
    pub board_margin_top: f32,
    pub board_scale: f32,
    pub block_depth: f32,
    pub grid_line_color: [u8; 4],
    pub grid_border_color: [u8; 4],
    pub grid_floor_color: [u8; 4],
    pub panel_bg: [u8; 4],
    pub panel_border: [u8; 4],
    pub text_color: [u8; 4],
    pub dim_color: [u8; 4],
}

pub const THEME: Theme = Theme {
    win_w: 580,
    win_h: 680,
    bg: [0, 0, 0, 255],
    board_margin_left: 30.0,
    board_margin_top: 40.0,
    board_scale: 0.95,
    block_depth: 8.0,
    grid_line_color: [15, 15, 25, 255],
    grid_border_color: [30, 50, 80, 150],
    grid_floor_color: [3, 3, 8, 255],
    panel_bg: [8, 8, 16, 220],
    panel_border: [25, 40, 65, 150],
    text_color: [160, 170, 200, 255],
    dim_color: [70, 80, 100, 255],
};

pub const DEFAULT_CAM_ANGLE: f32 = 0.6;
