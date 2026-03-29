// Firework bursts — radial streaks that spawn behind the board on strong beats.
// Multi-stage shells launch, detonate, and cascade with gravity-affected trails.
// Smoke puffs expand and dissipate from each burst, illuminated by burst glow.

use super::{AudioEffect, AudioFrame, RenderContext, RenderPass};
use crate::gui::drawing::Vertex;

struct Spark {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    max_life: f32,
    color: [f32; 4],
}

struct BurstTrail {
    x: f32, y: f32,
    life: f32,
    color: [f32; 4],
}

struct SmokePuff {
    x: f32, y: f32,
    vx: f32, vy: f32,
    size: f32,
    life: f32,
    max_life: f32,
    color: [f32; 3], // base tint from burst color
}

struct Burst {
    sparks: Vec<Spark>,
    trails: Vec<BurstTrail>,
    smoke: Vec<SmokePuff>,
    flash_x: f32,
    flash_y: f32,
    flash_timer: f32,
    flash_color: [f32; 4],
}

// --- Multi-stage firework shells (rare dramatic events) ---

#[derive(PartialEq)]
enum ShellPhase {
    Launch,
    Waiting(f32),  // at apex, waiting for beat. f32 = max wait timer
    Detonate,
    Cascade,
}

struct TrailParticle {
    x: f32, y: f32,
    vx: f32, vy: f32,
    life: f32, max_life: f32,
    color: [f32; 4],
    size: f32,
}

struct Shell {
    phase: ShellPhase,
    x: f32, y: f32,
    vx: f32, vy: f32,
    launch_timer: f32,
    streamers: Vec<Spark>,
    trails: Vec<TrailParticle>,
    smoke: Vec<SmokePuff>,
    color: [f32; 4],
    timer: f32,
}

pub struct Fireworks {
    bursts: Vec<Burst>,
    shells: Vec<Shell>,
    pub shell_cooldown: f32,
    rng: u64,
    prev_flux: f32,
    pub trigger_band: Option<usize>,
    pub shells_only: bool,
    pub bursts_only: bool,
    // Wind — random direction per theme, affects smoke drift
    wind_x: f32,
    #[allow(dead_code)]
    wind_z: f32, // future: Z-depth drift for smoke
    // Beat-gated burst spawning
    burst_cooldown: f32,
    pending_bursts: Vec<(f32, f32, [f32; 4], usize, f32)>, // (x, y, color, spark_count, wait_timer)
}

fn rng_next(rng: &mut u64) -> f32 {
    *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    ((*rng >> 33) as f32) / (u32::MAX as f32 / 2.0)
}

impl Fireworks {
    pub fn new() -> Self {
        let mut rng: u64 = 0xCAFEBABE42;
        let wind_x = rng_next(&mut rng) * 1.5 - 0.75;
        let wind_z = rng_next(&mut rng) * 0.4 - 0.2;
        Fireworks {
            bursts: Vec::new(),
            shells: Vec::new(),
            shell_cooldown: 1.0, // first shell after 1s
            rng,
            prev_flux: 0.0,
            trigger_band: None,
            shells_only: false,
            bursts_only: false,
            wind_x,
            wind_z,
            burst_cooldown: 0.0,
            pending_bursts: Vec::new(),
        }
    }

    /// Randomize wind direction — call on theme change.
    #[allow(dead_code)]
    pub fn randomize_wind(&mut self) {
        self.wind_x = rng_next(&mut self.rng) * 1.5 - 0.75;
        self.wind_z = rng_next(&mut self.rng) * 0.4 - 0.2;
    }

