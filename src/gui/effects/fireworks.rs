// Firework bursts — radial streaks that spawn behind the board on strong beats.

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

struct Burst {
    sparks: Vec<Spark>,
    trails: Vec<BurstTrail>,
    flash_x: f32,
    flash_y: f32,
    flash_timer: f32,
    flash_color: [f32; 4],
}

// --- Multi-stage firework shells (rare dramatic events) ---

#[derive(PartialEq)]
enum ShellPhase { Launch, Detonate, Cascade }

struct TrailParticle {
    x: f32, y: f32,
    vx: f32, vy: f32,
    life: f32, max_life: f32,
    color: [f32; 4],
    size: f32,
}

struct Shell {
    phase: ShellPhase,
    // Launch state
    x: f32, y: f32,
    vx: f32, vy: f32,
    launch_timer: f32,
    // Streamers + trails (populated on detonate)
    streamers: Vec<Spark>,
    trails: Vec<TrailParticle>,
    color: [f32; 4],
    timer: f32, // total time alive
}

pub struct Fireworks {
    bursts: Vec<Burst>,
    shells: Vec<Shell>,
    pub shell_cooldown: f32, // seconds until next shell can spawn
    rng: u64,
    prev_beat: [bool; 7],
    prev_flux: f32,
    pub trigger_band: Option<usize>, // None = any band (legacy), Some(n) = only band n
    pub shells_only: bool, // skip quick bursts, only spawn multi-stage shells
    pub bursts_only: bool, // skip shells, only spawn quick bursts
}

fn rng_next(rng: &mut u64) -> f32 {
    *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    ((*rng >> 33) as f32) / (u32::MAX as f32 / 2.0)
}

impl Fireworks {
    pub fn new() -> Self {
        Fireworks {
            bursts: Vec::new(),
            shells: Vec::new(),
            shell_cooldown: 5.0, // first shell after 5s
            rng: 0xCAFEBABE42,
            prev_beat: [false; 7],
            prev_flux: 0.0,
            trigger_band: None,
            shells_only: false,
            bursts_only: false,
        }
    }
}

impl AudioEffect for Fireworks {
    fn pass(&self) -> RenderPass {
        RenderPass::Transparent
    }

