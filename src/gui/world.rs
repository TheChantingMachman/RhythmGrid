// Game world — wraps pipeline's GameSession with GUI-specific state.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use rhythm_grid::config::{config_dir, load_settings};
use rhythm_grid::game::*;
use rhythm_grid::grid::*;
use rhythm_grid::input::GameAction;
use rhythm_grid::pieces::*;
use rhythm_grid::render::*;

use super::audio_output::{self, AudioState};
use super::drawing::*;
use super::particles::ParticleSystem;
use super::theme::*;

pub struct GameWorld {
    pub session: GameSession,
    pub last_tick: Instant,
    pub camera_angle: f32,
    preview_angle: f32,
    preview_rotation: usize,
    preview_timer: f32,
    pub audio: Arc<Mutex<AudioState>>,
    pub beat_intensity: f32,
    pub amplitude: f32,
    pub particles: ParticleSystem,
    prev_beat: bool, // edge detect for beat spawning
}

impl GameWorld {
    pub fn new() -> Self {
        // Load settings to check for music folder
        let settings_path = config_dir().join("settings.toml");
        let settings = load_settings(&settings_path);
        let audio = audio_output::start_audio(settings.music_folder.as_deref());
        GameWorld {
            session: GameSession::new(),
            last_tick: Instant::now(),
            camera_angle: DEFAULT_CAM_ANGLE,
            preview_angle: 0.0,
            preview_rotation: 0,
            preview_timer: 0.0,
            audio,
            beat_intensity: 0.0,
            amplitude: 0.0,
            particles: ParticleSystem::new(),
            prev_beat: false,
        }
    }

    pub fn iso_offsets(&self) -> (f32, f32) {
        let mag = 0.8;
        let dx = self.camera_angle.cos() * mag;
        let dy = -self.camera_angle.sin().abs() * mag * 0.7;
        (dx, dy)
    }

    /// Effective block depth — increases when paused for dramatic orbit effect
    pub fn effective_depth(&self) -> f32 {
        if self.session.state == GameState::Paused {
            THEME.block_depth * 3.0
        } else {
            THEME.block_depth
        }
    }

    /// Board position shift based on camera angle (parallax)
    pub fn board_parallax(&self) -> (f32, f32) {
        if self.session.state == GameState::Paused {
            let shift_x = (self.camera_angle - DEFAULT_CAM_ANGLE).sin() * 20.0;
            let shift_y = (self.camera_angle - DEFAULT_CAM_ANGLE).cos() * 5.0 - 5.0;
            (shift_x, shift_y)
        } else {
            (0.0, 0.0)
        }
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f64();

        // Pull audio state for rendering
        let mut got_beat = false;
        if let Ok(mut audio) = self.audio.try_lock() {
            audio.tick(dt as f32);
            self.beat_intensity = audio.beat_intensity;
            self.amplitude = audio.amplitude;
            got_beat = audio.beat_intensity > 0.9; // fresh beat
        }

        // Spawn beat particles (edge-triggered)
        if got_beat && !self.prev_beat {
            let t = &THEME;
            let cs = CELL_SIZE as f32 * t.board_scale;
            let (px, py) = self.board_parallax();
            let bx = t.board_margin_left + px;
            let by = t.board_margin_top + py;
            let bw = WIDTH as f32 * cs;
            let bh = HEIGHT as f32 * cs;
            self.particles.spawn_beat_pulse(bx, by, bw, bh, 1.0);
        }
        self.prev_beat = got_beat;

        // Update particles
        self.particles.update(dt as f32);

        // Always advance preview rotation (even when paused)
        self.preview_angle += dt as f32 * 0.8;
        self.preview_timer += dt as f32;
        const ROTATION_INTERVAL: f32 = 1.5; // seconds per 90° step
        if self.preview_timer >= ROTATION_INTERVAL {
            self.preview_timer -= ROTATION_INTERVAL;
            self.preview_rotation = (self.preview_rotation + 1) % 4;
        }

        if self.session.state != GameState::Playing {
            self.last_tick = now;
            return;
        }
        self.last_tick = now;

        self.session.gravity_accumulator_ms += (dt * 1000.0) as u64;
        let level = level_for_lines(self.session.total_lines);
        let interval = gravity_interval_ms(level);
        if self.session.gravity_accumulator_ms >= interval {
            if !move_down(&self.session.grid, &mut self.session.active_piece) {
                let lines = lock_piece(&mut self.session.grid, &self.session.active_piece);
                let next_type = TETROMINO_TYPES[self.session.bag.next()];
                match try_spawn(next_type, &self.session.grid) {
                    None => { self.session.state = GameState::GameOver; }
                    Some((row, col)) => {
                        self.session.active_piece = ActivePiece {
                            piece_type: next_type, rotation: 0, row, col
                        };
                        self.session.total_lines += lines;
                        let new_level = level_for_lines(self.session.total_lines);
                        self.session.score += score_for_lines(lines, new_level);
                    }
                }
            }
            self.session.gravity_accumulator_ms = 0;
        }
    }