    fn spawn_smoke(rng: &mut u64, x: f32, y: f32, color: [f32; 3], count: usize) -> Vec<SmokePuff> {
        // Spawn multiple overlapping layers per "puff" for volumetric look.
        // Each layer is large, nearly invisible, at slightly different positions.
        let mut smoke = Vec::with_capacity(count * 4);
        for _ in 0..count {
            let base_angle = rng_next(rng) * std::f32::consts::TAU;
            let base_speed = 0.2 + rng_next(rng).abs() * 0.5;
            let base_life = 4.0 + rng_next(rng).abs() * 5.0;
            // 1-2 sub-layers per puff
            let layers = 1 + (rng_next(rng).abs() * 1.5) as usize;
            for l in 0..layers {
                let offset = rng_next(rng) * 0.3;
                let angle_jitter = rng_next(rng) * 0.5;
                let a = base_angle + angle_jitter;
                smoke.push(SmokePuff {
                    x: x + a.cos() * offset,
                    y: y + a.sin() * offset,
                    vx: a.cos() * base_speed * (0.8 + l as f32 * 0.15),
                    vy: a.sin() * base_speed * (0.8 + l as f32 * 0.15),
                    size: 0.6 + rng_next(rng).abs() * 0.8 + l as f32 * 0.3,
                    life: base_life + rng_next(rng).abs() * 0.5,
                    max_life: base_life + 0.5,
                    color,
                });
            }
        }
        smoke
    }
}

impl AudioEffect for Fireworks {
    fn pass(&self) -> RenderPass {
        RenderPass::Transparent
    }

