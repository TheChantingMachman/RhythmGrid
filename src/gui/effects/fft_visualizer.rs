// FFT visualizer — 7-band spectral display with peak hold and lock toggle.

use super::{AudioEffect, AudioFrame, RenderContext, RenderPass};
use super::themes::FftParams;
use crate::gui::drawing::{Vertex, rgba_to_f32, push_slab_3d};

const FFT_X: f32 = -4.5;
const FFT_Y: f32 = 14.0;
const FFT_MAX_H: f32 = 5.0;
const COL_W: f32 = 0.12;
const COL_GAP: f32 = 0.1;
const FFT_DEPTH: f32 = 0.35;

pub struct FftVisualizer {
    bands: [f32; 7],
    peaks: [f32; 7],
    pub locked: bool,
    pub lock_hovered: bool,
    params: FftParams,
}

impl FftVisualizer {
    pub fn new(params: FftParams) -> Self {
        FftVisualizer {
            bands: [0.0; 7],
            peaks: [0.0; 7],
            locked: false,
            lock_hovered: false,
            params,
        }
    }
}

impl AudioEffect for FftVisualizer {
    fn pass(&self) -> RenderPass {
        RenderPass::Transparent
    }

    fn update(&mut self, audio: &AudioFrame) {
        self.bands = audio.bands;
        self.peaks = audio.peak_bands;
    }

    fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, ctx: &RenderContext) {
        let fft_a = if self.locked { 1.0 } else { ctx.hud_opacity };

        for (i, (val, color)) in self.bands.iter().zip(&self.params.band_colors).enumerate() {
            let color = [color[0], color[1], color[2], (220.0 * fft_a) as u8];
            let bx = FFT_X + i as f32 * (COL_W + COL_GAP);
            let filled_h = (FFT_MAX_H * val).max(0.05);
            let bg_color = rgba_to_f32([12, 12, 25, (120.0 * fft_a) as u8]);
            push_slab_3d(verts, indices, bx, FFT_Y, COL_W, FFT_MAX_H, FFT_DEPTH * 0.3, bg_color);
            let fill_y = FFT_Y + (FFT_MAX_H - filled_h);
            push_slab_3d(verts, indices, bx, fill_y, COL_W, filled_h, FFT_DEPTH, rgba_to_f32(color));
            let peak_h = (FFT_MAX_H * self.peaks[i]).max(0.05);
            let peak_y = FFT_Y + (FFT_MAX_H - peak_h);
            let peak_color = rgba_to_f32([255, 255, 255, (160.0 * fft_a) as u8]);
            push_slab_3d(verts, indices, bx, peak_y, COL_W, 0.1, FFT_DEPTH + 0.1, peak_color);
        }

        // Lock toggle button
        let fft_total_w = 7.0 * COL_W + 6.0 * COL_GAP;
        let lock_color = if self.locked {
            rgba_to_f32([80, 120, 80, (240.0 * ctx.hud_opacity) as u8])
        } else if self.lock_hovered {
            rgba_to_f32([60, 80, 60, (240.0 * ctx.hud_opacity) as u8])
        } else {
            rgba_to_f32([30, 30, 50, (180.0 * ctx.hud_opacity) as u8])
        };
        push_slab_3d(verts, indices, FFT_X, FFT_Y + FFT_MAX_H + 0.2, fft_total_w, 0.3, 0.3, lock_color);
    }
}
