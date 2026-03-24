// Game world — wraps pipeline's GameSession with GUI-specific state.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use rhythm_grid::config::{config_dir, load_settings, save_settings};
use rhythm_grid::game::*;
use rhythm_grid::grid::*;
use rhythm_grid::input::GameAction;
use rhythm_grid::pieces::*;
use rhythm_grid::audio::{RollingEnergy, BeatConfidence};
use rhythm_grid::render::{piece_color, board_state, held_piece_state, next_piece_state, game_status, BoardRenderState, GameStatusRender, HeldPieceRender, NextPieceRender};
use super::audio_output::{self, AudioState};
use super::camera::CameraReactor;
use super::drawing::{Vertex, rgba_to_f32};
use super::effects::AudioFrame;
use super::effects::beat_rings::BeatRings;
use super::effects::hex_background::HexBackground;
use super::effects::fft_visualizer::FftVisualizer;
use super::effects::grid_lines::GridLines;
use super::effects::fireworks::Fireworks;
use super::effects::themes;
use super::particles::ParticleSystem;
use super::renderer::{Uniforms, perspective, look_at, mat4_mul};
use super::theme::{DEFAULT_CAM_ANGLE, THEME};

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
    pub(super) bands: [f32; 7],
    pub(super) peak_bands: [f32; 7],   // slow decay — for visual peak hold indicator
    pub(super) norm_ceil: [f32; 7],    // fast decay — normalization ceiling
    pub(super) bands_norm: [f32; 7],   // each band normalized to its own ceiling (0-1)
    pub(super) band_beat_intensity: [f32; 7], // per-band beat decay (1.0 on beat, decays)
    pub(super) centroid: f32,         // spectral centroid 0-1 (dark↔bright)
    pub(super) flux: f32,             // spectral flux (rate of spectral change)
    pub(super) t_spin_flash: f32, // 1.0 on t-spin, decays to 0
    pub particles: ParticleSystem,
    pub(super) prev_beat: bool,
    pub(super) clearing_cells: Vec<ClearingCell>,
    pub(super) drop_trails: Vec<DropTrail>,
    pub(super) settle_cells: Vec<SettleCell>,
    pub(super) shatter_fragments: Vec<ShatterFragment>,
    pub(super) bg_rings: Vec<BgRing>, // legacy — kept for level-up rings
    pub(super) beat_rings: BeatRings,
    pub(super) hex_background: HexBackground,
    pub(super) fft_vis: FftVisualizer,
    pub(super) grid_lines: GridLines,
    pub(super) fireworks: Fireworks,
    pub(super) effect_flags: themes::EffectFlags,
    pub(super) danger_level: f32,
    pub(super) level_up_flash: f32, // 1.0 on level up, decays to 0.0
    last_level: u32,
    pub(super) window_aspect: f32,
    pub(super) hud_opacity: f32,  // 0.0 = invisible, 1.0 = full
    hud_fade_timer: f32,          // seconds until fade starts
    pub camera: CameraReactor,
    pub(super) audio_frame: AudioFrame,
    pub cursor_pos: [f32; 2],
    pub window_size: [f32; 2],
    pub(super) buttons: Vec<Button>,
    pub(super) fft_locked: bool, // when true, FFT bars don't fade
    pub(super) piece_colors: Option<[[u8; 4]; 7]>,
    pub(super) render_board: BoardRenderState,
    pub(super) render_status: GameStatusRender,
    pub(super) render_held: Option<HeldPieceRender>,
    pub(super) render_next: NextPieceRender,
    pub(super) toast_text: String,
    pub(super) toast_timer: f32,
    pub(super) theme_index: usize,
    pub color_grade: [f32; 3],
    pub(super) music_folder: Option<String>,
    pub saved_window_width: u32,
    pub saved_window_height: u32,
    pub saved_window_x: Option<i32>,
    pub saved_window_y: Option<i32>,
    pub logical_window_size: [u32; 2],
    // Dynamic audio-visual mapping
    rolling_energy: RollingEnergy,
    beat_confidence: BeatConfidence,
    pub(super) bindings: themes::EffectBindings,
    pub(super) resolved_ranks: [usize; 3],  // band indices for rank 1, 2, 3
    pub(super) energy_averages: [f32; 7],   // cached for debug dashboard
    pub(super) confidence_values: [f32; 7], // cached for debug dashboard
    pub(super) track_time: f64,               // seconds into current track
    ranks_locked: bool,                      // true after analysis window
    pub(super) demo_mode: bool,
    pub demo_idle_timer: f32,  // seconds since last player input
    demo_action_timer: f32,   // countdown to next AI action
    demo_rng: u64,
    pub(super) track_queue_rects: Vec<([f32; 4], usize)>, // (x,y,w,h), track_index
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum ButtonId {
    Folder,
    VolUp,
    VolDown,
    FftLock,
    PlayPause,
    Back,
    Skip,
    Shuffle,
}

pub(super) struct Button {
    pub id: ButtonId,
    pub world_x: f32,
    pub world_y: f32,
    pub world_w: f32,
    pub world_h: f32,
    pub screen_rect: [f32; 4],
    pub hovered: bool,
}

/// Expanding ring in the background
pub(super) struct BgRing {
    pub radius: f32,
    pub max_radius: f32,
    pub life: f32,
    pub max_life: f32,
    pub color: [f32; 4],
}

/// Per-cell clearing animation
pub(super) struct ClearingCell {
    pub col: i32,
    pub row: i32,
    pub timer: f32,
    pub _color: [f32; 4],  // original piece color (reserved for future non-white dissolve)
    pub scale: f32,         // 1.0 → 0.0 as it dissolves
}

