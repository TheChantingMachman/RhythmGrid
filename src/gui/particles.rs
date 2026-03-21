// Particle system — spawns, updates, and renders floating particles.
// Driven by game events (line clears, beats) for audio-reactive visuals.

use super::drawing::*;

pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,     // remaining lifetime (seconds)
    pub max_life: f32, // initial lifetime
    pub color: [f32; 4],
    pub size: f32,
}

pub struct ParticleSystem {
    pub particles: Vec<Particle>,
    rng_state: u64,
}

impl ParticleSystem {
    pub fn new() -> Self {
        ParticleSystem {
            particles: Vec::with_capacity(2000),
            rng_state: 0xDEADBEEF12345678,
        }
    }

    fn rand_f32(&mut self) -> f32 {
        self.rng_state = self.rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.rng_state >> 33) as f32) / (u32::MAX as f32 / 2.0)
    }

    /// Spawn particles for a line clear spread across a vertical range.
    pub fn spawn_line_clear(&mut self, top_y: f32, height: f32, board_x: f32, board_w: f32, color: [f32; 4]) {
        let count = 45;
        for _ in 0..count {
            let x = board_x + self.rand_f32().abs() * board_w;
            let y_offset = self.rand_f32().abs() * height;
            let vx = (self.rand_f32() - 0.5) * 90.0;
            let vy = (self.rand_f32() - 0.7) * 120.0;
            let life = 1.0 + self.rand_f32().abs() * 1.2;
            let size = 2.0 + self.rand_f32().abs() * 4.0;
            self.particles.push(Particle {
                x, y: top_y + y_offset, vx, vy, life, max_life: life, color, size,
            });
        }
    }

    /// Spawn particles on a beat — dramatic burst from board edges.
    pub fn spawn_beat_pulse(&mut self, board_x: f32, board_y: f32, board_w: f32, board_h: f32, intensity: f32) {
        let count = (15.0 * intensity) as usize;
        for _ in 0..count {
            let edge = (self.rand_f32().abs() * 4.0) as u8;
            let speed = 40.0 + intensity * 30.0;
            let (x, y, vx, vy) = match edge {
                0 => (board_x + self.rand_f32().abs() * board_w, board_y, (self.rand_f32() - 0.5) * speed, -self.rand_f32().abs() * speed * 1.5),
                1 => (board_x + self.rand_f32().abs() * board_w, board_y + board_h, (self.rand_f32() - 0.5) * speed, self.rand_f32().abs() * speed * 1.5),
                2 => (board_x, board_y + self.rand_f32().abs() * board_h, -self.rand_f32().abs() * speed * 1.5, (self.rand_f32() - 0.5) * speed),
                _ => (board_x + board_w, board_y + self.rand_f32().abs() * board_h, self.rand_f32().abs() * speed * 1.5, (self.rand_f32() - 0.5) * speed),
            };
            let life = 0.8 + self.rand_f32().abs() * 1.0;
            let r = 0.3 + self.rand_f32().abs() * 0.3;
            let g = 0.4 + self.rand_f32().abs() * 0.4;
            let b = 0.7 + self.rand_f32().abs() * 0.3;
            let size = 2.0 + self.rand_f32().abs() * 3.0;
            self.particles.push(Particle {
                x, y, vx, vy, life, max_life: life, color: [r, g, b, 0.7], size,
            });
        }
    }

    /// Update all particles. Returns number of live particles.
    pub fn update(&mut self, dt: f32) -> usize {
        for p in &mut self.particles {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.vy += 15.0 * dt; // slight gravity
            p.vx *= 0.98; // drag
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);
        self.particles.len()
    }

    /// Render particles into the vertex buffer.
    pub fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>) {
        for p in &self.particles {
            let alpha = (p.life / p.max_life).clamp(0.0, 1.0);
            let color = [p.color[0], p.color[1], p.color[2], p.color[3] * alpha];
            let half = p.size * (0.5 + alpha * 0.5); // shrink as they fade
            push_quad(verts, indices, p.x - half, p.y - half, half * 2.0, half * 2.0, color, 0.07);
        }
    }
}
