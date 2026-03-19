use std::sync::Arc;
use std::time::Instant;

use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use rhythm_grid::game::*;
use rhythm_grid::grid::*;
use rhythm_grid::pieces::*;

// --- Inline render constants (will be replaced by pipeline render module) ---
const CELL_SIZE: u32 = 30;
const BOARD_W: u32 = WIDTH as u32 * CELL_SIZE;
const BOARD_H: u32 = HEIGHT as u32 * CELL_SIZE;
const SIDEBAR_W: u32 = 160;
const WIN_W: u32 = BOARD_W + SIDEBAR_W;
const WIN_H: u32 = BOARD_H;
const BG: [u8; 4] = [20, 20, 30, 255];
const SIDEBAR_BG: [u8; 4] = [30, 30, 45, 255];
const TEXT_COLOR: [u8; 4] = [180, 180, 200, 255];
const DIM_COLOR: [u8; 4] = [100, 100, 120, 255];

fn piece_color(type_index: u32) -> [u8; 4] {
    match type_index {
        0 => [0, 255, 255, 255],   // I - cyan
        1 => [255, 255, 0, 255],   // O - yellow
        2 => [128, 0, 128, 255],   // T - purple
        3 => [0, 255, 0, 255],     // S - green
        4 => [255, 0, 0, 255],     // Z - red
        5 => [0, 0, 255, 255],     // J - blue
        6 => [255, 165, 0, 255],   // L - orange
        _ => [128, 128, 128, 255],
    }
}

// --- Tiny 3x5 bitmap font for HUD ---
const FONT: &[(char, [u8; 5])] = &[
    ('0', [0b111, 0b101, 0b101, 0b101, 0b111]),
    ('1', [0b010, 0b110, 0b010, 0b010, 0b111]),
    ('2', [0b111, 0b001, 0b111, 0b100, 0b111]),
    ('3', [0b111, 0b001, 0b111, 0b001, 0b111]),
    ('4', [0b101, 0b101, 0b111, 0b001, 0b001]),
    ('5', [0b111, 0b100, 0b111, 0b001, 0b111]),
    ('6', [0b111, 0b100, 0b111, 0b101, 0b111]),
    ('7', [0b111, 0b001, 0b010, 0b010, 0b010]),
    ('8', [0b111, 0b101, 0b111, 0b101, 0b111]),
    ('9', [0b111, 0b101, 0b111, 0b001, 0b111]),
    ('A', [0b010, 0b101, 0b111, 0b101, 0b101]),
    ('B', [0b110, 0b101, 0b110, 0b101, 0b110]),
    ('C', [0b111, 0b100, 0b100, 0b100, 0b111]),
    ('D', [0b110, 0b101, 0b101, 0b101, 0b110]),
    ('E', [0b111, 0b100, 0b111, 0b100, 0b111]),
    ('G', [0b111, 0b100, 0b101, 0b101, 0b111]),
    ('H', [0b101, 0b101, 0b111, 0b101, 0b101]),
    ('I', [0b111, 0b010, 0b010, 0b010, 0b111]),
    ('K', [0b101, 0b110, 0b100, 0b110, 0b101]),
    ('L', [0b100, 0b100, 0b100, 0b100, 0b111]),
    ('M', [0b101, 0b111, 0b111, 0b101, 0b101]),
    ('N', [0b101, 0b111, 0b111, 0b101, 0b101]),
    ('O', [0b111, 0b101, 0b101, 0b101, 0b111]),
    ('P', [0b111, 0b101, 0b111, 0b100, 0b100]),
    ('R', [0b111, 0b101, 0b111, 0b110, 0b101]),
    ('S', [0b111, 0b100, 0b111, 0b001, 0b111]),
    ('T', [0b111, 0b010, 0b010, 0b010, 0b010]),
    ('U', [0b101, 0b101, 0b101, 0b101, 0b111]),
    ('V', [0b101, 0b101, 0b101, 0b101, 0b010]),
    ('W', [0b101, 0b101, 0b111, 0b111, 0b101]),
    ('X', [0b101, 0b101, 0b010, 0b101, 0b101]),
    ('Z', [0b111, 0b001, 0b010, 0b100, 0b111]),
    ('-', [0b000, 0b000, 0b111, 0b000, 0b000]),
    (':', [0b000, 0b010, 0b000, 0b010, 0b000]),
    (' ', [0b000, 0b000, 0b000, 0b000, 0b000]),
];