pub(super) const LINE_CLEAR_DURATION: f32 = 0.4;

pub(super) struct ShatterFragment {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub size: f32,
    pub color: [f32; 4],
    pub timer: f32,
    pub max_life: f32,
}

pub(super) const SHATTER_DURATION: f32 = 0.6;

pub(super) struct DropTrail {
    pub col: i32,
    pub start_row: i32,   // where the piece was before drop
    pub end_row: i32,     // where it landed
    pub type_index: u32,
    pub timer: f32,
}

pub(super) const DROP_TRAIL_DURATION: f32 = 0.2;

pub(super) struct SettleCell {
    pub col: i32,
    pub row: i32,
    pub timer: f32,
}

pub(super) const SETTLE_DURATION: f32 = 0.15;

impl GameWorld {
    /// Resolve a SignalRank to an actual band index using analysis results.
    pub fn resolve_rank(&self, rank: themes::SignalRank) -> usize {
        match rank {
            themes::SignalRank::First => self.resolved_ranks[0],
            themes::SignalRank::Second => self.resolved_ranks[1],
            themes::SignalRank::Third => self.resolved_ranks[2],
            themes::SignalRank::Fixed(band) => band.min(6),
        }
    }

    pub fn themed_piece_color(&self, type_index: u32) -> [u8; 4] {
        if let Some(colors) = &self.piece_colors {
            colors[type_index as usize]
        } else {
            piece_color(type_index)
        }
    }

    pub fn save_settings(&self) {
        let vol = if let Ok(audio) = self.audio.lock() { audio.volume } else { 0.8 };
        let shuffled = if let Ok(audio) = self.audio.lock() { audio.shuffled } else { false };
        let theme_names = ["Default", "Water", "Debug"];
        let settings = rhythm_grid::config::Settings {
            volume: vol,
            speed: 1.0,
            music_folder: self.music_folder.clone(),
            theme: theme_names.get(self.theme_index).unwrap_or(&"Default").to_string(),
            shuffle: shuffled,
            window_width: self.logical_window_size[0],
            window_height: self.logical_window_size[1],
            window_x: self.saved_window_x,
            window_y: self.saved_window_y,
            ..rhythm_grid::config::Settings::default()
        };
        let path = config_dir().join("settings.toml");
        let _ = save_settings(&settings, &path);
    }

    pub fn new() -> Self {
        // Load settings to restore previous state
        let settings_path = config_dir().join("settings.toml");
        let settings = load_settings(&settings_path);
        let (theme, theme_index) = match settings.theme.as_str() {
            "Water" => (themes::water_theme(), 1),
            "Debug" => (themes::debug_theme(), 2),
            _ => (themes::default_theme(), 0),
        };
        let audio = audio_output::start_audio(settings.music_folder.as_deref());
        if let Ok(mut a) = audio.try_lock() { a.volume = settings.volume; }
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
            bands: [0.0; 7],
            peak_bands: [0.0; 7],
            norm_ceil: [0.01; 7],
            bands_norm: [0.0; 7],
            band_beat_intensity: [0.0; 7],
            centroid: 0.0,
            flux: 0.0,
            t_spin_flash: 0.0,
            particles: ParticleSystem::new(),
            prev_beat: false,
            clearing_cells: Vec::new(),
            drop_trails: Vec::new(),
            settle_cells: Vec::new(),
            shatter_fragments: Vec::new(),
            bg_rings: Vec::new(),
            beat_rings: BeatRings::new(theme.rings),
            hex_background: HexBackground::new(theme.hex),
            fft_vis: FftVisualizer::new(theme.fft),
            grid_lines: GridLines::new(theme.grid),
            fireworks: { let mut fw = Fireworks::new(); fw.shells_only = theme.name == "Debug"; fw },
            effect_flags: theme.effects.clone(),
            piece_colors: theme.piece_colors,
            render_board: BoardRenderState { occupied: vec![], active: vec![], ghost: vec![] },
            render_status: GameStatusRender {
                score: 0, level: 1, total_lines: 0, combo_count: 0,
                max_combo: 0, pieces_placed: 0, time_played_secs: 0.0,
                state: GameState::Menu, can_hold: true,
            },
            render_held: None,
            render_next: next_piece_state(&GameSession::new()),
            toast_text: String::new(),
            toast_timer: 0.0,
            theme_index,
            color_grade: theme.color_grade,
            music_folder: settings.music_folder.clone(),
            saved_window_width: settings.window_width,
            saved_window_height: settings.window_height,
            saved_window_x: settings.window_x,
            saved_window_y: settings.window_y,
            logical_window_size: [settings.window_width, settings.window_height],
            rolling_energy: RollingEnergy::new(10.0, 60.0),  // 10s window, ~60fps
            beat_confidence: BeatConfidence::new(),
            bindings: theme.bindings.clone(),
            resolved_ranks: [0, 1, 2],  // default: sub-bass, bass, low-mids
            energy_averages: [0.0; 7],
            confidence_values: [0.0; 7],
            track_time: 0.0,
            ranks_locked: false,
            danger_level: 0.0,
            level_up_flash: 0.0,
            last_level: 1,
            window_aspect: THEME.win_w as f32 / THEME.win_h as f32,
            hud_opacity: 1.0,
            hud_fade_timer: 1.5,
            camera: CameraReactor::new(theme.camera),
            audio_frame: AudioFrame {
                bands: [0.0; 7], bands_norm: [0.0; 7], peak_bands: [0.0; 7],
                band_beats: [0.0; 7], centroid: 0.0, flux: 0.0, danger: 0.0, dt: 0.0,
            },
            cursor_pos: [0.0; 2],
            window_size: [THEME.win_w as f32, THEME.win_h as f32],
            buttons: vec![
                Button { id: ButtonId::VolDown, world_x: 12.5, world_y: 15.5, world_w: 0.5, world_h: 0.5, screen_rect: [0.0; 4], hovered: false },
                Button { id: ButtonId::VolUp, world_x: 15.0, world_y: 15.5, world_w: 0.5, world_h: 0.5, screen_rect: [0.0; 4], hovered: false },
                Button { id: ButtonId::FftLock, world_x: -4.5, world_y: 19.2, world_w: 1.44, world_h: 0.3, screen_rect: [0.0; 4], hovered: false },
                Button { id: ButtonId::Back, world_x: 12.5, world_y: 17.0, world_w: 0.6, world_h: 0.5, screen_rect: [0.0; 4], hovered: false },
                Button { id: ButtonId::PlayPause, world_x: 13.3, world_y: 17.0, world_w: 0.6, world_h: 0.5, screen_rect: [0.0; 4], hovered: false },
                Button { id: ButtonId::Skip, world_x: 14.1, world_y: 17.0, world_w: 0.6, world_h: 0.5, screen_rect: [0.0; 4], hovered: false },
                Button { id: ButtonId::Shuffle, world_x: 14.9, world_y: 17.0, world_w: 0.6, world_h: 0.5, screen_rect: [0.0; 4], hovered: false },
                Button { id: ButtonId::Folder, world_x: 12.5, world_y: 18.2, world_w: 3.0, world_h: 0.6, screen_rect: [0.0; 4], hovered: false },
            ],
            fft_locked: false,
            demo_mode: false,
            demo_idle_timer: 0.0,
            demo_action_timer: 0.0,
            demo_rng: 0xDEADBEEF42,
            track_queue_rects: Vec::new(),
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
            self.bands = audio.bands;
            self.centroid = audio.centroid;
            self.flux = audio.flux;
            for i in 0..7 {
                if audio.band_beats[i] {
                    self.band_beat_intensity[i] = 1.0;
                }
            }
            audio.band_beats = [false; 7]; // clear after reading
            got_beat = audio.beat_intensity > 0.9; // fresh beat

            // Feed analysis trackers
            self.rolling_energy.update(&self.bands);
            self.beat_confidence.update(&[
                self.band_beat_intensity[0] > 0.95,
                self.band_beat_intensity[1] > 0.95,
                self.band_beat_intensity[2] > 0.95,
                self.band_beat_intensity[3] > 0.95,
                self.band_beat_intensity[4] > 0.95,
                self.band_beat_intensity[5] > 0.95,
                self.band_beat_intensity[6] > 0.95,
            ], self.track_time);
            self.energy_averages = self.rolling_energy.averages();
            self.confidence_values = self.beat_confidence.confidence();
        }

