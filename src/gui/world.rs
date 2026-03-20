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
use super::renderer::{Uniforms, perspective, look_at, mat4_mul};
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
    pub bass: f32,
    pub mids: f32,
    pub highs: f32,
    pub particles: ParticleSystem,
    prev_beat: bool,
    line_clear_anim: Vec<LineClearAnim>,
    bg_rings: Vec<BgRing>,
    danger_level: f32, // smoothly transitions 0.0 (normal) → 1.0 (danger)
}

/// Expanding ring in the background
struct BgRing {
    radius: f32,
    max_radius: f32,
    life: f32,
    max_life: f32,
    color: [f32; 4],
}

/// Active line clear flash animation
struct LineClearAnim {
    row: i32,        // grid row that was cleared
    timer: f32,      // countdown (starts at DURATION)
    color: [f32; 4], // flash color
}

const LINE_CLEAR_DURATION: f32 = 0.25;

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
            bass: 0.0,
            mids: 0.0,
            highs: 0.0,
            particles: ParticleSystem::new(),
            prev_beat: false,
            line_clear_anim: Vec::new(),
            bg_rings: Vec::new(),
            danger_level: 0.0,
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
            self.bass = audio.bass;
            self.mids = audio.mids;
            self.highs = audio.highs;
            got_beat = audio.beat_intensity > 0.9; // fresh beat
        }

        // Spawn background ring on beat
        if got_beat && !self.prev_beat {
            let d = self.danger_level;
            self.bg_rings.push(BgRing {
                radius: 0.5,
                max_radius: 18.0,
                life: 3.0 - d * 1.0, // faster rings in danger
                max_life: 3.0 - d * 1.0,
                color: [
                    0.1 + d * 0.5,
                    0.15 - d * 0.05,
                    0.4 - d * 0.3,
                    0.3 + d * 0.15 + self.bass * 0.2, // bass amplifies ring brightness
                ],
            });
        }

        // Update rings
        for ring in &mut self.bg_rings {
            let progress = 1.0 - ring.life / ring.max_life;
            ring.radius = 0.5 + progress * ring.max_radius;
            ring.life -= dt as f32;
        }
        self.bg_rings.retain(|r| r.life > 0.0);

        // Spawn beat particles (edge-triggered)
        if got_beat && !self.prev_beat {
            // Approximate board screen bounds (centered, ~45% width, ~85% height)
            let w = THEME.win_w as f32;
            let h = THEME.win_h as f32;
            let bw = w * 0.35;
            let bh = h * 0.85;
            let bx = (w - bw) / 2.0;
            let by = (h - bh) / 2.0;
            self.particles.spawn_beat_pulse(bx, by, bw, bh, 1.0);
        }
        self.prev_beat = got_beat;

        // Update particles and line clear animations
        self.particles.update(dt as f32);
        for anim in &mut self.line_clear_anim {
            anim.timer -= dt as f32;
        }
        self.line_clear_anim.retain(|a| a.timer > 0.0);

        // Smooth escalation transition
        let target_danger = if escalation_stage(&self.session.grid) == EscalationStage::Danger { 1.0 } else { 0.0 };
        let danger_speed = 2.0; // transition speed
        if self.danger_level < target_danger {
            self.danger_level = (self.danger_level + dt as f32 * danger_speed).min(1.0);
        } else {
            self.danger_level = (self.danger_level - dt as f32 * danger_speed).max(0.0);
        }

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
                        let color = match lines {
                            1 => [0.5, 0.8, 1.0, 1.0],
                            2 => [0.4, 1.0, 0.6, 1.0],
                            3 => [1.0, 0.8, 0.2, 1.0],
                            _ => [1.0, 0.3, 0.8, 1.0],
                        };
                        // Approximate cleared row positions from the landing row upward
                        for i in 0..lines {
                            self.line_clear_anim.push(LineClearAnim {
                                row: land_bottom - i as i32,
                                timer: LINE_CLEAR_DURATION,
                                color,
                            });
                        }
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
        let cells = piece_cells(self.session.active_piece.piece_type, self.session.active_piece.rotation);
        let max_dr = cells.iter().map(|(dr, _)| *dr).max().unwrap_or(0);
        let piece_row = self.session.active_piece.row + max_dr;

        let lines = lock_piece(&mut self.session.grid, &self.session.active_piece);
        if lines > 0 {
            self.spawn_line_clear_particles(lines, piece_row);
            let color = match lines {
                1 => [0.5, 0.8, 1.0, 1.0],
                2 => [0.4, 1.0, 0.6, 1.0],
                3 => [1.0, 0.8, 0.2, 1.0],
                _ => [1.0, 0.3, 0.8, 1.0],
            };
            // Approximate cleared row positions from the piece bottom upward
            for i in 0..lines {
                self.line_clear_anim.push(LineClearAnim {
                    row: piece_row - i as i32,
                    timer: LINE_CLEAR_DURATION,
                    color,
                });
            }
        }
        self.session.total_lines += lines;
        let level = level_for_lines(self.session.total_lines);
        self.session.score += score_for_lines(lines, level);
        self.spawn_or_game_over();
    }

    fn spawn_line_clear_particles(&mut self, lines: u32, piece_row: i32) {
        // Approximate screen-space bounds for particle spawning
        let w = THEME.win_w as f32;
        let h = THEME.win_h as f32;
        let bw = w * 0.35;
        let bx = (w - bw) / 2.0;
        let by_top = (h - h * 0.85) / 2.0;
        let cell_h = (h * 0.85) / HEIGHT as f32;
        let bottom_y = by_top + (piece_row.max(0) as f32 + 1.0) * cell_h;
        let clear_height = lines as f32 * cell_h;
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

    pub fn compute_uniforms(&self) -> Uniforms {
        // Board center in world space: x=5, y=-10, z=0
        let board_cx = WIDTH as f32 / 2.0;
        let board_cy = -(HEIGHT as f32) / 2.0;

        // Camera orbits when paused
        // Camera centered on board, straight-on with slight elevation
        let orbit = if self.session.state == GameState::Paused {
            (self.camera_angle - DEFAULT_CAM_ANGLE).sin() * 4.0
        } else {
            0.0
        };
        let cam_x = board_cx + orbit;
        let cam_y = board_cy;
        let cam_z = 16.0;

        let eye = [cam_x, cam_y, cam_z];
        let target = [board_cx, board_cy, 0.0];
        let up = [0.0, 1.0, 0.0];

        let aspect = THEME.win_w as f32 / THEME.win_h as f32;

        let view = look_at(eye, target, up);
        let proj = perspective(1.2, aspect, 0.1, 200.0);
        let vp = mat4_mul(&proj, &view);

        Uniforms::from_mat(vp)
    }

    /// Build 3D scene (world-space cubes) and 2D HUD (NDC overlay) separately
    pub fn build_scene_and_hud(&self) -> ((Vec<Vertex>, Vec<u32>), (Vec<Vertex>, Vec<u32>)) {
        let amp = self.amplitude;
        let beat = self.beat_intensity;
        let cube_depth = 0.5; // depth of cubes in grid units

        // === 3D SCENE (world space: x=col, y=-row, z=depth) ===
        let mut sv = Vec::new();
        let mut si = Vec::new();

        let gw = WIDTH as f32;
        let gh = HEIGHT as f32;

        // Ambient rotating geometry (always present, breathes with amplitude)
        let d = self.danger_level;
        let geo_cx = gw / 2.0;
        let geo_cy = -gh / 2.0;
        let geo_z = -2.0;
        let geo_n = [0.0f32, 0.0, 1.0];
        let geo_time = self.preview_angle * (0.3 + d * 0.4); // spins faster in danger
        let geo_alpha = 0.03 + self.mids * 0.15 + d * 0.05; // mids drive hex brightness

        // Slow-rotating hexagonal grid of dots
        let hex_rings = 4;
        let dot_size = 0.12 + self.mids * 0.12;
        for ring in 1..=hex_rings {
            let r = ring as f32 * 3.5;
            let points = ring * 6;
            for i in 0..points {
                let angle = (i as f32 / points as f32) * std::f32::consts::TAU + geo_time;
                let dx = angle.cos() * r;
                let dy = angle.sin() * r;

                let dist_factor = 1.0 - (ring as f32 / hex_rings as f32) * 0.5;
                let dot_alpha = geo_alpha * dist_factor;
                // Blue → orange/red shift with danger
                let dot_color = [
                    0.15 + d * 0.45,  // r: 0.15 → 0.6
                    0.2 - d * 0.08,   // g: 0.2 → 0.12
                    0.5 - d * 0.35,   // b: 0.5 → 0.15
                    dot_alpha,
                ];

                let base = sv.len() as u32;
                sv.push(Vertex { position: [geo_cx + dx - dot_size, geo_cy + dy - dot_size, geo_z], normal: geo_n, color: dot_color });
                sv.push(Vertex { position: [geo_cx + dx + dot_size, geo_cy + dy - dot_size, geo_z], normal: geo_n, color: dot_color });
                sv.push(Vertex { position: [geo_cx + dx + dot_size, geo_cy + dy + dot_size, geo_z], normal: geo_n, color: dot_color });
                sv.push(Vertex { position: [geo_cx + dx - dot_size, geo_cy + dy + dot_size, geo_z], normal: geo_n, color: dot_color });
                si.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }
        }

        // Connecting lines between hex points (subtle geometric web)
        for ring in 1..=hex_rings {
            let r = ring as f32 * 3.5;
            let points = ring * 6;
            let line_alpha = geo_alpha * 0.4;
            let line_color = [
                0.1 + d * 0.35,
                0.15 - d * 0.05,
                0.35 - d * 0.25,
                line_alpha,
            ];
            let line_w = 0.03;

            for i in 0..points {
                let a0 = (i as f32 / points as f32) * std::f32::consts::TAU + geo_time;
                let a1 = ((i + 1) as f32 / points as f32) * std::f32::consts::TAU + geo_time;
                let x0 = geo_cx + a0.cos() * r;
                let y0 = geo_cy + a0.sin() * r;
                let x1 = geo_cx + a1.cos() * r;
                let y1 = geo_cy + a1.sin() * r;

                // Line as thin quad
                let dx = x1 - x0;
                let dy = y1 - y0;
                let len = (dx * dx + dy * dy).sqrt();
                if len < 0.001 { continue; }
                let nx = -dy / len * line_w;
                let ny = dx / len * line_w;

                let base = sv.len() as u32;
                sv.push(Vertex { position: [x0 + nx, y0 + ny, geo_z], normal: geo_n, color: line_color });
                sv.push(Vertex { position: [x1 + nx, y1 + ny, geo_z], normal: geo_n, color: line_color });
                sv.push(Vertex { position: [x1 - nx, y1 - ny, geo_z], normal: geo_n, color: line_color });
                sv.push(Vertex { position: [x0 - nx, y0 - ny, geo_z], normal: geo_n, color: line_color });
                si.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }
        }

        // Background rings (behind the board)
        let ring_cx = gw / 2.0;  // board center x in world
        let ring_cy = -gh / 2.0; // board center y in world
        let ring_z = -1.0;       // behind the grid
        let ring_segments = 32;
        let ring_n = [0.0f32, 0.0, 1.0];

        for ring in &self.bg_rings {
            let progress = 1.0 - ring.life / ring.max_life;
            let alpha = ring.color[3] * (1.0 - progress).powi(2); // fade out
            if alpha < 0.005 { continue; }
            let inner_r = ring.radius;
            let outer_r = ring.radius + 0.3 + (1.0 - progress) * 0.5; // ring thickness thins as it expands
            let color_inner = [ring.color[0], ring.color[1], ring.color[2], alpha];
            let color_outer = [ring.color[0] * 0.5, ring.color[1] * 0.5, ring.color[2] * 0.5, 0.0];

            for i in 0..ring_segments {
                let a0 = (i as f32 / ring_segments as f32) * std::f32::consts::TAU;
                let a1 = ((i + 1) as f32 / ring_segments as f32) * std::f32::consts::TAU;
                let (c0, s0) = (a0.cos(), a0.sin());
                let (c1, s1) = (a1.cos(), a1.sin());

                let base = sv.len() as u32;
                // Inner edge (brighter)
                sv.push(Vertex { position: [ring_cx + c0 * inner_r, ring_cy + s0 * inner_r, ring_z], normal: ring_n, color: color_inner });
                sv.push(Vertex { position: [ring_cx + c1 * inner_r, ring_cy + s1 * inner_r, ring_z], normal: ring_n, color: color_inner });
                // Outer edge (fades to transparent)
                sv.push(Vertex { position: [ring_cx + c1 * outer_r, ring_cy + s1 * outer_r, ring_z], normal: ring_n, color: color_outer });
                sv.push(Vertex { position: [ring_cx + c0 * outer_r, ring_cy + s0 * outer_r, ring_z], normal: ring_n, color: color_outer });
                si.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }
        }

        // Grid floor (very dark, mostly transparent)
        let floor_color = rgba_to_f32([5, 5, 12, 200]);
        push_grid_floor(&mut sv, &mut si, gw, gh, floor_color);


        // Grid lines — beat + highs drive shimmer
        let line_boost = (beat * 40.0) as u8;
        let highs_boost = (self.highs * 60.0) as u8;
        let lc: [u8; 4] = [40, 45, 70, 255];
        let line_color = rgba_to_f32([
            lc[0].saturating_add(line_boost).saturating_add(highs_boost / 3),
            lc[1].saturating_add(line_boost).saturating_add(highs_boost / 2),
            lc[2].saturating_add(line_boost * 2).saturating_add(highs_boost),
            lc[3],
        ]);
        for col in 0..=WIDTH {
            push_grid_line_v(&mut sv, &mut si, col as f32, gh, line_color);
        }
        for row in 0..=HEIGHT {
            push_grid_line_h(&mut sv, &mut si, -(row as f32), gw, line_color);
        }

        // Occupied cells as 3D cubes
        for row in 0..HEIGHT {
            for col in 0..WIDTH {
                if let CellState::Occupied(ti) = self.session.grid.cells[row][col] {
                    let color = rgba_to_f32(piece_color(ti));
                    push_cube_3d(&mut sv, &mut si, col as f32, row as f32, cube_depth, color, amp * 2.0);
                }
            }
        }

        // Ghost piece (thin translucent cubes)
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
                    push_cube_3d(&mut sv, &mut si, c as f32, r as f32, cube_depth * 0.2, ghost_color, 0.0);
                }
            }

            // Active piece as 3D cubes
            let color = rgba_to_f32(piece_color(self.session.active_piece.piece_type as u32));
            for &(dr, dc) in &cells {
                let r = self.session.active_piece.row + dr;
                let c = self.session.active_piece.col + dc;
                if r >= 0 && c >= 0 && (r as usize) < HEIGHT && (c as usize) < WIDTH {
                    push_cube_3d(&mut sv, &mut si, c as f32, r as f32, cube_depth, color, amp * 2.0);
                }
            }
        }

        // === 2D HUD (NDC, same as before) ===
        // Line clear flash animations (3D world space)
        for anim in &self.line_clear_anim {
            let progress = anim.timer / LINE_CLEAR_DURATION; // 1.0 → 0.0
            // Bright white flash that fades to colored, then transparent
            let white_mix = (progress * 2.0).min(1.0); // white for first half
            let r = anim.color[0] + (1.0 - anim.color[0]) * white_mix;
            let g = anim.color[1] + (1.0 - anim.color[1]) * white_mix;
            let b = anim.color[2] + (1.0 - anim.color[2]) * white_mix;
            let alpha = progress * 0.7;
            let flash_color = [r, g, b, alpha];
            let n = [0.0f32, 0.0, 1.0];
            let row = anim.row as f32;
            let base = sv.len() as u32;
            sv.push(Vertex { position: [0.0, -row, cube_depth + 0.05], normal: n, color: flash_color });
            sv.push(Vertex { position: [gw, -row, cube_depth + 0.05], normal: n, color: flash_color });
            sv.push(Vertex { position: [gw, -row - 1.0, cube_depth + 0.05], normal: n, color: flash_color });
            sv.push(Vertex { position: [0.0, -row - 1.0, cube_depth + 0.05], normal: n, color: flash_color });
            si.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
        }

        let (hv, hi) = self.build_hud();

        ((sv, si), (hv, hi))
    }

    /// Build HUD as minimal overlay — board is the star, HUD floats around edges
    fn build_hud(&self) -> (Vec<Vertex>, Vec<u32>) {
        let mut verts = Vec::new();
        let mut indices = Vec::new();
        let t = &THEME;
        let w = t.win_w as f32;
        let h = t.win_h as f32;
        let text_col = rgba_to_f32(t.text_color);
        let dim_col = rgba_to_f32(t.dim_color);

        // Next piece — top right corner
        let np_x = w - 120.0;
        let np_y = 12.0;
        push_panel(&mut verts, &mut indices, np_x, np_y, 108.0, 85.0, 0.03);
        push_text(&mut verts, &mut indices, np_x + 6.0, np_y + 6.0, "NEXT", dim_col, 1.0);
        let next_type_idx = self.session.bag.peek();
        let next_type = TETROMINO_TYPES[next_type_idx];
        let next_cells = piece_cells(next_type, 0); // always spawn rotation for base shape
        let next_color = rgba_to_f32(piece_color(next_type_idx as u32));
        let preview_cx = np_x + 54.0;
        let preview_cy = np_y + 52.0;
        let preview_scale = 18.0; // pixels per grid unit
        let cube_half = 0.42; // half-size of each mini cube

        // 3-axis rotation angles
        let ax = self.preview_angle * 0.3;  // slow pitch
        let ay = self.preview_angle * 0.7;  // medium yaw (primary rotation)
        let az = self.preview_angle * 0.15; // very slow roll

        // Rotation matrices (applied as Rz * Ry * Rx)
        let (sx, cx_r) = (ax.sin(), ax.cos());
        let (sy, cy) = (ay.sin(), ay.cos());
        let (sz, cz) = (az.sin(), az.cos());

        // Find piece center for rotation origin
        let mut center = [0.0f32; 3];
        for &(dr, dc) in &next_cells {
            center[0] += dc as f32;
            center[1] += dr as f32;
        }
        center[0] /= next_cells.len() as f32;
        center[1] /= next_cells.len() as f32;

        // For each cell, build a mini cube with 3 visible faces, rotated
        for &(dr, dc) in &next_cells {
            let local_x = dc as f32 - center[0];
            let local_y = dr as f32 - center[1];
            let local_z = 0.0;

            // Cube corners relative to cell center
            let corners_local: [[f32; 3]; 8] = [
                [local_x - cube_half, local_y - cube_half, local_z - cube_half],
                [local_x + cube_half, local_y - cube_half, local_z - cube_half],
                [local_x + cube_half, local_y + cube_half, local_z - cube_half],
                [local_x - cube_half, local_y + cube_half, local_z - cube_half],
                [local_x - cube_half, local_y - cube_half, local_z + cube_half],
                [local_x + cube_half, local_y - cube_half, local_z + cube_half],
                [local_x + cube_half, local_y + cube_half, local_z + cube_half],
                [local_x - cube_half, local_y + cube_half, local_z + cube_half],
            ];

            // Rotate and project each corner
            let mut projected = [[0.0f32; 2]; 8];
            let mut z_vals = [0.0f32; 8];
            for (i, c) in corners_local.iter().enumerate() {
                // Rx
                let y1 = c[1] * cx_r - c[2] * sx;
                let z1 = c[1] * sx + c[2] * cx_r;
                // Ry
                let x2 = c[0] * cy + z1 * sy;
                let z2 = -c[0] * sy + z1 * cy;
                // Rz
                let x3 = x2 * cz - y1 * sz;
                let y3 = x2 * sz + y1 * cz;

                // Simple perspective projection
                let persp = 4.0 / (4.0 + z2 * 0.3);
                projected[i] = [
                    preview_cx + x3 * preview_scale * persp,
                    preview_cy + y3 * preview_scale * persp,
                ];
                z_vals[i] = z2;
            }

            // Draw 3 faces (front, top, right) with painter's algorithm
            // Face definitions: [corner indices], normal direction for sorting
            let faces: &[([usize; 4], [f32; 3])] = &[
                ([4, 5, 6, 7], [0.0, 0.0, 1.0]),   // front
                ([5, 1, 2, 6], [1.0, 0.0, 0.0]),    // right
                ([0, 1, 5, 4], [0.0, -1.0, 0.0]),   // top
                ([0, 3, 7, 4], [-1.0, 0.0, 0.0]),   // left
                ([3, 2, 6, 7], [0.0, 1.0, 0.0]),    // bottom
                ([0, 1, 2, 3], [0.0, 0.0, -1.0]),   // back
            ];

            // Sort faces back-to-front by rotated normal z
            let mut face_order: Vec<(usize, f32)> = faces.iter().enumerate().map(|(i, (_, n))| {
                // Rotate the normal
                let _ny1 = n[1] * cx_r - n[2] * sx;
                let nz1 = n[1] * sx + n[2] * cx_r;
                let nz2 = -n[0] * sy + nz1 * cy;
                (i, nz2)
            }).collect();
            face_order.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            for &(fi, nz) in &face_order {
                if nz < 0.0 { continue; } // back-face cull
                let (ci, _normal) = &faces[fi];

                // Shade based on rotated normal z (facing camera = bright)
                let shade = 0.4 + nz * 0.6;
                let fc = [next_color[0] * shade, next_color[1] * shade, next_color[2] * shade, next_color[3]];

                let base = verts.len() as u32;
                for &idx in ci {
                    let px = projected[idx];
                    let (nx, ny) = px_to_ndc(px[0], px[1], w, h);
                    verts.push(Vertex { position: [nx, ny, 0.06], normal: HUD_NORMAL, color: fc });
                }
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }
        }

        // Score — top left
        push_text(&mut verts, &mut indices, 12.0, 12.0, "SCORE", dim_col, 1.0);
        push_text(&mut verts, &mut indices, 12.0, 24.0,
            &format!("{}", self.session.score), text_col, 2.0);

        // Level — below score
        push_text(&mut verts, &mut indices, 12.0, 50.0, "LEVEL", dim_col, 1.0);
        let level = level_for_lines(self.session.total_lines);
        push_text(&mut verts, &mut indices, 12.0, 62.0, &format!("{}", level), text_col, 2.0);

        // Lines — below level
        push_text(&mut verts, &mut indices, 12.0, 88.0, "LINES", dim_col, 1.0);
        push_text(&mut verts, &mut indices, 12.0, 100.0,
            &format!("{}", self.session.total_lines), text_col, 2.0);

        // Now playing — bottom center
        let track_name = if let Ok(audio) = self.audio.try_lock() {
            audio.track_name.clone()
        } else {
            String::new()
        };
        if !track_name.is_empty() {
            let display_name: String = track_name.chars().take(20).collect();
            let tw = display_name.len() as f32 * 4.0; // approx width at scale 1
            push_text(&mut verts, &mut indices, (w - tw) / 2.0, h - 20.0,
                &display_name.to_uppercase(), dim_col, 1.0);
        }

        // State overlays — full screen
        if self.session.state == GameState::GameOver {
            push_quad(&mut verts, &mut indices, 0.0, 0.0, w, h, rgba_to_f32([180, 0, 0, 60]), 0.08);
            let go_w = 200.0; let go_h = 50.0;
            let go_x = (w - go_w) / 2.0;
            let go_y = (h - go_h) / 2.0;
            push_panel(&mut verts, &mut indices, go_x, go_y, go_w, go_h, 0.09);
            push_text(&mut verts, &mut indices, go_x + 12.0, go_y + 8.0, "GAME OVER",
                rgba_to_f32([255, 80, 80, 255]), 2.0);
            push_text(&mut verts, &mut indices, go_x + 12.0, go_y + 30.0, "ENTER TO RESTART", dim_col, 1.0);
        }

        if self.session.state == GameState::Paused {
            push_quad(&mut verts, &mut indices, 0.0, 0.0, w, h, rgba_to_f32([0, 0, 0, 60]), 0.08);
            let pa_w = 220.0; let pa_h = 120.0;
            let pa_x = (w - pa_w) / 2.0;
            let pa_y = (h - pa_h) / 2.0;
            push_panel(&mut verts, &mut indices, pa_x, pa_y, pa_w, pa_h, 0.09);
            push_text(&mut verts, &mut indices, pa_x + 16.0, pa_y + 8.0, "PAUSED",
                rgba_to_f32([255, 255, 100, 255]), 2.0);
            push_text(&mut verts, &mut indices, pa_x + 16.0, pa_y + 30.0, "L-R  ORBIT", dim_col, 2.0);
            push_text(&mut verts, &mut indices, pa_x + 16.0, pa_y + 50.0, "L-R MOVE", dim_col, 1.0);
            push_text(&mut verts, &mut indices, pa_x + 16.0, pa_y + 62.0, "DN  DROP", dim_col, 1.0);
            push_text(&mut verts, &mut indices, pa_x + 16.0, pa_y + 74.0, "SPC HARD", dim_col, 1.0);
            push_text(&mut verts, &mut indices, pa_x + 16.0, pa_y + 86.0, "UP CW  Z CCW", dim_col, 1.0);
            push_text(&mut verts, &mut indices, pa_x + 16.0, pa_y + 98.0, "P RESUME", dim_col, 1.0);
        }

        // Particles
        self.particles.render(&mut verts, &mut indices);

        (verts, indices)
    }
}