fn draw_char(frame: &mut [u8], stride: usize, x: usize, y: usize, ch: char, color: [u8; 4], scale: usize) {
    let upper = ch.to_ascii_uppercase();
    let glyph = FONT.iter().find(|(c, _)| *c == upper).map(|(_, g)| g);
    let glyph = match glyph {
        Some(g) => g,
        None => return,
    };
    for (row, bits) in glyph.iter().enumerate() {
        for col in 0..3 {
            if bits & (1 << (2 - col)) != 0 {
                for sy in 0..scale {
                    for sx in 0..scale {
                        set_pixel(frame, stride, x + col * scale + sx, y + row * scale + sy, color);
                    }
                }
            }
        }
    }
}

fn draw_text(frame: &mut [u8], stride: usize, x: usize, y: usize, text: &str, color: [u8; 4], scale: usize) {
    for (i, ch) in text.chars().enumerate() {
        draw_char(frame, stride, x + i * (3 * scale + scale), y, ch, color, scale);
    }
}

// --- Vanish-zone-aware position check ---
// Pipeline's is_valid_position rejects r < 0, but try_spawn allows it.
// This wrapper permits cells above the grid (vanish zone).
fn is_valid_position_vz(grid: &Grid, cells: &[(i32, i32)], row: i32, col: i32) -> bool {
    for &(dr, dc) in cells {
        let r = row + dr;
        let c = col + dc;
        if c < 0 || c as usize >= WIDTH {
            return false;
        }
        if r < 0 {
            continue; // above grid = vanish zone, allowed
        }
        if r as usize >= HEIGHT {
            return false;
        }
        if grid.cells[r as usize][c as usize] != CellState::Empty {
            return false;
        }
    }
    true
}

fn move_horizontal_vz(grid: &Grid, piece: &mut ActivePiece, delta: i32) -> bool {
    let cells = piece_cells(piece.piece_type, piece.rotation);
    let new_col = piece.col + delta;
    if is_valid_position_vz(grid, &cells, piece.row, new_col) {
        piece.col = new_col;
        true
    } else {
        false
    }
}

fn move_down_vz(grid: &Grid, piece: &mut ActivePiece) -> bool {
    let cells = piece_cells(piece.piece_type, piece.rotation);
    let new_row = piece.row + 1;
    if is_valid_position_vz(grid, &cells, new_row, piece.col) {
        piece.row = new_row;
        true
    } else {
        false
    }
}

fn rotate_vz(grid: &Grid, piece: &mut ActivePiece, clockwise: bool) -> bool {
    let new_rotation = if clockwise {
        (piece.rotation + 1) % 4
    } else {
        (piece.rotation + 3) % 4
    };
    let cells = piece_cells(piece.piece_type, new_rotation);

    if is_valid_position_vz(grid, &cells, piece.row, piece.col) {
        piece.rotation = new_rotation;
        return true;
    }

    let kicks = srs_kicks(piece.piece_type, piece.rotation, clockwise);
    for k in &kicks {
        let test_col = piece.col + k.0;
        let test_row = piece.row + k.1;
        if is_valid_position_vz(grid, &cells, test_row, test_col) {
            piece.rotation = new_rotation;
            piece.col = test_col;
            piece.row = test_row;
            return true;
        }
    }
    false
}

// --- Game World ---
struct GameWorld {
    grid: Grid,
    piece: ActivePiece,
    bag: PieceBag,
    state: GameState,
    gravity_acc_ms: u64,
    total_lines: u32,
    score: u32,
    last_tick: Instant,
}