        // Track time + two-phase rank resolution
        // Phase 1: sample 0-7s, lock at 7s (catches songs that start strong)
        // Phase 2: sample 30-45s, reapply at 45s (catches slow ramp-ups)
        self.track_time += dt;
        let band_names = ["SUB", "BASS", "LMID", "MID", "UMID", "PRES", "BRIL"];

        // Resolve ranks from current analysis state
        let resolve = |energy: &RollingEnergy, conf: &BeatConfidence| -> [usize; 3] {
            let dominant = energy.dominant_bands(3);
            let confidence = conf.confidence();
            let rank1 = *dominant.iter()
                .max_by(|&&a, &&b| confidence[a].partial_cmp(&confidence[b]).unwrap())
                .unwrap_or(&0);
            let others: Vec<usize> = dominant.iter().filter(|&&b| b != rank1).copied().collect();
            let rank2 = others.first().copied().unwrap_or(1);
            let rank3 = others.get(1).copied().unwrap_or(2);
            [rank1, rank2, rank3]
        };

        // Phase labels
        if self.track_time < 7.0 {
            self.toast_text = format!("SAMPLING {:.0}S", 7.0 - self.track_time);
            self.toast_timer = 2.0;
        } else if self.track_time >= 30.0 && self.track_time < 45.0 {
            self.toast_text = format!("RESAMPLING {:.0}S", 45.0 - self.track_time);
            self.toast_timer = 2.0;
        }

        // Phase 1: first lock at 7s
        if !self.ranks_locked && self.track_time > 7.0 {
            self.resolved_ranks = resolve(&self.rolling_energy, &self.beat_confidence);
            self.ranks_locked = true;
            let [r1, r2, r3] = self.resolved_ranks;
            self.toast_text = format!("MAPPED: {} {} {}",
                band_names[r1], band_names[r2], band_names[r3]);
            self.toast_timer = 3.0;
        }

        // Phase 2: reapply at 45s after 15s of fresh sampling
        if self.track_time > 45.0 && self.track_time - dt <= 45.0 {
            let new_ranks = resolve(&self.rolling_energy, &self.beat_confidence);
            if new_ranks != self.resolved_ranks {
                self.resolved_ranks = new_ranks;
                let [r1, r2, r3] = self.resolved_ranks;
                self.toast_text = format!("REMAPPED: {} {} {}",
                    band_names[r1], band_names[r2], band_names[r3]);
                self.toast_timer = 3.0;
            }
        }

