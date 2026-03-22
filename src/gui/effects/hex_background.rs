// Hex background — rotating dot grid + connecting lines.
// Breathes with low-mids, warms with sub-bass, brightens on flux.

use super::{AudioEffect, AudioFrame, RenderContext, RenderPass};
use super::themes::HexParams;
use crate::gui::drawing::Vertex;

pub struct HexBackground {
    time: f32,
    danger: f32,
    sub_bass: f32,
    geo_alpha: f32,
    dot_size: f32,
    params: HexParams,
}

impl HexBackground {
    pub fn new(params: HexParams) -> Self {
        HexBackground {
            time: 0.0,
            danger: 0.0,
            sub_bass: 0.0,
            geo_alpha: params.base_alpha,
            dot_size: params.dot_min_size,
            params,
        }
    }
}

impl AudioEffect for HexBackground {
    fn pass(&self) -> RenderPass {
        RenderPass::Transparent
    }

    fn update(&mut self, audio: &AudioFrame) {
        let p = &self.params;
        self.time += audio.dt * (p.base_speed + audio.danger * p.danger_speed_mult);
        self.danger = audio.danger;
        self.sub_bass = audio.bands_norm[0];
        let low_mids = audio.bands_norm[2];
        let flux_boost = (audio.flux * 0.3).min(0.15);
        self.geo_alpha = p.base_alpha + low_mids * 0.15 + audio.danger * 0.05 + flux_boost;
        self.dot_size = p.dot_min_size + low_mids * (p.dot_max_size - p.dot_min_size);
    }

    fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, ctx: &RenderContext) {
        let geo_cx = ctx.board_width / 2.0;
        let geo_cy = -ctx.board_height / 2.0;
        let geo_z = -2.0;
        let geo_n = [0.0f32, 0.0, 1.0];
        let d = self.danger;
        let p = &self.params;

        let hex_rings = p.hex_rings;
        for ring in 1..=hex_rings {
            let r = ring as f32 * p.ring_spacing;
            let points = ring * 6;
            for i in 0..points {
                let angle = (i as f32 / points as f32) * std::f32::consts::TAU + self.time;
                let dx = angle.cos() * r;
                let dy = angle.sin() * r;
                let dist_factor = 1.0 - (ring as f32 / hex_rings as f32) * 0.5;
                let dot_alpha = self.geo_alpha * dist_factor;
                let dot_color = [
                    p.base_r + d * 0.45 + self.sub_bass * 0.2,
                    p.base_g - d * 0.08,
                    p.base_b - d * 0.35 - self.sub_bass * 0.15,
                    dot_alpha,
                ];

                let base = verts.len() as u32;
                verts.push(Vertex { position: [geo_cx + dx - self.dot_size, geo_cy + dy - self.dot_size, geo_z], normal: geo_n, color: dot_color });
                verts.push(Vertex { position: [geo_cx + dx + self.dot_size, geo_cy + dy - self.dot_size, geo_z], normal: geo_n, color: dot_color });
                verts.push(Vertex { position: [geo_cx + dx + self.dot_size, geo_cy + dy + self.dot_size, geo_z], normal: geo_n, color: dot_color });
                verts.push(Vertex { position: [geo_cx + dx - self.dot_size, geo_cy + dy + self.dot_size, geo_z], normal: geo_n, color: dot_color });
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }
        }

        // Connecting lines
        for ring in 1..=hex_rings {
            let r = ring as f32 * p.ring_spacing;
            let points = ring * 6;
            let line_alpha = self.geo_alpha * 0.4;
            let line_color = [p.base_r * 0.7 + d * 0.35, p.base_g * 0.75 - d * 0.05, p.base_b * 0.7 - d * 0.25, line_alpha];
            let line_w = 0.03;
            for i in 0..points {
                let a0 = (i as f32 / points as f32) * std::f32::consts::TAU + self.time;
                let a1 = ((i + 1) as f32 / points as f32) * std::f32::consts::TAU + self.time;
                let x0 = geo_cx + a0.cos() * r;
                let y0 = geo_cy + a0.sin() * r;
                let x1 = geo_cx + a1.cos() * r;
                let y1 = geo_cy + a1.sin() * r;
                let ddx = x1 - x0;
                let ddy = y1 - y0;
                let len = (ddx * ddx + ddy * ddy).sqrt();
                if len < 0.001 { continue; }
                let nx = -ddy / len * line_w;
                let ny = ddx / len * line_w;

                let base = verts.len() as u32;
                verts.push(Vertex { position: [x0 + nx, y0 + ny, geo_z], normal: geo_n, color: line_color });
                verts.push(Vertex { position: [x1 + nx, y1 + ny, geo_z], normal: geo_n, color: line_color });
                verts.push(Vertex { position: [x1 - nx, y1 - ny, geo_z], normal: geo_n, color: line_color });
                verts.push(Vertex { position: [x0 - nx, y0 - ny, geo_z], normal: geo_n, color: line_color });
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }
        }
    }
}