impl GameWorld {
    fn new() -> Self {
        let mut bag = PieceBag::new();
        let piece_idx = bag.next();
        let tt = tetromino_from_index(piece_idx);
        let grid = Grid::new();
        let (row, col) = try_spawn(tt, &grid).unwrap_or((0, 4));
        GameWorld {
            grid,
            piece: ActivePiece { piece_type: tt, rotation: 0, row, col },
            bag,
            state: GameState::Playing,
            gravity_acc_ms: 0,
            total_lines: 0,
            score: 0,
            last_tick: Instant::now(),
        }
    }

    fn spawn_next(&mut self) -> bool {
        let idx = self.bag.next();
        let tt = tetromino_from_index(idx);
        if let Some((r, c)) = try_spawn(tt, &self.grid) {
            self.piece = ActivePiece { piece_type: tt, rotation: 0, row: r, col: c };
            true
        } else {
            false
        }
    }

    fn lock_and_clear(&mut self) {
        let lines = lock_piece(&mut self.grid, &self.piece);
        self.total_lines += lines;
        let level = level_for_lines(self.total_lines);
        self.score += score_for_lines(lines, level);
        if !self.spawn_next() {
            self.state = GameState::GameOver;
        }
    }

    fn tick(&mut self) {
        if self.state != GameState::Playing { return; }
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_millis() as u64;
        self.last_tick = now;
        self.gravity_acc_ms += dt;

        let level = level_for_lines(self.total_lines);
        let interval = gravity_interval_ms(level);
        if self.gravity_acc_ms >= interval {
            if !move_down_vz(&self.grid, &mut self.piece) {
                self.lock_and_clear();
            }
            self.gravity_acc_ms = 0;
        }
    }

    fn handle_key(&mut self, key: KeyCode) {
        match self.state {
            GameState::Playing => match key {
                KeyCode::ArrowLeft => { move_horizontal_vz(&self.grid, &mut self.piece, -1); }
                KeyCode::ArrowRight => { move_horizontal_vz(&self.grid, &mut self.piece, 1); }
                KeyCode::ArrowDown => {
                    if !move_down_vz(&self.grid, &mut self.piece) {
                        self.lock_and_clear();
                        self.gravity_acc_ms = 0;
                    }
                }
                KeyCode::ArrowUp => { rotate_vz(&self.grid, &mut self.piece, true); }
                KeyCode::KeyZ => { rotate_vz(&self.grid, &mut self.piece, false); }
                KeyCode::Space | KeyCode::KeyX => {
                    let lines = hard_drop(&mut self.grid, &self.piece);
                    self.total_lines += lines;
                    let level = level_for_lines(self.total_lines);
                    self.score += score_for_lines(lines, level);
                    if !self.spawn_next() {
                        self.state = GameState::GameOver;
                    }
                    self.gravity_acc_ms = 0;
                }
                KeyCode::KeyP => { self.state = GameState::Paused; }
                _ => {}
            }
            GameState::Paused => {
                if key == KeyCode::KeyP { self.state = GameState::Playing; self.last_tick = Instant::now(); }
            }
            GameState::GameOver | GameState::Menu => {
                if key == KeyCode::Enter { *self = GameWorld::new(); }
            }
        }
    }

