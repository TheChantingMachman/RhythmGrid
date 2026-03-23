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

struct Burst {
    sparks: Vec<Spark>,
}

pub struct Fireworks {
    bursts: Vec<Burst>,
    rng: u64,
    prev_beat: [bool; 7],
    pub trigger_band: Option<usize>, // None = any band (legacy), Some(n) = only band n
}

impl Fireworks {
    pub fn new() -> Self {
        Fireworks {
            bursts: Vec::new(),
            rng: 0xCAFEBABE42,
            prev_beat: [false; 7],
            trigger_band: None,
        }
    }

    fn rand(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.rng >> 33) as f32) / (u32::MAX as f32 / 2.0)
    }
}

impl AudioEffect for Fireworks {
    fn pass(&self) -> RenderPass {
        RenderPass::Transparent
    }

    fn update(&mut self, audio: &AudioFrame) {
        // Spawn burst on band beat (edge-triggered)
        let band_range: Vec<usize> = match self.trigger_band {
            Some(b) => vec![b],
            None => (0..7).collect(),
        };
        for band in band_range {
            let is_beat = audio.band_beats[band] > 0.95;
            if is_beat && !self.prev_beat[band] {
                // Only spawn for stronger bands — skip if energy is low
                if audio.bands_norm[band] > 0.85 {
                    let cx = self.rand() * 22.0 - 6.0;  // full visible width
                    let cy = -(self.rand().abs() * 24.0); // full visible height
                    let spark_count = 20 + (audio.bands_norm[band] * 20.0) as usize;

                    // Color based on band — low=warm, high=cool
                    let t = band as f32 / 6.0;
                    let base_color = [
                        1.0 - t * 0.6,      // red fades with frequency
                        0.3 + t * 0.5,      // green rises
                        0.2 + t * 0.8,      // blue rises
                        0.9,
                    ];

                    let mut sparks = Vec::with_capacity(spark_count);
                    for _ in 0..spark_count {
                        let angle = self.rand() * std::f32::consts::TAU;
                        let speed = 2.0 + self.rand().abs() * 6.0;
                        let life = 1.5 + self.rand().abs() * 3.0;
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
                    self.bursts.push(Burst { sparks });
                }
            }
            self.prev_beat[band] = is_beat;
        }

        // Update all sparks
        for burst in &mut self.bursts {
            for spark in &mut burst.sparks {
                spark.x += spark.vx * audio.dt;
                spark.y += spark.vy * audio.dt;
                spark.vy += 0.5 * audio.dt; // very slight gravity
                spark.vx *= 0.99; // less drag — preserve horizontal spread
                spark.life -= audio.dt;
            }
            burst.sparks.retain(|s| s.life > 0.0);
        }
        self.bursts.retain(|b| !b.sparks.is_empty());
    }

    fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, _ctx: &RenderContext) {
        let n = [0.0f32, 0.0, 1.0];
        let z = -1.5; // behind board, in front of hex grid

        for burst in &self.bursts {
            for spark in &burst.sparks {
                let alpha = (spark.life / spark.max_life).powf(0.7);
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
        }
    }
}