    fn update(&mut self, audio: &AudioFrame) {
        // Spawn burst on band beat (edge-triggered) — skip if shells_only
        if self.shells_only { for band in 0..7 { self.prev_beat[band] = audio.band_beats[band] > 0.95; } }
        let band_range: Vec<usize> = if self.shells_only { vec![] } else { match self.trigger_band {
            Some(b) => vec![b],
            None => (0..7).collect(),
        } };
        for band in band_range {
            let is_beat = audio.band_beats[band] > 0.95;
            if is_beat && !self.prev_beat[band] {
                // Only spawn for stronger bands — skip if energy is low
                if audio.bands_norm[band] > 0.85 {
                    let cx = rng_next(&mut self.rng) * 30.0 - 10.0;  // wide spread: -10 to 20
                    let cy = -(rng_next(&mut self.rng).abs() * 24.0); // full visible height
                    let spark_count = 20 + (audio.bands_norm[band] * 20.0) as usize;

                    // White bursts — clean HDR flash
                    let base_color = [1.8, 1.8, 1.8, 0.9];

                    let mut sparks = Vec::with_capacity(spark_count);
                    for _ in 0..spark_count {
                        let angle = rng_next(&mut self.rng) * std::f32::consts::TAU;
                        let speed = 3.0 + rng_next(&mut self.rng).abs() * 7.0;
                        let life = 0.3 + rng_next(&mut self.rng).abs() * 0.45; // snap decay
                        sparks.push(Spark {
                            x: cx,
                            y: cy,
                            vx: angle.cos() * speed,
                            vy: angle.sin() * speed,
                            life,
                            max_life: life,
                            color: base_color,
                        });
                    }
                    // Flash at burst origin
                    let flash_color = [
                        base_color[0].min(1.0) * 0.5 + 0.5,
                        base_color[1].min(1.0) * 0.5 + 0.5,
                        base_color[2].min(1.0) * 0.5 + 0.5,
                        1.0,
                    ];
                    self.bursts.push(Burst { sparks, trails: Vec::new(), flash_x: cx, flash_y: cy, flash_timer: 0.15, flash_color });
                }
            }
            self.prev_beat[band] = is_beat;
        }

        // Update all sparks + flash
        for burst in &mut self.bursts {
            burst.flash_timer -= audio.dt;
            for spark in &mut burst.sparks {
                spark.x += spark.vx * audio.dt;
                spark.y += spark.vy * audio.dt;
                spark.vy += 0.3 * audio.dt; // gentle droop
                spark.vx *= 0.9975;
                spark.vy *= 0.9975;
                spark.life -= audio.dt;

                // Drop trail particle at spark position
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

            // Decay trails
            for t in &mut burst.trails {
                t.life -= audio.dt;
            }
            burst.trails.retain(|t| t.life > 0.0);
        }
        self.bursts.retain(|b| !b.sparks.is_empty() || !b.trails.is_empty() || b.flash_timer > 0.0);

        // --- Multi-stage shells ---
        if self.bursts_only { self.shell_cooldown = 1.0; } else {
        self.shell_cooldown -= audio.dt;

        // Spawn shell on flux spike, strong bass, or any decent beat
        let flux_spike = audio.flux > 0.5 && self.prev_flux < 0.3;
        let strong_bass = audio.band_beats[0] > 0.8 && audio.bands_norm[0] > 0.6;
        let any_strong = audio.band_beats.iter().any(|&b| b > 0.9);
        if self.shell_cooldown <= 0.0 && (flux_spike || strong_bass || any_strong) && self.shells.len() < 3 {
            let cx = rng_next(&mut self.rng) * 26.0 - 8.0; // wider spread: -8 to 18
            let start_y = 22.0; // below bottom of board (row 20 = bottom)
            // Color: random piece color (I=cyan, O=yellow, T=purple, S=green, Z=red, J=blue, L=orange)
            let piece_colors: [[f32; 3]; 8] = [
                [0.0, 1.0, 1.0],     // I - cyan
                [1.0, 1.0, 0.0],     // O - yellow
                [0.5, 0.0, 0.5],     // T - purple
                [0.0, 1.0, 0.0],     // S - green
                [1.0, 0.0, 0.0],     // Z - red
                [0.0, 0.0, 1.0],     // J - blue
                [1.0, 0.47, 0.0],    // L - orange
                [1.0, 1.0, 1.0],     // white
            ];
            let ci = (rng_next(&mut self.rng).abs() * 8.0) as usize % 8;
            let pc = piece_colors[ci];
            let color = [pc[0], pc[1], pc[2], 1.0];
            self.shells.push(Shell {
                phase: ShellPhase::Launch,
                x: cx, y: start_y,
                vx: rng_next(&mut self.rng) * 1.2,
                vy: -(5.8 + rng_next(&mut self.rng).abs() * 3.4), // launch from below board, detonate mid-screen
                launch_timer: 0.0,
                streamers: Vec::new(),
                trails: Vec::new(),
                color,
                timer: 0.0,
            });
            self.shell_cooldown = 3.0 + rng_next(&mut self.rng).abs() * 7.0; // 3-10s between shells
        }
        self.prev_flux = audio.flux;
        } // end !bursts_only

        // Update shells (extract rng to avoid borrow conflict)
        let mut rng = self.rng;
        for shell in &mut self.shells {
            shell.timer += audio.dt;

            match shell.phase {
                ShellPhase::Launch => {
                    shell.launch_timer += audio.dt;
                    shell.x += shell.vx * audio.dt;
                    shell.y += shell.vy * audio.dt;
                    shell.vy += 1.2 * audio.dt; // gravity decelerates upward motion

                    // Spawn trail sparkles during launch
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

                    // Detonate at apex (vy crosses zero or after 2.5s)
                    if shell.vy > 0.0 || shell.timer > 2.5 {
                        shell.phase = ShellPhase::Detonate;
                        // Spawn 30-50 primary streamers
                        let count = 30 + (rng_next(&mut rng).abs() * 20.0) as usize;
                        for _ in 0..count {
                            let angle = rng_next(&mut rng) * std::f32::consts::TAU;
                            let speed = 2.2 + rng_next(&mut rng).abs() * 3.3;
                            let life = 2.0 + rng_next(&mut rng).abs() * 2.0;
                            shell.streamers.push(Spark {
                                x: shell.x, y: shell.y,
                                vx: angle.cos() * speed,
                                vy: angle.sin() * speed,
                                life, max_life: life,
                                color: shell.color,
                            });
                        }
                    }
                }
                ShellPhase::Detonate => {
                    // Brief phase — immediately transitions to Cascade
                    shell.phase = ShellPhase::Cascade;
                }
                ShellPhase::Cascade => {
                    // Update streamers
                    for s in &mut shell.streamers {
                        s.x += s.vx * audio.dt;
                        s.y += s.vy * audio.dt;
                        s.vy += 0.4 * audio.dt; // very gentle gravity — round burst that slowly droops
                        s.life -= audio.dt;

                        // Secondary trail particles from streamers
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

            // Update trail particles (all phases)
            for t in &mut shell.trails {
                t.x += t.vx * audio.dt;
                t.y += t.vy * audio.dt;
                t.vy += 1.5 * audio.dt; // gravity droop
                t.life -= audio.dt;
            }
            shell.trails.retain(|t| t.life > 0.0);
        }
        self.rng = rng; // write back rng state
        // Remove dead shells
        self.shells.retain(|s| !s.streamers.is_empty() || !s.trails.is_empty() || s.phase == ShellPhase::Launch);
    }

    fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, _ctx: &RenderContext) {
        let n = [0.0f32, 0.0, 1.0];
        let z = -1.5; // behind board, in front of hex grid

        for burst in &self.bursts {
            // Flash glow at burst origin
            if burst.flash_timer > 0.0 {
                let ft = (burst.flash_timer / 0.15).clamp(0.0, 1.0);
                let flash_alpha = ft * ft * ft; // cubic — very fast snap
                let flash_size = 0.5 + (1.0 - ft) * 0.8; // expands as it fades
                let fc = [
                    burst.flash_color[0] * 4.0, // bright HDR flash — heavy bloom
                    burst.flash_color[1] * 4.0,
                    burst.flash_color[2] * 4.0,
                    flash_alpha,
                ];
                let base = verts.len() as u32;
                verts.push(Vertex { position: [burst.flash_x - flash_size, burst.flash_y - flash_size, z], normal: n, color: fc });
                verts.push(Vertex { position: [burst.flash_x + flash_size, burst.flash_y - flash_size, z], normal: n, color: fc });
                verts.push(Vertex { position: [burst.flash_x + flash_size, burst.flash_y + flash_size, z], normal: n, color: fc });
                verts.push(Vertex { position: [burst.flash_x - flash_size, burst.flash_y + flash_size, z], normal: n, color: fc });
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }

            for spark in &burst.sparks {
                let alpha = (spark.life / spark.max_life).powf(0.4); // slower decay curve
                let color = [
                    spark.color[0],
                    spark.color[1],
                    spark.color[2],
                    spark.color[3] * alpha,
                ];
                // Render as a small stretched quad in the direction of motion
                let speed = (spark.vx * spark.vx + spark.vy * spark.vy).sqrt();
                let half_w = 0.04;
                let half_len = (0.05 + speed * 0.02).min(0.3);

                if speed < 0.01 {
                    // Dot for stationary sparks
                    let base = verts.len() as u32;
                    verts.push(Vertex { position: [spark.x - half_w, spark.y - half_w, z], normal: n, color });
                    verts.push(Vertex { position: [spark.x + half_w, spark.y - half_w, z], normal: n, color });
                    verts.push(Vertex { position: [spark.x + half_w, spark.y + half_w, z], normal: n, color });
                    verts.push(Vertex { position: [spark.x - half_w, spark.y + half_w, z], normal: n, color });
                    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
                } else {
                    // Streak aligned to velocity
                    let dx = spark.vx / speed;
                    let dy = spark.vy / speed;
                    let nx = -dy * half_w;
                    let ny = dx * half_w;
                    let fx = dx * half_len;
                    let fy = dy * half_len;

                    let base = verts.len() as u32;
                    verts.push(Vertex { position: [spark.x - fx + nx, spark.y - fy + ny, z], normal: n, color });
                    verts.push(Vertex { position: [spark.x + fx + nx, spark.y + fy + ny, z], normal: n, color });
                    verts.push(Vertex { position: [spark.x + fx - nx, spark.y + fy - ny, z], normal: n, color });
                    verts.push(Vertex { position: [spark.x - fx - nx, spark.y - fy - ny, z], normal: n, color });
                    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
                }
            }

            // Burst trails — fading dots along spark paths
            for t in &burst.trails {
                let alpha = (t.life / 0.3).clamp(0.0, 1.0);
                let s = 0.03;
                let c = [t.color[0], t.color[1], t.color[2], t.color[3] * alpha * alpha];
                let base = verts.len() as u32;
                verts.push(Vertex { position: [t.x - s, t.y - s, z], normal: n, color: c });
                verts.push(Vertex { position: [t.x + s, t.y - s, z], normal: n, color: c });
                verts.push(Vertex { position: [t.x + s, t.y + s, z], normal: n, color: c });
                verts.push(Vertex { position: [t.x - s, t.y + s, z], normal: n, color: c });
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }
        }

        // --- Multi-stage shells ---
        let z_shell = -2.0; // behind bursts

        for shell in &self.shells {
            // Launch point (bright ascending dot)
            if shell.phase == ShellPhase::Launch {
                let s = 0.08;
                let c = [1.0, 0.95, 0.7, 1.0];
                let base = verts.len() as u32;
                verts.push(Vertex { position: [shell.x - s, -shell.y - s, z_shell], normal: n, color: c });
                verts.push(Vertex { position: [shell.x + s, -shell.y - s, z_shell], normal: n, color: c });
                verts.push(Vertex { position: [shell.x + s, -shell.y + s, z_shell], normal: n, color: c });
                verts.push(Vertex { position: [shell.x - s, -shell.y + s, z_shell], normal: n, color: c });
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }

            // Streamers (velocity-aligned streaks, same as bursts)
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
                    verts.push(Vertex { position: [spark.x - fx + nx_s, -(spark.y - fy + ny_s), z_shell], normal: n, color });
                    verts.push(Vertex { position: [spark.x + fx + nx_s, -(spark.y + fy + ny_s), z_shell], normal: n, color });
                    verts.push(Vertex { position: [spark.x + fx - nx_s, -(spark.y + fy - ny_s), z_shell], normal: n, color });
                    verts.push(Vertex { position: [spark.x - fx - nx_s, -(spark.y - fy - ny_s), z_shell], normal: n, color });
                    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
                }
            }

            // Trail particles (small fading dots)
            for t in &shell.trails {
                let alpha = (t.life / t.max_life).clamp(0.0, 1.0);
                let s = t.size * (0.5 + alpha * 0.5);
                let color = [t.color[0], t.color[1], t.color[2], t.color[3] * alpha];
                let base = verts.len() as u32;
                verts.push(Vertex { position: [t.x - s, -t.y - s, z_shell], normal: n, color });
                verts.push(Vertex { position: [t.x + s, -t.y - s, z_shell], normal: n, color });
                verts.push(Vertex { position: [t.x + s, -t.y + s, z_shell], normal: n, color });
                verts.push(Vertex { position: [t.x - s, -t.y + s, z_shell], normal: n, color });
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }
        }
    }
}