    fn draw(&self, frame: &mut [u8]) {
        // Clear background
        for pixel in frame.chunks_exact_mut(4) {
            pixel.copy_from_slice(&BG);
        }

        let stride = WIN_W as usize;

        // Draw sidebar background
        for y in 0..WIN_H as usize {
            for x in (BOARD_W as usize + 1)..WIN_W as usize {
                set_pixel(frame, stride, x, y, SIDEBAR_BG);
            }
        }

        // Draw grid border
        for r in 0..BOARD_H as usize {
            set_pixel(frame, stride, BOARD_W as usize, r, [60, 60, 80, 255]);
        }

        // Draw occupied cells
        for row in 0..HEIGHT {
            for col in 0..WIDTH {
                if let CellState::Occupied(ti) = self.grid.cells[row][col] {
                    fill_cell(frame, stride, row, col, piece_color(ti));
                }
            }
        }

        // Draw ghost piece
        if self.state == GameState::Playing {
            let cells = piece_cells(self.piece.piece_type, self.piece.rotation);
            let mut ghost_row = self.piece.row;
            while is_valid_position_vz(&self.grid, &cells, ghost_row + 1, self.piece.col) { ghost_row += 1; }

            let mut ghost_color = piece_color(self.piece.piece_type as u32);
            ghost_color[3] = 80;
            for &(dr, dc) in &cells {
                let r = ghost_row + dr;
                let c = self.piece.col + dc;
                if r >= 0 && c >= 0 && (r as usize) < HEIGHT && (c as usize) < WIDTH {
                    fill_cell_alpha(frame, stride, r as usize, c as usize, ghost_color);
                }
            }

            // Draw active piece
            let color = piece_color(self.piece.piece_type as u32);
            for &(dr, dc) in &cells {
                let r = self.piece.row + dr;
                let c = self.piece.col + dc;
                if r >= 0 && c >= 0 && (r as usize) < HEIGHT && (c as usize) < WIDTH {
                    fill_cell(frame, stride, r as usize, c as usize, color);
                }
            }
        }

        // --- Sidebar ---
        let sx = BOARD_W as usize + 12;

        // Next piece label + preview
        draw_text(frame, stride, sx, 10, "NEXT", TEXT_COLOR, 2);
        let next_type = tetromino_from_index(self.bag.peek());
        let next_cells = piece_cells(next_type, 0);
        let color = piece_color(next_type as u32);
        for &(dr, dc) in &next_cells {
            let px = sx as i32 + 10 + (dc + 1) * CELL_SIZE as i32;
            let py = 30 + (dr + 1) * CELL_SIZE as i32;
            if px >= 0 && py >= 0 {
                fill_rect(frame, stride, px as usize, py as usize,
                    CELL_SIZE as usize - 1, CELL_SIZE as usize - 1, color);
            }
        }

        // Score
        draw_text(frame, stride, sx, 140, "SCORE", TEXT_COLOR, 2);
        let score_str = format!("{}", self.score);
        draw_text(frame, stride, sx, 160, &score_str, TEXT_COLOR, 2);

        // Level
        let level = level_for_lines(self.total_lines);
        draw_text(frame, stride, sx, 195, "LEVEL", TEXT_COLOR, 2);
        let level_str = format!("{}", level);
        draw_text(frame, stride, sx, 215, &level_str, TEXT_COLOR, 2);

        // Lines
        draw_text(frame, stride, sx, 250, "LINES", TEXT_COLOR, 2);
        let lines_str = format!("{}", self.total_lines);
        draw_text(frame, stride, sx, 270, &lines_str, TEXT_COLOR, 2);

        // Controls
        let cy = 330;
        draw_text(frame, stride, sx, cy, "CONTROLS", DIM_COLOR, 1);
        draw_text(frame, stride, sx, cy + 14, "MOVE  L-R", DIM_COLOR, 1);
        draw_text(frame, stride, sx, cy + 26, "DROP  DOWN", DIM_COLOR, 1);
        draw_text(frame, stride, sx, cy + 38, "HARD  SPC-X", DIM_COLOR, 1);
        draw_text(frame, stride, sx, cy + 50, "CW    UP", DIM_COLOR, 1);
        draw_text(frame, stride, sx, cy + 62, "CCW   Z", DIM_COLOR, 1);
        draw_text(frame, stride, sx, cy + 74, "PAUSE P", DIM_COLOR, 1);

        // State overlays
        if self.state == GameState::GameOver {
            // Tint the board red
            for row in 0..HEIGHT {
                for col in 0..WIDTH {
                    fill_cell_alpha(frame, stride, row, col, [200, 0, 0, 120]);
                }
            }
            draw_text(frame, stride, sx, 500, "GAME OVER", [255, 80, 80, 255], 2);
            draw_text(frame, stride, sx, 525, "ENTER", DIM_COLOR, 1);
        }

        if self.state == GameState::Paused {
            // Dim the board
            for row in 0..HEIGHT {
                for col in 0..WIDTH {
                    fill_cell_alpha(frame, stride, row, col, [0, 0, 0, 100]);
                }
            }
            draw_text(frame, stride, sx, 500, "PAUSED", [255, 255, 100, 255], 2);
            draw_text(frame, stride, sx, 525, "P", DIM_COLOR, 1);
        }
    }
}

