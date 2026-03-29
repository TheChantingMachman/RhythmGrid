// Firework bursts — radial streaks that spawn behind the board on strong beats.
// Multi-stage shells launch, detonate, and cascade with gravity-affected trails.
// Smoke puffs expand and dissipate from each burst, illuminated by burst glow.

use super::{AudioEffect, AudioFrame, GpuEffect, RenderContext, RenderPass};
use crate::gui::drawing::Vertex;
use crate::gui::renderer::GpuOitDrawCmd;

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
    // GPU compute port
    gpu: Option<FireworksGpu>,
}

fn rng_next(rng: &mut u64) -> f32 {
    *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    ((*rng >> 33) as f32) / (u32::MAX as f32 / 2.0)
}

impl Fireworks {
    /// Push a particle to the GPU buffer if GPU is active.
    fn gpu_push(&mut self, p: FwParticleGpu) {
        if let Some(ref mut gpu) = self.gpu {
            gpu.pending_spawns.push(p);
        }
    }

    #[allow(dead_code)]
    fn gpu_push_spark(&mut self, x: f32, y: f32, vx: f32, vy: f32, life: f32, color: [f32; 4], z: f32) {
        self.gpu_push(FwParticleGpu {
            pos_vel: [x, y, vx, vy],
            life_meta: [life, life, 0.04, TYPE_SPARK],
            color,
            physics: [z, -0.3, 0.9975, 0.0], // negative gravity — burst Y is already negative
        });
    }

    #[allow(dead_code)]
    fn gpu_push_burst_trail(&mut self, x: f32, y: f32, life: f32, color: [f32; 4], z: f32) {
        self.gpu_push(FwParticleGpu {
            pos_vel: [x, y, 0.0, 0.0],
            life_meta: [life, life, 0.03, TYPE_BURST_TRAIL],
            color,
            physics: [z, 0.0, 1.0, 0.0],
        });
    }

    fn gpu_push_smoke(&mut self, x: f32, y: f32, vx: f32, vy: f32, size: f32, life: f32, max_life: f32, color: [f32; 3], z: f32, negate_y: bool) {
        self.gpu_push(FwParticleGpu {
            pos_vel: [x, y, vx, if negate_y { -vy } else { vy }],
            life_meta: [life, max_life, size, TYPE_SMOKE],
            color: [color[0], color[1], color[2], 1.0],
            physics: [z, 0.0, 0.98, 0.35],
        });
    }

    #[allow(dead_code)]
    fn gpu_push_shell_trail(&mut self, x: f32, y: f32, vx: f32, vy: f32, life: f32, color: [f32; 4], size: f32, z: f32) {
        self.gpu_push(FwParticleGpu {
            pos_vel: [x, -y, vx, -vy], // negate Y position + velocity
            life_meta: [life, life, size, TYPE_SHELL_TRAIL],
            color,
            physics: [z, -1.5, 1.0, 0.0], // negative gravity in Y-up space
        });
    }

    fn gpu_push_flash(&mut self, x: f32, y: f32, color: [f32; 4], z: f32) {
        self.gpu_push(FwParticleGpu {
            pos_vel: [x, y, 0.0, 0.0],
            life_meta: [0.15, 0.15, 0.5, TYPE_FLASH],
            color,
            physics: [z, 0.0, 1.0, 0.0],
        });
    }

