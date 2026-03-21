// Theme constants — all visual params grouped for future swappability.

pub struct Theme {
    pub win_w: u32,
    pub win_h: u32,
    pub panel_bg: [u8; 4],
    pub panel_border: [u8; 4],
    pub text_color: [u8; 4],
    pub dim_color: [u8; 4],
}

pub const THEME: Theme = Theme {
    win_w: 900,
    win_h: 720,
    panel_bg: [8, 8, 16, 220],
    panel_border: [25, 40, 65, 150],
    text_color: [160, 170, 200, 255],
    dim_color: [70, 80, 100, 255],
};

pub const DEFAULT_CAM_ANGLE: f32 = 0.6;
