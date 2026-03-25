// Fire effect — rising flame particles along the bottom of the board.
// Audio-reactive: louder music = taller, more intense flames.

use super::{AudioEffect, AudioFrame, RenderContext, RenderPass};
use crate::gui::drawing::Vertex;

struct Ember {
    x: f32,
    y: f32,
    origin_x: f32, // spawn x for sinusoidal offset
    vx: f32,
    vy: f32,
    life: f32,
    max_life: f32,
    size: f32,
    heat: f32, // 1.0 = white-hot core, 0.0 = dark smoke
    phase1: f32, // primary drift
    freq1: f32,
    amp1: f32,
    phase2: f32, // secondary detail — attenuates over life
    freq2: f32,
    amp2: f32,
}

struct GlowBlob {
    x: f32,
    y: f32,
    origin_x: f32,
    vy: f32,
    life: f32,
    max_life: f32,
    size: f32,
    phase: f32,
    freq: f32,
    amp: f32,
}

pub struct Fire {
    embers: Vec<Ember>,
    blobs: Vec<GlowBlob>,
    rng: u64,
    spawn_acc: f32,  // accumulator for ember spawn timing
    blob_acc: f32,   // accumulator for blob spawn timing
    intensity: f32,  // smoothed audio intensity
}

fn rng_next(rng: &mut u64) -> f32 {
    *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    ((*rng >> 33) as f32) / (u32::MAX as f32 / 2.0)
}

impl Fire {
    pub fn new() -> Self {
        Fire {
            embers: Vec::new(),
            blobs: Vec::new(),
            rng: 0xF12E0042,
            spawn_acc: 0.0,
            blob_acc: 0.0,
            intensity: 0.0,
        }
    }
}

impl AudioEffect for Fire {
    fn pass(&self) -> RenderPass {
        RenderPass::Transparent
    }

    fn update(&mut self, audio: &AudioFrame) {
        // Intensity tracks low-frequency energy (sub-bass + bass)
        let raw = audio.bands_norm[0] * 0.6 + audio.bands_norm[1] * 0.3 + audio.bands_norm[2] * 0.1;
        self.intensity += (raw - self.intensity) * 4.0 * audio.dt; // smooth follow

        // Spawn rate scales with intensity
        let spawn_rate = 150.0 + self.intensity * 600.0; // 150-750 embers/sec
        self.spawn_acc += spawn_rate * audio.dt;

        let board_w = 10.0; // grid width in world units

        while self.spawn_acc >= 1.0 {
            self.spawn_acc -= 1.0;

            let x = rng_next(&mut self.rng).abs() * (board_w + 4.0) - 2.0; // slightly wider than board
            let heat = 0.5 + rng_next(&mut self.rng).abs() * 0.5;
            let life = 4.0 + rng_next(&mut self.rng).abs() * 6.0 + self.intensity * 2.5;
            let speed = 1.4 + rng_next(&mut self.rng).abs() * 2.0 + self.intensity * 1.2;

            self.embers.push(Ember {
                x,
                y: -3.0 + rng_next(&mut self.rng).abs() * 1.0, // start below visible board
                origin_x: x,
                vx: rng_next(&mut self.rng) * 0.3,
                vy: speed,
                life,
                max_life: life,
                size: 0.01 + rng_next(&mut self.rng).abs() * 0.015 + self.intensity * 0.005,
                heat,
                phase1: rng_next(&mut self.rng) * std::f32::consts::TAU,
                freq1: 0.3 + rng_next(&mut self.rng).abs() * 0.7,  // slow primary drift
                amp1: 0.1 + rng_next(&mut self.rng).abs() * 0.35,  // wide variance: some barely drift, some wander
                phase2: rng_next(&mut self.rng) * std::f32::consts::TAU,
                freq2: 0.8 + rng_next(&mut self.rng).abs() * 1.5,  // faster detail layer
                amp2: 0.03 + rng_next(&mut self.rng).abs() * 0.12, // subtle — chaotic texture, not oscillation
            });
        }

        // Update embers
        for e in &mut self.embers {
            e.x += e.vx * audio.dt;
            e.y += e.vy * audio.dt;
            e.life -= audio.dt;
            e.heat = (e.heat - audio.dt * 0.4).max(0.0);
        }
        self.embers.retain(|e| e.life > 0.0);

        // --- Flame body: glow blobs ---
        let blob_rate = 20.0 + self.intensity * 60.0; // 20-80 blobs/sec
        self.blob_acc += blob_rate * audio.dt;

        while self.blob_acc >= 1.0 {
            self.blob_acc -= 1.0;

            let x = rng_next(&mut self.rng).abs() * (board_w + 2.0) - 1.0;
            let life = 3.5 + rng_next(&mut self.rng).abs() * 4.0 + self.intensity * 2.0;
            let speed = 0.8 + rng_next(&mut self.rng).abs() * 1.2 + self.intensity * 0.6;
            let size = 0.5 + rng_next(&mut self.rng).abs() * 1.5 + self.intensity * 0.6;

            self.blobs.push(GlowBlob {
                x,
                y: -3.0 + rng_next(&mut self.rng).abs() * 1.0, // start below visible board
                origin_x: x,
                vy: speed,
                life,
                max_life: life,
                size,
                phase: rng_next(&mut self.rng) * std::f32::consts::TAU,
                freq: 0.2 + rng_next(&mut self.rng).abs() * 0.4,
                amp: 0.15 + rng_next(&mut self.rng).abs() * 0.3,
            });
        }

        for b in &mut self.blobs {
            b.y += b.vy * audio.dt;
            b.life -= audio.dt;
        }
        self.blobs.retain(|b| b.life > 0.0);
    }

    fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, _ctx: &RenderContext) {
        let n = [0.0f32, 0.0, 1.0];
        let z_blob = -1.2; // behind sparks
        let z = -1.0;      // sparks

        // Flame body: large transparent glow blobs — overlap creates continuous flame
        for b in &self.blobs {
            let t = (b.life / b.max_life).clamp(0.0, 1.0);
            // Fade in briefly, then fade out
            let fade_in = ((1.0 - t) * 5.0).min(1.0); // ramp up over first 20% of life
            let fade_out = t;
            let alpha = fade_in * fade_out * 0.08; // very transparent — overlapping builds brightness

            // Height-based color: bright yellow-orange at base, red-dark higher up
            let height_frac = (b.y / 14.0).clamp(0.0, 1.0); // 0=base, 1=top
            let r = 1.5 - height_frac * 0.8;   // HDR orange at base, dim red at top
            let g = 0.6 - height_frac * 0.5;   // yellow component fades
            let bl = 0.05 - height_frac * 0.03; // minimal blue

            let color = [r, g, bl.max(0.0), alpha];

            let age = b.max_life - b.life;
            let wobble = (age * b.freq * std::f32::consts::TAU + b.phase).sin() * b.amp;
            let rx = b.origin_x + wobble;
            let ry = -20.0 + b.y;
            let s = b.size * (0.7 + t * 0.3); // slight shrink over life

            let base = verts.len() as u32;
            verts.push(Vertex { position: [rx - s, ry - s, z_blob], normal: n, color, uv: [-1.0, -1.0] });
            verts.push(Vertex { position: [rx + s, ry - s, z_blob], normal: n, color, uv: [1.0, -1.0] });
            verts.push(Vertex { position: [rx + s, ry + s, z_blob], normal: n, color, uv: [1.0, 1.0] });
            verts.push(Vertex { position: [rx - s, ry + s, z_blob], normal: n, color, uv: [-1.0, 1.0] });
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }

        // Spark embers
        for e in &self.embers {
            let t = (e.life / e.max_life).clamp(0.0, 1.0);
            let alpha = t * t; // fade out quadratically

            // Color: white-hot → yellow → orange → red → dark based on heat
            let (r, g, b) = if e.heat > 0.7 {
                // White-hot core
                let h = (e.heat - 0.7) / 0.3;
                (1.0 + h * 0.5, 0.9 + h * 0.3, 0.5 + h * 0.8) // HDR white-yellow
            } else if e.heat > 0.4 {
                // Yellow-orange
                let h = (e.heat - 0.4) / 0.3;
                (1.0, 0.4 + h * 0.5, 0.05 + h * 0.15)
            } else if e.heat > 0.15 {
                // Orange-red
                let h = (e.heat - 0.15) / 0.25;
                (0.6 + h * 0.4, 0.1 + h * 0.3, 0.02)
            } else {
                // Dark red / smoke
                let h = e.heat / 0.15;
                (0.2 + h * 0.4, 0.02 + h * 0.08, 0.01)
            };

            let color = [r, g, b, alpha * 0.8];
            let s = e.size * (0.6 + t * 0.4); // shrink as they die

            // Position: board bottom is at y = -20 (row 20, negated)
            // Dual-sine lateral drift — primary carries, secondary adds detail and fades out
            let age = e.max_life - e.life;
            let drift1 = (age * e.freq1 * std::f32::consts::TAU + e.phase1).sin() * e.amp1;
            let drift2 = (age * e.freq2 * std::f32::consts::TAU + e.phase2).sin() * e.amp2 * t; // attenuates to 0
            let rx = e.origin_x + drift1 + drift2;
            let ry = -20.0 + e.y;

            let base = verts.len() as u32;
            verts.push(Vertex { position: [rx - s, ry - s, z], normal: n, color, uv: [-1.0, -1.0] });
            verts.push(Vertex { position: [rx + s, ry - s, z], normal: n, color, uv: [1.0, -1.0] });
            verts.push(Vertex { position: [rx + s, ry + s, z], normal: n, color, uv: [1.0, 1.0] });
            verts.push(Vertex { position: [rx - s, ry + s, z], normal: n, color, uv: [-1.0, 1.0] });
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
}