    #[allow(dead_code)]
    fn gpu_push_shell_streamer(&mut self, x: f32, y: f32, vx: f32, vy: f32, life: f32, color: [f32; 4], z: f32) {
        self.gpu_push(FwParticleGpu {
            pos_vel: [x, -y, vx, -vy], // negate Y position + velocity
            life_meta: [life, life, 0.05, TYPE_SHELL_STREAMER],
            color,
            physics: [z, -0.4, 1.0, 0.0], // negative gravity in Y-up space
        });
    }

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
            gpu: None,
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
            let spark_count = 50 + (rng_next(&mut self.rng).abs() * 30.0) as usize;
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
                    let speed = (3.0 + rng_next(&mut self.rng).abs() * 7.0) * 0.64;
                    let life = 0.18 + rng_next(&mut self.rng).abs() * 0.24;
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
                // Mirror to GPU
                let z_burst = -1.5;
                self.gpu_push_flash(cx, cy, [flash_color[0] * 4.0, flash_color[1] * 4.0, flash_color[2] * 4.0, 1.0], z_burst);
                // Build burst trail particles — pre-generate to avoid borrow conflicts
                let mut burst_gpu: Vec<FwParticleGpu> = Vec::new();
                for s in &sparks {
                    burst_gpu.push(FwParticleGpu {
                        pos_vel: [s.x, s.y, s.vx, s.vy],
                        life_meta: [s.life, s.life, 0.04, TYPE_SPARK],
                        color: s.color,
                        physics: [z_burst, -0.3, 0.9975, 0.0],
                    });
                    let trail_per_spark = 40; // 30% of 64 = ~19, doubled = ~40
                    let sp_speed = (s.vx * s.vx + s.vy * s.vy).sqrt().max(0.01);
                    let sp_perp_x = -s.vy / sp_speed;
                    let sp_perp_y = s.vx / sp_speed;
                    for k in 0..trail_per_spark {
                        let frac = (k as f32 + 1.0) / trail_per_spark as f32;
                        // Only first 1/3 and last 30% (same pattern as cascade)
                        if frac > 0.33 && frac < 0.70 { continue; }
                        let t_jitter = rng_next(&mut self.rng) * 0.03;
                        let t = s.life * frac + t_jitter;
                        let gravity_burst = 0.3;
                        let pred_x = s.x + s.vx * t;
                        let pred_y = s.y + s.vy * t - gravity_burst * t * t * 0.5;
                        // Along-path stagger
                        let along = rng_next(&mut self.rng) * 0.06;
                        let px = pred_x + (s.vx / sp_speed) * along;
                        let py = pred_y + (s.vy / sp_speed) * along;
                        // Perpendicular drift velocity for wake
                        let drift = rng_next(&mut self.rng) * 0.24;
                        let trail_life = 0.192 + rng_next(&mut self.rng).abs() * 0.24;
                        // Forward velocity (20% of spark speed) + perpendicular drift
                        let fwd = 0.2;
                        burst_gpu.push(FwParticleGpu {
                            pos_vel: [px, py,
                                s.vx * fwd + sp_perp_x * drift + rng_next(&mut self.rng) * 0.02,
                                s.vy * fwd + sp_perp_y * drift + rng_next(&mut self.rng) * 0.02],
                            life_meta: [-t, trail_life, 0.024, TYPE_BURST_TRAIL],
                            color: [s.color[0] * 1.5, s.color[1] * 1.5, s.color[2] * 1.5, 0.8],
                            physics: [z_burst, -0.1, 1.0, 0.0],
                        });
                    }
                }
                if let Some(ref mut gpu) = self.gpu {
                    gpu.pending_spawns.extend(burst_gpu);
                }
                for puff in &smoke {
                    self.gpu_push_smoke(puff.x, puff.y, puff.vx, puff.vy, puff.size, puff.life, puff.max_life, puff.color, z_burst - 0.5, false);
                }

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
                let speed = (3.0 + rng_next(&mut self.rng).abs() * 7.0) * 0.8;
                let life = 0.15 + rng_next(&mut self.rng).abs() * 0.2;
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
        let mut gpu_shell_spawns: Vec<FwParticleGpu> = Vec::new();
        let z_shell = -2.0;
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
                        // Spawn a cluster of sparkles
                        // Perpendicular to shell velocity for spread
                        let sv_speed = (shell.vx * shell.vx + shell.vy * shell.vy).sqrt().max(0.01);
                        let sv_px = -shell.vy / sv_speed;
                        let sv_py = shell.vx / sv_speed;
                        for k in 0..10 {
                            let spread = rng_next(&mut rng) * 0.55;
                            let tvx = sv_px * spread + rng_next(&mut rng) * 0.12;
                            let tvy = sv_py * spread + rng_next(&mut rng) * 0.12;
                            // Stagger spawn position along shell trajectory (0-50ms worth of travel)
                            let stagger_t = k as f32 * 0.005;
                            let stagger_x = shell.vx * stagger_t;
                            let stagger_y = shell.vy * stagger_t;
                            let life = 0.30 + rng_next(&mut rng).abs() * 0.24;
                            shell.trails.push(TrailParticle {
                                x: shell.x + stagger_x, y: shell.y + stagger_y,
                                vx: tvx, vy: tvy,
                                life, max_life: life,
                                color: [1.0, 0.9, 0.6, 0.7],
                                size: 0.06,
                            });
                            gpu_shell_spawns.push(FwParticleGpu {
                                pos_vel: [shell.x + stagger_x, -(shell.y + stagger_y), tvx, -tvy],
                                life_meta: [life, life, 0.037, TYPE_SHELL_TRAIL],
                                color: [1.0, 0.9, 0.6, 0.7],
                                physics: [z_shell, -0.5, 1.0, 0.0],
                            });
                        }
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
                            let life = 2.5 + rng_next(&mut rng).abs() * 1.5;
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
                        // Mirror streamers + smoke to GPU
                        for s in &shell.streamers {
                            // Add white to the streamer color
                            let white_mix = 0.4;
                            let sc = [
                                s.color[0] + (1.0 - s.color[0]) * white_mix,
                                s.color[1] + (1.0 - s.color[1]) * white_mix,
                                s.color[2] + (1.0 - s.color[2]) * white_mix,
                                s.color[3],
                            ];
                            gpu_shell_spawns.push(FwParticleGpu {
                                pos_vel: [s.x, -s.y, s.vx, -s.vy],
                                life_meta: [s.life, s.max_life, 0.035, TYPE_SHELL_STREAMER],
                                color: sc,
                                physics: [z_shell, -0.92, 1.0, 0.0], // strong arc
                            });
                        }
                        for puff in &shell.smoke {
                            gpu_shell_spawns.push(FwParticleGpu {
                                pos_vel: [puff.x, -puff.y, puff.vx, -puff.vy], // negate vy
                                life_meta: [puff.life, puff.max_life, puff.size, TYPE_SMOKE],
                                color: [puff.color[0], puff.color[1], puff.color[2], 1.0],
                                physics: [z_shell - 0.5, 0.0, 0.98, 0.3],
                            });
                        }
                        // Pre-spawn cascade trail particles at predicted streamer positions.
                        // Each trail spawns where the streamer WILL BE at a future time,
                        // with near-zero velocity so it stays put and droops with gravity.
                        // Cascade trails appear in the back half of the streamer's life
                        // Trails emit in first 1/3 and last 1/3 of streamer life (CPU behavior)
                        let gravity_streamer = 0.92; // must match streamer GPU gravity
                        for s in &shell.streamers {
                            let trail_count = 300;
                            let s_speed = (s.vx * s.vx + s.vy * s.vy).sqrt().max(0.01);
                            let perp_ux = -s.vy / s_speed;
                            let perp_uy = s.vx / s_speed;
                            for j in 0..trail_count {
                                let frac = j as f32 / trail_count as f32;
                                // Only emit in first 1/3 and last 30%
                                if frac > 0.33 && frac < 0.70 { continue; }
                                // Stagger along the path — each trail offset by a small time delta
                                let t_jitter = rng_next(&mut rng) * 0.06; // ±60ms jitter
                                let t = s.life * frac + t_jitter;
                                // Predict base position
                                let base_x = s.x + s.vx * t;
                                let base_y = s.y + s.vy * t + 0.5 * gravity_streamer * t * t;
                                // Stagger along flight direction for sub-particle spread
                                let along_stagger = rng_next(&mut rng) * 0.15;
                                let pred_x = base_x + (s.vx / s_speed) * along_stagger;
                                let pred_y = base_y + (s.vy / s_speed) * along_stagger;
                                let trail_life = 1.2 + rng_next(&mut rng).abs() * 1.2;
                                // Spawn ON the path — perpendicular drift velocity creates wake
                                let drift = rng_next(&mut rng) * 0.55;
                                gpu_shell_spawns.push(FwParticleGpu {
                                    pos_vel: [pred_x, -pred_y,
                                        perp_ux * drift + rng_next(&mut rng) * 0.03,
                                        -(perp_uy * drift) + rng_next(&mut rng) * 0.03],
                                    life_meta: [-t, trail_life, 0.025, TYPE_CASCADE_TRAIL],
                                    color: [shell.color[0] * 0.7, shell.color[1] * 0.7, shell.color[2] * 0.7, 0.5],
                                    physics: [z_shell, -0.69, 1.0, 0.0],
                                });
                            }
                        }
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

