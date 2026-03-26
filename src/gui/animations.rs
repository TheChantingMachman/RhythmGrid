// Animation state — manages all transient visual animations triggered by game events.
// Owns clearing cells, drop trails, settle squish, shatter fragments, level-up rings,
// and flash timers. Updated each frame with physics/decay.

use rhythm_grid::grid::{WIDTH, HEIGHT};

pub struct BgRing {
    pub radius: f32,
    pub max_radius: f32,
    pub life: f32,
    pub max_life: f32,
    pub color: [f32; 4],
}

/// Per-cell clearing animation (fields used for spawn tracking; rendering replaced by shatter)
#[allow(dead_code)]
pub struct ClearingCell {
    pub col: i32,
    pub row: i32,
    pub timer: f32,
    pub _color: [f32; 4],
    pub scale: f32,
}

pub struct ShatterFragment {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub size: f32,
    pub color: [f32; 4],
    pub timer: f32,
    pub max_life: f32,
}

pub struct DropTrail {
    pub col: i32,
    pub start_row: i32,
    pub end_row: i32,
    pub type_index: u32,
    pub timer: f32,
}

pub struct SettleCell {
    pub col: i32,
    pub row: i32,
    pub timer: f32,
}

pub const LINE_CLEAR_DURATION: f32 = 0.4;
pub const SHATTER_DURATION: f32 = 0.3;
pub const DROP_TRAIL_DURATION: f32 = 0.2;
pub const SETTLE_DURATION: f32 = 0.15;

pub struct Animations {
    pub clearing_cells: Vec<ClearingCell>,
    pub drop_trails: Vec<DropTrail>,
    pub settle_cells: Vec<SettleCell>,
    pub shatter_fragments: Vec<ShatterFragment>,
    pub bg_rings: Vec<BgRing>,
    pub t_spin_flash: f32,
    pub level_up_flash: f32,
}

impl Animations {
    pub fn new() -> Self {
        Animations {
            clearing_cells: Vec::new(),
            drop_trails: Vec::new(),
            settle_cells: Vec::new(),
            shatter_fragments: Vec::new(),
            bg_rings: Vec::new(),
            t_spin_flash: 0.0,
            level_up_flash: 0.0,
        }
    }

    /// Update all animation timers and physics. Call once per frame.
    pub fn update(&mut self, dt: f32) {
        // Level-up rings
        for ring in &mut self.bg_rings {
            let progress = 1.0 - ring.life / ring.max_life;
            ring.radius = 0.5 + progress * ring.max_radius;
            ring.life -= dt;
        }
        self.bg_rings.retain(|r| r.life > 0.0);

        // Clearing cells dissolve
        for cell in &mut self.clearing_cells {
            cell.timer -= dt;
            let progress = 1.0 - (cell.timer / LINE_CLEAR_DURATION).max(0.0);
            cell.scale = 1.0 - progress;
        }
        self.clearing_cells.retain(|c| c.timer > 0.0);

        // Drop trail decay
        for trail in &mut self.drop_trails {
            trail.timer -= dt;
        }
        self.drop_trails.retain(|t| t.timer > 0.0);

        // Shatter fragment physics
        for frag in &mut self.shatter_fragments {
            frag.timer -= dt;
            frag.x += frag.vx * dt;
            frag.y += frag.vy * dt;
            frag.vy += 4.0 * dt; // gravity (halved)
            frag.vx *= 0.97;     // drag
        }
        self.shatter_fragments.retain(|f| f.timer > 0.0);

        // Settle animation decay
        for cell in &mut self.settle_cells {
            cell.timer -= dt;
        }
        self.settle_cells.retain(|c| c.timer > 0.0);

        // Flash decays
        self.t_spin_flash = (self.t_spin_flash - dt * 1.0).max(0.0);
        self.level_up_flash = (self.level_up_flash - dt * 1.5).max(0.0);
    }

    /// Spawn level-up celebratory rings.
    pub fn spawn_level_up_rings(&mut self) {
        self.level_up_flash = 1.0;
        for i in 0..3 {
            self.bg_rings.push(BgRing {
                radius: 0.5 + i as f32 * 0.3,
                max_radius: 25.0,
                life: 2.5 - i as f32 * 0.3,
                max_life: 2.5 - i as f32 * 0.3,
                color: [0.3, 0.8, 1.0, 0.5],
            });
        }
    }

    /// Spawn shatter fragments for cleared rows.
    pub fn spawn_shatter_for_row_range(&mut self, top_row: i32, lines: u32) {
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
                let frags = 30 + (pseudo(&mut seed) * 10.0) as u32;
                for _ in 0..frags {
                    let angle = pseudo(&mut seed) * std::f32::consts::TAU;
                    let speed = 3.0 + pseudo(&mut seed) * 6.0;
                    let size = 0.019 + pseudo(&mut seed) * 0.037;
                    self.shatter_fragments.push(ShatterFragment {
                        x: cx, y: cy,
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

    /// Clear transient state (used on game restart, new game, etc.)
    pub fn clear(&mut self) {
        self.clearing_cells.clear();
        self.bg_rings.clear();
    }
}