    pub fn handle_action(&mut self, action: GameAction) {
        match self.session.state {
            GameState::Playing => match action {
                GameAction::MoveLeft => { move_horizontal(&self.session.grid, &mut self.session.active_piece, -1); }
                GameAction::MoveRight => { move_horizontal(&self.session.grid, &mut self.session.active_piece, 1); }
                GameAction::SoftDrop => {
                    if !move_down(&self.session.grid, &mut self.session.active_piece) {
                        self.lock_and_spawn();
                    }
                    self.session.gravity_accumulator_ms = 0;
                }
                GameAction::HardDrop => {
                    // Find landing row (lowest cell) before hard_drop
                    let cells = piece_cells(self.session.active_piece.piece_type, self.session.active_piece.rotation);
                    let max_dr = cells.iter().map(|(dr, _)| *dr).max().unwrap_or(0);
                    let mut land_row = self.session.active_piece.row;
                    while is_valid_position(&self.session.grid, &cells, land_row + 1, self.session.active_piece.col) {
                        land_row += 1;
                    }
                    let land_bottom = land_row + max_dr;
                    let lines = hard_drop(&mut self.session.grid, &self.session.active_piece);
                    if lines > 0 {
                        self.spawn_line_clear_particles(lines, land_bottom);
                    }
                    self.session.total_lines += lines;
                    let level = level_for_lines(self.session.total_lines);
                    self.session.score += score_for_lines(lines, level);
                    self.spawn_or_game_over();
                    self.session.gravity_accumulator_ms = 0;
                }
                GameAction::RotateCW => { rotate(&self.session.grid, &mut self.session.active_piece, true); }
                GameAction::RotateCCW => { rotate(&self.session.grid, &mut self.session.active_piece, false); }
                GameAction::TogglePause => { self.session.state = GameState::Paused; }
                _ => {}
            }
            GameState::Paused => match action {
                GameAction::TogglePause => {
                    self.session.state = GameState::Playing;
                    self.last_tick = Instant::now();
                    self.camera_angle = DEFAULT_CAM_ANGLE;
                }
                GameAction::MoveLeft => { self.camera_angle -= 0.25; }
                GameAction::MoveRight => { self.camera_angle += 0.25; }
                _ => {}
            }
            GameState::GameOver | GameState::Menu => {
                if action == GameAction::StartGame {
                    *self = GameWorld::new();
                }
            }
        }
    }

    fn lock_and_spawn(&mut self) {
        // Find the lowest cell of the piece (where clears happen)
        let cells = piece_cells(self.session.active_piece.piece_type, self.session.active_piece.rotation);
        let max_dr = cells.iter().map(|(dr, _)| *dr).max().unwrap_or(0);
        let piece_row = self.session.active_piece.row + max_dr;
        let lines = lock_piece(&mut self.session.grid, &self.session.active_piece);
        if lines > 0 {
            self.spawn_line_clear_particles(lines, piece_row);
        }
        self.session.total_lines += lines;
        let level = level_for_lines(self.session.total_lines);
        self.session.score += score_for_lines(lines, level);
        self.spawn_or_game_over();
    }

    fn spawn_line_clear_particles(&mut self, lines: u32, piece_row: i32) {
        let t = &THEME;
        let cs = CELL_SIZE as f32 * t.board_scale;
        let (px, py) = self.board_parallax();
        let bx = t.board_margin_left + px;
        let by = t.board_margin_top + py;
        let bw = WIDTH as f32 * cs;
        // piece_row is the bottom of the piece; clears happen upward from there
        let bottom_y = by + (piece_row.max(0) as f32 + 1.0) * cs;
        let clear_height = lines as f32 * cs;
        let top_y = bottom_y - clear_height;
        let color = match lines {
            1 => [0.5, 0.8, 1.0, 0.8],
            2 => [0.4, 1.0, 0.6, 0.9],
            3 => [1.0, 0.8, 0.2, 0.9],
            _ => [1.0, 0.3, 0.8, 1.0], // tetris — magenta burst
        };
        for _ in 0..lines {
            self.particles.spawn_line_clear(top_y, clear_height, bx, bw, color);
        }
    }

