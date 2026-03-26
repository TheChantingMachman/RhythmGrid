// Starfield effect — flying through a field of stars with parallax depth layers.
// Audio-reactive: intensity drives flight speed, beats pulse brightness.

use super::{AudioEffect, AudioFrame, RenderContext, RenderPass};
use crate::gui::drawing::Vertex;

struct Star {
    x: f32,  // position relative to center (-1..1 normalized, then scaled)
    y: f32,
    z: f32,  // depth: 0.0 = far, 1.0 = near (controls speed + size)
    brightness: f32,
    twinkle_phase: f32,
}

struct BgStar {
    x: f32,
    y: f32,
    brightness: f32,
    twinkle_phase: f32,
    size: f32,
}

pub struct Starfield {
    stars: Vec<Star>,
    bg_stars: Vec<BgStar>,  // distant static/slow background layer
    rng: u64,
    speed: f32,           // smoothed flight speed
    beat_flash: f32,      // 1.0 on beat, decays
    warp_level: f32,      // 0.0 = cruise, 1.0 = full warp (smooth ramp)
    warp_flash: f32,      // flash on warp engage/disengage
    energy_smooth: f32,   // heavily smoothed energy for warp threshold
    pub warp_threshold: f32, // energy level that triggers warp (tunable)
}

fn rng_next(rng: &mut u64) -> f32 {
    *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    ((*rng >> 33) as f32) / (u32::MAX as f32 / 2.0)
}

const STAR_COUNT: usize = 300;
const CENTER_X: f32 = 5.0;  // board center in world coords
const CENTER_Y: f32 = -10.0;
const SPREAD: f32 = 25.0;   // how far stars can be from center

impl Starfield {
    pub fn new() -> Self {
        let mut rng: u64 = 0x57A2F1ED;
        let mut stars = Vec::with_capacity(STAR_COUNT);
        for _ in 0..STAR_COUNT {
            stars.push(Self::spawn_star(&mut rng));
        }
        // Background stars — scattered across a wide area, barely moving
        let mut bg_stars = Vec::with_capacity(300);
        for _ in 0..300 {
            let angle = rng_next(&mut rng) * std::f32::consts::TAU;
            let dist = rng_next(&mut rng).abs().sqrt() * SPREAD * 1.2;
            bg_stars.push(BgStar {
                x: CENTER_X + angle.cos() * dist,
                y: CENTER_Y + angle.sin() * dist,
                brightness: 0.2 + rng_next(&mut rng).abs() * 0.4,
                twinkle_phase: rng_next(&mut rng).abs() * std::f32::consts::TAU,
                size: 0.008 + rng_next(&mut rng).abs() * 0.015,
            });
        }
        Starfield {
            stars,
            bg_stars,
            rng,
            speed: 0.0,
            beat_flash: 0.0,
            warp_level: 0.0,
            warp_flash: 0.0,
            energy_smooth: 0.0,
            warp_threshold: 0.44,
        }
    }

    fn spawn_star(rng: &mut u64) -> Star {
        // Spawn at random position, biased toward center for natural density
        let angle = rng_next(rng) * std::f32::consts::TAU;
        let dist = rng_next(rng).abs().sqrt() * SPREAD; // sqrt for uniform area distribution
        Star {
            x: angle.cos() * dist,
            y: angle.sin() * dist,
            z: rng_next(rng).abs(), // depth layer 0..1
            brightness: 0.5 + rng_next(rng).abs() * 0.5,
            twinkle_phase: rng_next(rng).abs() * std::f32::consts::TAU,
        }
    }
}

impl AudioEffect for Starfield {
    fn pass(&self) -> RenderPass {
        RenderPass::Transparent
    }

