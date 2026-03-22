// Beat rings — expanding concentric rings spawned on bass/sub-bass beats.

use super::{AudioEffect, AudioFrame, RenderContext, RenderPass};
use super::themes::RingParams;
use crate::gui::drawing::Vertex;

pub struct BgRing {
    pub radius: f32,
    pub max_radius: f32,
    pub life: f32,
    pub max_life: f32,
    pub color: [f32; 4],
}

pub struct BeatRings {
    rings: Vec<BgRing>,
    prev_bass_beat: [bool; 2],
    params: RingParams,
}

impl BeatRings {
    pub fn new(params: RingParams) -> Self {
        BeatRings {
            rings: Vec::new(),
            prev_bass_beat: [false; 2],
            params,
        }
    }
}

impl AudioEffect for BeatRings {
    fn pass(&self) -> RenderPass {
        RenderPass::Transparent
    }

    fn update(&mut self, audio: &AudioFrame) {
        let d = audio.danger;
        let p = &self.params;

        // Spawn rings on bass/sub-bass beats (edge-triggered)
        for band in 0..2 {
            let is_beat = audio.band_beats[band] > 0.95;
            if is_beat && !self.prev_bass_beat[band] {
                let life = p.base_life - d * 1.0;
                self.rings.push(BgRing {
                    radius: 0.5,
                    max_radius: p.max_radius,
                    life,
                    max_life: life,
                    color: [
                        p.color_r + d * 0.5 + if band == 0 { 0.2 } else { 0.0 },
                        p.color_g - d * 0.05,
                        p.color_b - d * 0.3,
                        p.base_alpha + d * 0.15 + audio.bands_norm[band] * 0.2,
                    ],
                });
            }
            self.prev_bass_beat[band] = is_beat;
        }

        // Update rings
        for ring in &mut self.rings {
            let progress = 1.0 - ring.life / ring.max_life;
            ring.radius = 0.5 + progress * ring.max_radius;
            ring.life -= audio.dt;
        }
        self.rings.retain(|r| r.life > 0.0);
    }

    fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, ctx: &RenderContext) {
        let ring_cx = ctx.board_width / 2.0;
        let ring_cy = -ctx.board_height / 2.0;
        let ring_z = -1.0;
        let ring_n = [0.0f32, 0.0, 1.0];
        let ring_segments = 32;

        for ring in &self.rings {
            let progress = 1.0 - ring.life / ring.max_life;
            let alpha = ring.color[3] * (1.0 - progress).powi(2);
            if alpha < 0.005 { continue; }
            let inner_r = ring.radius;
            let outer_r = ring.radius + 0.3 + (1.0 - progress) * 0.5;
            let color_inner = [ring.color[0], ring.color[1], ring.color[2], alpha];
            let color_outer = [ring.color[0] * 0.5, ring.color[1] * 0.5, ring.color[2] * 0.5, 0.0];

            for i in 0..ring_segments {
                let a0 = (i as f32 / ring_segments as f32) * std::f32::consts::TAU;
                let a1 = ((i + 1) as f32 / ring_segments as f32) * std::f32::consts::TAU;
                let (c0, s0) = (a0.cos(), a0.sin());
                let (c1, s1) = (a1.cos(), a1.sin());

                let base = verts.len() as u32;
                verts.push(Vertex { position: [ring_cx + c0 * inner_r, ring_cy + s0 * inner_r, ring_z], normal: ring_n, color: color_inner });
                verts.push(Vertex { position: [ring_cx + c1 * inner_r, ring_cy + s1 * inner_r, ring_z], normal: ring_n, color: color_inner });
                verts.push(Vertex { position: [ring_cx + c1 * outer_r, ring_cy + s1 * outer_r, ring_z], normal: ring_n, color: color_outer });
                verts.push(Vertex { position: [ring_cx + c0 * outer_r, ring_cy + s0 * outer_r, ring_z], normal: ring_n, color: color_outer });
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }
        }
    }
}
