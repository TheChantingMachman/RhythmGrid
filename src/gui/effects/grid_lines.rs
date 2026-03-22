// Grid lines — wireframe with centroid color temperature, presence shimmer,
// and thickness pulse on presence beats.

use super::{AudioEffect, AudioFrame, RenderContext, RenderPass};
use super::themes::GridParams;
use crate::gui::drawing::{Vertex, rgba_to_f32, push_grid_line_v, push_grid_line_h};
use rhythm_grid::grid::{WIDTH, HEIGHT};

pub struct GridLines {
    beat: f32,
    presence: f32,
    presence_beat: f32,
    centroid: f32,
    params: GridParams,
}

impl GridLines {
    pub fn new(params: GridParams) -> Self {
        GridLines { beat: 0.0, presence: 0.0, presence_beat: 0.0, centroid: 0.0, params }
    }
}

impl AudioEffect for GridLines {
    fn pass(&self) -> RenderPass {
        RenderPass::Opaque
    }

    fn update(&mut self, audio: &AudioFrame) {
        // Use the max of all band beats as a general "beat" signal
        self.beat = audio.band_beats.iter().cloned().fold(0.0f32, f32::max);
        self.presence = audio.bands_norm[5];
        self.presence_beat = audio.band_beats[5];
        self.centroid = audio.centroid;
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
        for col in 0..=WIDTH {
            push_grid_line_v(verts, indices, col as f32, gh, line_color, thickness);
        }
        for row in 0..=HEIGHT {
            push_grid_line_h(verts, indices, -(row as f32), gw, line_color, thickness);
        }
    }
}