    fn update(&mut self, audio: &AudioFrame) {
        // Flight speed from overall energy
        let energy = audio.bands_norm[0] * 0.3 + audio.bands_norm[1] * 0.3
            + audio.bands_norm[3] * 0.2 + audio.bands_norm[5] * 0.2;

        // Warp detection — heavily smoothed energy vs threshold
        self.energy_smooth += (energy - self.energy_smooth) * 0.3 * audio.dt; // ~3s time constant
        let warp_target = if self.energy_smooth > self.warp_threshold { 1.0 } else { 0.0 };
        let prev_warp = self.warp_level;
        // Ramp in over ~1.5s, ramp out over ~2.5s
        let ramp_speed = if warp_target > self.warp_level { 0.7 } else { 0.4 };
        self.warp_level += (warp_target - self.warp_level) * ramp_speed * audio.dt;
        self.warp_level = self.warp_level.clamp(0.0, 1.0);

        // Flash on warp engage/disengage (crossing 0.5 threshold)
        if (prev_warp < 0.5 && self.warp_level >= 0.5) || (prev_warp >= 0.5 && self.warp_level < 0.5) {
            self.warp_flash = 1.0;
        }
        self.warp_flash = (self.warp_flash - audio.dt * 3.0).max(0.0);

        // Speed: cruise base + warp multiplier
        let warp_boost = 1.0 + self.warp_level * 6.0; // up to 7x at full warp
        let target_speed = (0.3 + energy * 2.0) * warp_boost;
        self.speed += (target_speed - self.speed) * 2.0 * audio.dt;

        // Beat flash
        if audio.band_beats[0] > 0.9 || audio.band_beats[1] > 0.9 {
            self.beat_flash = 1.0;
        }
        self.beat_flash = (self.beat_flash - audio.dt * 4.0).max(0.0);

        // Move stars outward from center (zoom effect)
        for star in &mut self.stars {
            let speed_mult = 0.2 + star.z * 0.8;
            let dx = star.x;
            let dy = star.y;
            let dist = (dx * dx + dy * dy).sqrt().max(0.01);
            let expand = self.speed * speed_mult * audio.dt * 3.0;
            star.x += (dx / dist) * expand;
            star.y += (dy / dist) * expand;

            // Twinkle
            star.twinkle_phase += audio.dt * (2.0 + star.z * 3.0);
        }

        // Respawn stars that fly out of view
        for star in &mut self.stars {
            let dist = (star.x * star.x + star.y * star.y).sqrt();
            if dist > SPREAD {
                // Respawn: wide scatter in cruise, tight point in warp
                let spawn_radius = 5.0 + self.warp_level * 8.0; // 5.0 cruise, 13.0 warp
                let min_radius = self.warp_level * 2.0; // exclude center during warp (~2 cube widths)
                let angle = rng_next(&mut self.rng) * std::f32::consts::TAU;
                let d = min_radius + rng_next(&mut self.rng).abs().sqrt() * (spawn_radius - min_radius);
                star.x = angle.cos() * d;
                star.y = angle.sin() * d;
                star.z = rng_next(&mut self.rng).abs();
                star.brightness = 0.5 + rng_next(&mut self.rng).abs() * 0.5;
                star.twinkle_phase = rng_next(&mut self.rng).abs() * std::f32::consts::TAU;
            }
        }

        // Background stars — twinkle + drift during warp
        for bg in &mut self.bg_stars {
            bg.twinkle_phase += audio.dt * 1.5;
            // During warp, bg stars expand outward like a slow travel layer
            if self.warp_level > 0.1 {
                let dx = bg.x - CENTER_X;
                let dy = bg.y - CENTER_Y;
                let dist = (dx * dx + dy * dy).sqrt().max(0.01);
                let expand = self.warp_level * 2.0 * audio.dt;
                bg.x += (dx / dist) * expand;
                bg.y += (dy / dist) * expand;
                // Respawn if too far out
                if dist > SPREAD * 1.2 {
                    let angle = rng_next(&mut self.rng) * std::f32::consts::TAU;
                    let d = 2.0 + rng_next(&mut self.rng).abs() * 6.0;
                    bg.x = CENTER_X + angle.cos() * d;
                    bg.y = CENTER_Y + angle.sin() * d;
                }
            }
        }
    }

    fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, _ctx: &RenderContext) {
        let n = [0.0f32, 0.0, 1.0];
        let z_pos = -3.0; // far behind everything
        let w = self.warp_level;

        // Warp flash — full-screen bright overlay
        if self.warp_flash > 0.01 {
            let fa = self.warp_flash * self.warp_flash * 0.3;
            let fc = [1.5, 1.5, 2.0, fa]; // HDR blue-white flash
            let base = verts.len() as u32;
            verts.push(Vertex { position: [-20.0, 10.0, z_pos - 0.1], normal: n, color: fc, uv: [0.0, 0.0] });
            verts.push(Vertex { position: [30.0, 10.0, z_pos - 0.1], normal: n, color: fc, uv: [0.0, 0.0] });
            verts.push(Vertex { position: [30.0, -30.0, z_pos - 0.1], normal: n, color: fc, uv: [0.0, 0.0] });
            verts.push(Vertex { position: [-20.0, -30.0, z_pos - 0.1], normal: n, color: fc, uv: [0.0, 0.0] });
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }

        // Background stars — distant, nearly static during cruise, normal travel during warp
        let z_bg = -3.5; // behind everything including main stars
        for bg in &self.bg_stars {
            let twinkle = 0.6 + 0.4 * bg.twinkle_phase.sin();
            let alpha = bg.brightness * twinkle * (0.4 + w * 0.4); // brighter during warp
            let color = [0.6, 0.65, 0.8, alpha.min(0.8)];
            let s = bg.size;
            let base = verts.len() as u32;
            verts.push(Vertex { position: [bg.x - s, bg.y - s, z_bg], normal: n, color, uv: [-1.0, -1.0] });
            verts.push(Vertex { position: [bg.x + s, bg.y - s, z_bg], normal: n, color, uv: [1.0, -1.0] });
            verts.push(Vertex { position: [bg.x + s, bg.y + s, z_bg], normal: n, color, uv: [1.0, 1.0] });
            verts.push(Vertex { position: [bg.x - s, bg.y + s, z_bg], normal: n, color, uv: [-1.0, 1.0] });
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }

        // Main traveling stars — skip 20% during warp to reduce noise
        let mut skip_counter = 0u32;
        for star in &self.stars {
            if w > 0.5 {
                skip_counter += 1;
                if skip_counter % 2 == 0 { continue; } // skip every other star (~50% culled)
            }
            let dist = (star.x * star.x + star.y * star.y).sqrt();

            // Size: near + distant from center = larger, warp makes everything brighter
            let base_size = 0.012 + star.z * 0.035;
            let size = (base_size + dist * 0.001) * (1.0 - w * 0.5); // shrink during warp

            // Twinkle brightness (suppressed during warp — everything bright)
            let twinkle = 0.7 + 0.3 * star.twinkle_phase.sin() * (1.0 - w * 0.8);

            // Alpha: boosted during warp
            let alpha = star.brightness * twinkle * (0.3 + star.z * 0.7)
                + self.beat_flash * 0.3
                + w * 0.4;

            // Color: blue-shifts during warp
            let warmth = star.z * 0.15 * (1.0 - w);
            let color = [
                0.7 + warmth + self.beat_flash * 0.3,
                0.8 + self.beat_flash * 0.2 + w * 0.2,
                1.0 + w * 0.5, // HDR blue at warp
                alpha.min(1.0),
            ];

            // Streak: dramatically longer during warp
            let streak = if star.z > 0.3 || w > 0.1 {
                let base_streak = (star.z - 0.3).max(0.0) * self.speed * 0.8;
                let warp_streak = w * dist * 0.15; // warp streaks scale with distance from center
                (base_streak + warp_streak).min(2.0 + w * 4.0) // up to 6 units at full warp
            } else {
                0.0
            };

            let wx = CENTER_X + star.x;
            let wy = CENTER_Y + star.y;

            if streak > 0.01 {
                // Velocity-aligned streak
                let dx = star.x / dist.max(0.01);
                let dy = star.y / dist.max(0.01);
                let half_w = size;
                let half_len = size + streak;
                let nx_s = -dy * half_w;
                let ny_s = dx * half_w;
                let fx = dx * half_len;
                let fy = dy * half_len;

                let base = verts.len() as u32;
                verts.push(Vertex { position: [wx - fx + nx_s, wy - fy + ny_s, z_pos], normal: n, color, uv: [-1.0, -1.0] });
                verts.push(Vertex { position: [wx + fx + nx_s, wy + fy + ny_s, z_pos], normal: n, color, uv: [1.0, -1.0] });
                verts.push(Vertex { position: [wx + fx - nx_s, wy + fy - ny_s, z_pos], normal: n, color, uv: [1.0, 1.0] });
                verts.push(Vertex { position: [wx - fx - nx_s, wy - fy - ny_s, z_pos], normal: n, color, uv: [-1.0, 1.0] });
                indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            } else {
                // Soft dot
                let base = verts.len() as u32;
                verts.push(Vertex { position: [wx - size, wy - size, z_pos], normal: n, color, uv: [-1.0, -1.0] });
                verts.push(Vertex { position: [wx + size, wy - size, z_pos], normal: n, color, uv: [1.0, -1.0] });
                verts.push(Vertex { position: [wx + size, wy + size, z_pos], normal: n, color, uv: [1.0, 1.0] });
                verts.push(Vertex { position: [wx - size, wy + size, z_pos], normal: n, color, uv: [-1.0, 1.0] });
                indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            }
        }
    }
}