        // Peak hold (slow decay — for visual indicator on FFT bars)
        let peak_decay = dt as f32 * 0.4;
        // Normalization ceiling (fast decay — adapts to current song section)
        let ceil_decay = dt as f32 * 0.05;
        for i in 0..7 {
            // Visual peak
            self.peak_bands[i] = if self.bands[i] > self.peak_bands[i] {
                self.bands[i]
            } else {
                (self.peak_bands[i] - peak_decay).max(self.bands[i])
            };
            // Normalization ceiling — snaps up instantly, decays slowly toward current level
            if self.bands[i] > self.norm_ceil[i] {
                self.norm_ceil[i] = self.bands[i];
            } else {
                self.norm_ceil[i] = (self.norm_ceil[i] - ceil_decay).max(0.01);
            }
            // Normalize
            self.bands_norm[i] = (self.bands[i] / self.norm_ceil[i]).min(1.0);
        }

        // (effect modules updated below after AudioFrame is built)

        // Level-up rings still use legacy bg_rings vec
        for ring in &mut self.bg_rings {
            let progress = 1.0 - ring.life / ring.max_life;
            ring.radius = 0.5 + progress * ring.max_radius;
            ring.life -= dt as f32;
        }
        self.bg_rings.retain(|r| r.life > 0.0);

        // Upper-mids/presence beats (bands 4-5) → particle burst
        let w = THEME.win_w as f32;
        let h = THEME.win_h as f32;
        let bw = w * 0.35;
        let bh = h * 0.85;
        let bx = (w - bw) / 2.0;
        let by = (h - bh) / 2.0;
        if self.effect_flags.particle_beat_pulse {
            for band in 4..6 {
                if self.band_beat_intensity[band] > 0.95 {
                    self.particles.spawn_beat_pulse(bx, by, bw, bh, 0.6);
                }
            }
            if got_beat && !self.prev_beat && self.band_beat_intensity[4..6].iter().all(|&b| b < 0.95) {
                self.particles.spawn_beat_pulse(bx, by, bw, bh, 1.0);
            }
        }
        self.prev_beat = got_beat;

        // Update particles and line clear animations
        self.particles.update(dt as f32);
        // HUD auto-fade (1.5s delay, then fast fade)
        self.hud_fade_timer -= dt as f32;
        if self.hud_fade_timer <= 0.0 {
            self.hud_opacity = (self.hud_opacity - dt as f32 * 3.5).max(0.0);
        }
        // Full opacity when paused or game over
        if self.session.state == GameState::Paused || self.session.state == GameState::GameOver {
            self.hud_opacity = 1.0;
            self.hud_fade_timer = 1.5;
        }

        for cell in &mut self.clearing_cells {
            cell.timer -= dt as f32;
            let progress = 1.0 - (cell.timer / LINE_CLEAR_DURATION).max(0.0);
            cell.scale = 1.0 - progress; // shrink to 0
        }
        self.clearing_cells.retain(|c| c.timer > 0.0);

        // Drop trail decay
        for trail in &mut self.drop_trails {
            trail.timer -= dt as f32;
        }
        self.drop_trails.retain(|t| t.timer > 0.0);

        // Shatter fragment physics
        for frag in &mut self.shatter_fragments {
            frag.timer -= dt as f32;
            frag.x += frag.vx * dt as f32;
            frag.y += frag.vy * dt as f32;
            frag.vy += 8.0 * dt as f32; // gravity (positive = downward in row coords)
            frag.vx *= 0.97; // drag
        }
        self.shatter_fragments.retain(|f| f.timer > 0.0);

        // Settle animation decay
        for cell in &mut self.settle_cells {
            cell.timer -= dt as f32;
        }
        self.settle_cells.retain(|c| c.timer > 0.0);

        // Level up detection
        let current_level = level_for_lines(self.session.total_lines);
        if current_level > self.last_level {
            self.hud_opacity = 1.0;
            self.hud_fade_timer = 2.0;
            if self.effect_flags.level_up_rings {
                self.level_up_flash = 1.0;
            }
            // Spawn celebratory rings
            for i in 0..3 {
                self.bg_rings.push(BgRing {
                    radius: 0.5 + i as f32 * 0.3,
                    max_radius: 25.0,
                    life: 2.5 - i as f32 * 0.3,
                    max_life: 2.5 - i as f32 * 0.3,
                    color: [0.3, 0.8, 1.0, 0.5], // bright cyan
                });
            }
            // Burst of particles from board center
            let w = THEME.win_w as f32;
            let h = THEME.win_h as f32;
            let cx = w / 2.0;
            let cy = h / 2.0;
            for _ in 0..120 {
                let angle = self.preview_angle + (self.particles.particles.len() as f32 * 0.7);
                let speed = 50.0 + (angle.sin().abs() * 80.0);
                let vx = angle.cos() * speed;
                let vy = angle.sin() * speed;
                self.particles.particles.push(super::particles::Particle {
                    x: cx + (angle * 3.0).cos() * 20.0,
                    y: cy + (angle * 3.0).sin() * 20.0,
                    vx, vy,
                    life: 3.0,
                    max_life: 3.0,
                    color: [0.4, 0.9, 1.0, 0.9],
                    size: 0.75,
                });
            }
            self.last_level = current_level;
        }
        self.level_up_flash = (self.level_up_flash - dt as f32 * 1.5).max(0.0);

        // Build AudioFrame BEFORE decay so effects see fresh beat triggers
        self.audio_frame = AudioFrame {
            bands: self.bands,
            bands_norm: self.bands_norm,
            peak_bands: self.peak_bands,
            band_beats: self.band_beat_intensity,
            centroid: self.centroid,
            flux: self.flux,
            danger: self.danger_level,
            dt: dt as f32,
        };