    fn update(&mut self, audio: &AudioFrame) {
        // Continuously queue bursts to keep 1-2 pending, detonation is beat-gated
        self.burst_cooldown -= audio.dt;
        if !self.shells_only && self.pending_bursts.len() < 2 && self.burst_cooldown <= 0.0 {
            self.burst_cooldown = 0.3;
            let cx = rng_next(&mut self.rng) * 30.0 - 10.0;
            let cy = -(rng_next(&mut self.rng).abs() * 24.0);
            let spark_count = 25 + (rng_next(&mut self.rng).abs() * 15.0) as usize;
            let base_color = [2.2, 2.2, 2.2, 0.95];
            self.pending_bursts.push((cx, cy, base_color, spark_count, 0.75));
        }

        // Tick pending burst timers
        for pending in &mut self.pending_bursts {
            pending.4 -= audio.dt;
        }
        // Release pending bursts on rank 1 beat (dominant rhythm — kick/pulse) or timeout
        let r1 = audio.resolved_ranks[0];
        let dominant_beat = audio.band_beats[r1] > 0.5;
        let ready: Vec<_> = self.pending_bursts.iter()
            .enumerate()
            .filter(|(_, p)| dominant_beat || p.4 <= 0.0)
            .map(|(i, _)| i)
            .collect();
        let to_release: Vec<_> = ready.into_iter().rev()
            .map(|i| self.pending_bursts.remove(i))
            .collect();
        if !to_release.is_empty() {
            for (cx, cy, base_color, spark_count, _) in to_release {
                let mut sparks = Vec::with_capacity(spark_count);
                for _ in 0..spark_count {
                    let angle = rng_next(&mut self.rng) * std::f32::consts::TAU;
                    let speed = (3.0 + rng_next(&mut self.rng).abs() * 7.0) * 0.8; // 20% slower
                    let life = 0.3 + rng_next(&mut self.rng).abs() * 0.45;
                    sparks.push(Spark {
                        x: cx, y: cy,
                        vx: angle.cos() * speed,
                        vy: angle.sin() * speed,
                        life, max_life: life,
                        color: base_color,
                    });
                }
                let flash_color = [
                    base_color[0].min(1.0) * 0.5 + 0.5,
                    base_color[1].min(1.0) * 0.5 + 0.5,
                    base_color[2].min(1.0) * 0.5 + 0.5,
                    1.0,
                ];
                // Smoke puffs at burst origin
                let smoke_color = [base_color[0].min(1.0) * 0.3, base_color[1].min(1.0) * 0.3, base_color[2].min(1.0) * 0.3];
                let smoke = Self::spawn_smoke(&mut self.rng, cx, cy, smoke_color, 5);
                self.bursts.push(Burst {
                    sparks, trails: Vec::new(), smoke,
                    flash_x: cx, flash_y: cy, flash_timer: 0.15, flash_color,
                });
            }
        }

        let wind_x = self.wind_x;

        // Update all bursts
        for burst in &mut self.bursts {
            burst.flash_timer -= audio.dt;
            for spark in &mut burst.sparks {
                spark.x += spark.vx * audio.dt;
                spark.y += spark.vy * audio.dt;
                spark.vy += 0.3 * audio.dt;
                spark.vx *= 0.9975;
                spark.vy *= 0.9975;
                spark.life -= audio.dt;

                if spark.life > 0.0 && burst.trails.len() < 500 {
                    let t = spark.life / spark.max_life;
                    burst.trails.push(BurstTrail {
                        x: spark.x, y: spark.y,
                        life: 0.15 + t * 0.15,
                        color: [spark.color[0] * 1.5, spark.color[1] * 1.5, spark.color[2] * 1.5, 0.8],
                    });
                }
            }
            burst.sparks.retain(|s| s.life > 0.0);

            for t in &mut burst.trails {
                t.life -= audio.dt;
            }
            burst.trails.retain(|t| t.life > 0.0);

            // Update smoke puffs — expand, drift with wind, dissipate
            for puff in &mut burst.smoke {
                puff.x += (puff.vx + wind_x * 0.5) * audio.dt;
                puff.y += puff.vy * audio.dt;
                puff.vx *= 0.98;
                puff.vy *= 0.98;
                puff.size += audio.dt * 0.4; // expand
                puff.life -= audio.dt;
            }
            burst.smoke.retain(|p| p.life > 0.0);
        }
        self.bursts.retain(|b| !b.sparks.is_empty() || !b.trails.is_empty() || b.flash_timer > 0.0 || !b.smoke.is_empty());

        // --- Multi-stage shells — continuous spawning, beat-gated detonation ---
        if self.bursts_only { self.shell_cooldown = 1.0; } else {
        self.shell_cooldown -= audio.dt;

        // Count shells in flight (launching or waiting)
        let in_flight = self.shells.iter().filter(|s| s.phase == ShellPhase::Launch || matches!(s.phase, ShellPhase::Waiting(_))).count();

        // Continuously spawn to keep 2-3 shells in flight
        if self.shell_cooldown <= 0.0 && in_flight < 3 {
            let cx = rng_next(&mut self.rng) * 26.0 - 8.0;
            let start_y = 22.0;
            let piece_colors: [[f32; 3]; 8] = [
                [0.0, 1.0, 1.0], [1.0, 1.0, 0.0], [0.5, 0.0, 0.5], [0.0, 1.0, 0.0],
                [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.47, 0.0], [1.0, 1.0, 1.0],
            ];
            let ci = (rng_next(&mut self.rng).abs() * 8.0) as usize % 8;
            let pc = piece_colors[ci];
            let color = [pc[0], pc[1], pc[2], 1.0];
            self.shells.push(Shell {
                phase: ShellPhase::Launch,
                x: cx, y: start_y,
                vx: rng_next(&mut self.rng) * 1.2,
                vy: -(5.8 + rng_next(&mut self.rng).abs() * 3.4),
                launch_timer: 0.0,
                streamers: Vec::new(),
                trails: Vec::new(),
                smoke: Vec::new(),
                color,
                timer: 0.0,
            });
            self.shell_cooldown = 0.6;
        }
        self.prev_flux = audio.flux;
        }

        // Overflow control: force-detonate oldest if too many in flight
        let in_flight_now = self.shells.iter().filter(|s| s.phase == ShellPhase::Launch || matches!(s.phase, ShellPhase::Waiting(_))).count();
        if in_flight_now > 3 {
            // Find oldest in-flight shell and force it to detonate
            if let Some(shell) = self.shells.iter_mut().find(|s| s.phase == ShellPhase::Launch || matches!(s.phase, ShellPhase::Waiting(_))) {
                shell.phase = ShellPhase::Waiting(0.0); // will detonate next frame
            }
            self.shell_cooldown = self.shell_cooldown.max(0.2); // brief throttle
        }

        // Overflow for bursts: force-release oldest pending if too many queued
        if self.pending_bursts.len() > 2 {
            let oldest = self.pending_bursts.remove(0);
            // Force spawn it now
            let (cx, cy, base_color, spark_count, _) = oldest;
            let mut sparks = Vec::with_capacity(spark_count);
            for _ in 0..spark_count {
                let angle = rng_next(&mut self.rng) * std::f32::consts::TAU;
                let speed = 5.2 + rng_next(&mut self.rng).abs() * 10.4;
                let life = 0.14 + rng_next(&mut self.rng).abs() * 0.21;
                sparks.push(Spark {
                    x: cx, y: cy, vx: angle.cos() * speed, vy: angle.sin() * speed,
                    life, max_life: life, color: base_color,
                });
            }
            let flash_color = [base_color[0].min(1.0) * 0.5 + 0.5, base_color[1].min(1.0) * 0.5 + 0.5, base_color[2].min(1.0) * 0.5 + 0.5, 1.0];
            let smoke_color = [base_color[0].min(1.0) * 0.3, base_color[1].min(1.0) * 0.3, base_color[2].min(1.0) * 0.3];
            let smoke = Self::spawn_smoke(&mut self.rng, cx, cy, smoke_color, 5);
            self.bursts.push(Burst { sparks, trails: Vec::new(), smoke, flash_x: cx, flash_y: cy, flash_timer: 0.15, flash_color });
        }

        // Update shells
        let mut rng = self.rng;
        for shell in &mut self.shells {
            shell.timer += audio.dt;

            match shell.phase {
                ShellPhase::Launch => {
                    shell.launch_timer += audio.dt;
                    shell.x += shell.vx * audio.dt;
                    shell.y += shell.vy * audio.dt;
                    shell.vy += 1.2 * audio.dt;

                    if shell.launch_timer > 0.05 {
                        shell.launch_timer = 0.0;
                        shell.trails.push(TrailParticle {
                            x: shell.x, y: shell.y,
                            vx: rng_next(&mut rng) * 0.3, vy: rng_next(&mut rng) * 0.3,
                            life: 0.8, max_life: 0.8,
                            color: [1.0, 0.9, 0.6, 0.7],
                            size: 0.06,
                        });
                    }

                    // Shells detonate on rank 2/3 beat (secondary rhythm — snare/accents)
                    let r2 = audio.resolved_ranks[1];
                    let r3 = audio.resolved_ranks[2];
                    let accent_beat = audio.band_beats[r2] > 0.7 || audio.band_beats[r3] > 0.7;
                    let early_detonate = shell.timer > 1.5 && accent_beat;
                    if shell.vy > 0.0 || shell.timer > 3.0 || early_detonate {
                        shell.vy = 0.0;
                        shell.vx *= 0.3;
                        if early_detonate {
                            // Beat caught it mid-flight — detonate immediately
                            shell.phase = ShellPhase::Waiting(0.0);
                        } else {
                            shell.phase = ShellPhase::Waiting(3.0);
                        }
                    }
                }
                ShellPhase::Waiting(ref mut wait) => {
                    // Drift slowly while waiting
                    shell.x += shell.vx * audio.dt;
                    shell.y += shell.vy * audio.dt;
                    shell.vy += 0.2 * audio.dt; // very gentle sag

                    *wait -= audio.dt;
                    // Detonate on accent beat or timeout
                    let r2 = audio.resolved_ranks[1];
                    let r3 = audio.resolved_ranks[2];
                    let accent_beat = audio.band_beats[r2] > 0.7 || audio.band_beats[r3] > 0.7;
                    if accent_beat || *wait <= 0.0 {
                        shell.phase = ShellPhase::Detonate;
                        let count = 30 + (rng_next(&mut rng).abs() * 20.0) as usize;
                        for _ in 0..count {
                            let angle = rng_next(&mut rng) * std::f32::consts::TAU;
                            let speed = (2.2 + rng_next(&mut rng).abs() * 3.3) * 0.8;
                            let life = 2.0 + rng_next(&mut rng).abs() * 2.0;
                            shell.streamers.push(Spark {
                                x: shell.x, y: shell.y,
                                vx: angle.cos() * speed,
                                vy: angle.sin() * speed,
                                life, max_life: life,
                                color: shell.color,
                            });
                        }
                        let sc = [shell.color[0] * 0.3, shell.color[1] * 0.3, shell.color[2] * 0.3];
                        shell.smoke = Self::spawn_smoke(&mut rng, shell.x, shell.y, sc, 5);
                    }
                }
                ShellPhase::Detonate => {
                    shell.phase = ShellPhase::Cascade;
                }
                ShellPhase::Cascade => {
                    for s in &mut shell.streamers {
                        s.x += s.vx * audio.dt;
                        s.y += s.vy * audio.dt;
                        s.vy += 0.4 * audio.dt;
                        s.life -= audio.dt;

                        if s.life > 0.0 && shell.trails.len() < 2000 {
                            shell.trails.push(TrailParticle {
                                x: s.x, y: s.y,
                                vx: rng_next(&mut rng) * 0.1,
                                vy: rng_next(&mut rng).abs() * 0.1,
                                life: 1.5 + rng_next(&mut rng).abs() * 1.5,
                                max_life: 2.5,
                                color: [
                                    shell.color[0] * 0.7,
                                    shell.color[1] * 0.7,
                                    shell.color[2] * 0.7,
                                    0.5,
                                ],
                                size: 0.04,
                            });
                        }
                    }
                    shell.streamers.retain(|s| s.life > 0.0);
                }
            }

            // Update trails
            for t in &mut shell.trails {
                t.x += t.vx * audio.dt;
                t.y += t.vy * audio.dt;
                t.vy += 1.5 * audio.dt;
                t.life -= audio.dt;
            }
            shell.trails.retain(|t| t.life > 0.0);

            // Update shell smoke
            for puff in &mut shell.smoke {
                puff.x += (puff.vx + wind_x * 0.5) * audio.dt;
                puff.y += puff.vy * audio.dt;
                puff.vx *= 0.98;
                puff.vy *= 0.98;
                puff.size += audio.dt * 0.3;
                puff.life -= audio.dt;
            }
            shell.smoke.retain(|p| p.life > 0.0);
        }
        self.rng = rng;
        self.shells.retain(|s| !s.streamers.is_empty() || !s.trails.is_empty() || s.phase == ShellPhase::Launch || matches!(s.phase, ShellPhase::Waiting(_)) || !s.smoke.is_empty());
    }

    fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, _ctx: &RenderContext) {
        let n = [0.0f32, 0.0, 1.0];
        let z = -1.5;

        for burst in &self.bursts {
            // Smoke puffs — multiple overlapping soft circles for volumetric look
            for (pi, puff) in burst.smoke.iter().enumerate() {
                let life_t = (puff.life / puff.max_life).clamp(0.0, 1.0);
                let alpha = life_t * life_t * 0.04; // very faint per layer — volume from overlap
                if alpha < 0.002 { continue; }
                let s = puff.size;
                let flash_glow = if burst.flash_timer > 0.0 { (burst.flash_timer / 0.15).clamp(0.0, 1.0) * 0.3 } else { 0.0 };
                let c = [
                    puff.color[0] + flash_glow,
                    puff.color[1] + flash_glow,
                    puff.color[2] + flash_glow,
                    alpha,
                ];
                // Vary Z per puff for depth spread
                let puff_z = z - 0.3 - (pi as f32 * 0.07) % 0.8;
                let base = verts.len() as u32;
                verts.push(Vertex { position: [puff.x - s, puff.y - s, puff_z], normal: n, color: c, uv: [-1.0, -1.0] });
                verts.push(Vertex { position: [puff.x + s, puff.y - s, puff_z], normal: n, color: c, uv: [ 1.0, -1.0] });
                verts.push(Vertex { position: [puff.x + s, puff.y + s, puff_z], normal: n, color: c, uv: [ 1.0,  1.0] });
                verts.push(Vertex { position: [puff.x - s, puff.y + s, puff_z], normal: n, color: c, uv: [-1.0,  1.0] });
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }

            // Flash glow at burst origin
            if burst.flash_timer > 0.0 {
                let ft = (burst.flash_timer / 0.15).clamp(0.0, 1.0);
                let flash_alpha = ft * ft * ft;
                let flash_size = 0.5 + (1.0 - ft) * 0.8;
                let fc = [
                    burst.flash_color[0] * 4.0,
                    burst.flash_color[1] * 4.0,
                    burst.flash_color[2] * 4.0,
                    flash_alpha,
                ];
                let base = verts.len() as u32;
                verts.push(Vertex { position: [burst.flash_x - flash_size, burst.flash_y - flash_size, z], normal: n, color: fc, uv: [-1.0, -1.0] });
                verts.push(Vertex { position: [burst.flash_x + flash_size, burst.flash_y - flash_size, z], normal: n, color: fc, uv: [ 1.0, -1.0] });
                verts.push(Vertex { position: [burst.flash_x + flash_size, burst.flash_y + flash_size, z], normal: n, color: fc, uv: [ 1.0,  1.0] });
                verts.push(Vertex { position: [burst.flash_x - flash_size, burst.flash_y + flash_size, z], normal: n, color: fc, uv: [-1.0,  1.0] });
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }

            for spark in &burst.sparks {
                let alpha = (spark.life / spark.max_life).powf(0.4);
                let color = [spark.color[0], spark.color[1], spark.color[2], spark.color[3] * alpha];
                let speed = (spark.vx * spark.vx + spark.vy * spark.vy).sqrt();
                let half_w = 0.04;
                let half_len = (0.05 + speed * 0.02).min(0.3);

                if speed < 0.01 {
                    let base = verts.len() as u32;
                    verts.push(Vertex { position: [spark.x - half_w, spark.y - half_w, z], normal: n, color, uv: [-1.0, -1.0] });
                    verts.push(Vertex { position: [spark.x + half_w, spark.y - half_w, z], normal: n, color, uv: [ 1.0, -1.0] });
                    verts.push(Vertex { position: [spark.x + half_w, spark.y + half_w, z], normal: n, color, uv: [ 1.0,  1.0] });
                    verts.push(Vertex { position: [spark.x - half_w, spark.y + half_w, z], normal: n, color, uv: [-1.0,  1.0] });
                    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
                } else {
                    let dx = spark.vx / speed;
                    let dy = spark.vy / speed;
                    let nx = -dy * half_w;
                    let ny = dx * half_w;
                    let fx = dx * half_len;
                    let fy = dy * half_len;
                    let base = verts.len() as u32;
                    verts.push(Vertex { position: [spark.x - fx + nx, spark.y - fy + ny, z], normal: n, color, uv: [-1.0, -1.0] });
                    verts.push(Vertex { position: [spark.x + fx + nx, spark.y + fy + ny, z], normal: n, color, uv: [ 1.0, -1.0] });
                    verts.push(Vertex { position: [spark.x + fx - nx, spark.y + fy - ny, z], normal: n, color, uv: [ 1.0,  1.0] });
                    verts.push(Vertex { position: [spark.x - fx - nx, spark.y - fy - ny, z], normal: n, color, uv: [-1.0,  1.0] });
                    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
                }
            }

            for t in &burst.trails {
                let alpha = (t.life / 0.3).clamp(0.0, 1.0);
                let s = 0.03;
                let c = [t.color[0], t.color[1], t.color[2], t.color[3] * alpha * alpha];
                let base = verts.len() as u32;
                verts.push(Vertex { position: [t.x - s, t.y - s, z], normal: n, color: c, uv: [-1.0, -1.0] });
                verts.push(Vertex { position: [t.x + s, t.y - s, z], normal: n, color: c, uv: [ 1.0, -1.0] });
                verts.push(Vertex { position: [t.x + s, t.y + s, z], normal: n, color: c, uv: [ 1.0,  1.0] });
                verts.push(Vertex { position: [t.x - s, t.y + s, z], normal: n, color: c, uv: [-1.0,  1.0] });
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }
        }

        // --- Multi-stage shells ---
        let z_shell = -2.0;

        for shell in &self.shells {
            // Shell smoke — multi-layer volumetric
            for (pi, puff) in shell.smoke.iter().enumerate() {
                let life_t = (puff.life / puff.max_life).clamp(0.0, 1.0);
                let alpha = life_t * life_t * 0.035;
                if alpha < 0.002 { continue; }
                let s = puff.size;
                let streamer_glow = if !shell.streamers.is_empty() { 0.2 } else { 0.0 };
                let c = [
                    puff.color[0] + shell.color[0] * streamer_glow,
                    puff.color[1] + shell.color[1] * streamer_glow,
                    puff.color[2] + shell.color[2] * streamer_glow,
                    alpha,
                ];
                let puff_z = z_shell - 0.3 - (pi as f32 * 0.07) % 0.8;
                let base = verts.len() as u32;
                verts.push(Vertex { position: [puff.x - s, -puff.y - s, puff_z], normal: n, color: c, uv: [-1.0, -1.0] });
                verts.push(Vertex { position: [puff.x + s, -puff.y - s, puff_z], normal: n, color: c, uv: [ 1.0, -1.0] });
                verts.push(Vertex { position: [puff.x + s, -puff.y + s, puff_z], normal: n, color: c, uv: [ 1.0,  1.0] });
                verts.push(Vertex { position: [puff.x - s, -puff.y + s, puff_z], normal: n, color: c, uv: [-1.0,  1.0] });
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }

            if shell.phase == ShellPhase::Launch || matches!(shell.phase, ShellPhase::Waiting(_)) {
                let s = 0.08;
                let c = [1.0, 0.95, 0.7, 1.0];
                let base = verts.len() as u32;
                verts.push(Vertex { position: [shell.x - s, -shell.y - s, z_shell], normal: n, color: c, uv: [-1.0, -1.0] });
                verts.push(Vertex { position: [shell.x + s, -shell.y - s, z_shell], normal: n, color: c, uv: [ 1.0, -1.0] });
                verts.push(Vertex { position: [shell.x + s, -shell.y + s, z_shell], normal: n, color: c, uv: [ 1.0,  1.0] });
                verts.push(Vertex { position: [shell.x - s, -shell.y + s, z_shell], normal: n, color: c, uv: [-1.0,  1.0] });
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }

            for spark in &shell.streamers {
                let alpha = (spark.life / spark.max_life).powf(0.5);
                let color = [spark.color[0], spark.color[1], spark.color[2], alpha];
                let speed = (spark.vx * spark.vx + spark.vy * spark.vy).sqrt();
                let half_w = 0.05;
                let half_len = (0.08 + speed * 0.03).min(0.4);

                if speed > 0.01 {
                    let dx = spark.vx / speed;
                    let dy = spark.vy / speed;
                    let nx_s = -dy * half_w;
                    let ny_s = dx * half_w;
                    let fx = dx * half_len;
                    let fy = dy * half_len;
                    let base = verts.len() as u32;
                    verts.push(Vertex { position: [spark.x - fx + nx_s, -(spark.y - fy + ny_s), z_shell], normal: n, color, uv: [-1.0, -1.0] });
                    verts.push(Vertex { position: [spark.x + fx + nx_s, -(spark.y + fy + ny_s), z_shell], normal: n, color, uv: [ 1.0, -1.0] });
                    verts.push(Vertex { position: [spark.x + fx - nx_s, -(spark.y + fy - ny_s), z_shell], normal: n, color, uv: [ 1.0,  1.0] });
                    verts.push(Vertex { position: [spark.x - fx - nx_s, -(spark.y - fy - ny_s), z_shell], normal: n, color, uv: [-1.0,  1.0] });
                    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
                }
            }

            for t in &shell.trails {
                let alpha = (t.life / t.max_life).clamp(0.0, 1.0);
                let s = t.size * (0.5 + alpha * 0.5);
                let color = [t.color[0], t.color[1], t.color[2], t.color[3] * alpha];
                let base = verts.len() as u32;
                verts.push(Vertex { position: [t.x - s, -t.y - s, z_shell], normal: n, color, uv: [-1.0, -1.0] });
                verts.push(Vertex { position: [t.x + s, -t.y - s, z_shell], normal: n, color, uv: [ 1.0, -1.0] });
                verts.push(Vertex { position: [t.x + s, -t.y + s, z_shell], normal: n, color, uv: [ 1.0,  1.0] });
                verts.push(Vertex { position: [t.x - s, -t.y + s, z_shell], normal: n, color, uv: [-1.0,  1.0] });
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }
        }
    }
}
