// Game world — wraps pipeline's GameSession with GUI-specific state.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use rhythm_grid::config::{config_dir, load_settings};
use rhythm_grid::game::*;
use rhythm_grid::grid::*;
use rhythm_grid::input::GameAction;
use rhythm_grid::pieces::*;
use super::audio_output::{self, AudioState};
use super::drawing::Vertex;
use super::particles::ParticleSystem;
use super::renderer::{Uniforms, perspective, look_at, mat4_mul};
use super::theme::*;

pub struct GameWorld {
    pub session: GameSession,
    pub last_tick: Instant,
    pub camera_angle: f32,
    pub(super) preview_angle: f32,
    pub(super) preview_rotation: usize,
    preview_timer: f32,
    pub audio: Arc<Mutex<AudioState>>,
    pub beat_intensity: f32,
    pub amplitude: f32,
    pub bass: f32,
    pub mids: f32,
    pub highs: f32,
    pub particles: ParticleSystem,
    pub(super) prev_beat: bool,
    pub(super) line_clear_anim: Vec<LineClearAnim>,
    pub(super) bg_rings: Vec<BgRing>,
    pub(super) danger_level: f32,
}

/// Expanding ring in the background
pub(super) struct BgRing {
    pub radius: f32,
    pub max_radius: f32,
    pub life: f32,
    pub max_life: f32,
    pub color: [f32; 4],
}

/// Active line clear flash animation
pub(super) struct LineClearAnim {
    pub row: i32,
    pub timer: f32,
    pub color: [f32; 4],
}

pub(super) const LINE_CLEAR_DURATION: f32 = 0.25;

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

    pub fn compute_uniforms(&self, aspect: f32) -> Uniforms {
        let board_cx = WIDTH as f32 / 2.0;
        let board_cy = -(HEIGHT as f32) / 2.0;

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

        let view = look_at(eye, target, up);
        let proj = perspective(1.2, aspect, 0.1, 200.0);
        let vp = mat4_mul(&proj, &view);

        Uniforms::from_mat(vp)
    }

    pub fn build_scene_and_hud(&self) -> ((Vec<Vertex>, Vec<u32>), (Vec<Vertex>, Vec<u32>)) {
        super::scene::build_scene_and_hud(self)
    }
}