        // Update all effect modules + camera (guarded by effect_flags)
        use super::effects::AudioEffect;
        let ef = &self.effect_flags;
        // Set resolved band indices on effects before update
        if ef.beat_rings {
            self.beat_rings.trigger_band = self.resolve_rank(self.bindings.beat_rings);
            self.beat_rings.update(&self.audio_frame);
        }
        if ef.hex_background { self.hex_background.update(&self.audio_frame); }
        self.fft_vis.locked = self.fft_locked;
        self.fft_vis.lock_hovered = self.btn_hovered(ButtonId::FftLock);
        if ef.fft_visualizer { self.fft_vis.update(&self.audio_frame); }
        if ef.grid_lines {
            self.grid_lines.distortion_enabled = ef.grid_distortion;
            self.grid_lines.update(&self.audio_frame);
        }
        if ef.fireworks {
            self.fireworks.trigger_band = Some(self.resolve_rank(self.bindings.fireworks));
            self.fireworks.update(&self.audio_frame);
        }
        if ef.camera_sway { self.camera.update(&self.audio_frame); }

        // Decay AFTER effects have consumed the frame
        for i in 0..7 {
            self.band_beat_intensity[i] = (self.band_beat_intensity[i] - dt as f32 * 8.0).max(0.0);
        }