        // Push collected GPU shell particles
        if let Some(ref mut gpu) = self.gpu {
            gpu.pending_spawns.extend(gpu_shell_spawns);
        }
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

// ===== GPU Compute Port =====

const FW_MAX_PARTICLES: usize = 65536; // 64K particles, 4MB buffer

// Type IDs for particle rendering
const TYPE_SPARK: f32 = 0.0;
const TYPE_BURST_TRAIL: f32 = 1.0;
const TYPE_SMOKE: f32 = 2.0;
const TYPE_SHELL_TRAIL: f32 = 3.0;
const TYPE_CASCADE_TRAIL: f32 = 4.0;
const TYPE_SHELL_STREAMER: f32 = 5.0; // shell detonation arcs
const TYPE_FLASH: f32 = 6.0;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct FwParticleGpu {
    pos_vel: [f32; 4],     // xy = position, zw = velocity
    life_meta: [f32; 4],   // x = life, y = max_life, z = size, w = type_id
    color: [f32; 4],       // rgba (HDR, values > 1.0 for bloom)
    physics: [f32; 4],     // x = z_depth, y = gravity, z = drag, w = expansion_rate
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct FwUniformsGpu {
    dt: f32,
    wind_x: f32,
    active_count: u32,
    _pad: f32,
}

const FW_COMPUTE_WGSL: &str = r#"
struct FwUniforms {
    dt: f32,
    wind_x: f32,
    active_count: u32,
    _pad: f32,
};

struct FwParticle {
    pos_vel: vec4<f32>,
    life_meta: vec4<f32>,
    color: vec4<f32>,
    physics: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: FwUniforms;
@group(0) @binding(1) var<storage, read_write> particles: array<FwParticle>;

@compute @workgroup_size(64)
fn cs_fireworks(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= u.active_count) { return; }

    var p = particles[idx];
    let type_id = u32(p.life_meta.w);

    // Spawn delay (negative life) — for cascade trails (4) and burst trails (1)
    // physics.w == 0.0 means not yet activated; once activated, set to 1.0
    if (p.life_meta.x < 0.0 && p.physics.w == 0.0) {
        if (type_id == 4u || type_id == 1u) {
            p.life_meta.x += u.dt;
            if (p.life_meta.x >= 0.0) {
                p.life_meta.x = p.life_meta.y; // activate with full life
                p.physics.w = 1.0; // mark as activated — won't re-trigger
            }
        }
        particles[idx] = p;
        return;
    }

    if (p.life_meta.x <= 0.0) { return; } // dead
    let gravity = p.physics.y;
    let drag = p.physics.z;
    let expansion = p.physics.w;

    // Gravity (Y axis)
    p.pos_vel.w += gravity * u.dt;

    // Drag (frame-rate independent)
    let drag_factor = pow(drag, u.dt * 60.0);
    p.pos_vel.z *= drag_factor;
    p.pos_vel.w *= drag_factor;

    // Position integration
    p.pos_vel.x += p.pos_vel.z * u.dt;
    p.pos_vel.y += p.pos_vel.w * u.dt;

    // Wind (smoke only: type 2)
    if (type_id == 2u) {
        p.pos_vel.x += u.wind_x * 0.5 * u.dt;
    }

    // Smoke expansion (type 2 only)
    if (type_id == 2u && expansion > 0.0) {
        p.life_meta.z += expansion * u.dt;
    }

    // Life decay
    p.life_meta.x -= u.dt;

    particles[idx] = p;
}
"#;

const FW_RENDER_WGSL: &str = r#"
struct SceneUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

struct FwParticle {
    pos_vel: vec4<f32>,
    life_meta: vec4<f32>,
    color: vec4<f32>,
    physics: vec4<f32>,
};

@group(0) @binding(0) var<uniform> scene: SceneUniforms;
@group(1) @binding(0) var<storage, read> particles: array<FwParticle>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world_pos: vec3<f32>,
};

@vertex
fn vs_firework(
    @builtin(vertex_index) vid: u32,
    @builtin(instance_index) iid: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let p = particles[iid];
    let life = p.life_meta.x;
    let max_life = p.life_meta.y;
    let size = p.life_meta.z;
    let type_id = u32(p.life_meta.w);
    let z_depth = p.physics.x;

    // Dead or delayed particle — degenerate
    if (life <= 0.0) {
        out.clip_position = vec4(0.0, 0.0, 2.0, 1.0);
        out.color = vec4(0.0);
        out.uv = vec2(0.0);
        out.world_pos = vec3(0.0);
        return out;
    }

    let life_t = clamp(life / max(max_life, 0.001), 0.0, 1.0);

    // Alpha based on type — aggressive fade
    var alpha: f32;
    if (type_id == 0u) {
        // Burst spark: slower fade (power 1.6)
        alpha = pow(life_t, 1.6) * p.color.a;
    } else if (type_id == 5u) {
        // Shell streamer: slower fade (power 2.0)
        alpha = life_t * life_t * p.color.a;
    } else if (type_id == 1u) {
        // Burst trail: hold full for 40% of life, then fade
        let fade_t = clamp((1.0 - life_t) / 0.6, 0.0, 1.0); // 0 for first 40%, ramps to 1
        alpha = (1.0 - fade_t * fade_t) * p.color.a;
    } else if (type_id == 2u) {
        // Smoke: very faint, quadratic
        alpha = life_t * life_t * 0.02;
    } else if (type_id == 3u || type_id == 4u) {
        // Shell trail / cascade trail: hold full for 40% of life, then fade
        let fade_t = clamp((1.0 - life_t) / 0.6, 0.0, 1.0);
        alpha = (1.0 - fade_t * fade_t) * p.color.a;
    } else {
        // Flash: cubic snap
        alpha = life_t * life_t * life_t * p.color.a;
    }

    let color = vec4(p.color.rgb, alpha);

    // Quad vertex offsets
    var offsets = array<vec2<f32>, 6>(
        vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(1.0, 1.0),
        vec2(-1.0, -1.0), vec2(1.0, 1.0), vec2(-1.0, 1.0),
    );
    let off = offsets[vid];

    var world_pos: vec3<f32>;

    if (type_id == 0u || type_id == 5u) {
        // Velocity-aligned streak (spark or shell streamer)
        let vx = p.pos_vel.z;
        let vy = p.pos_vel.w;
        let speed = length(vec2(vx, vy));
        let half_w = size; // use per-particle size
        let half_len = min(size + speed * 0.02, 0.3);

        if (speed > 0.01) {
            let dx = vx / speed;
            let dy = vy / speed;
            let nx = -dy * half_w;
            let ny = dx * half_w;
            let fx = dx * half_len;
            let fy = dy * half_len;
            world_pos = vec3(
                p.pos_vel.x + fx * off.x + nx * off.y,
                p.pos_vel.y + fy * off.x + ny * off.y,
                z_depth
            );
        } else {
            world_pos = vec3(p.pos_vel.x + off.x * half_w, p.pos_vel.y + off.y * half_w, z_depth);
        }
    } else if (type_id == 2u) {
        // Smoke: large expanding soft circle
        world_pos = vec3(p.pos_vel.x + off.x * size, p.pos_vel.y + off.y * size, z_depth);
    } else if (type_id == 6u) {
        // Flash: large HDR quad
        let flash_size = size * (0.5 + (1.0 - life_t) * 0.8);
        world_pos = vec3(p.pos_vel.x + off.x * flash_size, p.pos_vel.y + off.y * flash_size, z_depth);
    } else {
        // Dots (burst trail, shell trail, cascade trail)
        let s = size * (0.5 + life_t * 0.5);
        world_pos = vec3(p.pos_vel.x + off.x * s, p.pos_vel.y + off.y * s, z_depth);
    }

    out.clip_position = scene.view_proj * vec4(world_pos, 1.0);
    out.color = color;
    out.uv = off;
    out.world_pos = world_pos;
    return out;
}

struct OitOutput {
    @location(0) accum: vec4<f32>,
    @location(1) revealage: vec4<f32>,
};

@fragment
fn fs_firework(in: VertexOutput) -> OitOutput {
    let dist = length(in.uv);
    if (dist > 1.0) { discard; }
    let soft = smoothstep(1.0, 0.3, dist); // sharp circle edge with soft center
    let alpha = in.color.a * soft;
    if (alpha < 0.001) { discard; }

    let cam_dist = length(scene.camera_pos.xyz - in.world_pos);
    let d_norm = clamp(cam_dist / 40.0, 0.0, 1.0);
    let w = clamp(alpha * max(1e-2, 3e3 * pow(1.0 - d_norm, 4.0)), 1e-2, 3e3);

    var out: OitOutput;
    out.accum = vec4(in.color.rgb * alpha * w, alpha * w);
    out.revealage = vec4(alpha, 0.0, 0.0, 0.0);
    return out;
}
"#;

struct FireworksGpu {
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    render_bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    particle_buffer: wgpu::Buffer,
    spawn_cursor: usize,
    active_count: u32,
    pending_spawns: Vec<FwParticleGpu>,
}

impl FireworksGpu {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, scene_bgl: &wgpu::BindGroupLayout) -> Self {
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fw_uniforms"),
            size: std::mem::size_of::<FwUniformsGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fw_particles"),
            size: (FW_MAX_PARTICLES * std::mem::size_of::<FwParticleGpu>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Compute bind group
        let compute_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fw_compute_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false, min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false, min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fw_compute_bg"),
            layout: &compute_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: particle_buffer.as_entire_binding() },
            ],
        });

        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fw_compute"),
            source: wgpu::ShaderSource::Wgsl(FW_COMPUTE_WGSL.into()),
        });
        let compute_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None, bind_group_layouts: &[&compute_bgl], push_constant_ranges: &[],
        });
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("fw_compute"), layout: Some(&compute_layout),
            module: &compute_shader, entry_point: Some("cs_fireworks"),
            compilation_options: Default::default(), cache: None,
        });

        // Render bind group
        let render_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fw_render_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0, visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false, min_binding_size: None,
                },
                count: None,
            }],
        });
        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fw_render_bg"),
            layout: &render_bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: particle_buffer.as_entire_binding() }],
        });

        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fw_render"),
            source: wgpu::ShaderSource::Wgsl(FW_RENDER_WGSL.into()),
        });
        let render_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None, bind_group_layouts: &[scene_bgl, &render_bgl], push_constant_ranges: &[],
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("fw_particle"),
            layout: Some(&render_layout),
            vertex: wgpu::VertexState {
                module: &render_shader, entry_point: Some("vs_firework"),
                buffers: &[], compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader, entry_point: Some("fs_firework"),
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::R8Unorm,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::Zero,
                                dst_factor: wgpu::BlendFactor::OneMinusSrc,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::Zero,
                                dst_factor: wgpu::BlendFactor::OneMinusSrc,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 4, mask: !0, alpha_to_coverage_enabled: false,
            },
            multiview: None, cache: None,
        });

        FireworksGpu {
            compute_pipeline, compute_bind_group,
            render_pipeline, render_bind_group,
            uniform_buffer, particle_buffer,
            spawn_cursor: 0, active_count: 0,
            pending_spawns: Vec::with_capacity(256),
        }
    }
}