fn set_pixel(frame: &mut [u8], stride: usize, x: usize, y: usize, color: [u8; 4]) {
    if x < stride && y < WIN_H as usize {
        let idx = (y * stride + x) * 4;
        if idx + 3 < frame.len() {
            frame[idx..idx + 4].copy_from_slice(&color);
        }
    }
}

fn fill_rect(frame: &mut [u8], stride: usize, x: usize, y: usize, w: usize, h: usize, color: [u8; 4]) {
    for dy in 0..h {
        for dx in 0..w {
            set_pixel(frame, stride, x + dx, y + dy, color);
        }
    }
}

fn fill_cell(frame: &mut [u8], stride: usize, row: usize, col: usize, color: [u8; 4]) {
    let x = col * CELL_SIZE as usize;
    let y = row * CELL_SIZE as usize;
    fill_rect(frame, stride, x, y, CELL_SIZE as usize - 1, CELL_SIZE as usize - 1, color);
}

fn fill_cell_alpha(frame: &mut [u8], stride: usize, row: usize, col: usize, color: [u8; 4]) {
    let x = col * CELL_SIZE as usize;
    let y = row * CELL_SIZE as usize;
    let alpha = color[3] as u16;
    for dy in 0..CELL_SIZE as usize - 1 {
        for dx in 0..CELL_SIZE as usize - 1 {
            let px = x + dx;
            let py = y + dy;
            if px < stride && py < WIN_H as usize {
                let idx = (py * stride + px) * 4;
                if idx + 3 < frame.len() {
                    frame[idx] = ((color[0] as u16 * alpha + frame[idx] as u16 * (255 - alpha)) / 255) as u8;
                    frame[idx + 1] = ((color[1] as u16 * alpha + frame[idx + 1] as u16 * (255 - alpha)) / 255) as u8;
                    frame[idx + 2] = ((color[2] as u16 * alpha + frame[idx + 2] as u16 * (255 - alpha)) / 255) as u8;
                    frame[idx + 3] = 255;
                }
            }
        }
    }
}

fn tetromino_from_index(i: usize) -> TetrominoType {
    match i {
        0 => TetrominoType::I,
        1 => TetrominoType::O,
        2 => TetrominoType::T,
        3 => TetrominoType::S,
        4 => TetrominoType::Z,
        5 => TetrominoType::J,
        6 => TetrominoType::L,
        _ => TetrominoType::T,
    }
}

// --- Winit App ---
struct App {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    world: GameWorld,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let attrs = Window::default_attributes()
            .with_title("RhythmGrid")
            .with_inner_size(winit::dpi::LogicalSize::new(WIN_W, WIN_H))
            .with_min_inner_size(winit::dpi::LogicalSize::new(WIN_W / 2, WIN_H / 2));
        let window = Arc::new(event_loop.create_window(attrs).expect("failed to create window"));

        let size = window.inner_size();
        let surface = SurfaceTexture::new(size.width, size.height, window.clone());
        let px = Pixels::new(WIN_W, WIN_H, surface).expect("failed to create pixels");
        // SAFETY: Pixels borrows the window's surface, but we hold the Arc<Window> for the
        // entire lifetime of App, so the borrow is valid for the program's duration.
        let px: Pixels<'static> = unsafe { std::mem::transmute(px) };

        self.pixels = Some(px);
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(new_size) => {
                if let Some(px) = &mut self.pixels {
                    px.resize_surface(new_size.width.max(1), new_size.height.max(1)).ok();
                }
            }
            WindowEvent::KeyboardInput { event: KeyEvent { physical_key: PhysicalKey::Code(code), state: ElementState::Pressed, .. }, .. } => {
                self.world.handle_key(code);
            }
            WindowEvent::RedrawRequested => {
                self.world.tick();
                if let Some(px) = &mut self.pixels {
                    let frame: &mut [u8] = px.frame_mut();
                    self.world.draw(frame);
                    px.render().ok();
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut app = App {
        window: None,
        pixels: None,
        world: GameWorld::new(),
    };
    event_loop.run_app(&mut app).expect("event loop failed");
}