        // T-spin flash decay
        self.t_spin_flash = (self.t_spin_flash - dt as f32 * 1.0).max(0.0);
        self.toast_timer = (self.toast_timer - dt as f32).max(0.0);

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
            self.render_board = board_state(&self.session);
            self.render_status = game_status(&self.session);
            self.render_held = held_piece_state(&self.session);
            self.render_next = next_piece_state(&self.session);
            return;
        }
        self.last_tick = now;

        // Capture pre-tick state for visual effects
        let pre_piece = self.session.active_piece;
        let pre_was_rotate = self.session.last_move_was_rotate;
        let pre_is_t_spin = detect_t_spin(&self.session.grid, &pre_piece, pre_was_rotate);

        match tick(&mut self.session, dt) {
            TickResult::PieceLocked { lines_cleared } => {
                // Settle animation on the locked piece cells
                let lock_cells = piece_cells(pre_piece.piece_type, pre_piece.rotation);
                for &(dr, dc) in &lock_cells {
                    let r = pre_piece.row + dr;
                    let c = pre_piece.col + dc;
                    if r >= 0 && r < HEIGHT as i32 {
                        self.settle_cells.push(SettleCell { col: c, row: r, timer: SETTLE_DURATION });
                    }
                }
                if pre_is_t_spin {
                    self.t_spin_flash = 1.0;
                }
                if lines_cleared > 0 {
                    let cells = piece_cells(pre_piece.piece_type, pre_piece.rotation);
                    let max_dr = cells.iter().map(|(dr, _)| *dr).max().unwrap_or(0);
                    let piece_row = pre_piece.row + max_dr;
                    if self.effect_flags.line_clear_particles {
                        self.spawn_line_clear_particles(lines_cleared, piece_row);
                    }
                    // Shatter fragments for cleared rows (tick path)
                    if self.effect_flags.clearing_flash {
                        self.spawn_shatter_for_row_range(piece_row - lines_cleared as i32 + 1, lines_cleared);
                    }
                    if self.effect_flags.camera_shake {
                        self.camera.trigger_shake((lines_cleared as f32 * 0.3).min(1.0));
                    }
                    if self.effect_flags.grid_distortion {
                        let cx = pre_piece.col as f32;
                        let cy = piece_row as f32;
                        self.grid_lines.add_force(cx, cy, lines_cleared as f32 * 0.6);
                    }
                }
                // Secondary game over check: if new piece spawned in vanish zone
                // and can't move down, the board is full
                if self.session.active_piece.row < 0 {
                    let cells = piece_cells(self.session.active_piece.piece_type, 0);
                    if !is_valid_position(&self.session.grid, &cells,
                        self.session.active_piece.row + 1, self.session.active_piece.col) {
                        self.session.state = GameState::GameOver;
                    }
                }
            }
            _ => {}
        }

        // Demo mode: auto-play when idle
        const DEMO_IDLE_THRESHOLD: f32 = 15.0; // seconds before demo activates
        self.demo_idle_timer += dt as f32;
        if self.demo_idle_timer >= DEMO_IDLE_THRESHOLD && !self.demo_mode {
            self.demo_mode = true;
            // If game over, restart
            if self.session.state == GameState::GameOver {
                self.session = GameSession::new();
                self.clearing_cells.clear();
                self.bg_rings.clear();
                self.danger_level = 0.0;
                self.level_up_flash = 0.0;
                self.last_level = 1;
                self.camera.reset();
            }
        }
        if self.demo_mode && self.session.state == GameState::Playing {
            self.demo_action_timer -= dt as f32;
            if self.demo_action_timer <= 0.0 {
                // Random action
                self.demo_rng = self.demo_rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let action = (self.demo_rng >> 33) % 10;
                match action {
                    0..=2 => { self.session.move_horizontal(-1); }  // move left
                    3..=5 => { self.session.move_horizontal(1); }   // move right
                    6 => { self.session.rotate(true); }             // rotate CW
                    7 => { self.session.rotate(false); }            // rotate CCW
                    8 => { move_down(&self.session.grid, &mut self.session.active_piece); } // soft drop
                    _ => { self.session.hard_drop(); }              // hard drop
                    }
                // Faster actions at higher levels
                let level = level_for_lines(self.session.total_lines);
                self.demo_action_timer = 0.15 - (level as f32 * 0.005).min(0.1);
            }
            // Auto-restart on game over in demo mode
            if self.session.state == GameState::GameOver {
                self.session = GameSession::new();
                self.clearing_cells.clear();
                self.bg_rings.clear();
                self.danger_level = 0.0;
                self.level_up_flash = 0.0;
                self.last_level = 1;
                self.camera.reset();
            }
        }

        // Update render state snapshot for scene.rs
        self.render_board = board_state(&self.session);
        self.render_status = game_status(&self.session);
        self.render_held = held_piece_state(&self.session);
        self.render_next = next_piece_state(&self.session);
    }

    pub fn cycle_theme(&mut self) {
        let theme_fns: &[fn() -> themes::VisualTheme] = &[
            themes::default_theme,
            themes::water_theme,
            themes::debug_theme,
        ];
        self.theme_index = (self.theme_index + 1) % theme_fns.len();
        let theme = theme_fns[self.theme_index]();
        self.effect_flags = theme.effects.clone();
        self.bindings = theme.bindings.clone();
        self.color_grade = theme.color_grade;
        self.piece_colors = theme.piece_colors;
        self.beat_rings = BeatRings::new(theme.rings);
        self.hex_background = HexBackground::new(theme.hex);
        self.fft_vis = FftVisualizer::new(theme.fft);
        self.grid_lines = GridLines::new(theme.grid);
        self.camera = CameraReactor::new(theme.camera);
        self.fireworks.shells_only = theme.name == "Debug";
        self.toast_text = format!("THEME: {}", theme.name.to_uppercase());
        self.toast_timer = 2.0;
        self.save_settings();
    }

    pub fn hold_piece(&mut self) {
        self.exit_demo();
        if self.session.state == GameState::Playing {
            self.session.hold_piece();
        }
    }

    pub fn toggle_audio_pause(&mut self) {
        if let Ok(mut audio) = self.audio.lock() {
            audio.paused = !audio.paused;
        }
        self.on_mouse_activity();
    }

    pub fn prev_track(&mut self) {
        if let Ok(mut audio) = self.audio.lock() {
            audio.back_requested = true;
        }
        self.on_mouse_activity();
    }

    pub fn toggle_shuffle(&mut self) {
        if let Ok(mut audio) = self.audio.lock() {
            audio.shuffle_requested = true;
            audio.shuffled = !audio.shuffled;
            let state = if audio.shuffled { "ON" } else { "OFF" };
            self.toast_text = format!("SHUFFLE {}", state);
            self.toast_timer = 1.5;
        }
        self.on_mouse_activity();
    }

    pub fn skip_track(&mut self) {
        if let Ok(mut audio) = self.audio.lock() {
            audio.skip_requested = true;
        }
        self.on_mouse_activity();
    }

    pub fn adjust_volume(&mut self, delta: f32) {
        if let Ok(mut audio) = self.audio.lock() {
            audio.volume = (audio.volume + delta).clamp(0.0, 1.0);
        }
        self.on_mouse_activity();
        self.save_settings();
    }

    /// Call when mouse moves to reveal HUD
    pub fn on_mouse_activity(&mut self) {
        self.hud_opacity = 1.0;
        self.hud_fade_timer = 1.5;
    }

    fn exit_demo(&mut self) {
        if self.demo_mode {
            self.demo_mode = false;
            // Restart fresh game when exiting demo
            self.session = GameSession::new();
            self.last_tick = Instant::now();
            self.clearing_cells.clear();
            self.bg_rings.clear();
            self.danger_level = 0.0;
            self.level_up_flash = 0.0;
            self.last_level = 1;
            self.camera.reset();
        }
        self.demo_idle_timer = 0.0;
    }

    pub fn handle_action(&mut self, action: GameAction) {
        self.exit_demo();
        match self.session.state {
            GameState::Playing => match action {
                GameAction::MoveLeft => { self.session.move_horizontal(-1); }
                GameAction::MoveRight => { self.session.move_horizontal(1); }
                GameAction::SoftDrop => {
                    move_down(&self.session.grid, &mut self.session.active_piece);
                    self.session.gravity_accumulator_ms = 0;
                }
                GameAction::HardDrop => {
                    // Capture landing position and clearing cells before hard_drop
                    let cells = piece_cells(self.session.active_piece.piece_type, self.session.active_piece.rotation);
                    let max_dr = cells.iter().map(|(dr, _)| *dr).max().unwrap_or(0);
                    let mut land_row = self.session.active_piece.row;
                    while is_valid_position(&self.session.grid, &cells, land_row + 1, self.session.active_piece.col) {
                        land_row += 1;
                    }
                    let land_bottom = land_row + max_dr;

                    // Simulate placement to capture clearing cells for dissolve animation
                    let piece_type = self.session.active_piece.piece_type;
                    let piece_col = self.session.active_piece.col;
                    for &(dr, dc) in &cells {
                        let r = land_row + dr;
                        let c = piece_col + dc;
                        if r >= 0 && r < HEIGHT as i32 && c >= 0 && c < WIDTH as i32 {
                            self.session.grid.cells[r as usize][c as usize] = CellState::Occupied(piece_type as u32);
                        }
                    }
                    let mut cleared_rows = Vec::new();
                    for row in 0..HEIGHT {
                        if self.session.grid.cells[row].iter().all(|c| *c != CellState::Empty) {
                            cleared_rows.push(row as i32);
                            for col in 0..WIDTH {
                                if let CellState::Occupied(ti) = self.session.grid.cells[row][col] {
                                    self.clearing_cells.push(ClearingCell {
                                        col: col as i32, row: row as i32,
                                        timer: LINE_CLEAR_DURATION,
                                        _color: rgba_to_f32(self.themed_piece_color(ti)),
                                        scale: 1.0,
                                    });
                                }
                            }
                        }
                    }
                    // Shatter fragments for cleared rows (hard drop path)
                    for &row in &cleared_rows {
                        self.spawn_shatter_for_row_range(row, 1);
                    }
                    // Undo simulation
                    for &(dr, dc) in &cells {
                        let r = land_row + dr;
                        let c = piece_col + dc;
                        if r >= 0 && r < HEIGHT as i32 && c >= 0 && c < WIDTH as i32 {
                            self.session.grid.cells[r as usize][c as usize] = CellState::Empty;
                        }
                    }

                    // Spawn drop trails per piece cell (column streak from start to land)
                    let start_row = self.session.active_piece.row;
                    if land_row > start_row {
                        for &(dr, dc) in &cells {
                            let col = piece_col + dc;
                            let sr = start_row + dr;
                            let er = land_row + dr;
                            if col >= 0 && col < WIDTH as i32 {
                                self.drop_trails.push(DropTrail {
                                    col,
                                    start_row: sr,
                                    end_row: er,
                                    type_index: piece_type as u32,
                                    timer: DROP_TRAIL_DURATION,
                                });
                            }
                        }
                    }

                    // Settle animation on landing cells
                    for &(dr, dc) in &cells {
                        let r = land_row + dr;
                        let c = piece_col + dc;
                        if r >= 0 && r < HEIGHT as i32 {
                            self.settle_cells.push(SettleCell { col: c, row: r, timer: SETTLE_DURATION });
                        }
                    }

                    // Use session method — handles lock, score, spawn, lock delay reset
                    let result = self.session.hard_drop();
                    let lines = match result {
                        TickResult::PieceLocked { lines_cleared } => lines_cleared,
                        _ => 0,
                    };
                    if lines > 0 && self.effect_flags.line_clear_particles {
                        self.spawn_line_clear_particles(lines, land_bottom);
                    }
                    if self.effect_flags.camera_shake {
                        self.camera.trigger_shake((0.2 + lines as f32 * 0.25).min(1.0));
                    }
                    if self.effect_flags.grid_distortion {
                        let cx = self.session.active_piece.col as f32;
                        self.grid_lines.add_force(cx, land_bottom as f32, 0.3 + lines as f32 * 0.4);
                    }
                    // Secondary game over check for vanish zone spawn
                    if self.session.active_piece.row < 0 && self.session.state == GameState::Playing {
                        let hd_cells = piece_cells(self.session.active_piece.piece_type, 0);
                        if !is_valid_position(&self.session.grid, &hd_cells,
                            self.session.active_piece.row + 1, self.session.active_piece.col) {
                            self.session.state = GameState::GameOver;
                        }
                    }
                }
                GameAction::RotateCW => { self.session.rotate(true); }
                GameAction::RotateCCW => { self.session.rotate(false); }
                GameAction::Hold => { self.session.hold_piece(); }
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
                    // Reset game state without restarting audio
                    self.session = GameSession::new();
                    self.last_tick = Instant::now();
                    self.clearing_cells.clear();
                    self.bg_rings.clear();
                    self.danger_level = 0.0;
                    self.level_up_flash = 0.0;
                    self.last_level = 1;
                    self.camera.reset();
                    }
            }
        }
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

    fn spawn_shatter_for_row_range(&mut self, top_row: i32, lines: u32) {
        // Use a simple deterministic scatter based on cell position
        let mut seed = (top_row as u32).wrapping_mul(31).wrapping_add(lines * 17);
        let pseudo = |s: &mut u32| -> f32 {
            *s = s.wrapping_mul(1103515245).wrapping_add(12345);
            ((*s >> 16) & 0x7FFF) as f32 / 32767.0
        };
        for row in top_row..(top_row + lines as i32) {
            if row < 0 || row >= HEIGHT as i32 { continue; }
            for col in 0..WIDTH as i32 {
                let cx = col as f32 + 0.5;
                let cy = row as f32 + 0.5;
                let frags = 3 + (pseudo(&mut seed) * 2.0) as u32; // 3-4 fragments per cell
                for _ in 0..frags {
                    let angle = pseudo(&mut seed) * std::f32::consts::TAU;
                    let speed = 2.0 + pseudo(&mut seed) * 4.0;
                    let size = 0.1 + pseudo(&mut seed) * 0.2;
                    self.shatter_fragments.push(ShatterFragment {
                        x: cx,
                        y: cy,
                        vx: angle.cos() * speed,
                        vy: angle.sin() * speed,
                        size,
                        color: [1.0, 1.0, 1.0, 0.9],
                        timer: SHATTER_DURATION,
                        max_life: SHATTER_DURATION,
                    });
                }
            }
        }
    }

    /// Project a world-space point to screen pixel coords using the VP matrix.
    fn project_to_screen(vp: &[[f32; 4]; 4], point: [f32; 3], win_w: f32, win_h: f32) -> [f32; 2] {
        let [x, y, z] = point;
        // Multiply by VP matrix (column-major)
        let cx = vp[0][0]*x + vp[1][0]*y + vp[2][0]*z + vp[3][0];
        let cy = vp[0][1]*x + vp[1][1]*y + vp[2][1]*z + vp[3][1];
        let cw = vp[0][3]*x + vp[1][3]*y + vp[2][3]*z + vp[3][3];
        if cw.abs() < 1e-6 { return [0.0, 0.0]; }
        let nx = cx / cw;
        let ny = cy / cw;
        // NDC [-1,1] to pixels
        let px = (nx + 1.0) * 0.5 * win_w;
        let py = (1.0 - ny) * 0.5 * win_h;
        [px, py]
    }

    /// Update screen-space button rects from current VP matrix.
    pub fn update_button_rects(&mut self, uniforms: &Uniforms, _aspect: f32) {
        let vp = &uniforms.view_proj;
        let [win_w, win_h] = self.window_size;
        let [mx, my] = self.cursor_pos;
        for btn in &mut self.buttons {
            // push_slab_3d uses y-down: world y0 = -world_y, y1 = -(world_y + world_h)
            let tl = Self::project_to_screen(vp, [btn.world_x, -btn.world_y, 0.5], win_w, win_h);
            let br = Self::project_to_screen(vp, [btn.world_x + btn.world_w, -(btn.world_y + btn.world_h), 0.5], win_w, win_h);
            let x = tl[0].min(br[0]);
            let y = tl[1].min(br[1]);
            let w = (tl[0] - br[0]).abs();
            let h = (tl[1] - br[1]).abs();
            btn.screen_rect = [x, y, w, h];
            btn.hovered = mx >= x && mx <= x + w && my >= y && my <= y + h;
        }
    }

    pub fn update_track_queue_rects(&mut self) {
        self.track_queue_rects.clear();
        let vol_rect = self.btn_rect(ButtonId::VolDown);
        let track_x = vol_rect[0];
        let track_bottom = vol_rect[1] - 8.0;
        if let Ok(audio) = self.audio.try_lock() {
            let list = &audio.track_list;
            let idx = audio.current_track_index;
            let num_shown = 4.min(list.len());
            if !list.is_empty() {
                let track_top = track_bottom - num_shown as f32 * 10.0;
                for i in 0..num_shown {
                    let track_idx = (idx + i) % list.len();
                    let y = track_top + i as f32 * 10.0;
                    self.track_queue_rects.push(([track_x, y, 120.0, 10.0], track_idx));
                }
            }
        }
    }

    pub(super) fn btn_hovered(&self, id: ButtonId) -> bool {
        self.buttons.iter().any(|b| b.id == id && b.hovered)
    }

    pub(super) fn btn_rect(&self, id: ButtonId) -> [f32; 4] {
        self.buttons.iter().find(|b| b.id == id).map(|b| b.screen_rect).unwrap_or([0.0; 4])
    }

    pub fn handle_click(&mut self) {
        let clicked = self.buttons.iter().find(|b| b.hovered).map(|b| b.id);
        match clicked {
            Some(ButtonId::Folder) => self.pick_music_folder(),
            Some(ButtonId::VolUp) => self.adjust_volume(0.05),
            Some(ButtonId::VolDown) => self.adjust_volume(-0.05),
            Some(ButtonId::FftLock) => { self.fft_locked = !self.fft_locked; self.on_mouse_activity(); }
            Some(ButtonId::PlayPause) => self.toggle_audio_pause(),
            Some(ButtonId::Back) => self.prev_track(),
            Some(ButtonId::Skip) => self.skip_track(),
            Some(ButtonId::Shuffle) => self.toggle_shuffle(),
            None => {
                // Check track queue clicks
                let [mx, my] = self.cursor_pos;
                let mut jump_idx = None;
                for &(rect, track_idx) in &self.track_queue_rects {
                    if mx >= rect[0] && mx <= rect[0] + rect[2]
                        && my >= rect[1] && my <= rect[1] + rect[3] {
                        jump_idx = Some(track_idx);
                        break;
                    }
                }
                if let Some(idx) = jump_idx {
                    if let Ok(mut audio) = self.audio.lock() {
                        audio.jump_to_requested = Some(idx);
                    }
                    self.on_mouse_activity();
                }
            }
        }
    }

    fn pick_music_folder(&mut self) {
        let was_playing = self.session.state == GameState::Playing;
        if was_playing {
            self.session.state = GameState::Paused;
        }
        self.hud_opacity = 1.0;
        self.hud_fade_timer = 1.5;

        let folder = rfd::FileDialog::new()
            .set_title("Select Music Folder")
            .pick_folder();

        if let Some(path) = folder {
            let folder_str = path.to_string_lossy().to_string();
            self.music_folder = Some(folder_str.clone());
            self.save_settings();
            // Shut down old audio before starting new
            if let Ok(mut audio) = self.audio.lock() {
                audio.shutdown = true;
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
            self.audio = audio_output::start_audio(Some(&folder_str));
        }

        if was_playing {
            self.session.state = GameState::Playing;
            self.last_tick = Instant::now();
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
        let base_eye = [board_cx + orbit, board_cy, 16.0];
        let [cam_x, cam_y, cam_z] = self.camera.apply(&self.audio_frame, self.preview_angle, base_eye);

        let eye = [cam_x, cam_y, cam_z];
        let target = [board_cx, board_cy, 0.0];
        let up = [0.0, 1.0, 0.0];

        let view = look_at(eye, target, up);
        let proj = perspective(1.2, aspect, 0.1, 200.0);
        let vp = mat4_mul(&proj, &view);

        Uniforms { view_proj: vp, camera_pos: [cam_x, cam_y, cam_z, 0.0] }
    }

    pub fn build_scene_and_hud(&self) -> ((Vec<Vertex>, Vec<u32>), (Vec<Vertex>, Vec<u32>), (Vec<Vertex>, Vec<u32>)) {
        super::scene::build_scene_and_hud(self)
    }
}