impl Fireworks {
    pub fn gpu_draw_cmd(&self) -> Option<GpuOitDrawCmd<'_>> {
        let gpu = self.gpu.as_ref()?;
        if gpu.active_count == 0 { return None; }
        Some(GpuOitDrawCmd {
            pipeline: &gpu.render_pipeline,
            bind_group_1: &gpu.render_bind_group,
            instances: gpu.active_count,
        })
    }
}

impl GpuEffect for Fireworks {
    fn create_gpu_resources(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, scene_bgl: &wgpu::BindGroupLayout) {
        self.gpu = Some(FireworksGpu::new(device, queue, scene_bgl));
    }

    fn compute(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder, audio: &AudioFrame) {
        let gpu = self.gpu.as_mut().unwrap();

        // Upload pending spawns
        let spawns = std::mem::take(&mut gpu.pending_spawns);
        if !spawns.is_empty() {
            let particle_size = std::mem::size_of::<FwParticleGpu>();
            for p in &spawns {
                let slot = gpu.spawn_cursor % FW_MAX_PARTICLES;
                let offset = (slot * particle_size) as u64;
                queue.write_buffer(&gpu.particle_buffer, offset, bytemuck::cast_slice(std::slice::from_ref(p)));
                gpu.spawn_cursor += 1;
            }
            gpu.active_count = gpu.active_count.max((gpu.spawn_cursor.min(FW_MAX_PARTICLES)) as u32);
        }

        // Upload uniforms
        let uniforms = FwUniformsGpu {
            dt: audio.dt,
            wind_x: self.wind_x,
            active_count: gpu.active_count,
            _pad: 0.0,
        };
        queue.write_buffer(&gpu.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        // Dispatch compute
        if gpu.active_count > 0 {
            let mut cpass = encoder.begin_compute_pass(&Default::default());
            cpass.set_pipeline(&gpu.compute_pipeline);
            cpass.set_bind_group(0, &gpu.compute_bind_group, &[]);
            cpass.dispatch_workgroups((gpu.active_count + 63) / 64, 1, 1);
        }
    }

    fn render_gpu<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, scene_bg: &'a wgpu::BindGroup) {
        let gpu = match self.gpu.as_ref() {
            Some(g) if g.active_count > 0 => g,
            _ => return,
        };
        pass.set_pipeline(&gpu.render_pipeline);
        pass.set_bind_group(0, scene_bg, &[]);
        pass.set_bind_group(1, &gpu.render_bind_group, &[]);
        pass.draw(0..6, 0..gpu.active_count);
    }

    fn gpu_active(&self) -> bool { self.gpu.is_some() }
}
