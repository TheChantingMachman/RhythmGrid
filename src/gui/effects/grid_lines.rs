// Grid lines — wireframe with centroid color temperature, presence shimmer,
// thickness pulse, and beat-driven vertex distortion (Geometry Wars style).

use super::{AudioEffect, AudioFrame, RenderContext, RenderPass};
use super::themes::GridParams;
use crate::gui::drawing::{Vertex, rgba_to_f32};
use rhythm_grid::grid::{WIDTH, HEIGHT};

const SEGMENTS_PER_LINE: usize = 10; // subdivision for smooth distortion
const MAX_FORCES: usize = 4;

struct DistortForce {
    x: f32,
    y: f32,
    strength: f32, // decays over time
}

pub struct GridLines {
    beat: f32,
    presence: f32,
    presence_beat: f32,
    centroid: f32,
    forces: Vec<DistortForce>,
    pub distortion_enabled: bool,
    params: GridParams,
}

impl GridLines {
    pub fn new(params: GridParams) -> Self {
        GridLines {
            beat: 0.0, presence: 0.0, presence_beat: 0.0, centroid: 0.0,
            forces: Vec::new(), distortion_enabled: false, params,
        }
    }

    /// Trigger a distortion force at a grid position (called externally on beats/events)
    pub fn add_force(&mut self, x: f32, y: f32, strength: f32) {
        if self.forces.len() >= MAX_FORCES {
            // Replace weakest
            if let Some(weakest) = self.forces.iter_mut().min_by(|a, b| a.strength.partial_cmp(&b.strength).unwrap()) {
                *weakest = DistortForce { x, y, strength };
            }
        } else {
            self.forces.push(DistortForce { x, y, strength });
        }
    }

    fn displacement_at(&self, px: f32, py: f32) -> (f32, f32) {
        let mut dx = 0.0f32;
        let mut dy = 0.0f32;
        for force in &self.forces {
            let fx = px - force.x;
            let fy = py - force.y;
            let dist_sq = fx * fx + fy * fy + 0.5; // +0.5 to avoid singularity
            let inv_dist = 1.0 / dist_sq.sqrt();
            let push = force.strength * inv_dist * 0.2;
            dx += fx * push;
            dy += fy * push;
        }
        (dx.clamp(-0.5, 0.5), dy.clamp(-0.5, 0.5))
    }
}

impl AudioEffect for GridLines {
    fn pass(&self) -> RenderPass {
        RenderPass::Opaque
    }

    fn update(&mut self, audio: &AudioFrame) {
        self.beat = audio.band_beats.iter().cloned().fold(0.0f32, f32::max);
        self.presence = audio.bands_norm[5];
        self.presence_beat = audio.band_beats[5];
        self.centroid = audio.centroid;

        // Spawn gentle forces on bass beats at random grid positions
        if self.distortion_enabled && (audio.band_beats[0] > 0.95 || audio.band_beats[1] > 0.95) {
            // Multiple small ripple points instead of one big center push
            let t = audio.dt * 1000.0; // use dt as cheap pseudo-random seed
            let x1 = ((t * 7.3).sin() * 0.5 + 0.5) * WIDTH as f32;
            let y1 = ((t * 11.7).cos() * 0.5 + 0.5) * HEIGHT as f32;
            let x2 = ((t * 3.1).cos() * 0.5 + 0.5) * WIDTH as f32;
            let y2 = ((t * 5.9).sin() * 0.5 + 0.5) * HEIGHT as f32;
            self.add_force(x1, y1, self.beat * 0.6);
            self.add_force(x2, y2, self.beat * 0.4);
        }

        // Decay forces
        for force in &mut self.forces {
            force.strength *= 0.92; // fast decay
        }
        self.forces.retain(|f| f.strength > 0.01);
    }

    fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, ctx: &RenderContext) {
        let p = &self.params;
        let line_boost = (self.beat * 40.0) as u8;
        let presence_boost = (self.presence * 80.0) as u8;
        let c = self.centroid;
        let lc_r = (p.base_r + (1.0 - c) * 25.0) as u8;
        let lc_g = p.base_g as u8;
        let lc_b = (p.base_b + c * 30.0) as u8;
        let line_color = rgba_to_f32([
            lc_r.saturating_add(line_boost).saturating_add(presence_boost / 3),
            lc_g.saturating_add(line_boost).saturating_add(presence_boost / 2),
            lc_b.saturating_add(line_boost * 2).saturating_add(presence_boost),
            255,
        ]);
        let thickness = p.base_thickness + self.presence_beat * p.beat_thickness_add;
        let gw = ctx.board_width;
        let gh = ctx.board_height;
        let has_forces = !self.forces.is_empty();
        let n = [0.0f32, 0.0, 1.0];

        // Vertical lines — segmented for distortion
        for col in 0..=WIDTH {
            let x = col as f32;
            for seg in 0..SEGMENTS_PER_LINE {
                let t0 = seg as f32 / SEGMENTS_PER_LINE as f32;
                let t1 = (seg + 1) as f32 / SEGMENTS_PER_LINE as f32;
                let y0 = t0 * gh;
                let y1 = t1 * gh;

                let (dx0, dy0) = if has_forces { self.displacement_at(x, y0) } else { (0.0, 0.0) };
                let (dx1, dy1) = if has_forces { self.displacement_at(x, y1) } else { (0.0, 0.0) };

                let base = verts.len() as u32;
                verts.push(Vertex { position: [x - thickness + dx0, -(y0 + dy0), 0.0], normal: n, color: line_color });
                verts.push(Vertex { position: [x + thickness + dx0, -(y0 + dy0), 0.0], normal: n, color: line_color });
                verts.push(Vertex { position: [x + thickness + dx1, -(y1 + dy1), 0.0], normal: n, color: line_color });
                verts.push(Vertex { position: [x - thickness + dx1, -(y1 + dy1), 0.0], normal: n, color: line_color });
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }
        }

        // Horizontal lines — segmented for distortion
        for row in 0..=HEIGHT {
            let y = row as f32;
            for seg in 0..SEGMENTS_PER_LINE {
                let t0 = seg as f32 / SEGMENTS_PER_LINE as f32;
                let t1 = (seg + 1) as f32 / SEGMENTS_PER_LINE as f32;
                let x0 = t0 * gw;
                let x1 = t1 * gw;

                let (dx0, dy0) = if has_forces { self.displacement_at(x0, y) } else { (0.0, 0.0) };
                let (dx1, dy1) = if has_forces { self.displacement_at(x1, y) } else { (0.0, 0.0) };

                let base = verts.len() as u32;
                verts.push(Vertex { position: [x0 + dx0, -(y + dy0) - thickness, 0.0], normal: n, color: line_color });
                verts.push(Vertex { position: [x1 + dx1, -(y + dy1) - thickness, 0.0], normal: n, color: line_color });
                verts.push(Vertex { position: [x1 + dx1, -(y + dy1) + thickness, 0.0], normal: n, color: line_color });
                verts.push(Vertex { position: [x0 + dx0, -(y + dy0) + thickness, 0.0], normal: n, color: line_color });
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }
        }
    }
}