    fn spawn_or_game_over(&mut self) {
        let s = &mut self.session;
        let next_type = TETROMINO_TYPES[s.bag.next()];
        match try_spawn(next_type, &s.grid) {
            None => { s.state = GameState::GameOver; }
            Some((r, c)) => {
                s.active_piece = ActivePiece { piece_type: next_type, rotation: 0, row: r, col: c };
            }
        }
    }

    pub fn build_vertices(&self) -> (Vec<Vertex>, Vec<u32>) {
        let mut verts = Vec::new();
        let mut indices = Vec::new();
        let t = &THEME;
        let (iso_dx, iso_dy) = self.iso_offsets();

        let cs = CELL_SIZE as f32 * t.board_scale;
        let (px, py) = self.board_parallax();
        let bx = t.board_margin_left + px;
        let by = t.board_margin_top + py;
        let bw = WIDTH as f32 * cs;
        let bh = HEIGHT as f32 * cs;
        let depth = self.effective_depth();

        let text_col = rgba_to_f32(t.text_color);
        let dim_col = rgba_to_f32(t.dim_color);
        let beat = self.beat_intensity;
        let amp = self.amplitude;

        // Black expanse
        let bg = rgba_to_f32(t.bg);
        push_quad(&mut verts, &mut indices, 0.0, 0.0, t.win_w as f32, t.win_h as f32, bg, 0.0);

        // Ambient radial field behind board — pulses with amplitude
        let field_intensity = (amp * 1.5 + beat * 0.5).min(1.0);
        if field_intensity > 0.01 {
            let cx = bx + bw * 0.5;
            let cy = by + bh * 0.5;
            let radius = bh * 0.7;
            let field_color = [0.05 * field_intensity, 0.08 * field_intensity, 0.2 * field_intensity, 0.4 * field_intensity];
            let field_edge = [0.0, 0.0, 0.02 * field_intensity, 0.0];
            // Radial glow as 8 triangles from center
            let segments = 8;
            let base = verts.len() as u32;
            verts.push(Vertex { position: [
                (cx / t.win_w as f32) * 2.0 - 1.0,
                1.0 - (cy / t.win_h as f32) * 2.0,
                0.005
            ], color: field_color });
            for i in 0..segments {
                let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
                let ex = cx + angle.cos() * radius;
                let ey = cy + angle.sin() * radius;
                let (nx, ny) = px_to_ndc(ex, ey, t.win_w as f32, t.win_h as f32);
                verts.push(Vertex { position: [nx, ny, 0.005], color: field_edge });
            }
            for i in 0..segments {
                let next = (i + 1) % segments;
                indices.extend_from_slice(&[base, base + 1 + i as u32, base + 1 + next as u32]);
            }
        }

        // Grid skeleton — lines pulse with beat
        let line_base = t.grid_line_color;
        let line_boost = (beat * 40.0) as u8;
        let line = rgba_to_f32([
            line_base[0].saturating_add(line_boost),
            line_base[1].saturating_add(line_boost),
            line_base[2].saturating_add(line_boost * 2),
            line_base[3],
        ]);
        for col in 0..=WIDTH {
            push_quad(&mut verts, &mut indices, bx + col as f32 * cs, by, 1.0, bh, line, 0.015);
        }
        for row in 0..=HEIGHT {
            push_quad(&mut verts, &mut indices, bx, by + row as f32 * cs, bw, 1.0, line, 0.015);
        }

        // Board glow (neon edge — pulses with beat)
        let glow_alpha = 50 + (beat * 80.0) as u8;
        let glow_spread = 8.0 + beat * 6.0;
        let glow = rgba_to_f32([
            25 + (beat * 30.0) as u8,
            50 + (beat * 40.0) as u8,
            100 + (beat * 50.0) as u8,
            glow_alpha.min(200),
        ]);
        let gw = glow_spread;
        push_quad(&mut verts, &mut indices, bx - gw, by - gw, bw + gw * 2.0, gw, glow, 0.018);
        push_quad(&mut verts, &mut indices, bx - gw, by + bh, bw + gw * 2.0, gw, glow, 0.018);
        push_quad(&mut verts, &mut indices, bx - gw, by, gw, bh, glow, 0.018);
        push_quad(&mut verts, &mut indices, bx + bw, by, gw, bh, glow, 0.018);

        // Board border (crisp — brightens with beat)
        let bc = t.grid_border_color;
        let border = rgba_to_f32([
            bc[0].saturating_add((beat * 40.0) as u8),
            bc[1].saturating_add((beat * 50.0) as u8),
            bc[2].saturating_add((beat * 60.0) as u8),
            bc[3],
        ]);
        let bb = 1.0;
        push_quad(&mut verts, &mut indices, bx - bb, by - bb, bw + bb * 2.0, bb, border, 0.02);
        push_quad(&mut verts, &mut indices, bx - bb, by + bh, bw + bb * 2.0, bb, border, 0.02);
        push_quad(&mut verts, &mut indices, bx - bb, by, bb, bh, border, 0.02);
        push_quad(&mut verts, &mut indices, bx + bw, by, bb, bh, border, 0.02);

        // Occupied cells
        for row in 0..HEIGHT {
            for col in 0..WIDTH {
                if let CellState::Occupied(ti) = self.session.grid.cells[row][col] {
                    let color = rgba_to_f32(piece_color(ti));
                    let row_z = 0.04 - (row as f32 * 0.001); // higher rows draw in front
                    push_block_ex(&mut verts, &mut indices,
                        bx + col as f32 * cs, by + row as f32 * cs, cs, color, depth, row_z, iso_dx, iso_dy, amp * 2.0);
                }
            }
        }

        // Ghost + active piece
        if self.session.state == GameState::Playing {
            let cells = piece_cells(self.session.active_piece.piece_type, self.session.active_piece.rotation);
            let mut ghost_row = self.session.active_piece.row;
            while is_valid_position(&self.session.grid, &cells, ghost_row + 1, self.session.active_piece.col) {
                ghost_row += 1;
            }
            let base_color = piece_color(self.session.active_piece.piece_type as u32);
            let ghost_color = rgba_to_f32([base_color[0], base_color[1], base_color[2], 40]);
            for &(dr, dc) in &cells {
                let r = ghost_row + dr;
                let c = self.session.active_piece.col + dc;
                if r >= 0 && c >= 0 && (r as usize) < HEIGHT && (c as usize) < WIDTH {
                    push_quad(&mut verts, &mut indices,
                        bx + c as f32 * cs + 1.0, by + r as f32 * cs + 1.0,
                        cs - 2.0, cs - 2.0, ghost_color, 0.035);
                }
            }

            let color = rgba_to_f32(piece_color(self.session.active_piece.piece_type as u32));
            for &(dr, dc) in &cells {
                let r = self.session.active_piece.row + dr;
                let c = self.session.active_piece.col + dc;
                if r >= 0 && c >= 0 && (r as usize) < HEIGHT && (c as usize) < WIDTH {
                    let row_z = 0.05 - (r as f32 * 0.001);
                    push_block_ex(&mut verts, &mut indices,
                        bx + c as f32 * cs, by + r as f32 * cs, cs, color, depth, row_z, iso_dx, iso_dy, amp * 2.0);
                }
            }
        }

        // --- Particles ---
        self.particles.render(&mut verts, &mut indices);

        // --- HUD Panels ---
        let hx = bx + bw + 24.0;
        let panel_w = 130.0;
        let preview_cell = (cs * 0.65) as u32;

        // Next piece (slowly rotating)
        let np_y = by;
        push_panel(&mut verts, &mut indices, hx, np_y, panel_w, 100.0, 0.03);
        push_text(&mut verts, &mut indices, hx + 8.0, np_y + 8.0, "NEXT", dim_col, 2.0);
        let next_type_idx = self.session.bag.peek();
        let next_type = TETROMINO_TYPES[next_type_idx];
        let next_cells = piece_cells(next_type, self.preview_rotation);
        let next_color = rgba_to_f32(piece_color(next_type_idx as u32));
        let pc = preview_cell as f32;
        let preview_cx = hx + panel_w / 2.0;
        let preview_cy = np_y + 58.0;
        let preview_iso_dx = self.preview_angle.cos() * 1.2;
        let preview_iso_dy = -self.preview_angle.sin().abs() * 0.8;
        for &(dr, dc) in &next_cells {
            let px_x = preview_cx + (dc as f32 - 0.5) * pc;
            let px_y = preview_cy + (dr as f32) * pc;
            push_block_cam(&mut verts, &mut indices, px_x, px_y, pc, next_color,
                depth * 0.6, 0.06, preview_iso_dx, preview_iso_dy);
        }

        // Score
        let sp_y = np_y + 112.0;
        push_panel(&mut verts, &mut indices, hx, sp_y, panel_w, 48.0, 0.03);
        push_text(&mut verts, &mut indices, hx + 8.0, sp_y + 6.0, "SCORE", dim_col, 1.0);
        push_text(&mut verts, &mut indices, hx + 8.0, sp_y + 20.0, &format!("{}", self.session.score), text_col, 2.0);

        // Level
        let lp_y = sp_y + 56.0;
        push_panel(&mut verts, &mut indices, hx, lp_y, panel_w, 48.0, 0.03);
        push_text(&mut verts, &mut indices, hx + 8.0, lp_y + 6.0, "LEVEL", dim_col, 1.0);
        let level = level_for_lines(self.session.total_lines);
        push_text(&mut verts, &mut indices, hx + 8.0, lp_y + 20.0, &format!("{}", level), text_col, 2.0);

        // Lines
        let ln_y = lp_y + 56.0;
        push_panel(&mut verts, &mut indices, hx, ln_y, panel_w, 48.0, 0.03);
        push_text(&mut verts, &mut indices, hx + 8.0, ln_y + 6.0, "LINES", dim_col, 1.0);
        push_text(&mut verts, &mut indices, hx + 8.0, ln_y + 20.0, &format!("{}", self.session.total_lines), text_col, 2.0);

        // Controls
        let cp_y = ln_y + 64.0;
        push_panel(&mut verts, &mut indices, hx, cp_y, panel_w, 115.0, 0.03);
        push_text(&mut verts, &mut indices, hx + 8.0, cp_y + 6.0, "CONTROLS", dim_col, 1.0);
        let kx = hx + 8.0;
        let ky = cp_y + 22.0;
        push_text(&mut verts, &mut indices, kx, ky,        "L-R MOVE", dim_col, 2.0);
        push_text(&mut verts, &mut indices, kx, ky + 16.0, "DN  DROP", dim_col, 2.0);
        push_text(&mut verts, &mut indices, kx, ky + 32.0, "SPC HARD", dim_col, 2.0);
        push_text(&mut verts, &mut indices, kx, ky + 48.0, "UP  CW", dim_col, 2.0);
        push_text(&mut verts, &mut indices, kx, ky + 64.0, "Z   CCW", dim_col, 2.0);
        push_text(&mut verts, &mut indices, kx, ky + 80.0, "P  PAUSE", dim_col, 2.0);

        // Now playing
        let track_name = if let Ok(audio) = self.audio.try_lock() {
            audio.track_name.clone()
        } else {
            String::new()
        };
        if !track_name.is_empty() {
            let np_panel_y = cp_y + 123.0;
            push_panel(&mut verts, &mut indices, hx, np_panel_y, panel_w, 30.0, 0.03);
            push_text(&mut verts, &mut indices, hx + 8.0, np_panel_y + 6.0, "NOW", dim_col, 1.0);
            // Truncate long names to fit panel
            let display_name: String = track_name.chars().take(15).collect();
            push_text(&mut verts, &mut indices, hx + 8.0, np_panel_y + 16.0, &display_name.to_uppercase(), text_col, 1.0);
        }

        // State overlays
        if self.session.state == GameState::GameOver {
            push_quad(&mut verts, &mut indices, bx, by, bw, bh, rgba_to_f32([180, 0, 0, 100]), 0.08);
            let go_w = 200.0; let go_h = 50.0;
            let go_x = bx + (bw - go_w) / 2.0; let go_y = by + (bh - go_h) / 2.0;
            push_panel(&mut verts, &mut indices, go_x, go_y, go_w, go_h, 0.09);
            push_text(&mut verts, &mut indices, go_x + 12.0, go_y + 8.0, "GAME OVER", rgba_to_f32([255, 80, 80, 255]), 2.0);
            push_text(&mut verts, &mut indices, go_x + 12.0, go_y + 30.0, "ENTER TO RESTART", dim_col, 1.0);
        }

        if self.session.state == GameState::Paused {
            push_quad(&mut verts, &mut indices, bx, by, bw, bh, rgba_to_f32([0, 0, 0, 80]), 0.08);
            let pa_w = 200.0; let pa_h = 50.0;
            let pa_x = bx + (bw - pa_w) / 2.0; let pa_y = by + (bh - pa_h) / 2.0;
            push_panel(&mut verts, &mut indices, pa_x, pa_y, pa_w, pa_h, 0.09);
            push_text(&mut verts, &mut indices, pa_x + 16.0, pa_y + 8.0, "PAUSED", rgba_to_f32([255, 255, 100, 255]), 2.0);
            push_text(&mut verts, &mut indices, pa_x + 16.0, pa_y + 28.0, "L-R ORBIT", dim_col, 1.0);
        }

        (verts, indices)
    }
}
